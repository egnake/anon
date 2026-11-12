use std::env;
use std::fs;
use std::process::Command;
use std::thread;
use std::time::Duration;

use crate::config::AppConfig;
use crate::helpers::{
    cmd_exists, get_default_iface, get_gateway_ip, get_gateway_v6, is_mountpoint, is_tor_running,
    is_wsl, resolve_username_any, run_capture, run_quiet, service_ctrl, sysctl_read, sysctl_write,
};
use crate::modules::network::KillSwitch;
use crate::modules::obfuscation::spawn_self_daemon;
use crate::registry::AnonModule;
use crate::state::{set_module_pid, set_module_status, AnonState};
use crate::sys::{
    is_pid_alive, send_signal, set_process_name, sync_filesystems, wipe_free_ram_native,
    InotifyWatcher, NetlinkWatcher, RTMGRP_IPV6_ROUTE, RTMGRP_NEIGH, SIGKILL, SIGTERM,
};
use crate::ui::{info, notify, warn};

// ═══════════════════════════════════════════════════════════════════
// 1. Anti-MITM — Event-driven Netlink + Neighbor Table Sentry
// ═══════════════════════════════════════════════════════════════════
pub struct AntiMitm;

impl AnonModule for AntiMitm {
    fn name(&self) -> &'static str {
        "anti_mitm"
    }

    fn is_live(&self, cfg: &AppConfig) -> bool {
        let pid = AnonState::load(&cfg.state_file).get_pid("anti_mitm");
        is_pid_alive(pid)
    }

    fn start(&self, cfg: &AppConfig) -> Result<(), String> {
        let state = AnonState::load(&cfg.state_file);
        let existing = state.get_pid("anti_mitm");
        if is_pid_alive(existing) {
            warn(&format!("Anti mitm is already running (PID: {existing})"));
            return Ok(());
        }
        let pid = spawn_self_daemon(cfg, "anti-mitm")?;
        // Verify daemon didn't immediately exit 1 (e.g. no gateway)
        thread::sleep(Duration::from_millis(60));
        if !is_pid_alive(pid) {
            let _ = set_module_pid(&cfg.state_file, "anti_mitm", 0);
            let _ = set_module_status(&cfg.state_file, "anti_mitm", false);
            warn("Anti mitm could not attach (no active default gateway/interface).");
            return Ok(());
        }
        set_module_pid(&cfg.state_file, "anti_mitm", pid)?;
        set_module_status(&cfg.state_file, "anti_mitm", true)?;
        info("Anti mitm successfully enabled");
        Ok(())
    }

    fn stop(&self, cfg: &AppConfig) -> Result<(), String> {
        let pid = AnonState::load(&cfg.state_file).get_pid("anti_mitm");
        if is_pid_alive(pid) {
            let _ = send_signal(pid, SIGTERM);
            thread::sleep(Duration::from_millis(80));
            if is_pid_alive(pid) {
                let _ = send_signal(pid, SIGKILL);
            }
        }
        set_module_pid(&cfg.state_file, "anti_mitm", 0)?;
        set_module_status(&cfg.state_file, "anti_mitm", false)?;
        info("Anti mitm successfully disabled");
        Ok(())
    }
}

pub fn extract_lladdr(neigh_output: &str) -> Option<String> {
    for line in neigh_output.lines() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        for i in 0..tokens.len() {
            if tokens[i] == "lladdr" && i + 1 < tokens.len() {
                return Some(tokens[i + 1].to_ascii_lowercase());
            }
        }
    }
    None
}

fn get_gateway_mac(gw_ip: &str) -> Option<String> {
    if let Some(out) = run_capture("ip", &["neigh", "show", gw_ip]) {
        if let Some(mac) = extract_lladdr(&out) {
            return Some(mac);
        }
    }
    let _ = run_quiet("ping", &["-c1", "-W1", gw_ip]);
    if let Some(out) = run_capture("ip", &["neigh", "show", gw_ip]) {
        return extract_lladdr(&out);
    }
    None
}

fn get_gateway6_mac(gw6_ip: &str) -> Option<String> {
    let out = run_capture("ip", &["-6", "neigh", "show", gw6_ip])?;
    extract_lladdr(&out)
}

fn is_valid_mac(mac: &str) -> bool {
    let parts: Vec<&str> = mac.split(':').collect();
    parts.len() == 6
        && parts
            .iter()
            .all(|p| p.len() == 2 && p.chars().all(|c| c.is_ascii_hexdigit()))
}

fn mitm_lockdown(cfg: &AppConfig, iface: &str, spoofed_mac: &str, gw_ip: &str) -> ! {
    eprintln!("[ANON ANTI-MITM] *** ARP SPOOFING DETECTED ***");
    eprintln!("[ANON ANTI-MITM] Gateway {gw_ip} MAC changed to {spoofed_mac}");
    eprintln!("[ANON ANTI-MITM] Initiating network lockdown on {iface}...");

    notify(
        "ANON - CRITICAL SECURITY ALERT",
        &format!(
            "ARP Spoofing detected on {iface}! Gateway MAC changed to {spoofed_mac}. Network severed."
        ),
        "critical",
    );

    let _ = run_quiet("ip", &["link", "set", "dev", iface, "down"]);

    if is_valid_mac(spoofed_mac) {
        if cmd_exists("iptables")
            && !run_quiet("iptables", &["-C", "INPUT", "-m", "mac", "--mac-source", spoofed_mac, "-j", "DROP"])
        {
            let _ = run_quiet("iptables", &["-A", "INPUT", "-m", "mac", "--mac-source", spoofed_mac, "-j", "DROP"]);
        }
        if cmd_exists("ip6tables")
            && !run_quiet("ip6tables", &["-C", "INPUT", "-m", "mac", "--mac-source", spoofed_mac, "-j", "DROP"])
        {
            let _ = run_quiet("ip6tables", &["-A", "INPUT", "-m", "mac", "--mac-source", spoofed_mac, "-j", "DROP"]);
        }
    }

    let _ = set_module_pid(&cfg.state_file, "anti_mitm", 0);
    let _ = set_module_status(&cfg.state_file, "anti_mitm", false);
    eprintln!("[ANON ANTI-MITM] Network interface {iface} is DOWN. Manual intervention required.");
    std::process::exit(2);
}

pub fn run_anti_mitm_daemon(cfg: &AppConfig) -> ! {
    if env::var_os("ANON_STEALTH_ACTIVE").is_some() {
        set_process_name("[kworker/u4:2]");
    }

    let Some(iface) = get_default_iface() else {
        eprintln!("[ANON ANTI-MITM] ERROR: No default network interface found.");
        std::process::exit(1);
    };
    let Some(gw_ip) = get_gateway_ip() else {
        eprintln!("[ANON ANTI-MITM] ERROR: No default gateway found.");
        std::process::exit(1);
    };

    if cmd_exists("arping") {
        let _ = run_quiet("arping", &["-c1", "-w1", "-I", &iface, &gw_ip]);
    }
    let _ = run_quiet("ping", &["-c1", "-W1", &gw_ip]);

    let baseline_mac = match get_gateway_mac(&gw_ip) {
        Some(m) if m != "failed" && m != "incomplete" => m,
        _ => {
            eprintln!("[ANON ANTI-MITM] ERROR: Cannot resolve gateway MAC for {gw_ip}.");
            std::process::exit(1);
        }
    };

    let gw6_ip = get_gateway_v6();
    let baseline_mac6 = gw6_ip.as_deref().and_then(get_gateway6_mac);

    let netlink = NetlinkWatcher::new(RTMGRP_NEIGH);
    let poll_ms = (cfg.mitm_poll_interval_secs * 1000).min(i32::MAX as u64) as i32;

    loop {
        if let Some(ref nl) = netlink {
            let _ = nl.wait_event(poll_ms);
        } else {
            thread::sleep(Duration::from_secs(cfg.mitm_poll_interval_secs));
        }

        if let Some(current_mac) = get_gateway_mac(&gw_ip) {
            if current_mac != baseline_mac {
                mitm_lockdown(cfg, &iface, &current_mac, &gw_ip);
            }
        }
        if let (Some(ref g6), Some(ref base6)) = (&gw6_ip, &baseline_mac6) {
            if let Some(cur6) = get_gateway6_mac(g6) {
                if &cur6 != base6 {
                    mitm_lockdown(cfg, &iface, &cur6, g6);
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// 2. Leak Auditor — Event-driven inotify/Netlink + One-shot Audit
// ═══════════════════════════════════════════════════════════════════
pub struct LeakAuditor;

impl AnonModule for LeakAuditor {
    fn name(&self) -> &'static str {
        "leak_auditor"
    }

    fn is_live(&self, cfg: &AppConfig) -> bool {
        let pid = AnonState::load(&cfg.state_file).get_pid("leak_auditor");
        is_pid_alive(pid)
    }

    fn start(&self, cfg: &AppConfig) -> Result<(), String> {
        let state = AnonState::load(&cfg.state_file);
        let existing = state.get_pid("leak_auditor");
        if is_pid_alive(existing) {
            warn(&format!(
                "Background leak auditor is already running (PID: {existing})"
            ));
            return Ok(());
        }
        let pid = spawn_self_daemon(cfg, "leak-auditor")?;
        set_module_pid(&cfg.state_file, "leak_auditor", pid)?;
        set_module_status(&cfg.state_file, "leak_auditor", true)?;
        info("Background leak auditor successfully enabled");
        Ok(())
    }

    fn stop(&self, cfg: &AppConfig) -> Result<(), String> {
        let pid = AnonState::load(&cfg.state_file).get_pid("leak_auditor");
        if is_pid_alive(pid) {
            let _ = send_signal(pid, SIGTERM);
            thread::sleep(Duration::from_millis(80));
            if is_pid_alive(pid) {
                let _ = send_signal(pid, SIGKILL);
            }
        }
        set_module_pid(&cfg.state_file, "leak_auditor", 0)?;
        set_module_status(&cfg.state_file, "leak_auditor", false)?;
        info("Background leak auditor successfully disabled");
        Ok(())
    }
}

pub fn check_dns_leak(cfg: &AppConfig, state: &AnonState) -> Result<(), String> {
    let ip_active = state.is_enabled("ip_changer");
    let doh_active = state.is_enabled("doh_shield");
    if !ip_active && !doh_active {
        return Ok(());
    }
    let Ok(content) = fs::read_to_string(&cfg.resolv_conf) else {
        return Ok(());
    };
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("nameserver") {
            if !rest.starts_with(char::is_whitespace) {
                continue;
            }
            if let Some(ns) = rest.split_whitespace().next() {
                if ip_active {
                    if ns != "127.0.0.1" && ns != "::1" && ns != "10.200.200.1" {
                        return Err(format!(
                            "DNS resolver leak ({ns} found in {} while Tor active)",
                            cfg.resolv_conf.display()
                        ));
                    }
                } else if doh_active && ns != "127.0.2.1" && ns != "127.0.0.1" && ns != "::1" {
                    return Err(format!(
                        "DNS resolver leak ({ns} found in {} while DoH Shield active)",
                        cfg.resolv_conf.display()
                    ));
                }
            }
        }
    }
    Ok(())
}

fn check_tor_liveness(state: &AnonState) -> Result<(), String> {
    if state.is_enabled("ip_changer") && !is_tor_running() {
        return Err("Tor service became inactive while IP Changer is enabled".to_string());
    }
    Ok(())
}

fn check_ip_leak(state: &AnonState) -> Result<(), String> {
    if !state.is_enabled("ip_changer") {
        return Ok(());
    }
    if let Some(res) = run_capture(
        "curl",
        &["-s", "--max-time", "10", "https://check.torproject.org/api/ip"],
    ) {
        if !res.is_empty() {
            if res.contains("\"IsTor\":false") {
                return Err(
                    "Direct clearnet egress detected (IsTor: false returned from Tor check)"
                        .to_string(),
                );
            } else if !res.contains("\"IsTor\":true") {
                eprintln!("[ANON LEAK AUDITOR] WARNING: Tor check returned unverified payload.");
            }
        }
    }

    if !is_wsl() && cmd_exists("ip") {
        if let Some(v6_def) = run_capture("ip", &["-6", "route", "show", "default"]) {
            if !v6_def.trim().is_empty() {
                let v6_disabled = fs::read_to_string("/proc/sys/net/ipv6/conf/all/disable_ipv6")
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|_| "0".to_string());
                if v6_disabled != "1" {
                    eprintln!("[ANON LEAK AUDITOR] WARNING: Active IPv6 default route detected while Tor is enabled; locking IPv6.");
                    let _ = sysctl_write("net.ipv6.conf.all.disable_ipv6", "1");
                    let _ = run_quiet("ip6tables", &["-P", "OUTPUT", "DROP"]);
                }
            }
        }
    }
    Ok(())
}

fn trigger_leak_lockdown(cfg: &AppConfig, reason: &str) -> ! {
    eprintln!("\x1b[1;31m[ANON LEAK AUDITOR] *** CRITICAL LEAK DETECTED ***\x1b[0m");
    eprintln!("\x1b[1;31m[ANON LEAK AUDITOR] Reason: {reason}\x1b[0m");
    eprintln!("\x1b[1;33m[ANON LEAK AUDITOR] Enforcing immediate network lockdown...\x1b[0m");

    notify(
        "ANON - CRITICAL LEAK AUDIT",
        &format!("Leak detected: {reason}! Activating kill switch."),
        "critical",
    );

    if cmd_exists("iptables") {
        for p in ["OUTPUT", "INPUT", "FORWARD"] {
            let _ = run_quiet("iptables", &["-P", p, "DROP"]);
        }
    }
    if cmd_exists("ip6tables") {
        for p in ["OUTPUT", "INPUT", "FORWARD"] {
            let _ = run_quiet("ip6tables", &["-P", p, "DROP"]);
        }
    }

    let _ = KillSwitch.start(cfg);
    let _ = set_module_pid(&cfg.state_file, "leak_auditor", 0);
    let _ = set_module_status(&cfg.state_file, "leak_auditor", false);
    eprintln!("\x1b[1;31m[ANON LEAK AUDITOR] Network lockdown enforced. Terminating auditor.\x1b[0m");
    std::process::exit(2);
}

pub fn run_audit_once(cfg: &AppConfig) -> Result<String, (i32, String)> {
    let mut out = String::new();
    out.push_str("\x1b[1;36m[*] ANON Leak Auditor: Running one-shot audit...\x1b[0m\n");
    let state = AnonState::load(&cfg.state_file);

    if let Err(reason) = check_dns_leak(cfg, &state)
        .and_then(|_| check_tor_liveness(&state))
        .and_then(|_| check_ip_leak(&state))
    {
        out.push_str(&format!(
            "[ANON LEAK AUDITOR] CRITICAL LEAK DETECTED: {reason}\n"
        ));
        if cmd_exists("iptables") {
            for p in ["OUTPUT", "INPUT", "FORWARD"] {
                let _ = run_quiet("iptables", &["-P", p, "DROP"]);
            }
        }
        if cmd_exists("ip6tables") {
            for p in ["OUTPUT", "INPUT", "FORWARD"] {
                let _ = run_quiet("ip6tables", &["-P", p, "DROP"]);
            }
        }
        let _ = KillSwitch.start(cfg);
        return Err((2, out));
    }

    out.push_str("\x1b[1;32m[✔] Audit complete: No leaks detected.\x1b[0m\n");
    Ok(out)
}

pub fn run_leak_auditor_daemon(cfg: &AppConfig) -> ! {
    if env::var_os("ANON_STEALTH_ACTIVE").is_some() {
        set_process_name("[kworker/u4:2]");
    }

    let inotify = InotifyWatcher::new();
    if let Some(ref ino) = inotify {
        let _ = ino.watch_path(&cfg.resolv_conf.to_string_lossy());
    }
    let _netlink = NetlinkWatcher::new(RTMGRP_IPV6_ROUTE);
    let wait_ms = (cfg.audit_interval_secs * 1000).min(i32::MAX as u64) as i32;

    loop {
        if let Some(ref ino) = inotify {
            let _ = ino.wait_event(wait_ms);
        } else {
            thread::sleep(Duration::from_secs(cfg.audit_interval_secs));
        }

        let state = AnonState::load(&cfg.state_file);
        if let Err(reason) = check_dns_leak(cfg, &state)
            .and_then(|_| check_tor_liveness(&state))
            .and_then(|_| check_ip_leak(&state))
        {
            trigger_leak_lockdown(cfg, &reason);
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// 3. Volatile Logging (Tails OS tmpfs /var/log)
// ═══════════════════════════════════════════════════════════════════
pub struct VolatileLogging;

impl AnonModule for VolatileLogging {
    fn name(&self) -> &'static str {
        "volatile_logging"
    }

    fn is_live(&self, _cfg: &AppConfig) -> bool {
        is_mountpoint("/var/log")
    }

    fn start(&self, cfg: &AppConfig) -> Result<(), String> {
        let state = AnonState::load(&cfg.state_file);
        if state.is_enabled("volatile_logging") {
            warn("Volatile logging is already active");
            return Ok(());
        }

        info(&format!(
            "Initializing volatile logging (Tails OS paradigm, size: {})...",
            cfg.volatile_log_size
        ));

        if is_mountpoint("/var/log") {
            warn("/var/log is already a mountpoint; skipping tmpfs overlay");
        } else {
            let opts = format!("size={},mode=0755", cfg.volatile_log_size);
            if !run_quiet("mount", &["-t", "tmpfs", "-o", &opts, "tmpfs", "/var/log"]) {
                warn("Failed to mount tmpfs on /var/log");
                return Err("mount tmpfs failed".to_string());
            }
            info(&format!(
                "Mounted tmpfs ({}) over /var/log",
                cfg.volatile_log_size
            ));
        }

        for sub in ["tor", "i2pd", "dnscrypt-proxy", "apt", "cups", "samba", "journal", "private"] {
            let _ = fs::create_dir_all(format!("/var/log/{sub}"));
        }
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions("/var/log/private", fs::Permissions::from_mode(0o700));

        if let Some(u) = resolve_username_any(&["debian-tor", "_tor"]) {
            let _ = run_quiet("chown", &[&format!("{u}:adm"), "/var/log/tor"]);
        }
        if let Some(u) = resolve_username_any(&["_i2pd", "i2pd"]) {
            let _ = run_quiet("chown", &[&format!("{u}:{u}"), "/var/log/i2pd"]);
        }
        if let Some(u) = resolve_username_any(&["_dnscrypt-proxy", "dnscrypt-proxy"]) {
            let _ = run_quiet("chown", &[&u, "/var/log/dnscrypt-proxy"]);
        }

        let _ = service_ctrl("restart", "systemd-journald", None);
        if service_ctrl("is-active", "tor.service", None) {
            let _ = service_ctrl("restart", "tor.service", Some(&cfg.torrc));
        }
        if service_ctrl("is-active", "i2pd", None) {
            let _ = service_ctrl("restart", "i2pd", None);
        }
        if let Some(pids) = run_capture("pidof", &["rsyslogd"]) {
            if let Some(pid) = pids.split_whitespace().next().and_then(|s| s.parse::<i32>().ok()) {
                let _ = send_signal(pid, crate::sys::SIGHUP);
                info("Sent SIGHUP to rsyslog → writing to RAM");
            }
        }

        set_module_status(&cfg.state_file, "volatile_logging", true)?;
        info("Volatile logging enabled — all logs now write to volatile RAM");
        Ok(())
    }

    fn stop(&self, cfg: &AppConfig) -> Result<(), String> {
        info("Restoring persistent logging...");
        if is_mountpoint("/var/log") {
            sync_filesystems();
            if run_quiet("umount", &["-l", "/var/log"]) || run_quiet("umount", &["/var/log"]) {
                info("Unmounted tmpfs from /var/log — historical logs restored");
            } else {
                warn("Could not unmount /var/log — it may still be in use");
            }
        } else {
            info("/var/log is not a tmpfs mountpoint; nothing to unmount");
        }

        let _ = service_ctrl("restart", "systemd-journald", None);
        let _ = service_ctrl("restart", "rsyslog", None);
        if service_ctrl("is-active", "tor.service", None) {
            let _ = service_ctrl("restart", "tor.service", Some(&cfg.torrc));
        }
        if service_ctrl("is-active", "i2pd", None) {
            let _ = service_ctrl("restart", "i2pd", None);
        }

        set_module_status(&cfg.state_file, "volatile_logging", false)?;
        info("Volatile logging disabled");
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════
// 4. Bluetooth Disabler
// ═══════════════════════════════════════════════════════════════════
pub struct BluetoothDisabler;

impl AnonModule for BluetoothDisabler {
    fn name(&self) -> &'static str {
        "bluetooth_disabler"
    }

    fn is_live(&self, cfg: &AppConfig) -> bool {
        cfg.backup_dir.join("bluetooth_was_on").is_file()
    }

    fn start(&self, cfg: &AppConfig) -> Result<(), String> {
        let state = AnonState::load(&cfg.state_file);
        if state.is_enabled("bluetooth_disabler") {
            warn("Bluetooth disabler is already running");
            return Ok(());
        }
        if !cmd_exists("rfkill") {
            warn("rfkill not found; cannot disable Bluetooth.");
            return Err("rfkill missing".to_string());
        }

        let bt_state = run_capture("rfkill", &["list", "bluetooth"]).unwrap_or_default();
        if bt_state.trim().is_empty() {
            warn("No Bluetooth radio detected by rfkill.");
            return Err("no bluetooth".to_string());
        }

        if !bt_state.to_ascii_lowercase().contains("soft blocked: yes") {
            let _ = fs::write(cfg.backup_dir.join("bluetooth_was_on"), "");
        }
        let _ = run_quiet("rfkill", &["block", "bluetooth"]);
        set_module_status(&cfg.state_file, "bluetooth_disabler", true)?;
        info("Bluetooth disabler successfully enabled (Blocked)");
        Ok(())
    }

    fn stop(&self, cfg: &AppConfig) -> Result<(), String> {
        let marker = cfg.backup_dir.join("bluetooth_was_on");
        if marker.is_file() {
            let _ = run_quiet("rfkill", &["unblock", "bluetooth"]);
            let _ = fs::remove_file(marker);
        }
        set_module_status(&cfg.state_file, "bluetooth_disabler", false)?;
        info("Bluetooth disabler successfully disabled (Unblocked)");
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════
// 5. Anti Cold Boot (Swap Lock + Core Dump Lock + Native/sdmem Wipe)
// ═══════════════════════════════════════════════════════════════════
pub struct AntiColdBoot;

pub fn active_swap_devices() -> Vec<String> {
    let mut devs = Vec::new();
    if let Ok(swaps) = fs::read_to_string("/proc/swaps") {
        for line in swaps.lines().skip(1) {
            if let Some(dev) = line.split_whitespace().next() {
                if !dev.is_empty() {
                    devs.push(dev.to_string());
                }
            }
        }
    }
    if devs.is_empty() && cmd_exists("swapon") {
        if let Some(out) = run_capture("swapon", &["--show=NAME", "--noheadings"]) {
            for line in out.lines() {
                let t = line.trim();
                if !t.is_empty() {
                    devs.push(t.to_string());
                }
            }
        }
    }
    devs
}

impl AnonModule for AntiColdBoot {
    fn name(&self) -> &'static str {
        "anti_cold_boot"
    }

    fn is_live(&self, cfg: &AppConfig) -> bool {
        cfg.backup_dir.join("min_free_kbytes.bak").is_file()
    }

    fn start(&self, cfg: &AppConfig) -> Result<(), String> {
        let state = AnonState::load(&cfg.state_file);
        if state.is_enabled("anti_cold_boot") {
            warn("Anti cold boot is already running");
            return Ok(());
        }

        info("Initiating anti cold-boot & anti-forensic memory lockdown...");

        let swaps = active_swap_devices();
        if !swaps.is_empty() {
            let _ = fs::write(cfg.backup_dir.join("swap_was_active"), "");
        }
        let _ = run_quiet("swapoff", &["-a"]);

        if !is_wsl() && !swaps.is_empty() && cmd_exists("sswap") {
            for dev in &swaps {
                let _ = run_quiet("sswap", &[dev]);
            }
        }

        let proc_pairs = [
            ("/proc/sys/vm/min_free_kbytes", "min_free_kbytes.bak"),
            ("/proc/sys/vm/oom_kill_allocating_task", "oom_kill_allocating_task.bak"),
            ("/proc/sys/vm/overcommit_memory", "overcommit_memory.bak"),
            ("/proc/sys/vm/oom_dump_tasks", "oom_dump_tasks.bak"),
        ];
        for (src, dst) in proc_pairs {
            if let Ok(v) = fs::read_to_string(src) {
                let _ = fs::write(cfg.backup_dir.join(dst), v);
            }
        }
        for (key, dst) in [
            ("fs.suid_dumpable", "suid_dumpable.bak"),
            ("kernel.core_pattern", "core_pattern.bak"),
        ] {
            if let Some(v) = sysctl_read(key) {
                let _ = fs::write(cfg.backup_dir.join(dst), format!("{v}\n"));
            }
        }

        let _ = sysctl_write("vm.min_free_kbytes", "1024");
        let _ = sysctl_write("vm.drop_caches", "3");
        let _ = sysctl_write("vm.oom_kill_allocating_task", "1");
        let _ = sysctl_write("vm.overcommit_memory", "1");
        let _ = sysctl_write("vm.oom_dump_tasks", "0");
        let _ = sysctl_write("fs.suid_dumpable", "0");
        let _ = sysctl_write("kernel.core_pattern", "|/bin/false");

        if is_wsl() {
            warn("WSL environment detected! Skipping RAM/Swap wipe to prevent fatal E_UNEXPECTED host crash.");
        } else if cmd_exists("sdmem") {
            info("Wiping free RAM securely (this may take a moment)...");
            let _ = Command::new("sdmem")
                .args(["-f", "-ll"])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
        } else {
            info("Performing native Rust RAM page zeroization...");
            wipe_free_ram_native(128);
        }

        set_module_status(&cfg.state_file, "anti_cold_boot", true)?;
        info("Anti cold boot & memory lockdown successfully enabled (Swap locked OFF)");
        Ok(())
    }

    fn stop(&self, cfg: &AppConfig) -> Result<(), String> {
        let restore = [
            ("vm.min_free_kbytes", "min_free_kbytes.bak", "67584"),
            ("vm.oom_kill_allocating_task", "oom_kill_allocating_task.bak", "0"),
            ("vm.overcommit_memory", "overcommit_memory.bak", "0"),
            ("vm.oom_dump_tasks", "oom_dump_tasks.bak", "1"),
            ("fs.suid_dumpable", "suid_dumpable.bak", "2"),
            ("kernel.core_pattern", "core_pattern.bak", "core"),
        ];
        for (key, fname, default_val) in restore {
            let p = cfg.backup_dir.join(fname);
            let val = fs::read_to_string(&p)
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| default_val.to_string());
            let _ = sysctl_write(key, &val);
            let _ = fs::remove_file(p);
        }

        let swap_marker = cfg.backup_dir.join("swap_was_active");
        if swap_marker.is_file() {
            let _ = run_quiet("swapon", &["-a"]);
            let _ = fs::remove_file(swap_marker);
        }

        set_module_status(&cfg.state_file, "anti_cold_boot", false)?;
        info("Anti cold boot successfully disabled");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::set_module_status;

    #[test]
    fn lladdr_and_mac_validation() {
        let sample = "192.168.1.1 dev eth0 lladdr AA:BB:CC:DD:EE:FF router REACHABLE";
        let mac = extract_lladdr(sample).unwrap();
        assert_eq!(mac, "aa:bb:cc:dd:ee:ff");
        assert!(is_valid_mac(&mac));
        assert!(!is_valid_mac("invalid"));
    }

    #[test]
    fn dns_leak_check_enforces_tor_and_doh_isolation() {
        let mut dir = std::env::temp_dir();
        dir.push(format!("anon_dns_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let state_file = dir.join("anon.state");
        let resolv = dir.join("resolv.conf");
        let mut cfg = AppConfig::load().unwrap();
        cfg.state_file = state_file.clone();
        cfg.resolv_conf = resolv.clone();

        set_module_status(&state_file, "ip_changer", true).unwrap();
        fs::write(&resolv, "nameserver 1.1.1.1\n").unwrap();
        let st = AnonState::load(&state_file);
        assert!(check_dns_leak(&cfg, &st).is_err());

        fs::write(&resolv, "nameserver 127.0.0.1\n").unwrap();
        assert!(check_dns_leak(&cfg, &st).is_ok());

        set_module_status(&state_file, "ip_changer", false).unwrap();
        set_module_status(&state_file, "doh_shield", true).unwrap();
        let st2 = AnonState::load(&state_file);
        assert!(check_dns_leak(&cfg, &st2).is_ok());
        fs::write(&resolv, "nameserver 8.8.8.8\n").unwrap();
        assert!(check_dns_leak(&cfg, &st2).is_err());
        fs::write(&resolv, "nameserver 127.0.2.1\n").unwrap();
        assert!(check_dns_leak(&cfg, &st2).is_ok());

        let _ = fs::remove_dir_all(&dir);
    }
}
