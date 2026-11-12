use std::env;
use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use crate::config::AppConfig;
use crate::helpers::{
    cmd_exists, find_in_path, get_default_iface, is_netns_active, is_wsl, rand_u64, run_capture,
    run_quiet, service_ctrl, shred_dir_recursive, sysctl_read, sysctl_write,
};
use crate::i18n::t;
use crate::registry::AnonModule;
use crate::state::{set_module_pid, set_module_status, AnonState};
use crate::sys::{is_pid_alive, send_signal, set_process_name, SIGKILL, SIGTERM};
use crate::ui::{info, msg, warn};

const BROWSER_PREF_DIRS: [&str; 7] = [
    "/etc/firefox-esr",
    "/etc/firefox",
    "/etc/firefox/pref",
    "/etc/librewolf",
    "/usr/lib/firefox-esr/defaults/pref",
    "/usr/lib/firefox/defaults/pref",
    "/usr/lib/librewolf/defaults/pref",
];

pub const ANON_JS_EMBEDDED: &str = include_str!("../../assets/sources/anon.js");

const TIMEZONES: [&str; 8] = [
    "America/New_York",
    "Asia/Tokyo",
    "Europe/London",
    "Australia/Sydney",
    "Europe/Berlin",
    "Asia/Dubai",
    "America/Los_Angeles",
    "UTC",
];

// ═══════════════════════════════════════════════════════════════════
// 1. Timezone Changer
// ═══════════════════════════════════════════════════════════════════
pub struct TimezoneChanger;

fn detect_current_timezone() -> String {
    if cmd_exists("timedatectl") {
        if let Some(out) = run_capture("timedatectl", &["show", "--property=Timezone", "--value"]) {
            let tz = out.trim().to_string();
            if !tz.is_empty() {
                return tz;
            }
        }
    }
    if let Ok(tz) = fs::read_to_string("/etc/timezone") {
        let trimmed: String = tz.chars().filter(|c| !c.is_whitespace()).collect();
        if !trimmed.is_empty() {
            return trimmed;
        }
    }
    if let Ok(target) = fs::canonicalize("/etc/localtime") {
        let s = target.to_string_lossy();
        if let Some((_, zone)) = s.rsplit_once("/zoneinfo/") {
            if !zone.is_empty() {
                return zone.to_string();
            }
        }
    }
    "UTC".to_string()
}

pub fn apply_timezone(target_tz: &str) -> bool {
    let mut applied = false;
    if cmd_exists("timedatectl") && run_quiet("timedatectl", &["set-timezone", target_tz]) {
        applied = true;
    }
    let zone_path = format!("/usr/share/zoneinfo/{target_tz}");
    if Path::new(&zone_path).is_file() {
        let _ = fs::remove_file("/etc/localtime");
        if symlink(&zone_path, "/etc/localtime").is_ok() {
            let _ = fs::write("/etc/timezone", format!("{target_tz}\n"));
            applied = true;
        }
    }
    applied
}

impl AnonModule for TimezoneChanger {
    fn name(&self) -> &'static str {
        "timezone_changer"
    }

    fn is_live(&self, cfg: &AppConfig) -> bool {
        cfg.backup_dir.join("timezone.bak").is_file()
    }

    fn start(&self, cfg: &AppConfig) -> Result<(), String> {
        let state = AnonState::load(&cfg.state_file);
        if state.is_enabled("timezone_changer") {
            warn(t(
                "Timezone changer is already running",
                "Saat dilimi değiştirici zaten çalışıyor"
            ));
            return Ok(());
        }
        let orig = detect_current_timezone();
        let bak = cfg.backup_dir.join("timezone.bak");
        let _ = fs::write(&bak, format!("{orig}\n"));

        let selected = TIMEZONES[(rand_u64() as usize) % TIMEZONES.len()];
        if apply_timezone(selected) {
            set_module_status(&cfg.state_file, "timezone_changer", true)?;
            info(&format!(
                "{} ({}: {selected})",
                t(
                    "Timezone changer successfully enabled",
                    "Saat dilimi değiştirici başarıyla etkinleştirildi"
                ),
                t("Spoofed to", "Sahte saat dilimi")
            ));
        } else {
            warn(&format!(
                "{} ({selected}); {}",
                t(
                    "Failed to apply spoofed timezone",
                    "Sahte saat dilimi uygulanamadı"
                ),
                t(
                    "timedatectl or zoneinfo unavailable.",
                    "timedatectl veya zoneinfo kullanılamıyor."
                )
            ));
            let _ = fs::remove_file(bak);
        }
        Ok(())
    }

    fn stop(&self, cfg: &AppConfig) -> Result<(), String> {
        let bak = cfg.backup_dir.join("timezone.bak");
        if bak.is_file() {
            let orig = fs::read_to_string(&bak)
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "UTC".to_string());
            let _ = apply_timezone(&orig);
            let _ = fs::remove_file(bak);
        }
        set_module_status(&cfg.state_file, "timezone_changer", false)?;
        info(t(
            "Timezone changer successfully disabled",
            "Saat dilimi değiştirici başarıyla devre dışı bırakıldı"
        ));
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════
// 2. Hostname Changer
// ═══════════════════════════════════════════════════════════════════
pub struct HostnameChanger;

const HOST_PREFIXES: [&str; 5] = ["DESKTOP-", "LAPTOP-", "MSI-", "PC-", "WIN-"];

impl AnonModule for HostnameChanger {
    fn name(&self) -> &'static str {
        "hostname_changer"
    }

    fn is_live(&self, cfg: &AppConfig) -> bool {
        cfg.backup_dir.join("hostname.bak").is_file()
    }

    fn start(&self, cfg: &AppConfig) -> Result<(), String> {
        let state = AnonState::load(&cfg.state_file);
        if state.is_enabled("hostname_changer") {
            warn(t(
                "Hostname changer is already running",
                "Ana makine adı değiştirici zaten çalışıyor"
            ));
            return Ok(());
        }

        let orig_hostname = fs::read_to_string("/etc/hostname")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| run_capture("hostname", &[]))
            .unwrap_or_else(|| "localhost\n".to_string());
        let _ = fs::write(cfg.backup_dir.join("hostname.bak"), orig_hostname);

        if let Ok(hosts) = fs::read(&cfg.hosts_file) {
            let _ = fs::write(cfg.backup_dir.join("hosts.bak"), hosts);
        }
        let dhclient = Path::new("/etc/dhcp/dhclient.conf");
        if let Ok(dh) = fs::read(dhclient) {
            let _ = fs::write(cfg.backup_dir.join("dhclient.conf.bak"), dh);
        }

        let r = rand_u64();
        let pfx = HOST_PREFIXES[(r as usize) % HOST_PREFIXES.len()];
        let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        let mut suffix = String::with_capacity(7);
        let mut v = rand_u64();
        for i in 0..7 {
            if i == 4 {
                v = rand_u64();
            }
            suffix.push(alphabet[(v as usize) % alphabet.len()] as char);
            v >>= 6;
        }
        let new_hostname = format!("{pfx}{suffix}");

        let _ = fs::write("/etc/hostname", format!("{new_hostname}\n"));
        let _ = run_quiet("hostname", &[&new_hostname]);
        if cmd_exists("hostnamectl") {
            let _ = run_quiet("hostnamectl", &["set-hostname", &new_hostname]);
        }

        if let Ok(hosts_str) = fs::read_to_string(&cfg.hosts_file) {
            let mut found = false;
            let mut out = String::with_capacity(hosts_str.len() + 32);
            for line in hosts_str.lines() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("127.0.1.1")
                    && trimmed["127.0.1.1".len()..].starts_with(char::is_whitespace)
                {
                    out.push_str(&format!("127.0.1.1\t{new_hostname}\n"));
                    found = true;
                } else {
                    out.push_str(line);
                    out.push('\n');
                }
            }
            if !found {
                out.push_str(&format!("127.0.1.1\t{new_hostname}\n"));
            }
            let _ = fs::write(&cfg.hosts_file, out);
        }

        if let Ok(dh_str) = fs::read_to_string(dhclient) {
            let mut found = false;
            let mut out = String::with_capacity(dh_str.len() + 32);
            for line in dh_str.lines() {
                let mut parts = line.split_whitespace();
                if parts.next() == Some("send") && parts.next() == Some("host-name") {
                    out.push_str(&format!("send host-name \"{new_hostname}\";\n"));
                    found = true;
                } else {
                    out.push_str(line);
                    out.push('\n');
                }
            }
            if !found {
                out.push_str(&format!("send host-name \"{new_hostname}\";\n"));
            }
            let _ = fs::write(dhclient, out);
        }

        set_module_status(&cfg.state_file, "hostname_changer", true)?;
        info(&format!(
            "{} ({}: {new_hostname})",
            t(
                "Hostname changer successfully enabled",
                "Ana makine adı değiştirici başarıyla etkinleştirildi"
            ),
            t("Spoofed to", "Sahte ad")
        ));
        Ok(())
    }

    fn stop(&self, cfg: &AppConfig) -> Result<(), String> {
        let host_bak = cfg.backup_dir.join("hostname.bak");
        if let Ok(orig) = fs::read_to_string(&host_bak) {
            let orig_host = orig.trim();
            let target = if orig_host.is_empty() {
                "localhost"
            } else {
                orig_host
            };
            let _ = fs::write("/etc/hostname", format!("{target}\n"));
            let _ = run_quiet("hostname", &[target]);
            if cmd_exists("hostnamectl") {
                let _ = run_quiet("hostnamectl", &["set-hostname", target]);
            }
            let _ = fs::remove_file(host_bak);
        }

        let hosts_bak = cfg.backup_dir.join("hosts.bak");
        if hosts_bak.is_file() {
            if let Ok(data) = fs::read(&hosts_bak) {
                let _ = fs::write(&cfg.hosts_file, data);
            }
            let _ = fs::remove_file(hosts_bak);
        }

        let dh_bak = cfg.backup_dir.join("dhclient.conf.bak");
        if dh_bak.is_file() {
            if let Ok(data) = fs::read(&dh_bak) {
                let _ = fs::write("/etc/dhcp/dhclient.conf", data);
            }
            let _ = fs::remove_file(dh_bak);
        }

        set_module_status(&cfg.state_file, "hostname_changer", false)?;
        info(t(
            "Hostname changer successfully disabled",
            "Ana makine adı değiştirici başarıyla devre dışı bırakıldı"
        ));
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════
// 3. Browser Anonymization
// ═══════════════════════════════════════════════════════════════════
pub struct BrowserAnonymization;

impl AnonModule for BrowserAnonymization {
    fn name(&self) -> &'static str {
        "browser_anonymization"
    }

    fn is_live(&self, cfg: &AppConfig) -> bool {
        AnonState::load(&cfg.state_file).is_enabled("browser_anonymization")
    }

    fn start(&self, cfg: &AppConfig) -> Result<(), String> {
        let state = AnonState::load(&cfg.state_file);
        if state.is_enabled("browser_anonymization") {
            warn(t(
                "Browser anonymization is already running",
                "Tarayıcı anonimleştirme zaten çalışıyor"
            ));
            return Ok(());
        }

        let anon_js_path = cfg.src_dir.join("sources/anon.js");
        let js_bytes = fs::read(&anon_js_path).unwrap_or_else(|_| ANON_JS_EMBEDDED.as_bytes().to_vec());

        let mut installed = false;
        for dir in BROWSER_PREF_DIRS {
            let d = Path::new(dir);
            if d.is_dir() && fs::write(d.join("anon.js"), &js_bytes).is_ok() {
                installed = true;
            }
        }

        if installed {
            set_module_status(&cfg.state_file, "browser_anonymization", true)?;
            info(t(
                "Browser anonymization successfully enabled",
                "Tarayıcı anonimleştirme başarıyla etkinleştirildi"
            ));
        } else {
            warn(t(
                "No supported Firefox/LibreWolf directory found (checked /etc/firefox-esr, /etc/firefox, /etc/librewolf, etc.)",
                "Desteklenen Firefox/LibreWolf dizini bulunamadı (/etc/firefox-esr, /etc/firefox, /etc/librewolf vb. kontrol edildi)"
            ));
        }
        Ok(())
    }

    fn stop(&self, cfg: &AppConfig) -> Result<(), String> {
        for dir in BROWSER_PREF_DIRS {
            let _ = fs::remove_file(Path::new(dir).join("anon.js"));
        }
        set_module_status(&cfg.state_file, "browser_anonymization", false)?;
        info(t(
            "Browser anonymization successfully disabled",
            "Tarayıcı anonimleştirme başarıyla devre dışı bırakıldı"
        ));
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════
// 4. OS Obfuscation
// ═══════════════════════════════════════════════════════════════════
pub struct OsObfuscation;

impl AnonModule for OsObfuscation {
    fn name(&self) -> &'static str {
        "os_obfuscation"
    }

    fn is_live(&self, cfg: &AppConfig) -> bool {
        cfg.backup_dir.join("ip_default_ttl.bak").is_file()
    }

    fn start(&self, cfg: &AppConfig) -> Result<(), String> {
        let state = AnonState::load(&cfg.state_file);
        if state.is_enabled("os_obfuscation") {
            warn(t(
                "OS Obfuscation is already running",
                "İşletim Sistemi İz Gizleme zaten çalışıyor"
            ));
            return Ok(());
        }

        let pairs = [
            ("net.ipv4.ip_default_ttl", "ip_default_ttl.bak"),
            ("net.ipv4.tcp_timestamps", "tcp_timestamps.bak"),
            ("net.ipv4.ip_local_port_range", "ip_local_port_range.bak"),
            ("net.ipv4.tcp_window_scaling", "tcp_window_scaling.bak"),
            ("net.ipv4.icmp_echo_ignore_broadcasts", "icmp_echo_ignore_broadcasts.bak"),
            ("net.ipv4.tcp_sack", "tcp_sack.bak"),
        ];
        for (key, fname) in pairs {
            if let Some(val) = sysctl_read(key) {
                let _ = fs::write(cfg.backup_dir.join(fname), format!("{val}\n"));
            } else {
                let _ = fs::write(cfg.backup_dir.join(fname), "");
            }
        }

        let _ = sysctl_write("net.ipv4.ip_default_ttl", "128");
        if !run_quiet("iptables", &["-t", "mangle", "-C", "POSTROUTING", "-j", "TTL", "--ttl-set", "128"]) {
            let _ = run_quiet("iptables", &["-t", "mangle", "-A", "POSTROUTING", "-j", "TTL", "--ttl-set", "128"]);
        }
        let _ = sysctl_write("net.ipv4.tcp_timestamps", "0");
        let _ = sysctl_write("net.ipv4.ip_local_port_range", "49152 65535");
        let _ = sysctl_write("net.ipv4.tcp_window_scaling", "0");
        let _ = sysctl_write("net.ipv4.icmp_echo_ignore_broadcasts", "1");
        let _ = sysctl_write("net.ipv4.tcp_sack", "0");

        if !run_quiet(
            "iptables",
            &[
                "-t", "mangle", "-C", "POSTROUTING", "-p", "tcp", "--tcp-flags", "SYN,RST", "SYN",
                "-j", "TCPMSS", "--set-mss", "1460",
            ],
        ) {
            let _ = run_quiet(
                "iptables",
                &[
                    "-t", "mangle", "-A", "POSTROUTING", "-p", "tcp", "--tcp-flags", "SYN,RST",
                    "SYN", "-j", "TCPMSS", "--set-mss", "1460",
                ],
            );
        }

        set_module_status(&cfg.state_file, "os_obfuscation", true)?;
        info(t(
            "OS Obfuscation enabled (Windows 10/11 fingerprint emulation)",
            "İşletim Sistemi İz Gizleme etkinleştirildi (Windows 10/11 parmak izi taklidi)"
        ));
        Ok(())
    }

    fn stop(&self, cfg: &AppConfig) -> Result<(), String> {
        let restore_pairs = [
            ("net.ipv4.ip_default_ttl", "ip_default_ttl.bak", "64"),
            ("net.ipv4.tcp_timestamps", "tcp_timestamps.bak", "1"),
            ("net.ipv4.ip_local_port_range", "ip_local_port_range.bak", "32768 60999"),
            ("net.ipv4.tcp_window_scaling", "tcp_window_scaling.bak", "1"),
            ("net.ipv4.icmp_echo_ignore_broadcasts", "icmp_echo_ignore_broadcasts.bak", "1"),
            ("net.ipv4.tcp_sack", "tcp_sack.bak", "1"),
        ];
        for (key, fname, default_val) in restore_pairs {
            let p = cfg.backup_dir.join(fname);
            let val = fs::read_to_string(&p)
                .ok()
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| default_val.to_string());
            let _ = sysctl_write(key, &val);
            let _ = fs::remove_file(p);
        }

        let _ = run_quiet("iptables", &["-t", "mangle", "-D", "POSTROUTING", "-j", "TTL", "--ttl-set", "128"]);
        let _ = run_quiet(
            "iptables",
            &[
                "-t", "mangle", "-D", "POSTROUTING", "-p", "tcp", "--tcp-flags", "SYN,RST", "SYN",
                "-j", "TCPMSS", "--set-mss", "1460",
            ],
        );

        set_module_status(&cfg.state_file, "os_obfuscation", false)?;
        info(t(
            "OS Obfuscation disabled — original TCP/IP fingerprint restored",
            "İşletim Sistemi İz Gizleme devre dışı bırakıldı — orijinal TCP/IP parmak izi geri yüklendi"
        ));
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════
// 5. Process Obfuscation (hidepid=2 + RAM stealth wrappers)
// ═══════════════════════════════════════════════════════════════════
pub struct ProcessObfuscation;

impl AnonModule for ProcessObfuscation {
    fn name(&self) -> &'static str {
        "process_obfuscation"
    }

    fn is_live(&self, cfg: &AppConfig) -> bool {
        cfg.backup_dir.join("proc_mount_opts.bak").is_file()
    }

    fn start(&self, cfg: &AppConfig) -> Result<(), String> {
        let state = AnonState::load(&cfg.state_file);
        if state.is_enabled("process_obfuscation") {
            warn(t(
                "Process obfuscation is already running",
                "Süreç gizleme zaten çalışıyor"
            ));
            return Ok(());
        }

        let orig_opts = fs::read_to_string("/proc/mounts")
            .ok()
            .and_then(|m| {
                m.lines()
                    .find(|l| l.split_whitespace().nth(1) == Some("/proc"))
                    .and_then(|l| l.split_whitespace().nth(3).map(str::to_string))
            })
            .unwrap_or_else(|| "rw".to_string());
        let _ = fs::write(cfg.backup_dir.join("proc_mount_opts.bak"), format!("{orig_opts}\n"));

        info(t(
            "Remounting /proc with hidepid=2...",
            "/proc dizini hidepid=2 ile yeniden bağlanıyor..."
        ));
        if run_quiet("mount", &["-o", "remount,rw,hidepid=2", "/proc"]) {
            info(t(
                "/proc remounted with hidepid=2 — process isolation active",
                "/proc hidepid=2 ile yeniden bağlandı — süreç izolasyonu etkin"
            ));
        } else {
            warn(t(
                "Could not remount /proc with hidepid=2 (kernel may not support it)",
                "/proc hidepid=2 ile yeniden bağlanamadı (çekirdek desteklemiyor olabilir)"
            ));
        }

        let ramdir = format!("/dev/shm/.systemd_kworker_{:06x}", rand_u64() & 0xff_ffff);
        if fs::create_dir_all(&ramdir).is_ok() {
            let _ = fs::set_permissions(&ramdir, fs::Permissions::from_mode(0o700));
            let _ = fs::write(
                cfg.backup_dir.join("process_obfuscation_ramdir.bak"),
                format!("{ramdir}\n"),
            );
        }

        let _ = fs::create_dir_all(&cfg.wrapper_dir);

        if Path::new("/usr/bin/nmap").exists() {
            let local_nmap = cfg.wrapper_dir.join("nmap");
            if local_nmap.exists()
                && !fs::read_to_string(&local_nmap)
                    .map(|s| s.contains("ANON_PROC_WRAPPER"))
                    .unwrap_or(false)
            {
                let _ = fs::copy(&local_nmap, cfg.backup_dir.join("local_bin_nmap.bak"));
            }
            let target_kworker = PathBuf::from(&ramdir).join("kworker");
            if fs::copy("/usr/bin/nmap", &target_kworker).is_ok() {
                let _ = fs::set_permissions(&target_kworker, fs::Permissions::from_mode(0o755));
                let wrapper = format!(
                    "#!/bin/bash\n# ANON_PROC_WRAPPER\nexec -a \"[kworker/u4:2]\" \"{}\" \"$@\"\n",
                    target_kworker.display()
                );
                if fs::write(&local_nmap, wrapper).is_ok() {
                    let _ = fs::set_permissions(&local_nmap, fs::Permissions::from_mode(0o755));
                }
            }
        }

        if Path::new("/usr/bin/sqlmap").exists() {
            let local_sqlmap = cfg.wrapper_dir.join("sqlmap");
            if local_sqlmap.exists()
                && !fs::read_to_string(&local_sqlmap)
                    .map(|s| s.contains("ANON_PROC_WRAPPER"))
                    .unwrap_or(false)
            {
                let _ = fs::copy(&local_sqlmap, cfg.backup_dir.join("local_bin_sqlmap.bak"));
            }
            let is_elf = fs::read("/usr/bin/sqlmap")
                .map(|b| b.starts_with(b"\x7fELF"))
                .unwrap_or(false);
            let wrapper = if is_elf {
                let target_res = PathBuf::from(&ramdir).join("systemd-resolve");
                let _ = fs::copy("/usr/bin/sqlmap", &target_res);
                let _ = fs::set_permissions(&target_res, fs::Permissions::from_mode(0o755));
                format!(
                    "#!/bin/bash\n# ANON_PROC_WRAPPER\nexec -a \"/lib/systemd/systemd-resolved\" \"{}\" \"$@\"\n",
                    target_res.display()
                )
            } else {
                "#!/bin/bash\n# ANON_PROC_WRAPPER\nexec -a \"/lib/systemd/systemd-resolved\" /usr/bin/sqlmap \"$@\"\n".to_string()
            };
            if fs::write(&local_sqlmap, wrapper).is_ok() {
                let _ = fs::set_permissions(&local_sqlmap, fs::Permissions::from_mode(0o755));
            }
        }

        set_module_status(&cfg.state_file, "process_obfuscation", true)?;
        info(t(
            "Process obfuscation enabled (hidepid=2 + RAM-based stealth execution)",
            "Süreç gizleme etkinleştirildi (hidepid=2 + RAM tabanlı gizli yürütme)"
        ));
        Ok(())
    }

    fn stop(&self, cfg: &AppConfig) -> Result<(), String> {
        let mut restore_opts = "rw,hidepid=0".to_string();
        let bak = cfg.backup_dir.join("proc_mount_opts.bak");
        if let Ok(saved) = fs::read_to_string(&bak) {
            let s = saved.trim();
            if !s.is_empty() {
                if s.contains("hidepid=") {
                    restore_opts = s.to_string();
                } else {
                    restore_opts = format!("{s},hidepid=0");
                }
            }
            let _ = fs::remove_file(bak);
        }

        if run_quiet("mount", &["-o", &format!("remount,{restore_opts}"), "/proc"]) {
            info(&format!(
                "{} ({restore_opts})",
                t(
                    "/proc mount options restored",
                    "/proc bağlama seçenekleri geri yüklendi"
                )
            ));
        } else {
            warn(t(
                "Could not restore /proc mount options",
                "/proc bağlama seçenekleri geri yüklenemedi"
            ));
        }

        for tool in ["nmap", "sqlmap"] {
            let bin_path = cfg.wrapper_dir.join(tool);
            let bak_path = cfg.backup_dir.join(format!("local_bin_{tool}.bak"));
            if fs::read_to_string(&bin_path)
                .map(|s| s.contains("ANON_PROC_WRAPPER") || s.contains("systemd-kworker"))
                .unwrap_or(false)
            {
                let _ = fs::remove_file(&bin_path);
                if bak_path.exists() {
                    let _ = fs::rename(&bak_path, &bin_path);
                }
            }
            let _ = fs::remove_file(&bak_path);
        }

        let ramdir_bak = cfg.backup_dir.join("process_obfuscation_ramdir.bak");
        if let Ok(ramdir) = fs::read_to_string(&ramdir_bak) {
            let trimmed = ramdir.trim();
            if trimmed.starts_with("/dev/shm/.") {
                shred_dir_recursive(Path::new(trimmed));
            }
            let _ = fs::remove_file(ramdir_bak);
        }
        shred_dir_recursive(Path::new("/dev/shm/.systemd-kworker"));

        set_module_status(&cfg.state_file, "process_obfuscation", false)?;
        info(t(
            "Process obfuscation disabled",
            "Süreç gizleme devre dışı bırakıldı"
        ));
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════
// 6. Sandbox Isolation (Kernel netns + AppArmor + Firejail)
// ═══════════════════════════════════════════════════════════════════
pub struct SandboxIsolation;

const APPARMOR_TOR_CONTENT: &str = r#"# AppArmor profile for Tor — ANON Framework
# Restricts Tor (and pluggable transports) to only required resources

#include <tunables/global>

/usr/bin/tor {
    #include <abstractions/base>
    #include <abstractions/nameservice>
    #include <abstractions/openssl>

    capability net_bind_service,
    capability setuid,
    capability setgid,
    capability dac_override,
    capability chown,

    # Tor binary & pluggable transports
    /usr/bin/tor                  mr,
    /usr/sbin/tor                 mr,
    /usr/bin/obfs4proxy           ix,
    /usr/bin/snowflake-client     ix,

    # Configuration, entropy & runtime
    /etc/tor/**                   r,
    /usr/share/tor/**             r,
    /dev/urandom                  r,
    /var/lib/tor/**               rw,
    /var/lib/tor/                 rw,
    /var/log/tor/**               rw,
    /var/log/tor/                 rw,
    /run/tor/**                   rw,
    /run/tor/                     rw,

    # Network access
    network inet stream,
    network inet dgram,
    network inet6 stream,
    network inet6 dgram,
}
"#;

impl AnonModule for SandboxIsolation {
    fn name(&self) -> &'static str {
        "sandbox_isolation"
    }

    fn is_live(&self, _cfg: &AppConfig) -> bool {
        is_netns_active("anon_jail")
    }

    fn start(&self, cfg: &AppConfig) -> Result<(), String> {
        let state = AnonState::load(&cfg.state_file);
        if state.is_enabled("sandbox_isolation") {
            warn(t(
                "Sandbox Isolation is already active",
                "İzolasyon Alanı zaten etkin"
            ));
            return Ok(());
        }

        if let Some(routes) = run_capture("ip", &["-4", "route", "show"]) {
            if routes.contains("10.200.200.") {
                warn(t(
                    "Existing route overlaps 10.200.200.0/24; namespace routing may conflict.",
                    "Mevcut rota 10.200.200.0/24 ile çakışıyor; ad alanı yönlendirmesi çakışabilir."
                ));
            }
        }

        info(t(
            "Setting up Kernel-level Network Namespace + MAC enforcement...",
            "Çekirdek düzeyinde Ağ Ad Alanı + MAC zorlaması kuruluyor..."
        ));

        let _ = run_quiet("ip", &["link", "del", "veth-anon"]);
        let _ = run_quiet("ip", &["netns", "del", "anon_jail"]);

        let _ = run_quiet("ip", &["netns", "add", "anon_jail"]);
        let _ = run_quiet("ip", &["netns", "exec", "anon_jail", "ip", "link", "set", "lo", "up"]);

        let _ = fs::create_dir_all(&cfg.netns_etc_dir);
        let _ = fs::write(
            cfg.netns_etc_dir.join("resolv.conf"),
            "# ANON Sandbox Isolation DNS (routed via veth-anon -> Tor DNSPort)\nnameserver 10.200.200.1\n",
        );

        let _ = run_quiet("ip", &["link", "add", "veth-anon", "type", "veth", "peer", "name", "veth-jail"]);
        let _ = run_quiet("ip", &["link", "set", "veth-jail", "netns", "anon_jail"]);

        let _ = run_quiet("ip", &["addr", "add", "10.200.200.1/24", "dev", "veth-anon"]);
        let _ = run_quiet("ip", &["link", "set", "veth-anon", "up"]);

        let _ = run_quiet("ip", &["netns", "exec", "anon_jail", "ip", "addr", "add", "10.200.200.2/24", "dev", "veth-jail"]);
        let _ = run_quiet("ip", &["netns", "exec", "anon_jail", "ip", "link", "set", "veth-jail", "up"]);
        let _ = run_quiet("ip", &["netns", "exec", "anon_jail", "ip", "route", "add", "default", "via", "10.200.200.1"]);

        crate::modules::network::apply_sandbox_tor_rules();

        if !is_wsl() {
            let _ = sysctl_write("net.ipv4.ip_forward", "1");
        }

        if cfg.torrc.is_file() {
            if let Ok(mut content) = fs::read_to_string(&cfg.torrc) {
                if !content.contains("10.200.200.1:9040") {
                    if !content.is_empty() && !content.ends_with('\n') {
                        content.push('\n');
                    }
                    content.push_str(
                        "TransPort 10.200.200.1:9040 IsolateClientAddr IsolateClientProtocol IsolateDestAddr IsolateDestPort # ANON_SANDBOX\n\
                         SocksPort 10.200.200.1:9050 # ANON_SANDBOX\n\
                         DNSPort 10.200.200.1:5353 # ANON_SANDBOX\n",
                    );
                    let _ = fs::write(&cfg.torrc, content);
                    if service_ctrl("is-active", "tor.service", None) {
                        let _ = service_ctrl("restart", "tor.service", Some(&cfg.torrc));
                    }
                }
            }
        }

        info(t(
            "Network namespace 'anon_jail' created and routed through Tor",
            "'anon_jail' ağ ad alanı oluşturuldu ve Tor üzerinden yönlendirildi"
        ));

        // AppArmor MAC profile
        if cmd_exists("apparmor_parser") {
            if let Some(p) = cfg.apparmor_tor_profile.parent() {
                let _ = fs::create_dir_all(p);
            }
            if fs::write(&cfg.apparmor_tor_profile, APPARMOR_TOR_CONTENT).is_ok() {
                if run_quiet("apparmor_parser", &["-r", &cfg.apparmor_tor_profile.to_string_lossy()]) {
                    info(t(
                        "AppArmor profile loaded for /usr/bin/tor",
                        "/usr/bin/tor için AppArmor profili yüklendi"
                    ));
                } else {
                    warn(t(
                        "Failed to load AppArmor profile for Tor",
                        "Tor için AppArmor profili yüklenemedi"
                    ));
                }
            }
        } else {
            info(t(
                "AppArmor not available — skipping MAC profile installation",
                "AppArmor mevcut değil — MAC profil kurulumu atlanıyor"
            ));
        }

        // Firejail wrappers
        if cmd_exists("firejail") {
            let _ = fs::create_dir_all(&cfg.wrapper_dir);
            for tool in ["curl", "wget", "nmap"] {
                let Some(real_path) = find_in_path(tool) else {
                    continue;
                };
                if real_path.starts_with(&cfg.wrapper_dir) {
                    continue;
                }
                if fs::read_to_string(&real_path)
                    .map(|s| s.contains("firejail"))
                    .unwrap_or(false)
                {
                    continue;
                }
                let wrap_path = cfg.wrapper_dir.join(format!("{tool}.anon-sandbox"));
                let content = format!(
                    "#!/bin/bash\nexec firejail --quiet --caps.drop=all --no3d --nodvd --nosound \\\n    --nogroups --nonewprivs --seccomp \\\n    \"{}\" \"$@\"\n",
                    real_path.display()
                );
                if fs::write(&wrap_path, content).is_ok() {
                    let _ = fs::set_permissions(&wrap_path, fs::Permissions::from_mode(0o755));
                }
            }
            info(t(
                "Firejail sandboxed wrappers created for external tools",
                "Harici araçlar için Firejail sarmalayıcıları oluşturuldu"
            ));
        } else {
            info(t(
                "Firejail not available — skipping sandboxed tool wrappers",
                "Firejail mevcut değil — sarmalanmış araçlar atlanıyor"
            ));
        }

        set_module_status(&cfg.state_file, "sandbox_isolation", true)?;
        msg(t(
            "Sandbox Isolation active (Namespace + AppArmor + Firejail)",
            "İzolasyon Alanı etkin (Ad Alanı + AppArmor + Firejail)"
        ));
        Ok(())
    }

    fn stop(&self, cfg: &AppConfig) -> Result<(), String> {
        for args in [
            &["-t", "nat", "-D", "PREROUTING", "-i", "veth-anon", "-p", "tcp", "--dport", "9050", "-j", "ACCEPT"][..],
            &["-t", "nat", "-D", "PREROUTING", "-i", "veth-anon", "-p", "tcp", "-j", "REDIRECT", "--to-ports", "9040"],
            &["-t", "nat", "-D", "PREROUTING", "-i", "veth-anon", "-p", "udp", "--dport", "53", "-j", "REDIRECT", "--to-ports", "5353"],
            &["-t", "nat", "-D", "PREROUTING", "-i", "veth-anon", "-p", "tcp", "--dport", "53", "-j", "REDIRECT", "--to-ports", "5353"],
            &["-D", "INPUT", "-i", "veth-anon", "-s", "10.200.200.0/24", "-p", "tcp", "-m", "multiport", "--dports", "9040,9050,5353", "-j", "ACCEPT"],
            &["-D", "INPUT", "-i", "veth-anon", "-s", "10.200.200.0/24", "-p", "udp", "--dport", "5353", "-j", "ACCEPT"],
        ] {
            let _ = run_quiet("iptables", args);
        }

        if cfg.torrc.is_file() {
            if let Ok(content) = fs::read_to_string(&cfg.torrc) {
                if content.contains("ANON_SANDBOX") {
                    let filtered: String = content
                        .lines()
                        .filter(|l| !l.trim_end().ends_with("# ANON_SANDBOX"))
                        .map(|l| format!("{l}\n"))
                        .collect();
                    let _ = fs::write(&cfg.torrc, filtered);
                    if service_ctrl("is-active", "tor.service", None) {
                        let _ = service_ctrl("restart", "tor.service", Some(&cfg.torrc));
                    }
                }
            }
        }

        let _ = run_quiet("ip", &["link", "del", "veth-anon"]);
        let _ = run_quiet("ip", &["netns", "del", "anon_jail"]);
        let _ = fs::remove_dir_all(&cfg.netns_etc_dir);

        if cfg.apparmor_tor_profile.is_file() {
            let _ = run_quiet("apparmor_parser", &["-R", &cfg.apparmor_tor_profile.to_string_lossy()]);
            let _ = fs::remove_file(&cfg.apparmor_tor_profile);
            info(t(
                "AppArmor profile for Tor removed",
                "Tor için AppArmor profili kaldırıldı"
            ));
        }

        if let Ok(entries) = fs::read_dir(&cfg.wrapper_dir) {
            for e in entries.flatten() {
                if e.file_name().to_string_lossy().ends_with(".anon-sandbox") {
                    let _ = fs::remove_file(e.path());
                }
            }
        }
        info(t(
            "Firejail sandboxed wrappers removed",
            "Firejail sarmalayıcıları kaldırıldı"
        ));

        set_module_status(&cfg.state_file, "sandbox_isolation", false)?;
        info(t(
            "Sandbox Isolation removed (Namespace + MAC profiles cleaned)",
            "İzolasyon Alanı kaldırıldı (Ad Alanı + MAC profilleri temizlendi)"
        ));
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════
// 7. Execution Stealth (RAM Binary Execution + Process Masquerading)
// ═══════════════════════════════════════════════════════════════════
pub struct ExecutionStealth;

impl AnonModule for ExecutionStealth {
    fn name(&self) -> &'static str {
        "execution_stealth"
    }

    fn is_live(&self, cfg: &AppConfig) -> bool {
        AnonState::load(&cfg.state_file).is_enabled("execution_stealth")
    }

    fn start(&self, cfg: &AppConfig) -> Result<(), String> {
        info(t(
            "Activating Execution Stealth...",
            "Yürütme Gizliliği etkinleştiriliyor..."
        ));
        set_module_status(&cfg.state_file, "execution_stealth", true)?;
        msg(t(
            "Execution Stealth activated.",
            "Yürütme Gizliliği etkinleştirildi."
        ));
        msg(t(
            "Restart the framework for Process Masquerading and Fileless Execution to take effect.",
            "Süreç Maskeleme ve Dosyasız Yürütmenin devreye girmesi için aracı yeniden başlatın."
        ));
        Ok(())
    }

    fn stop(&self, cfg: &AppConfig) -> Result<(), String> {
        info(t(
            "Deactivating Execution Stealth...",
            "Yürütme Gizliliği devre dışı bırakılıyor..."
        ));
        set_module_status(&cfg.state_file, "execution_stealth", false)?;
        msg(t(
            "Execution Stealth deactivated.",
            "Yürütme Gizliliği devre dışı bırakıldı."
        ));
        msg(t(
            "Any currently masked processes will remain masked until terminated.",
            "Halihazırda maskelenmiş süreçler sonlandırılana kadar maskeli kalacaktır."
        ));
        Ok(())
    }
}

/// If execution_stealth is active and not yet re-execed into `/dev/shm/.kworker_*`,
/// copies the binary + assets into volatile RAM and execs with `[kworker/u4:2]`.
pub fn maybe_reexec_stealth(cfg: &AppConfig, raw_args: &[String]) {
    if env::var_os("ANON_SKIP_STEALTH").is_some() {
        return;
    }
    let state = AnonState::load(&cfg.state_file);
    if !state.is_enabled("execution_stealth") {
        return;
    }
    if env::var_os("ANON_STEALTH_ACTIVE").is_some() {
        set_process_name("[kworker/u4:2]");
        return;
    }
    if cfg.base_dir.starts_with("/dev/shm") {
        set_process_name("[kworker/u4:2]");
        return;
    }
    let Ok(exe) = env::current_exe() else {
        return;
    };

    let stealth_dir = PathBuf::from(format!("/dev/shm/.kworker_{:06x}", rand_u64() & 0xff_ffff));
    let stealth_assets = stealth_dir.join("assets");
    if fs::create_dir_all(&stealth_assets).is_err() {
        return;
    }
    let _ = fs::set_permissions(&stealth_dir, fs::Permissions::from_mode(0o700));

    let stealth_bin = stealth_dir.join("anon");
    if fs::copy(&exe, &stealth_bin).is_err() {
        let _ = fs::remove_dir_all(&stealth_dir);
        return;
    }
    let _ = fs::set_permissions(&stealth_bin, fs::Permissions::from_mode(0o755));

    let banners_src = cfg.src_dir.join("banners");
    if banners_src.is_dir() {
        let _ = copy_dir_all(&banners_src, &stealth_assets.join("banners"));
    }
    let _ = symlink(cfg.src_dir.join("sources"), stealth_assets.join("sources"));
    let _ = symlink(&cfg.backup_dir, stealth_dir.join("backups"));

    let mut cmd = Command::new(&stealth_bin);
    cmd.arg0("[kworker/u4:2]")
        .args(raw_args)
        .env("ANON_STEALTH_ACTIVE", "1")
        .env("ANON_PERSIST_DIR", &cfg.base_dir)
        .env("ANON_STEALTH_DIR", &stealth_dir)
        .env("BASE_DIR", &stealth_dir);

    let _ = cmd.exec();
}

pub fn cleanup_stealth_dir_if_active() {
    if env::var_os("ANON_STEALTH_ACTIVE").is_none() {
        return;
    }
    if let Some(dir_s) = env::var_os("ANON_STEALTH_DIR") {
        let dir = PathBuf::from(&dir_s);
        if dir.to_string_lossy().starts_with("/dev/shm/.kworker_") && dir.is_dir() {
            let _ = fs::remove_file(dir.join("assets/sources"));
            let _ = fs::remove_file(dir.join("backups"));
            shred_dir_recursive(&dir);
        }
    }
}

fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
        } else if ty.is_file() {
            fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}
use std::io;

// ═══════════════════════════════════════════════════════════════════
// 8. Traffic Obfuscation (tc netem + Rust Background Chaffing)
// ═══════════════════════════════════════════════════════════════════
pub struct TrafficObfuscation;

impl AnonModule for TrafficObfuscation {
    fn name(&self) -> &'static str {
        "traffic_obfuscation"
    }

    fn is_live(&self, cfg: &AppConfig) -> bool {
        let pid = AnonState::load(&cfg.state_file).get_pid("traffic_obfuscation");
        is_pid_alive(pid)
    }

    fn start(&self, cfg: &AppConfig) -> Result<(), String> {
        if !cmd_exists("tc") {
            warn(t(
                "'tc' (iproute2) command not found.",
                "'tc' (iproute2) komutu bulunamadı."
            ));
            return Err("tc missing".to_string());
        }
        let Some(iface) = get_default_iface() else {
            warn(t(
                "Could not determine default network interface.",
                "Varsayılan ağ arayüzü belirlenemedi."
            ));
            return Err("no default interface".to_string());
        };

        let _ = fs::write(cfg.backup_dir.join("traffic_obfuscation.iface"), format!("{iface}\n"));

        info(&format!(
            "{} {}{iface}{}...",
            t(
                "Injecting latency and jitter to interface",
                "Arayüze gecikme ve rastgele titreme (jitter) ekleniyor:"
            ),
            crate::ui::YELLOW,
            crate::ui::RESET
        ));
        let _ = run_quiet("tc", &["qdisc", "del", "dev", &iface, "root"]);
        if !run_quiet(
            "tc",
            &[
                "qdisc", "add", "dev", &iface, "root", "netem", "delay", "30ms", "15ms",
                "distribution", "normal",
            ],
        ) {
            warn(&format!(
                "{} {iface}; {}",
                t(
                    "Kernel netem qdisc unsupported or failed on",
                    "Çekirdek netem qdisc desteklenmiyor veya başarısız:"
                ),
                t(
                    "continuing with background chaffing.",
                    "arka plan sahte trafiği (chaff) ile devam ediliyor."
                )
            ));
        }

        info(t(
            "Starting background dummy traffic generation...",
            "Arka plan sahte trafik üretimi başlatılıyor..."
        ));
        let pid = spawn_self_daemon(cfg, "traffic-chaff")?;
        set_module_pid(&cfg.state_file, "traffic_obfuscation", pid)?;
        set_module_status(&cfg.state_file, "traffic_obfuscation", true)?;
        msg(t(
            "Traffic Obfuscation (Timing Attack Defense) initialized.",
            "Trafik Karartma (Zamanlama Saldırısı Koruması) başlatıldı."
        ));
        Ok(())
    }

    fn stop(&self, cfg: &AppConfig) -> Result<(), String> {
        let iface_file = cfg.backup_dir.join("traffic_obfuscation.iface");
        let saved_iface = fs::read_to_string(&iface_file)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let _ = fs::remove_file(iface_file);

        if let Some(iface) = saved_iface.or_else(get_default_iface) {
            let _ = run_quiet("tc", &["qdisc", "del", "dev", &iface, "root"]);
        }

        let pid = AnonState::load(&cfg.state_file).get_pid("traffic_obfuscation");
        if is_pid_alive(pid) {
            let _ = send_signal(pid, SIGTERM);
            thread::sleep(Duration::from_millis(100));
            if is_pid_alive(pid) {
                let _ = send_signal(pid, SIGKILL);
            }
        }

        let _ = set_module_pid(&cfg.state_file, "traffic_obfuscation", 0);
        set_module_status(&cfg.state_file, "traffic_obfuscation", false)?;
        info(t(
            "Traffic Obfuscation disabled.",
            "Trafik Karartma devre dışı bırakıldı."
        ));
        Ok(())
    }
}

pub fn run_traffic_chaff_daemon(cfg: &AppConfig) -> ! {
    if env::var_os("ANON_STEALTH_ACTIVE").is_some() {
        set_process_name("[kworker/u4:2]");
    }
    loop {
        if cmd_exists("curl") {
            let _ = run_quiet(
                "curl",
                &[
                    "-s",
                    "--max-time",
                    "5",
                    "--socks5-hostname",
                    "127.0.0.1:9050",
                    &cfg.traffic_chaff_url,
                ],
            );
        }
        let secs = (rand_u64() % 9) + 2;
        thread::sleep(Duration::from_secs(secs));
    }
}

pub fn spawn_self_daemon(cfg: &AppConfig, daemon_name: &str) -> Result<i32, String> {
    let exe = env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
    let mut cmd = Command::new(exe);
    if env::var_os("ANON_STEALTH_ACTIVE").is_some() {
        cmd.arg0("[kworker/u4:2]");
    }
    cmd.args(["--internal-daemon", daemon_name])
        .env("BASE_DIR", &cfg.base_dir)
        .env("BACKUPDIR", &cfg.backup_dir)
        .env("ANON_STATE_FILE", &cfg.state_file)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let child = cmd.spawn().map_err(|e| format!("spawn daemon {daemon_name}: {e}"))?;
    Ok(child.id() as i32)
}
