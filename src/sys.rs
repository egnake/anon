//! Low-level POSIX/Linux FFI helpers using only standard libc symbols.
//! Keeps external crate dependencies at zero for offline/reproducible OPSEC builds.

use std::ffi::{CStr, CString};
use std::fs::File;
use std::os::fd::AsRawFd;
use std::process::{Command, Stdio};

pub const STDIN_FILENO: i32 = 0;
pub const STDOUT_FILENO: i32 = 1;

pub const LOCK_EX: i32 = 2;
pub const LOCK_UN: i32 = 8;

pub const PR_SET_NAME: i32 = 15;
pub const TIOCGWINSZ: usize = 0x5413;

pub const POLLIN: i16 = 0x0001;

pub const AF_NETLINK: i32 = 16;
pub const SOCK_RAW: i32 = 3;
pub const SOCK_NONBLOCK: i32 = 0o4000;
pub const SOCK_CLOEXEC: i32 = 0o2000000;
pub const NETLINK_ROUTE: i32 = 0;
pub const RTMGRP_NEIGH: u32 = 4;
pub const RTMGRP_IPV6_ROUTE: u32 = 0x400;

pub const IN_NONBLOCK: i32 = 0o4000;
pub const IN_CLOEXEC: i32 = 0o2000000;
pub const IN_MODIFY: u32 = 0x0000_0002;
pub const IN_ATTRIB: u32 = 0x0000_0004;
pub const IN_CLOSE_WRITE: u32 = 0x0000_0008;
pub const IN_MOVED_TO: u32 = 0x0000_0080;
pub const IN_CREATE: u32 = 0x0000_0100;
pub const IN_DELETE_SELF: u32 = 0x0000_0400;
pub const IN_MOVE_SELF: u32 = 0x0000_0800;

pub const SIGTERM: i32 = 15;
pub const SIGKILL: i32 = 9;
pub const SIGHUP: i32 = 1;

#[repr(C)]
#[derive(Default)]
struct WinSize {
    ws_row: u16,
    ws_col: u16,
    ws_xpixel: u16,
    ws_ypixel: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PollFd {
    pub fd: i32,
    pub events: i16,
    pub revents: i16,
}

#[repr(C)]
struct SockAddrNl {
    nl_family: u16,
    nl_pad: u16,
    nl_pid: u32,
    nl_groups: u32,
}

extern "C" {
    fn geteuid() -> u32;
    fn getpid() -> i32;
    fn kill(pid: i32, sig: i32) -> i32;
    fn isatty(fd: i32) -> i32;
    fn flock(fd: i32, operation: i32) -> i32;
    fn prctl(option: i32, arg2: usize, arg3: usize, arg4: usize, arg5: usize) -> i32;
    fn ioctl(fd: i32, request: usize, ...) -> i32;
    fn poll(fds: *mut PollFd, nfds: usize, timeout: i32) -> i32;
    fn socket(domain: i32, ty: i32, protocol: i32) -> i32;
    fn bind(sockfd: i32, addr: *const SockAddrNl, addrlen: u32) -> i32;
    fn recv(sockfd: i32, buf: *mut core::ffi::c_void, len: usize, flags: i32) -> isize;
    fn read(fd: i32, buf: *mut core::ffi::c_void, count: usize) -> isize;
    fn close(fd: i32) -> i32;
    fn inotify_init1(flags: i32) -> i32;
    fn inotify_add_watch(fd: i32, pathname: *const core::ffi::c_char, mask: u32) -> i32;
    fn mlock(addr: *const core::ffi::c_void, len: usize) -> i32;
    fn munlock(addr: *const core::ffi::c_void, len: usize) -> i32;
    fn sync();
}

pub fn effective_uid() -> u32 {
    unsafe { geteuid() }
}

pub fn current_pid() -> i32 {
    unsafe { getpid() }
}

pub fn is_stdout_tty() -> bool {
    unsafe { isatty(STDOUT_FILENO) == 1 }
}

pub fn is_stdin_tty() -> bool {
    unsafe { isatty(STDIN_FILENO) == 1 }
}

pub fn terminal_size() -> (usize, usize) {
    let mut ws = WinSize::default();
    let rc = unsafe { ioctl(STDOUT_FILENO, TIOCGWINSZ, &mut ws as *mut WinSize) };
    let cols = if rc == 0 && ws.ws_col >= 20 {
        ws.ws_col as usize
    } else {
        std::env::var("COLUMNS")
            .ok()
            .and_then(|c| c.trim().parse::<usize>().ok())
            .filter(|&n| n >= 20)
            .unwrap_or(80)
    };
    let rows = if rc == 0 && ws.ws_row >= 10 {
        ws.ws_row as usize
    } else {
        std::env::var("LINES")
            .ok()
            .and_then(|r| r.trim().parse::<usize>().ok())
            .filter(|&n| n >= 10)
            .unwrap_or(24)
    };
    (cols, rows)
}

pub fn terminal_cols() -> usize {
    terminal_size().0
}

pub fn terminal_rows() -> usize {
    terminal_size().1
}

pub fn send_signal(pid: i32, sig: i32) -> bool {
    if pid <= 0 {
        return false;
    }
    unsafe { kill(pid, sig) == 0 }
}

pub fn is_pid_alive(pid: i32) -> bool {
    if pid <= 0 {
        return false;
    }
    unsafe { kill(pid, 0) == 0 }
}

pub fn set_process_name(name: &str) {
    let mut buf = [0u8; 16];
    let bytes = name.as_bytes();
    let len = bytes.len().min(15);
    buf[..len].copy_from_slice(&bytes[..len]);
    if let Ok(c) = CStr::from_bytes_until_nul(&buf) {
        unsafe {
            prctl(PR_SET_NAME, c.as_ptr() as usize, 0, 0, 0);
        }
    }
}

pub fn sync_filesystems() {
    unsafe {
        sync();
    }
}

pub struct FileLock {
    file: File,
}

impl FileLock {
    pub fn acquire(file: File) -> Option<Self> {
        let rc = unsafe { flock(file.as_raw_fd(), LOCK_EX) };
        if rc == 0 {
            Some(Self { file })
        } else {
            None
        }
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        unsafe {
            flock(self.file.as_raw_fd(), LOCK_UN);
        }
    }
}

/// RAII raw-ish terminal mode guard for TUI single-keypress input while keeping ISIG (Ctrl+C).
pub struct TuiTermGuard {
    saved_stty: Option<String>,
}

impl TuiTermGuard {
    pub fn enter() -> Self {
        let saved = if is_stdin_tty() {
            Command::new("stty")
                .arg("-g")
                .stdin(Stdio::inherit())
                .stderr(Stdio::null())
                .output()
                .ok()
                .filter(|o| o.status.success())
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        } else {
            None
        };

        if saved.is_some() {
            let _ = Command::new("stty")
                .args(["-icanon", "-echo", "min", "1", "time", "0"])
                .stdin(Stdio::inherit())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
        Self { saved_stty: saved }
    }
}

impl Drop for TuiTermGuard {
    fn drop(&mut self) {
        if let Some(ref saved) = self.saved_stty {
            let _ = Command::new("stty")
                .arg(saved)
                .stdin(Stdio::inherit())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
        if is_stdout_tty() {
            print!("\x1b[?25h");
            let _ = std::io::Write::flush(&mut std::io::stdout());
        }
    }
}

/// Wait on a file descriptor with timeout in milliseconds; returns true if readable.
pub fn poll_fd(fd: i32, timeout_ms: i32) -> bool {
    let mut pfd = PollFd {
        fd,
        events: POLLIN,
        revents: 0,
    };
    let rc = unsafe { poll(&mut pfd as *mut PollFd, 1, timeout_ms) };
    rc > 0 && (pfd.revents & POLLIN) != 0
}

pub fn read_fd_byte(fd: i32) -> Option<u8> {
    let mut b = 0u8;
    let n = unsafe { read(fd, (&mut b as *mut u8).cast::<core::ffi::c_void>(), 1) };
    if n == 1 {
        Some(b)
    } else {
        None
    }
}

/// Event-driven Netlink route socket for neighbor (ARP/NDP) and IPv6 route updates.
pub struct NetlinkWatcher {
    fd: i32,
}

impl NetlinkWatcher {
    pub fn new(groups: u32) -> Option<Self> {
        let fd = unsafe {
            socket(
                AF_NETLINK,
                SOCK_RAW | SOCK_NONBLOCK | SOCK_CLOEXEC,
                NETLINK_ROUTE,
            )
        };
        if fd < 0 {
            return None;
        }
        let addr = SockAddrNl {
            nl_family: AF_NETLINK as u16,
            nl_pad: 0,
            nl_pid: 0,
            nl_groups: groups,
        };
        let rc = unsafe {
            bind(
                fd,
                &addr as *const SockAddrNl,
                core::mem::size_of::<SockAddrNl>() as u32,
            )
        };
        if rc != 0 {
            unsafe {
                close(fd);
            }
            return None;
        }
        Some(Self { fd })
    }

    /// Blocks up to `timeout_ms` for any Netlink event and drains queued datagrams.
    pub fn wait_event(&self, timeout_ms: i32) -> bool {
        if !poll_fd(self.fd, timeout_ms) {
            return false;
        }
        let mut buf = [0u8; 4096];
        loop {
            let n = unsafe { recv(self.fd, buf.as_mut_ptr().cast::<core::ffi::c_void>(), buf.len(), 0) };
            if n <= 0 {
                break;
            }
        }
        true
    }
}

impl Drop for NetlinkWatcher {
    fn drop(&mut self) {
        if self.fd >= 0 {
            unsafe {
                close(self.fd);
            }
        }
    }
}

/// Event-driven inotify file watcher for instant leak detection (e.g. /etc/resolv.conf).
pub struct InotifyWatcher {
    fd: i32,
}

impl InotifyWatcher {
    pub fn new() -> Option<Self> {
        let fd = unsafe { inotify_init1(IN_NONBLOCK | IN_CLOEXEC) };
        if fd >= 0 {
            Some(Self { fd })
        } else {
            None
        }
    }

    pub fn watch_path(&self, path: &str) -> bool {
        if let Ok(c_path) = CString::new(path) {
            let mask = IN_MODIFY
                | IN_ATTRIB
                | IN_CLOSE_WRITE
                | IN_MOVED_TO
                | IN_CREATE
                | IN_DELETE_SELF
                | IN_MOVE_SELF;
            unsafe { inotify_add_watch(self.fd, c_path.as_ptr(), mask) >= 0 }
        } else {
            false
        }
    }

    pub fn wait_event(&self, timeout_ms: i32) -> bool {
        if !poll_fd(self.fd, timeout_ms) {
            return false;
        }
        let mut buf = [0u8; 1024];
        loop {
            let n = unsafe { read(self.fd, buf.as_mut_ptr().cast::<core::ffi::c_void>(), buf.len()) };
            if n <= 0 {
                break;
            }
        }
        true
    }
}

impl Drop for InotifyWatcher {
    fn drop(&mut self) {
        if self.fd >= 0 {
            unsafe {
                close(self.fd);
            }
        }
    }
}

fn available_ram_bytes() -> Option<usize> {
    let meminfo = std::fs::read_to_string("/proc/meminfo").ok()?;
    for line in meminfo.lines() {
        if let Some(rest) = line.strip_prefix("MemAvailable:") {
            let kb: usize = rest.split_whitespace().next()?.parse().ok()?;
            return kb.checked_mul(1024);
        }
    }
    None
}

/// Fast native RAM wipe of bounded free memory chunks using mlock + volatile writes.
/// Complements or replaces external `sdmem` safely without triggering OOM killer.
pub fn wipe_free_ram_native(max_mb: usize) {
    let chunk_size: usize = 16 * 1024 * 1024; // 16 MiB chunks
    let requested = max_mb.saturating_mul(1024 * 1024);
    let safe_cap = available_ram_bytes()
        .map(|avail| {
            let half = avail / 2;
            if half < chunk_size {
                0
            } else {
                half
            }
        })
        .unwrap_or(requested);
    let target_bytes = requested.min(safe_cap);
    let words_per_chunk = chunk_size / core::mem::size_of::<u64>();
    let mut wiped = 0usize;

    while wiped + chunk_size <= target_bytes {
        let mut buf = vec![0u64; words_per_chunk];
        let byte_len = buf.len() * core::mem::size_of::<u64>();
        let locked = unsafe { mlock(buf.as_ptr().cast::<core::ffi::c_void>(), byte_len) == 0 };
        for word in buf.iter_mut() {
            unsafe {
                core::ptr::write_volatile(word, 0xAAAA_AAAA_AAAA_AAAA);
                core::ptr::write_volatile(word, 0);
            }
        }
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
        if locked {
            unsafe {
                munlock(buf.as_ptr().cast::<core::ffi::c_void>(), byte_len);
            }
        }
        wiped += chunk_size;
    }
}

