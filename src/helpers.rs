use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::TcpStream;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::config::AppConfig;
use crate::ui::{info, warn};

static RNG_COUNTER: AtomicU64 = AtomicU64::new(0x9E37_79B9_7F4A_7C15);

pub fn rand_u64() -> u64 {
    if let Ok(mut f) = File::open("/dev/urandom") {
        let mut buf = [0u8; 8];
        if f.read_exact(&mut buf).is_ok() {
            return u64::from_le_bytes(buf);
        }
    }
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x1234_5678);
    let cnt = RNG_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut x = nanos ^ cnt.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

pub fn cmd_exists(cmd: &str) -> bool {
    if cmd.contains('/') {
        return is_executable(Path::new(cmd));
    }
    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            let full = dir.join(cmd);
            if is_executable(&full) {
                return true;
            }
        }
    }
    false
}

pub fn find_in_path(cmd: &str) -> Option<PathBuf> {
    if cmd.contains('/') {
        let p = PathBuf::from(cmd);
        return if is_executable(&p) { Some(p) } else { None };
    }
    let paths = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&paths) {
        let full = dir.join(cmd);
        if is_executable(&full) {
            return Some(full);
        }
    }
    None
}

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path)
        .map(|m| m.is_file() && (m.permissions().mode() & 0o111 != 0))
        .unwrap_or(false)
}

pub fn run_quiet(cmd: &str, args: &[&str]) -> bool {
    Command::new(cmd)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn run_capture(cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd)
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        None
    }
}

pub fn run_stdin(cmd: &str, args: &[&str], input: &str) -> bool {
    let mut child = match Command::new(cmd)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(input.as_bytes());
    }
    child.wait().map(|s| s.success()).unwrap_or(false)
}

pub fn is_wsl() -> bool {
    fs::read_to_string("/proc/version")
        .map(|s| s.to_ascii_lowercase().contains("microsoft"))
        .unwrap_or(false)
}

pub fn service_ctrl(action: &str, svc: &str, torrc: Option<&Path>) -> bool {
    let base_svc = svc.strip_suffix(".service").unwrap_or(svc);

    if cmd_exists("systemctl") {
        let ok = if svc.is_empty() {
            run_quiet("systemctl", &[action])
        } else {
            run_quiet("systemctl", &[action, svc])
        };
        if ok {
            return true;
        }
        if Path::new("/run/systemd/system").is_dir() {
            return false;
        }
    }

    if cmd_exists("service") {
        match action {
            "is-active" => {
                if run_quiet("service", &[base_svc, "status"]) || run_quiet("pgrep", &["-x", base_svc]) {
                    return true;
                }
                return false;
            }
            "unmask" | "daemon-reload" => return true,
            _ => {
                if run_quiet("service", &[base_svc, action]) {
                    return true;
                }
            }
        }
    }

    match (base_svc, action) {
        ("tor", "start") | ("tor", "restart") => {
            if cmd_exists("tor") {
                let _ = run_quiet("pkill", &["-x", "tor"]);
                thread::sleep(Duration::from_secs(1));
                let torrc_s = torrc
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "/etc/tor/torrc".to_string());
                return run_quiet("tor", &["-f", &torrc_s, "--RunAsDaemon", "1"]);
            }
        }
        ("tor", "stop") => return run_quiet("pkill", &["-x", "tor"]),
        ("tor", "is-active") => return run_quiet("pgrep", &["-x", "tor"]),
        ("i2pd", "start") | ("i2pd", "restart") => {
            if cmd_exists("i2pd") {
                let _ = run_quiet("pkill", &["-x", "i2pd"]);
                return run_quiet("i2pd", &["--daemon"]);
            }
        }
        ("i2pd", "stop") => return run_quiet("pkill", &["-x", "i2pd"]),
        ("i2pd", "is-active") => return run_quiet("pgrep", &["-x", "i2pd"]),
        ("dnscrypt-proxy", "start") | ("dnscrypt-proxy", "restart") => {
            if cmd_exists("dnscrypt-proxy") {
                let _ = run_quiet("pkill", &["-x", "dnscrypt-proxy"]);
                let cfg_path = "/etc/dnscrypt-proxy/dnscrypt-proxy.toml";
                if Path::new(cfg_path).is_file() {
                    return Command::new("dnscrypt-proxy")
                        .args(["-config", cfg_path])
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn()
                        .is_ok();
                }
            }
        }
        ("dnscrypt-proxy", "stop") => return run_quiet("pkill", &["-x", "dnscrypt-proxy"]),
        ("dnscrypt-proxy", "is-active") => return run_quiet("pgrep", &["-x", "dnscrypt-proxy"]),
        _ => {}
    }
    false
}

pub fn lock_nm_dns(cfg: &AppConfig) {
    let nm_dir = Path::new("/etc/NetworkManager/conf.d");
    if nm_dir.is_dir() {
        let _ = fs::write(nm_dir.join("anon-dns-lock.conf"), "[main]\ndns=none\n");
    }

    let nsswitch = Path::new("/etc/nsswitch.conf");
    let bak = cfg.backup_dir.join("nsswitch.conf.bak");
    if nsswitch.is_file() && !bak.exists() {
        if let Ok(content) = fs::read_to_string(nsswitch) {
            let mut has_resolve = false;
            let mut updated = String::with_capacity(content.len());
            for line in content.lines() {
                if line.starts_with("hosts:") && line.split_whitespace().any(|w| w == "resolve") {
                    has_resolve = true;
                    let filtered = remove_nss_resolve(line);
                    updated.push_str(&filtered);
                } else {
                    updated.push_str(line);
                }
                updated.push('\n');
            }
            if has_resolve {
                let _ = fs::write(&bak, &content);
                let _ = fs::write(nsswitch, updated);
            }
        }
    }
}

fn remove_nss_resolve(line: &str) -> String {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    let mut out: Vec<&str> = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        if tokens[i] == "resolve" {
            if i + 1 < tokens.len() && tokens[i + 1].starts_with('[') && tokens[i + 1].ends_with(']')
            {
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        out.push(tokens[i]);
        i += 1;
    }
    out.join(" ")
}

pub fn unlock_nm_dns(cfg: &AppConfig) {
    let lock_conf = Path::new("/etc/NetworkManager/conf.d/anon-dns-lock.conf");
    if lock_conf.exists() {
        let _ = fs::remove_file(lock_conf);
        let _ = service_ctrl("reload", "NetworkManager", None);
    }
    let bak = cfg.backup_dir.join("nsswitch.conf.bak");
    if bak.is_file() {
        if let Ok(data) = fs::read(&bak) {
            let _ = fs::write("/etc/nsswitch.conf", data);
        }
        let _ = fs::remove_file(bak);
    }
}

pub fn backup_resolv_conf(cfg: &AppConfig, tag: &str) {
    let resolv = &cfg.resolv_conf;
    let sym_bak = cfg.backup_dir.join(format!("resolv.{tag}.symlink.bak"));
    let file_bak = cfg.backup_dir.join(format!("resolv.{tag}.bak"));
    let _ = run_quiet("chattr", &["-i", &resolv.to_string_lossy()]);

    if let Ok(meta) = fs::symlink_metadata(resolv) {
        if meta.file_type().is_symlink() {
            if let Ok(target) = fs::read_link(resolv) {
                let _ = fs::remove_file(&sym_bak);
                let _ = symlink(target, &sym_bak);
            }
            if let Ok(data) = fs::read(resolv) {
                let _ = fs::write(&file_bak, data);
            }
            let _ = fs::remove_file(resolv);
        } else if meta.is_file() {
            if let Ok(data) = fs::read(resolv) {
                let _ = fs::write(&file_bak, data);
            }
        }
    }
}

pub fn restore_resolv_conf(cfg: &AppConfig, tag: &str) {
    let resolv = &cfg.resolv_conf;
    let sym_bak = cfg.backup_dir.join(format!("resolv.{tag}.symlink.bak"));
    let file_bak = cfg.backup_dir.join(format!("resolv.{tag}.bak"));
    let _ = run_quiet("chattr", &["-i", &resolv.to_string_lossy()]);

    if let Ok(meta) = fs::symlink_metadata(&sym_bak) {
        let _ = fs::remove_file(resolv);
        if meta.file_type().is_symlink() {
            if let Ok(target) = fs::read_link(&sym_bak) {
                let _ = symlink(target, resolv);
            }
            let _ = fs::remove_file(&sym_bak);
        } else {
            let _ = fs::rename(&sym_bak, resolv);
        }
    } else if file_bak.is_file() {
        if let Ok(data) = fs::read(&file_bak) {
            let _ = fs::write(resolv, data);
        }
    }
    let _ = fs::remove_file(&file_bak);
    let _ = fs::remove_file(&sym_bak);
}

pub fn get_default_iface() -> Option<String> {
    let out = run_capture("ip", &["route"])?;
    for line in out.lines() {
        if line.starts_with("default") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(iface) = parts.get(4) {
                return Some(iface.split('@').next().unwrap_or(iface).to_string());
            }
            if let Some(pos) = parts.iter().position(|&w| w == "dev") {
                if let Some(iface) = parts.get(pos + 1) {
                    return Some(iface.split('@').next().unwrap_or(iface).to_string());
                }
            }
        }
    }
    None
}

pub fn get_gateway_ip() -> Option<String> {
    let out = run_capture("ip", &["route"])?;
    for line in out.lines() {
        if line.starts_with("default") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(pos) = parts.iter().position(|&w| w == "via") {
                if let Some(gw) = parts.get(pos + 1) {
                    return Some((*gw).to_string());
                }
            }
            if let Some(gw) = parts.get(2) {
                return Some((*gw).to_string());
            }
        }
    }
    None
}

pub fn get_gateway_v6() -> Option<String> {
    let out = run_capture("ip", &["-6", "route", "show", "default"])?;
    for line in out.lines() {
        if line.starts_with("default") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(pos) = parts.iter().position(|&w| w == "via") {
                if let Some(gw) = parts.get(pos + 1) {
                    return Some((*gw).to_string());
                }
            }
        }
    }
    None
}

pub fn list_interfaces() -> Vec<String> {
    let mut ifaces = Vec::new();
    if let Some(out) = run_capture("ip", &["-o", "link", "show"]) {
        for line in out.lines() {
            if let Some((_, rest)) = line.split_once(": ") {
                if let Some(name) = rest.split(':').next() {
                    let clean = name.split('@').next().unwrap_or(name).trim();
                    if !clean.is_empty() {
                        ifaces.push(clean.to_string());
                    }
                }
            }
        }
    }
    if ifaces.is_empty() {
        if let Ok(entries) = fs::read_dir("/sys/class/net") {
            for e in entries.flatten() {
                if let Some(name) = e.file_name().to_str() {
                    ifaces.push(name.to_string());
                }
            }
        }
    }
    ifaces
}

pub fn is_virtual_iface(ifname: &str) -> bool {
    ifname == "lo"
        || ifname.starts_with("veth")
        || ifname.starts_with("docker")
        || ifname.starts_with("br-")
        || ifname.starts_with("virbr")
        || ifname.starts_with("tun")
        || ifname.starts_with("tap")
        || ifname.starts_with("wg")
        || ifname.starts_with("anon")
}

pub fn wait_for_network(max_wait_secs: u64) -> bool {
    for _ in 0..max_wait_secs {
        if get_default_iface().is_some() {
            return true;
        }
        thread::sleep(Duration::from_secs(1));
    }
    false
}

pub fn resolve_bridges_file(cfg: &AppConfig) -> Option<PathBuf> {
    let custom = cfg.src_dir.join("sources/bridges.txt");
    if is_non_empty_file(&custom) {
        return Some(custom);
    }
    let default = cfg.src_dir.join("sources/bridges.default.txt");
    if is_non_empty_file(&default) {
        return Some(default);
    }
    None
}

pub fn is_non_empty_file(p: &Path) -> bool {
    fs::metadata(p).map(|m| m.is_file() && m.len() > 0).unwrap_or(false)
}

pub fn resolve_uid_any(candidates: &[&str]) -> Option<u32> {
    if let Ok(passwd) = fs::read_to_string("/etc/passwd") {
        for name in candidates {
            for line in passwd.lines() {
                let mut cols = line.split(':');
                if cols.next() == Some(*name) {
                    let _ = cols.next();
                    if let Some(uid) = cols.next().and_then(|u| u.parse::<u32>().ok()) {
                        return Some(uid);
                    }
                }
            }
        }
    }
    for name in candidates {
        if let Some(out) = run_capture("id", &["-u", name]) {
            if let Ok(uid) = out.trim().parse::<u32>() {
                return Some(uid);
            }
        }
    }
    None
}

pub fn resolve_username_any(candidates: &[&str]) -> Option<String> {
    for name in candidates {
        if resolve_uid_any(&[name]).is_some() {
            return Some((*name).to_string());
        }
    }
    None
}

pub fn resolve_user_home(user: &str) -> String {
    if let Ok(passwd) = fs::read_to_string("/etc/passwd") {
        for line in passwd.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.first() == Some(&user) && parts.len() >= 6 && !parts[5].is_empty() {
                return parts[5].to_string();
            }
        }
    }
    if let Some(out) = run_capture("getent", &["passwd", user]) {
        let parts: Vec<&str> = out.trim().split(':').collect();
        if parts.len() >= 6 && !parts[5].is_empty() {
            return parts[5].to_string();
        }
    }
    if user == "root" {
        "/root".to_string()
    } else {
        format!("/home/{user}")
    }
}

pub fn read_tor_control_cookie_hex() -> Option<String> {
    for path in ["/run/tor/control.authcookie", "/var/lib/tor/control_auth_cookie"] {
        if let Ok(bytes) = fs::read(path) {
            if !bytes.is_empty() {
                let mut hex = String::with_capacity(bytes.len() * 2);
                for b in bytes {
                    use std::fmt::Write as _;
                    let _ = write!(hex, "{b:02x}");
                }
                return Some(hex);
            }
        }
    }
    None
}

fn tor_control_cmd(cmd: &str) -> Option<String> {
    let auth = match read_tor_control_cookie_hex() {
        Some(hex) => hex,
        None => "\"\"".to_string(),
    };
    let addr = "127.0.0.1:9051".parse().ok()?;
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(3)).ok()?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(4)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(3)));

    let payload = format!("AUTHENTICATE {auth}\r\n{cmd}\r\nQUIT\r\n");
    stream.write_all(payload.as_bytes()).ok()?;
    let mut resp = String::new();
    let _ = stream.read_to_string(&mut resp);
    Some(resp)
}

pub fn tor_bootstrap_progress() -> Option<u32> {
    let resp = tor_control_cmd("GETINFO status/bootstrap-phase")?;
    let idx = resp.find("PROGRESS=")?;
    let rest = &resp[idx + "PROGRESS=".len()..];
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse::<u32>().ok()
}

pub fn tor_signal_newnym() -> bool {
    if let Some(resp) = tor_control_cmd("SIGNAL NEWNYM") {
        let ok_count = resp.matches("250 OK").count();
        if ok_count >= 2 {
            return true;
        }
    }
    run_quiet("pkill", &["-HUP", "-x", "tor"]) || run_quiet("killall", &["-HUP", "tor"])
}

pub fn is_tor_running() -> bool {
    service_ctrl("is-active", "tor.service", None)
        || run_quiet("pgrep", &["-x", "tor"])
        || run_quiet("pgrep", &["-x", "_tor"])
}

pub fn is_netns_active(name: &str) -> bool {
    if let Some(out) = run_capture("ip", &["netns", "list"]) {
        return out
            .lines()
            .any(|l| l.split_whitespace().next() == Some(name));
    }
    false
}

pub fn is_mountpoint(path: &str) -> bool {
    run_quiet("mountpoint", &["-q", path])
}

pub fn sysctl_read(key: &str) -> Option<String> {
    if let Some(out) = run_capture("sysctl", &["-n", key]) {
        let trimmed = out.trim().to_string();
        if !trimmed.is_empty() {
            return Some(trimmed);
        }
    }
    let proc_path = format!("/proc/sys/{}", key.replace('.', "/"));
    fs::read_to_string(proc_path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn sysctl_write(key: &str, val: &str) -> bool {
    let arg = format!("{key}={val}");
    if run_quiet("sysctl", &["-w", &arg]) {
        return true;
    }
    let proc_path = format!("/proc/sys/{}", key.replace('.', "/"));
    fs::write(proc_path, format!("{val}\n")).is_ok()
}

/// Fast native secure file shred + optional external `shred` compatibility when present in mock/system.
pub fn shred_file(path: &Path) {
    if !path.is_file() {
        return;
    }
    if cmd_exists("shred") {
        if run_quiet("shred", &["-fzu", "--", &path.to_string_lossy()]) {
            return;
        }
    }
    if let Ok(meta) = fs::metadata(path) {
        let len = meta.len() as usize;
        if let Ok(mut f) = OpenOptions::new().write(true).open(path) {
            if len > 0 {
                let mut buf = vec![0u8; len.min(65536)];
                // Pass 1: pseudo-random
                for chunk in buf.chunks_mut(8) {
                    let r = rand_u64().to_le_bytes();
                    let n = chunk.len().min(8);
                    chunk[..n].copy_from_slice(&r[..n]);
                }
                let mut rem = len;
                let _ = f.seek(SeekFrom::Start(0));
                while rem > 0 {
                    let n = rem.min(buf.len());
                    if f.write_all(&buf[..n]).is_err() {
                        break;
                    }
                    rem -= n;
                }
                let _ = f.sync_data();
                // Pass 2: zeros
                buf.fill(0);
                let mut rem = len;
                let _ = f.seek(SeekFrom::Start(0));
                while rem > 0 {
                    let n = rem.min(buf.len());
                    if f.write_all(&buf[..n]).is_err() {
                        break;
                    }
                    rem -= n;
                }
                let _ = f.sync_data();
            }
        }
    }
    let _ = fs::remove_file(path);
}

pub fn shred_dir_recursive(dir: &Path) {
    if !dir.is_dir() {
        return;
    }
    let mut files = Vec::new();
    collect_files_recursive(dir, &mut files);
    for f in files {
        shred_file(&f);
    }
    let _ = fs::remove_dir_all(dir);
}

pub fn collect_files_recursive(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if let Ok(meta) = fs::symlink_metadata(&p) {
                if meta.file_type().is_symlink() {
                    continue;
                }
                if meta.is_dir() {
                    collect_files_recursive(&p, out);
                } else if meta.is_file() {
                    out.push(p);
                }
            }
        }
    }
}

pub fn json_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 16);
    for ch in input.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                use std::fmt::Write as _;
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

pub fn extract_json_string_field(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{key}\"");
    let idx = json.find(&pattern)?;
    let rest = &json[idx + pattern.len()..];
    let colon = rest.find(':')?;
    let after_colon = rest[colon + 1..].trim_start();
    if !after_colon.starts_with('"') {
        return None;
    }
    let mut out = String::new();
    let mut escaped = false;
    for ch in after_colon[1..].chars() {
        if escaped {
            out.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return Some(out);
        } else {
            out.push(ch);
        }
    }
    None
}

pub fn map_pkg_name<'a>(cmd: &'a str, mgr: &str) -> &'a str {
    match (mgr, cmd) {
        ("apt", "sdmem") => "secure-delete",
        ("apt", "killall") => "psmisc",
        ("apt", "notify-send") => "libnotify-bin",
        ("apt", "apparmor_parser") => "apparmor-utils",
        ("apt", "ip") => "iproute2",
        ("apt", "ping") => "iputils-ping",
        ("apt", "exiftool") => "libimage-exiftool-perl",
        ("dnf" | "pacman" | "apk", "sdmem") => "srm",
        ("dnf" | "pacman" | "apk", "notify-send") => "libnotify",
        ("dnf" | "pacman" | "apk", "killall") => "psmisc",
        ("dnf", "exiftool") => "perl-Image-ExifTool",
        ("pacman" | "apk", "exiftool") => "exiftool",
        _ => cmd,
    }
}

pub fn dep_check(cmds: &[&str]) {
    let mgr = if cmd_exists("apt-get") {
        "apt"
    } else if cmd_exists("dnf") {
        "dnf"
    } else if cmd_exists("pacman") {
        "pacman"
    } else if cmd_exists("apk") {
        "apk"
    } else {
        "apt"
    };

    let mut missing = Vec::new();
    let mut pkgs = Vec::new();
    for &cmd in cmds {
        if !cmd_exists(cmd) {
            warn(&format!("{cmd}: could not be found."));
            missing.push(cmd);
            pkgs.push(map_pkg_name(cmd, mgr));
        }
    }

    if missing.is_empty() {
        return;
    }

    println!(
        "\n {}ℹ INFO:{} Some required dependencies are missing: {}{}{}",
        crate::ui::CYAN,
        crate::ui::RESET,
        crate::ui::BOLD,
        missing.join(" "),
        crate::ui::RESET
    );

    let mut choice = String::from("Y");
    if crate::sys::is_stdin_tty() {
        print!(
            " {}➤ Do you want Anon to automatically install them for you? (Y/n): {}",
            crate::ui::YELLOW,
            crate::ui::RESET
        );
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        if std::io::stdin().read_line(&mut line).is_ok() {
            choice = line.trim().to_string();
        }
    }

    match choice.as_str() {
        "y" | "Y" | "" => {
            println!(
                "\n {}ℹ INFO:{} Installing missing packages...",
                crate::ui::CYAN,
                crate::ui::RESET
            );
            let ok = match mgr {
                "apt" => {
                    if !cmd_exists("apt-get") {
                        crate::ui::err(&format!(
                            "Package manager unavailable. Please install manually: {}",
                            pkgs.join(" ")
                        ));
                    }
                    let mut args = vec!["install", "-y"];
                    args.extend(pkgs.iter().copied());
                    run_quiet("apt-get", &["update"])
                        && Command::new("apt-get")
                            .env("DEBIAN_FRONTEND", "noninteractive")
                            .args(&args)
                            .status()
                            .map(|s| s.success())
                            .unwrap_or(false)
                }
                "dnf" => {
                    let mut args = vec!["install", "-y"];
                    args.extend(pkgs.iter().copied());
                    Command::new("dnf")
                        .args(&args)
                        .status()
                        .map(|s| s.success())
                        .unwrap_or(false)
                }
                "pacman" => {
                    let mut args = vec!["-Sy", "--noconfirm"];
                    args.extend(pkgs.iter().copied());
                    Command::new("pacman")
                        .args(&args)
                        .status()
                        .map(|s| s.success())
                        .unwrap_or(false)
                }
                "apk" => {
                    let mut args = vec!["add", "--no-cache"];
                    args.extend(pkgs.iter().copied());
                    Command::new("apk")
                        .args(&args)
                        .status()
                        .map(|s| s.success())
                        .unwrap_or(false)
                }
                _ => false,
            };
            if !ok {
                info("Package installation command finished with warnings.");
            }
            for &cmd in cmds {
                if !cmd_exists(cmd) {
                    crate::ui::err(&format!(
                        "Failed to install {cmd}. Please install it manually."
                    ));
                }
            }
            crate::ui::msg("All dependencies installed successfully!\n");
        }
        _ => crate::ui::err("Dependencies are required to run Anon. Exiting."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_helpers_work() {
        let sample = r#"{"IsTor":true,"IP":"198.51.100.7"}"#;
        assert_eq!(extract_json_string_field(sample, "IP").as_deref(), Some("198.51.100.7"));
        assert_eq!(json_escape("a\"b\\c\nd"), "a\\\"b\\\\c\\nd");
    }

    #[test]
    fn nsswitch_resolve_removal() {
        let line = "hosts:          files mdns4_minimal [NOTFOUND=return] resolve [!UNAVAIL=return] dns myhostname";
        let cleaned = remove_nss_resolve(line);
        assert!(!cleaned.contains("resolve"));
        assert!(cleaned.contains("files"));
        assert!(cleaned.contains("dns"));
    }
}
