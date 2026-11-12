use std::env;
use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::thread;

use crate::config::AppConfig;
use crate::helpers::{
    cmd_exists, collect_files_recursive, extract_json_string_field, is_mountpoint, is_virtual_iface,
    is_wsl, list_interfaces, rand_u64, resolve_user_home, resolve_uid_any, run_capture, run_quiet,
    run_stdin, service_ctrl, shred_dir_recursive, shred_file, sysctl_write, unlock_nm_dns,
};
use crate::modules::defense::active_swap_devices;
use crate::modules::network::unpin_snowflake_hosts;
use crate::modules::obfuscation::{apply_timezone, ANON_JS_EMBEDDED};
use crate::state::{reset_all_module_states, AnonState};
use crate::sys::{send_signal, sync_filesystems, wipe_free_ram_native, SIGKILL};
use crate::ui::{err, info, msg, notify, warn, CYAN, GREEN, RED, RESET, YELLOW};

// ═══════════════════════════════════════════════════════════════════
// 1. System Backup & Recovery (--fix)
// ═══════════════════════════════════════════════════════════════════
pub fn get_backups(cfg: &AppConfig) {
    let _ = fs::create_dir_all(&cfg.backup_dir);

    let resolv = &cfg.resolv_conf;
    let sym_bak = cfg.backup_dir.join("resolv.conf.symlink.bak");
    let file_bak = cfg.backup_dir.join("resolv.conf.bak");
    if let Ok(meta) = fs::symlink_metadata(resolv) {
        if meta.file_type().is_symlink() {
            if let Ok(target) = fs::read_link(resolv) {
                let _ = fs::remove_file(&sym_bak);
                let _ = symlink(target, &sym_bak);
            }
        }
        if let Ok(data) = fs::read(resolv) {
            let _ = fs::write(&file_bak, data);
        }
    }

    let host = fs::read_to_string("/etc/hostname")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| run_capture("hostname", &[]))
        .unwrap_or_default();
    let _ = fs::write(cfg.backup_dir.join("hostname.bak"), host);

    if let Ok(hosts) = fs::read(&cfg.hosts_file) {
        let _ = fs::write(cfg.backup_dir.join("hosts.bak"), hosts);
    }
    if let Ok(torrc) = fs::read_to_string(&cfg.torrc) {
        let clean: String = torrc
            .lines()
            .filter(|l| !l.trim_end().ends_with("# ANON_SANDBOX"))
            .map(|l| format!("{l}\n"))
            .collect();
        let _ = fs::write(cfg.backup_dir.join("torrc.bak"), clean);
    }

    if let Some(rules) = run_capture("iptables-save", &[]).filter(|s| !s.is_empty()) {
        let _ = fs::write(cfg.backup_dir.join("iptables.rules.bak"), rules);
    }
    if cmd_exists("ip6tables-save") {
        if let Some(rules6) = run_capture("ip6tables-save", &[]).filter(|s| !s.is_empty()) {
            let _ = fs::write(cfg.backup_dir.join("ip6tables.rules.bak"), rules6);
        }
    }

    let mac_dir = cfg.backup_dir.join("mac_addresses");
    let _ = fs::create_dir_all(&mac_dir);
    for iface in list_interfaces() {
        if !is_virtual_iface(&iface) {
            let p = format!("/sys/class/net/{iface}/address");
            if let Ok(addr) = fs::read_to_string(p) {
                let _ = fs::write(mac_dir.join(&iface), addr);
            }
        }
    }

    let mut tz = String::new();
    if cmd_exists("timedatectl") {
        if let Some(out) = run_capture("timedatectl", &["show"]) {
            for line in out.lines() {
                if let Some(v) = line.strip_prefix("Timezone=") {
                    tz = v.trim().to_string();
                    break;
                }
            }
        }
    }
    if tz.is_empty() {
        if let Ok(s) = fs::read_to_string("/etc/timezone") {
            tz = s.trim().to_string();
        }
    }
    if !tz.is_empty() {
        let _ = fs::write(cfg.backup_dir.join("timezone.bak"), format!("{tz}\n"));
    }

    let mut tar_items: Vec<String> = Vec::new();
    if let Ok(entries) = fs::read_dir(&cfg.backup_dir) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.ends_with(".bak") || name == "mac_addresses" {
                tar_items.push(name);
            }
        }
    }

    if !tar_items.is_empty() {
        let mut args = vec!["-czf".to_string(), "anon_fix_backups.tar.gz".to_string(), "--".to_string()];
        args.extend(tar_items.iter().cloned());
        let ok = Command::new("tar")
            .current_dir(&cfg.backup_dir)
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            for item in tar_items {
                let p = cfg.backup_dir.join(item);
                if p.is_dir() {
                    let _ = fs::remove_dir_all(p);
                } else {
                    let _ = fs::remove_file(p);
                }
            }
        }
    }
}

pub fn restore_system(cfg: &AppConfig) {
    clean_bak_entries(&cfg.backup_dir);

    let archive = cfg.backup_dir.join("anon_fix_backups.tar.gz");
    if fs::metadata(&archive).map(|m| m.len() > 0).unwrap_or(false) {
        let _ = Command::new("tar")
            .current_dir(&cfg.backup_dir)
            .args(["-xzf", "anon_fix_backups.tar.gz"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }

    unlock_nm_dns(cfg);
    let _ = run_quiet("chattr", &["-i", &cfg.resolv_conf.to_string_lossy()]);

    let sym_bak = cfg.backup_dir.join("resolv.conf.symlink.bak");
    let file_bak = cfg.backup_dir.join("resolv.conf.bak");
    if let Ok(meta) = fs::symlink_metadata(&sym_bak) {
        let _ = fs::remove_file(&cfg.resolv_conf);
        if meta.file_type().is_symlink() {
            if let Ok(target) = fs::read_link(&sym_bak) {
                let _ = symlink(target, &cfg.resolv_conf);
            }
            let _ = fs::remove_file(&sym_bak);
        } else {
            let _ = fs::rename(&sym_bak, &cfg.resolv_conf);
        }
    } else if fs::metadata(&file_bak).map(|m| m.len() > 0).unwrap_or(false) {
        if let Ok(data) = fs::read(&file_bak) {
            let _ = fs::write(&cfg.resolv_conf, data);
        }
    }

    if let Ok(host) = fs::read_to_string(cfg.backup_dir.join("hostname.bak")) {
        let h = host.trim();
        if !h.is_empty() {
            let _ = fs::write("/etc/hostname", format!("{h}\n"));
            let _ = run_quiet("hostname", &[h]);
            if cmd_exists("hostnamectl") {
                let _ = run_quiet("hostnamectl", &["set-hostname", h]);
            }
        }
    }

    let hosts_bak = cfg.backup_dir.join("hosts.bak");
    if fs::metadata(&hosts_bak).map(|m| m.len() > 0).unwrap_or(false) {
        if let Ok(data) = fs::read(&hosts_bak) {
            let _ = fs::write(&cfg.hosts_file, data);
        }
    }
    unpin_snowflake_hosts(cfg);

    let dh_bak = cfg.backup_dir.join("dhclient.conf.bak");
    if fs::metadata(&dh_bak).map(|m| m.len() > 0).unwrap_or(false) && Path::new("/etc/dhcp").is_dir() {
        if let Ok(data) = fs::read(&dh_bak) {
            let _ = fs::write("/etc/dhcp/dhclient.conf", data);
        }
    }

    let torrc_bak = cfg.backup_dir.join("torrc.bak");
    if torrc_bak.is_file() {
        if let Ok(data) = fs::read(&torrc_bak) {
            let _ = fs::write(&cfg.torrc, data);
        }
    }

    if cmd_exists("rfkill") {
        let _ = run_quiet("rfkill", &["unblock", "all"]);
    }

    for iface in list_interfaces() {
        if iface == "lo" {
            continue;
        }
        let _ = run_quiet("tc", &["qdisc", "del", "dev", &iface, "root"]);
        let _ = run_quiet("ip", &["link", "set", &iface, "up"]);
    }

    let _ = run_quiet("ip", &["link", "del", "veth-anon"]);
    let _ = run_quiet("ip", &["netns", "del", "anon_jail"]);
    let _ = fs::remove_dir_all(&cfg.netns_etc_dir);

    if cfg.apparmor_tor_profile.is_file() {
        if cmd_exists("apparmor_parser") {
            let _ = run_quiet("apparmor_parser", &["-R", &cfg.apparmor_tor_profile.to_string_lossy()]);
        }
        let _ = fs::remove_file(&cfg.apparmor_tor_profile);
    }

    if let Ok(entries) = fs::read_dir(&cfg.wrapper_dir) {
        for e in entries.flatten() {
            if e.file_name().to_string_lossy().ends_with(".anon-sandbox") {
                let _ = fs::remove_file(e.path());
            }
        }
    }

    for tool in ["nmap", "sqlmap"] {
        let bin_path = PathBuf::from(format!("/usr/local/bin/{tool}"));
        let bak_path = cfg.backup_dir.join(format!("local_bin_{tool}.bak"));
        if fs::read_to_string(&bin_path)
            .map(|s| s.contains("ANON_PROC_WRAPPER") || s.contains("systemd-kworker") || s.contains("systemd_kworker"))
            .unwrap_or(false)
        {
            let _ = fs::remove_file(&bin_path);
        }
        if bak_path.exists() {
            let _ = fs::rename(&bak_path, &bin_path);
        }
    }

    if is_mountpoint("/var/log") {
        let _ = run_quiet("umount", &["-l", "/var/log"]);
        let _ = service_ctrl("restart", "systemd-journald", None);
        let _ = service_ctrl("restart", "rsyslog", None);
    }

    let proc_bak = cfg.backup_dir.join("proc_mount_opts.bak");
    if let Ok(orig) = fs::read_to_string(&proc_bak) {
        let o = orig.trim();
        let opt_arg = if !o.is_empty() && !o.contains("hidepid=") {
            format!("remount,{o},hidepid=0")
        } else if !o.is_empty() {
            format!("remount,{o}")
        } else {
            "remount,rw,hidepid=0".to_string()
        };
        let _ = run_quiet("mount", &["-o", &opt_arg, "/proc"]);
    } else if fs::read_to_string("/proc/mounts")
        .map(|s| s.contains("/proc") && s.contains("hidepid=2"))
        .unwrap_or(false)
    {
        let _ = run_quiet("mount", &["-o", "remount,hidepid=0", "/proc"]);
    }

    for pref in [
        "/etc/firefox/anon.js",
        "/etc/firefox-esr/anon.js",
        "/etc/librewolf/anon.js",
        "/etc/firefox/pref/anon.js",
        "/etc/firefox-esr/pref/anon.js",
        "/usr/lib/firefox-esr/defaults/pref/anon.js",
        "/usr/lib/firefox/defaults/pref/anon.js",
        "/usr/lib/librewolf/defaults/pref/anon.js",
    ] {
        let _ = fs::remove_file(pref);
    }

    let ipt_bak = cfg.backup_dir.join("iptables.rules.bak");
    if let Some(r) = fs::read_to_string(&ipt_bak).ok().filter(|s| !s.is_empty()) {
        let _ = run_stdin("iptables-restore", &[], &r);
    } else {
        for args in [
            &["-F"][..],
            &["-X"],
            &["-t", "nat", "-F"],
            &["-P", "INPUT", "ACCEPT"],
            &["-P", "FORWARD", "ACCEPT"],
            &["-P", "OUTPUT", "ACCEPT"],
        ] {
            let _ = run_quiet("iptables", args);
        }
    }

    if cmd_exists("ip6tables-restore") {
        let ipt6_bak = cfg.backup_dir.join("ip6tables.rules.bak");
        if let Some(r6) = fs::read_to_string(&ipt6_bak).ok().filter(|s| !s.is_empty()) {
            let _ = run_stdin("ip6tables-restore", &[], &r6);
        } else {
            for args in [
                &["-F"][..],
                &["-P", "INPUT", "ACCEPT"],
                &["-P", "FORWARD", "ACCEPT"],
                &["-P", "OUTPUT", "ACCEPT"],
            ] {
                let _ = run_quiet("ip6tables", args);
            }
        }
    }

    if !is_wsl() {
        let _ = sysctl_write("net.ipv6.conf.all.disable_ipv6", "0");
        let _ = sysctl_write("net.ipv6.conf.default.disable_ipv6", "0");
    }

    let mac_dir = cfg.backup_dir.join("mac_addresses");
    if mac_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&mac_dir) {
            for e in entries.flatten() {
                let p = e.path();
                if !p.is_file() {
                    continue;
                }
                if let Some(devname) = p.file_name().and_then(|s| s.to_str()) {
                    if !is_virtual_iface(devname) && Path::new(&format!("/sys/class/net/{devname}")).exists() {
                        if let Ok(mac) = fs::read_to_string(&p) {
                            let m = mac.trim();
                            if !m.is_empty() {
                                let _ = run_quiet("ip", &["link", "set", devname, "down"]);
                                let _ = run_quiet("ip", &["link", "set", devname, "address", m]);
                                let _ = run_quiet("ip", &["link", "set", devname, "up"]);
                            }
                        }
                    }
                }
            }
        }
    }

    if let Ok(tz) = fs::read_to_string(cfg.backup_dir.join("timezone.bak")) {
        let t = tz.trim();
        if !t.is_empty() {
            let _ = apply_timezone(t);
        }
    }

    let swap_marker = cfg.backup_dir.join("swap_was_active");
    if swap_marker.is_file() {
        let _ = run_quiet("swapon", &["-a"]);
        let _ = fs::remove_file(swap_marker);
    }

    reset_all_module_states(&cfg.state_file);

    let proc_ramdir_bak = cfg.backup_dir.join("process_obfuscation_ramdir.bak");
    if let Ok(ramdir) = fs::read_to_string(&proc_ramdir_bak) {
        let trimmed = ramdir.trim();
        if trimmed.starts_with("/dev/shm/.") {
            shred_dir_recursive(Path::new(trimmed));
        }
        let _ = fs::remove_file(proc_ramdir_bak);
    }
    let _ = fs::remove_file(cfg.backup_dir.join("traffic_obfuscation.iface"));
    let _ = fs::remove_file(cfg.backup_dir.join("bluetooth_was_on"));

    cleanup_ephemeral_paths(cfg);
    clean_bak_entries(&cfg.backup_dir);
}

fn clean_bak_entries(backup_dir: &Path) {
    if let Ok(entries) = fs::read_dir(backup_dir) {
        for e in entries.flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            if name.ends_with(".bak") {
                shred_file(&p);
            } else if name == "mac_addresses" && p.is_dir() {
                let _ = fs::remove_dir_all(p);
            }
        }
    }
}

fn cleanup_ephemeral_paths(cfg: &AppConfig) {
    for dir in ["/tmp", "/dev/shm"] {
        if let Ok(entries) = fs::read_dir(dir) {
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                let is_target = if dir == "/tmp" {
                    name.starts_with("anon")
                } else {
                    name.starts_with(".anon")
                        || name.starts_with("anon_browser_")
                        || name.starts_with(".systemd_kworker_")
                        || name.starts_with(".kworker_")
                };
                if is_target {
                    let p = e.path();
                    if p == cfg.base_dir || p == cfg.backup_dir {
                        continue;
                    }
                    if p.is_dir() {
                        shred_dir_recursive(&p);
                    } else {
                        shred_file(&p);
                    }
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// 2. Emergency Panic Protocol (--panic)
// ═══════════════════════════════════════════════════════════════════
pub fn trigger_panic(cfg: &AppConfig) {
    crate::ui::clear_screen();
    println!("\x1b[1;31m");
    println!("  ╔══════════════════════════════════════════════════════════════╗");
    println!("  ║        CRITICAL: EMERGENCY PANIC PROTOCOL ACTIVATED          ║");
    println!("  ╚══════════════════════════════════════════════════════════════╝");
    println!("\x1b[0m");

    let state = AnonState::load(&cfg.state_file);

    println!(" \x1b[33m[1/6]\x1b[0m Severing all network physical, wireless, and virtual interfaces...");
    notify(
        "ANON - EMERGENCY PANIC",
        "Panic protocol activated: interfaces severed and memory wiped.",
        "critical",
    );
    for iface in list_interfaces() {
        if iface == "lo" {
            continue;
        }
        let _ = run_quiet("tc", &["qdisc", "del", "dev", &iface, "root"]);
        let _ = run_quiet("ip", &["link", "set", "dev", &iface, "down"]);
    }
    if cmd_exists("rfkill") {
        let _ = run_quiet("rfkill", &["block", "all"]);
    }

    println!(" \x1b[33m[2/6]\x1b[0m Locking down kernel firewall (IPv4/IPv6 total drop)...");
    for args in [
        &["-F"][..],
        &["-X"],
        &["-t", "nat", "-F"],
        &["-t", "nat", "-X"],
        &["-t", "mangle", "-F"],
        &["-t", "mangle", "-X"],
        &["-P", "INPUT", "DROP"],
        &["-P", "FORWARD", "DROP"],
        &["-P", "OUTPUT", "DROP"],
    ] {
        let _ = run_quiet("iptables", args);
    }
    if cmd_exists("ip6tables") {
        for args in [
            &["-F"][..],
            &["-X"],
            &["-t", "mangle", "-F"],
            &["-t", "mangle", "-X"],
            &["-P", "INPUT", "DROP"],
            &["-P", "FORWARD", "DROP"],
            &["-P", "OUTPUT", "DROP"],
        ] {
            let _ = run_quiet("ip6tables", args);
        }
    }
    if !is_wsl() {
        let _ = sysctl_write("net.ipv6.conf.all.disable_ipv6", "1");
        let _ = sysctl_write("net.ipv6.conf.default.disable_ipv6", "1");
    }

    println!(" \x1b[33m[3/6]\x1b[0m Force-terminating all anonymization, daemon & browser processes...");
    for mod_name in ["anti_mitm", "traffic_obfuscation", "leak_auditor"] {
        let pid = state.get_pid(mod_name);
        if pid > 0 {
            let _ = send_signal(pid, SIGKILL);
        }
    }
    let _ = run_quiet(
        "killall",
        &[
            "-9",
            "firefox",
            "firefox-esr",
            "librewolf",
            "chromium",
            "brave",
            "tor",
            "i2pd",
            "dnscrypt-proxy",
            "snowflake-client",
            "obfs4proxy",
        ],
    );
    let _ = service_ctrl("stop", "tor.service", None);
    let _ = service_ctrl("stop", "i2pd", None);
    let _ = service_ctrl("stop", "dnscrypt-proxy", None);

    let _ = run_quiet("ip", &["link", "del", "veth-anon"]);
    let _ = run_quiet("ip", &["netns", "del", "anon_jail"]);
    let _ = fs::remove_dir_all(&cfg.netns_etc_dir);

    println!(" \x1b[33m[4/6]\x1b[0m Shredding ephemeral session directories and RAM profiles...");
    reset_all_module_states(&cfg.state_file);
    cleanup_ephemeral_paths(cfg);
    let _ = fs::remove_dir_all("/dev/shm/.systemd-kworker");

    if state.is_enabled("volatile_logging") || is_mountpoint("/var/log") {
        let _ = run_quiet("umount", &["-l", "/var/log"]);
    }

    println!(" \x1b[33m[5/6]\x1b[0m Flushing memory caches, sanitizing RAM & swap, and wiping history...");
    sync_filesystems();
    let _ = fs::write("/proc/sys/vm/drop_caches", "3\n");

    let swaps = active_swap_devices();
    let _ = run_quiet("swapoff", &["-a"]);
    if !is_wsl() {
        if !swaps.is_empty() && cmd_exists("sswap") {
            for dev in &swaps {
                let _ = run_quiet("sswap", &[dev]);
            }
        }
        if cmd_exists("sdmem") {
            let _ = run_quiet("sdmem", &["-f", "-ll"]);
        } else {
            wipe_free_ram_native(64);
        }
    }

    let mut hist_paths: Vec<PathBuf> = vec![
        PathBuf::from("/root/.bash_history"),
        PathBuf::from("/root/.zsh_history"),
    ];
    if let Ok(home) = env::var("HOME") {
        hist_paths.push(PathBuf::from(&home).join(".bash_history"));
        hist_paths.push(PathBuf::from(&home).join(".zsh_history"));
    }
    if let Ok(su) = env::var("SUDO_USER") {
        if !su.is_empty() && su != "root" {
            let shome = resolve_user_home(&su);
            hist_paths.push(PathBuf::from(&shome).join(".bash_history"));
            hist_paths.push(PathBuf::from(&shome).join(".zsh_history"));
        }
    }
    for hp in hist_paths {
        if hp.is_file() {
            shred_file(&hp);
        }
    }

    println!(" \x1b[33m[6/6]\x1b[0m Locking down module state (preserving strict network isolation)...");
    let _ = run_quiet("chattr", &["-i", &cfg.resolv_conf.to_string_lossy()]);

    println!("\n \x1b[1;32m✔ SUCCESS: System has been sanitized and locked down.\x1b[0m");
    println!(" \x1b[2mAll interfaces remain DOWN and firewall policies remain DROP.\x1b[0m");
    println!(" \x1b[2mRun 'sudo anon --fix' when safe to restore baseline network connectivity.\x1b[0m\n");
}

// ═══════════════════════════════════════════════════════════════════
// 3. Disposable Browser (--browser)
// ═══════════════════════════════════════════════════════════════════
pub fn start_disposable_browser(cfg: &AppConfig) -> Result<(), String> {
    let ff_bin = if cmd_exists("firefox-esr") {
        "firefox-esr"
    } else if cmd_exists("firefox") {
        "firefox"
    } else {
        println!("\n {RED}✖ ERROR:{RESET} Firefox is not installed. Please install it first.");
        return Err("firefox not installed".to_string());
    };

    println!("\n {CYAN}ℹ INFO:{RESET} Initializing true RAM-based disposable browser...");

    let state = AnonState::load(&cfg.state_file);
    let sandbox_active = state.is_enabled("sandbox_isolation");
    let socks_host = if sandbox_active {
        if !state.is_enabled("ip_changer") {
            warn("Sandbox Isolation is enabled without IP Changer; ensure Tor is listening on 10.200.200.1:9050.");
        }
        "10.200.200.1"
    } else {
        "127.0.0.1"
    };

    let sudo_user = env::var("SUDO_USER").ok().filter(|s| !s.is_empty());
    let run_user = sudo_user.as_deref().unwrap_or("root");
    let run_home = resolve_user_home(run_user);
    let run_uid = resolve_uid_any(&[run_user]).unwrap_or(0);

    let disp = env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string());
    let xauth = env::var("XAUTHORITY")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("{run_home}/.Xauthority"));
    let wayland_disp = env::var("WAYLAND_DISPLAY").unwrap_or_default();
    let xdg_rt = env::var("XDG_RUNTIME_DIR")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("/run/user/{run_uid}"));

    let ram_profile = PathBuf::from(format!("/dev/shm/anon_browser_{:06x}", rand_u64() & 0xff_ffff));
    fs::create_dir_all(&ram_profile).map_err(|e| format!("create ram profile: {e}"))?;
    let _ = fs::set_permissions(&ram_profile, fs::Permissions::from_mode(0o700));

    println!(" {CYAN}ℹ INFO:{RESET} Hardening profile with anon OPSEC configurations...");
    let anon_js = fs::read_to_string(cfg.src_dir.join("sources/anon.js"))
        .unwrap_or_else(|_| ANON_JS_EMBEDDED.to_string());
    let mut user_js = String::with_capacity(anon_js.len() + 1024);
    for line in anon_js.lines() {
        if let Some(rest) = line.strip_prefix("pref(") {
            user_js.push_str("user_pref(");
            user_js.push_str(rest);
        } else {
            user_js.push_str(line);
        }
        user_js.push('\n');
    }
    user_js.push_str(&format!(
        "user_pref(\"network.proxy.type\", 1);\n\
         user_pref(\"network.proxy.socks\", \"{socks_host}\");\n\
         user_pref(\"network.proxy.socks_port\", 9050);\n\
         user_pref(\"network.proxy.socks_version\", 5);\n\
         user_pref(\"network.proxy.socks_remote_dns\", true);\n\
         user_pref(\"network.trr.mode\", 5);\n\
         user_pref(\"network.dns.disablePrefetch\", true);\n\
         user_pref(\"network.prefetch-next\", false);\n\
         user_pref(\"network.connectivity-service.enabled\", false);\n\
         user_pref(\"browser.safebrowsing.malware.enabled\", false);\n\
         user_pref(\"browser.safebrowsing.phishing.enabled\", false);\n\
         user_pref(\"geo.enabled\", false);\n\
         user_pref(\"media.navigator.enabled\", false);\n\
         user_pref(\"dom.event.clipboardevents.enabled\", false);\n\
         user_pref(\"places.history.enabled\", false);\n\
         user_pref(\"browser.formfill.enable\", false);\n\
         user_pref(\"signon.rememberSignons\", false);\n"
    ));
    let _ = fs::write(ram_profile.join("user.js"), user_js);

    println!(" {CYAN}ℹ INFO:{RESET} Launching isolated instance. {YELLOW}Do NOT close this terminal!{RESET}");
    println!(" {CYAN}ℹ INFO:{RESET} The profile will be securely wiped the exact second you close the browser.\n");

    let _ = run_quiet("chown", &["-R", &format!("{run_user}:{run_user}"), &ram_profile.to_string_lossy()]);

    let mut xhost_added = false;
    if run_user != "root" && cmd_exists("xhost") {
        let ok = Command::new("xhost")
            .env("DISPLAY", &disp)
            .env("XAUTHORITY", &xauth)
            .arg(format!("+si:localuser:{run_user}"))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        xhost_added = ok;
    }

    let mut argv: Vec<String> = Vec::new();
    if sandbox_active {
        println!(" {GREEN}✔ SUCCESS:{RESET} Launching browser inside strictly isolated 'anon_jail' Kernel Namespace...");
        argv.extend(["ip", "netns", "exec", "anon_jail"].map(String::from));
    } else {
        println!(" {YELLOW}⚠ WARNING:{RESET} Sandbox Isolation is disabled. Launching on host network stack...");
    }

    if run_user != "root" {
        argv.extend(["sudo", "-u", run_user, "env"].map(String::from));
        argv.push(format!("DISPLAY={disp}"));
        argv.push(format!("XAUTHORITY={xauth}"));
        argv.push(format!("HOME={run_home}"));
        if !wayland_disp.is_empty() {
            argv.push(format!("WAYLAND_DISPLAY={wayland_disp}"));
        }
        if Path::new(&xdg_rt).is_dir() {
            argv.push(format!("XDG_RUNTIME_DIR={xdg_rt}"));
        }
    }

    if cmd_exists("firejail") {
        println!(" {GREEN}✔ SUCCESS:{RESET} Firejail sandbox active with strict seccomp and capability dropping...");
        argv.extend(
            [
                "firejail",
                "--caps.drop=all",
                "--nonewprivs",
                "--seccomp",
                "--private-tmp",
                "--no3d",
                "--nosound",
                "--nodvd",
            ]
            .map(String::from),
        );
    } else {
        println!(" {YELLOW}⚠ WARNING:{RESET} Firejail not installed. Skipping process isolation. (Run 'apt install firejail')");
    }

    argv.extend([
        ff_bin.to_string(),
        "--profile".to_string(),
        ram_profile.to_string_lossy().into_owned(),
        "--private-window".to_string(),
        "https://check.torproject.org/".to_string(),
        "--no-remote".to_string(),
    ]);

    let _ = Command::new(&argv[0])
        .args(&argv[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    println!("\n {CYAN}ℹ INFO:{RESET} Browser closed. Initiating secure deletion of RAM profile...");
    if xhost_added {
        let _ = Command::new("xhost")
            .env("DISPLAY", &disp)
            .env("XAUTHORITY", &xauth)
            .arg(format!("-si:localuser:{run_user}"))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    shred_dir_recursive(&ram_profile);
    println!(" {GREEN}✔ SUCCESS:{RESET} Disposable profile completely wiped. Zero traces left on disk.\n");
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// 4. Parallel Metadata Cleaner (--clean-meta)
// ═══════════════════════════════════════════════════════════════════
fn sanitize_single_file(path: &Path, has_mat2: bool, has_exiftool: bool) -> bool {
    let raw = path.to_string_lossy();
    let safe_arg = if raw.starts_with('-') {
        format!("./{raw}")
    } else {
        raw.into_owned()
    };

    if has_mat2 {
        if run_quiet("mat2", &["--inplace", &safe_arg]) {
            return true;
        }
        if has_exiftool && run_quiet("exiftool", &["-all=", "-overwrite_original", &safe_arg]) {
            return true;
        }
        return false;
    }
    if has_exiftool {
        return run_quiet("exiftool", &["-all=", "-overwrite_original", &safe_arg]);
    }
    false
}

pub fn clean_meta_target(target: &Path) -> bool {
    if !target.exists() {
        eprintln!(
            "\x1b[31m✖ ERROR: Target path does not exist: {}\x1b[0m",
            target.display()
        );
        return false;
    }

    let has_mat2 = cmd_exists("mat2");
    let has_exiftool = cmd_exists("exiftool");
    if !has_mat2 && !has_exiftool {
        eprintln!("\x1b[31m✖ ERROR: Neither 'mat2' nor 'exiftool' is installed.\x1b[0m");
        eprintln!("  Please install MAT2 or ExifTool:\n  \x1b[32msudo apt-get install -y mat2 libimage-exiftool-perl\x1b[0m");
        return false;
    }

    println!(
        "\x1b[1;36m[*] Sanitizing metadata for: {}\x1b[0m",
        target.display()
    );
    if has_mat2 {
        println!("  \x1b[32m[+] Primary Engine:\x1b[0m MAT2 (Metadata Anonymisation Toolkit v2)");
    } else {
        println!("  \x1b[33m[+] Primary Engine:\x1b[0m ExifTool");
    }

    let files: Vec<PathBuf> = if target.is_dir() {
        let mut collected = Vec::new();
        collect_files_recursive(target, &mut collected);
        if collected.is_empty() {
            eprintln!(
                "  \x1b[33m⚠ WARNING:\x1b[0m Directory is empty (no files found): {}",
                target.display()
            );
            return true;
        }
        collected
    } else {
        vec![target.to_path_buf()]
    };

    let total = files.len();
    let succeeded = AtomicUsize::new(0);
    let failed = AtomicUsize::new(0);
    let cursor = AtomicUsize::new(0);
    let print_lock = Mutex::new(());

    let workers = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(total)
        .max(1);

    thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| loop {
                let idx = cursor.fetch_add(1, Ordering::Relaxed);
                if idx >= total {
                    break;
                }
                let file = &files[idx];
                if sanitize_single_file(file, has_mat2, has_exiftool) {
                    succeeded.fetch_add(1, Ordering::Relaxed);
                } else {
                    failed.fetch_add(1, Ordering::Relaxed);
                    let _g = print_lock.lock();
                    eprintln!(
                        "  \x1b[33m⚠ WARNING:\x1b[0m Could not sanitize: {}",
                        file.display()
                    );
                }
            });
        }
    });

    let ok_cnt = succeeded.load(Ordering::Relaxed);
    let fail_cnt = failed.load(Ordering::Relaxed);

    if ok_cnt > 0 && fail_cnt == 0 {
        println!(
            "\x1b[1;32m✔ SUCCESS: Metadata sanitized successfully for: {} ({ok_cnt}/{total} file(s))\x1b[0m",
            target.display()
        );
        let name = target
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("target");
        notify(
            "ANON Metadata Cleaner",
            &format!("Sanitized metadata for: {name} ({ok_cnt}/{total})"),
            "normal",
        );
        true
    } else if ok_cnt > 0 {
        eprintln!(
            "\x1b[33m⚠ WARNING: Partially sanitized {}: {ok_cnt} succeeded, {fail_cnt} skipped/failed.\x1b[0m",
            target.display()
        );
        false
    } else {
        eprintln!(
            "\x1b[31m✖ ERROR: Metadata sanitization failed for: {}\x1b[0m",
            target.display()
        );
        false
    }
}

// ═══════════════════════════════════════════════════════════════════
// 5. Self-Update (--update)
// ═══════════════════════════════════════════════════════════════════
pub fn self_update(cfg: &AppConfig) {
    info("Starting Anon Self-Update...");
    if !cmd_exists("unzip") {
        warn("unzip is not installed. Installing...");
        if cmd_exists("apt-get") {
            let _ = run_quiet("apt-get", &["update"]);
            let _ = Command::new("apt-get")
                .env("DEBIAN_FRONTEND", "noninteractive")
                .args(["install", "-y", "unzip"])
                .status();
        }
    }

    let state = AnonState::load(&cfg.state_file);
    let mut curl_base = vec!["-fsSL".to_string(), "--max-time".to_string(), "30".to_string()];
    if state.is_enabled("ip_changer") {
        info("Routing update download through Tor SOCKS5 proxy...");
        curl_base.push("--socks5-hostname".to_string());
        curl_base.push("127.0.0.1:9050".to_string());
    }

    let tmp_dir = PathBuf::from(format!("/tmp/anon_upd_{:06x}", rand_u64() & 0xff_ffff));
    if fs::create_dir_all(&tmp_dir).is_err() {
        err("Failed to create temporary update directory.");
    }

    let result = (|| -> Result<(), String> {
        info("Checking latest release metadata from GitHub...");
        let mut rel_args = curl_base.clone();
        rel_args.push("https://api.github.com/repos/egnake/anon/releases/latest".to_string());
        let rel_refs: Vec<&str> = rel_args.iter().map(String::as_str).collect();
        let rel_json = run_capture("curl", &rel_refs).unwrap_or_default();

        let tag = extract_json_string_field(&rel_json, "tag_name");
        let tarball_url = find_asset_url(&rel_json, ".tar.gz");
        let sha_url = find_asset_url(&rel_json, "SHA256SUMS");

        if let (Some(tb_url), Some(s_url)) = (tarball_url, sha_url) {
            let tag_disp = tag.as_deref().unwrap_or("latest");
            info(&format!("Downloading release {tag_disp} and SHA256SUMS..."));
            let tb_name = tb_url.rsplit('/').next().unwrap_or("release.tar.gz");
            let archive_file = tmp_dir.join(tb_name);
            let sha_file = tmp_dir.join("SHA256SUMS");

            let mut a1 = curl_base.clone();
            a1.extend([tb_url.clone(), "-o".to_string(), archive_file.to_string_lossy().into_owned()]);
            let a1_refs: Vec<&str> = a1.iter().map(String::as_str).collect();
            if !run_quiet("curl", &a1_refs) {
                return Err("Failed to download release tarball".to_string());
            }

            let mut a2 = curl_base.clone();
            a2.extend([s_url, "-o".to_string(), sha_file.to_string_lossy().into_owned()]);
            let a2_refs: Vec<&str> = a2.iter().map(String::as_str).collect();
            if !run_quiet("curl", &a2_refs) {
                return Err("Failed to download SHA256SUMS".to_string());
            }

            let sha_txt = fs::read_to_string(&sha_file).unwrap_or_default();
            let matching_line = sha_txt
                .lines()
                .find(|l| l.contains(tb_name))
                .map(str::to_string);
            let Some(chk_line) = matching_line else {
                return Err("Release checksum verification failed! Aborting update.".to_string());
            };
            let chk_ok = Command::new("sha256sum")
                .current_dir(&tmp_dir)
                .args(["-c", "-"])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .ok()
                .and_then(|mut ch| {
                    use std::io::Write as _;
                    if let Some(mut si) = ch.stdin.take() {
                        let _ = writeln!(si, "{chk_line}");
                    }
                    ch.wait().ok()
                })
                .map(|s| s.success())
                .unwrap_or(false);
            if !chk_ok {
                return Err("Release checksum verification failed! Aborting update.".to_string());
            }

            info("SHA256 checksum verified.");
            let _ = run_quiet("tar", &["-xzf", &archive_file.to_string_lossy(), "-C", &tmp_dir.to_string_lossy()]);

            let ext_dir = if tmp_dir.join("Makefile").is_file() {
                Some(tmp_dir.clone())
            } else {
                fs::read_dir(&tmp_dir)
                    .ok()
                    .and_then(|rd| {
                        rd.flatten()
                            .map(|e| e.path())
                            .find(|p| p.is_dir() && p.join("Makefile").is_file())
                    })
            };

            if let Some(dir) = ext_dir {
                let status = Command::new("make")
                    .current_dir(&dir)
                    .arg("reinstall")
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false);
                if status {
                    msg(&format!("Anon ({tag_disp}) verified and updated!"));
                    return Ok(());
                }
            }
        }

        warn("No signed release tarball/SHA256SUMS found; falling back to main branch archive.");
        let zip_path = tmp_dir.join("anon.zip");
        let mut az = curl_base.clone();
        az.extend([
            "https://github.com/egnake/anon/archive/refs/heads/main.zip".to_string(),
            "-o".to_string(),
            zip_path.to_string_lossy().into_owned(),
        ]);
        let az_refs: Vec<&str> = az.iter().map(String::as_str).collect();
        if run_quiet("curl", &az_refs) && run_quiet("unzip", &["-tq", &zip_path.to_string_lossy()]) {
            info("Extracting...");
            let _ = run_quiet("unzip", &["-q", &zip_path.to_string_lossy(), "-d", &tmp_dir.to_string_lossy()]);
            let main_dir = tmp_dir.join("anon-main");
            if main_dir.is_dir() {
                info("Running make reinstall...");
                let ok = Command::new("make")
                    .current_dir(&main_dir)
                    .arg("reinstall")
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false);
                if ok {
                    msg("Anon has been successfully updated to the latest version!");
                    info("Please restart Anon by typing 'sudo anon'");
                    return Ok(());
                }
                return Err("make reinstall failed".to_string());
            }
            return Err("Corrupted update archive: anon-main directory not found.".to_string());
        }
        Err("Failed to download or verify the update archive.".to_string())
    })();

    let _ = fs::remove_dir_all(&tmp_dir);
    if let Err(e) = result {
        err(&e);
    }
}

fn find_asset_url(json: &str, needle: &str) -> Option<String> {
    let mut search_from = 0usize;
    let key = "\"browser_download_url\"";
    while let Some(pos) = json[search_from..].find(key) {
        let abs = search_from + pos;
        if let Some(url) = extract_json_string_field(&json[abs..], "browser_download_url") {
            if url.contains(needle) {
                return Some(url);
            }
        }
        search_from = abs + key.len();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_asset_url_matches_expected_deb() {
        let payload = r#"[{"browser_download_url":"https://example.com/anon.tar.gz"},{"browser_download_url":"https://example.com/anon_3.0.0_amd64.deb"}]"#;
        assert_eq!(
            find_asset_url(payload, ".deb").as_deref(),
            Some("https://example.com/anon_3.0.0_amd64.deb")
        );
    }

    #[test]
    fn clean_meta_target_handles_nonexistent_and_recursive_file_collection() {
        let mut base = std::env::temp_dir();
        base.push(format!("anon_meta_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        assert!(!clean_meta_target(&base));
        fs::create_dir_all(base.join("sub")).unwrap();
        let mut collected = Vec::new();
        collect_files_recursive(&base, &mut collected);
        assert!(collected.is_empty());

        fs::write(base.join("a.txt"), "x").unwrap();
        fs::write(base.join("sub").join("b.txt"), "y").unwrap();
        collect_files_recursive(&base, &mut collected);
        assert_eq!(collected.len(), 2);
        let _ = fs::remove_dir_all(&base);
    }
}
