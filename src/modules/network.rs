use std::fs;
use std::io::{self, BufRead};
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

use crate::config::AppConfig;
use crate::helpers::{
    backup_resolv_conf, cmd_exists, find_in_path, is_netns_active, is_tor_running, is_virtual_iface,
    is_wsl, list_interfaces, lock_nm_dns, rand_u64, resolve_bridges_file, resolve_uid_any,
    restore_resolv_conf, run_capture, run_quiet, run_stdin, service_ctrl, sysctl_write,
    tor_bootstrap_progress, unlock_nm_dns, wait_for_network,
};
use crate::i18n::t;
use crate::registry::AnonModule;
use crate::state::{set_module_status, AnonState};
use crate::sys::is_stdin_tty;
use crate::ui::{info, msg, notify, warn, DIM, RESET};

const ANON_PT_PIN_TAG: &str = "# ANON_PT_PIN";

pub const DEFAULT_BRIDGES_EMBEDDED: &str = include_str!("../../assets/sources/bridges.default.txt");

// ═══════════════════════════════════════════════════════════════════
// 1. IP Changer — Atomic Tor Transparent Proxy
// ═══════════════════════════════════════════════════════════════════
pub struct IpChanger;

impl AnonModule for IpChanger {
    fn name(&self) -> &'static str {
        "ip_changer"
    }

    fn is_live(&self, cfg: &AppConfig) -> bool {
        cfg.backup_dir.join("torrc.bak").is_file()
    }

    fn start(&self, cfg: &AppConfig) -> Result<(), String> {
        let state = AnonState::load(&cfg.state_file);

        if state.is_enabled("dns_changer") || state.is_enabled("doh_shield") {
            warn(t(
                "The Ip changer is not available. (Dns changer or DoH Shield enabled)",
                "IP değiştirici kullanılamıyor. (DNS değiştirici veya DoH Kalkanı etkin)"
            ));
            return Ok(());
        }
        if state.is_enabled("ip_changer") {
            warn(t(
                "Ip changer is already running",
                "IP değiştirici zaten çalışıyor"
            ));
            return Ok(());
        }

        if state.is_enabled("dpi_spoofing") {
            let _ = pin_snowflake_hosts(cfg);
        }

        // Backup baseline torrc (stripping any stale ANON_SANDBOX lines so stop never re-injects them)
        if let Ok(raw_torrc) = fs::read_to_string(&cfg.torrc) {
            let _ = fs::write(
                cfg.backup_dir.join("torrc.bak"),
                strip_sandbox_marker_lines(&raw_torrc),
            );
        } else {
            let _ = fs::write(cfg.backup_dir.join("torrc.bak"), "");
        }

        backup_resolv_conf(cfg, "tor");

        if let Some(rules) = run_capture("iptables-save", &[]) {
            let _ = fs::write(cfg.backup_dir.join("iptables.rules.bak"), rules);
        }
        if let Some(rules6) = run_capture("ip6tables-save", &[]) {
            let _ = fs::write(cfg.backup_dir.join("ip6tables.rules.bak"), rules6);
        }

        let tor_uid = resolve_uid_any(&["debian-tor", "tor", "_tor"]).unwrap_or(109);
        let sandbox_active = state.is_enabled("sandbox_isolation");

        if !apply_tor_iptables_atomic(tor_uid, sandbox_active) {
            warn("Failed to apply atomic Tor iptables rules.");
            return Err("iptables-restore failed".to_string());
        }

        let mut torrc_body = String::from(
            "VirtualAddrNetworkIPv4 10.192.0.0/10\n\
             AutomapHostsOnResolve 1\n\
             TransPort 9040 IsolateClientAddr IsolateClientProtocol IsolateDestAddr IsolateDestPort\n\
             SocksPort 9050\n\
             DNSPort 5353\n\
             ControlPort 9051\n\
             CookieAuthentication 1\n",
        );

        if sandbox_active {
            torrc_body.push_str(
                "TransPort 10.200.200.1:9040 IsolateClientAddr IsolateClientProtocol IsolateDestAddr IsolateDestPort # ANON_SANDBOX\n\
                 SocksPort 10.200.200.1:9050 # ANON_SANDBOX\n\
                 DNSPort 10.200.200.1:5353 # ANON_SANDBOX\n",
            );
        }

        let snowflake_bin = find_in_path("snowflake-client")
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| "/usr/bin/snowflake-client".to_string());
        let obfs4_bin = find_in_path("obfs4proxy")
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| "/usr/bin/obfs4proxy".to_string());

        if state.is_enabled("dpi_spoofing") {
            let last_front = cfg
                .snowflake_fronts
                .rsplit(',')
                .next()
                .unwrap_or(&cfg.snowflake_fronts);
            let sf_params = format!(
                "url={} fronts={} front={} ice={}",
                cfg.snowflake_url, cfg.snowflake_fronts, last_front, cfg.snowflake_ice
            );
            torrc_body.push_str("UseBridges 1\n");
            torrc_body.push_str(&format!(
                "ClientTransportPlugin snowflake exec {snowflake_bin}\n"
            ));
            torrc_body.push_str(&format!(
                "Bridge snowflake 192.0.2.3:80 2B280B23E1107BB62ABFC40DDCC8824814F80A72 fingerprint=2B280B23E1107BB62ABFC40DDCC8824814F80A72 {sf_params}\n"
            ));
            torrc_body.push_str(&format!(
                "Bridge snowflake 192.0.2.4:80 8838024498816A039FCBBAB14E6F40A0843051FA fingerprint=8838024498816A039FCBBAB14E6F40A0843051FA {sf_params}\n"
            ));
        } else if state.is_enabled("tor_bridges") {
            if let Some(bf) = resolve_bridges_file(cfg) {
                torrc_body.push_str("UseBridges 1\n");
                torrc_body.push_str(&format!(
                    "ClientTransportPlugin obfs4 exec {obfs4_bin}\n"
                ));
                append_bridges_to_string(&bf, &mut torrc_body);
            }
        }

        fs::write(&cfg.torrc, &torrc_body).map_err(|e| format!("write torrc: {e}"))?;

        lock_nm_dns(cfg);
        let _ = run_quiet("chattr", &["-i", &cfg.resolv_conf.to_string_lossy()]);
        let _ = fs::write(
            &cfg.resolv_conf,
            "# This file was edited by anon. Do not change manually!\nnameserver 127.0.0.1\n",
        );
        let _ = run_quiet("chattr", &["+i", &cfg.resolv_conf.to_string_lossy()]);
        let _ = service_ctrl("daemon-reload", "", None);

        if service_ctrl("is-active", "tor.service", None) {
            let _ = service_ctrl("stop", "tor.service", None);
        }

        if !is_wsl() {
            let _ = sysctl_write("net.ipv6.conf.all.disable_ipv6", "1");
            let _ = sysctl_write("net.ipv6.conf.default.disable_ipv6", "1");
        }
        let _ = service_ctrl("start", "tor.service", Some(&cfg.torrc));
        set_module_status(&cfg.state_file, "ip_changer", true)?;
        info(t(
            "Ip changer successfully enabled",
            "IP değiştirici başarıyla etkinleştirildi"
        ));

        let default_boot = if state.is_enabled("dpi_spoofing") || state.is_enabled("tor_bridges") {
            180
        } else {
            90
        };
        let boot_timeout = cfg.tor_bootstrap_timeout.unwrap_or(default_boot);
        let mut retries = if wait_for_tor_bootstrap(boot_timeout) {
            3
        } else {
            1
        };

        info(t(
            "Running automated leak test...",
            "Otomatik sızıntı testi çalıştırılıyor..."
        ));
        let mut success = false;
        for i in 1..=retries {
            info(&format!(
                "{} {i}/{retries}...",
                t("Leak test attempt", "Sızıntı testi denemesi")
            ));
            if check_tor_exit_ok(15) {
                msg(t(
                    "No IP leaks detected! You are successfully routed through Tor.",
                    "IP sızıntısı tespit edilmedi! Tor üzerinden başarıyla yönlendiriliyorsunuz."
                ));
                notify(
                    t("ANON - Tor Connected", "ANON - Tor Bağlandı"),
                    t(
                        "Transparent proxy active. No leaks detected.",
                        "Şeffaf vekil sunucu etkin. Sızıntı tespit edilmedi."
                    ),
                    "normal",
                );
                success = true;
                break;
            }
            warn(t(
                "Leak test failed or timed out. Retrying in 5 seconds...",
                "Sızıntı testi başarısız oldu veya zaman aşımına uğradı. 5 saniye içinde yeniden deneniyor..."
            ));
            thread::sleep(Duration::from_secs(5));
        }

        if !success {
            let active_bridges = resolve_bridges_file(cfg);
            if !state.is_enabled("tor_bridges") && active_bridges.is_some() {
                let bf = active_bridges.unwrap();
                warn(t(
                    "Tor connection failed. Attempting Auto-Fallback to Tor Bridges...",
                    "Tor bağlantısı başarısız. Tor Köprülerine otomatik geçiş deneniyor..."
                ));
                let _ = set_module_status(&cfg.state_file, "tor_bridges", true);
                info(t(
                    "Bridges enabled. Restarting Tor...",
                    "Köprüler etkinleştirildi. Tor yeniden başlatılıyor..."
                ));

                if !torrc_body.lines().any(|l| l.trim() == "UseBridges 1") {
                    torrc_body.push_str("UseBridges 1\n");
                }
                torrc_body.push_str(&format!(
                    "ClientTransportPlugin obfs4 exec {obfs4_bin}\n"
                ));
                append_bridges_to_string(&bf, &mut torrc_body);
                let _ = fs::write(&cfg.torrc, &torrc_body);

                let _ = service_ctrl("restart", "tor.service", Some(&cfg.torrc));
                info(t(
                    "Tor restarted with Obfs4 Bridges. Retrying leak test...",
                    "Tor Obfs4 köprüleriyle yeniden başlatıldı. Sızıntı testi tekrar deneniyor..."
                ));

                retries = if wait_for_tor_bootstrap(cfg.tor_bootstrap_timeout.unwrap_or(180)) {
                    3
                } else {
                    1
                };
                let mut fallback_ok = false;
                for j in 1..=retries {
                    info(&format!(
                        "{} {j}/{retries}...",
                        t("Fallback test attempt", "Yedek köprü testi denemesi")
                    ));
                    if check_tor_exit_ok(15) {
                        msg(t(
                            "Auto-Fallback successful! Connected via Tor Bridges.",
                            "Otomatik geçiş başarılı! Tor Köprüleri üzerinden bağlanıldı."
                        ));
                        notify(
                            t("ANON - Bridges Active", "ANON - Köprüler Etkin"),
                            t(
                                "Connected to Tor via obfuscated bridges.",
                                "Tor'a gizlenmiş köprüler üzerinden bağlanıldı."
                            ),
                            "normal",
                        );
                        fallback_ok = true;
                        break;
                    }
                    warn(t(
                        "Fallback test failed. Retrying in 5 seconds...",
                        "Köprü testi başarısız. 5 saniye içinde yeniden deneniyor..."
                    ));
                    thread::sleep(Duration::from_secs(5));
                }

                if !fallback_ok {
                    warn(t(
                        "CRITICAL: Auto-Fallback failed. Tor is completely blocked! Triggering Kill Switch...",
                        "KRİTİK: Otomatik geçiş başarısız. Tor tamamen engelli! Kill Switch tetikleniyor..."
                    ));
                    notify(
                        t("ANON - CRITICAL ALERT", "ANON - KRİTİK UYARI"),
                        t(
                            "Tor bridges blocked! Kill switch engaged.",
                            "Tor köprüleri engellendi! Kill switch devreye alındı."
                        ),
                        "critical",
                    );
                    let _ = set_module_status(&cfg.state_file, "ip_changer", false);
                    let _ = KillSwitch.start(cfg);
                    warn(t(
                        "Network locked down. Check your network or update your bridges.",
                        "Ağ kilitlendi. Ağınızı kontrol edin veya köprülerinizi güncelleyin."
                    ));
                    return Err("Tor fallback failed".to_string());
                }
            } else {
                warn(t(
                    "CRITICAL: IP Leak detected or Tor connection failed! Triggering Kill Switch for safety...",
                    "KRİTİK: IP Sızıntısı tespit edildi veya Tor bağlantısı başarısız! Güvenlik için Kill Switch tetikleniyor..."
                ));
                notify(
                    t("ANON - CRITICAL ALERT", "ANON - KRİTİK UYARI"),
                    t(
                        "IP Leak detected! Kill switch engaged.",
                        "IP Sızıntısı tespit edildi! Kill switch devreye alındı."
                    ),
                    "critical",
                );
                let _ = set_module_status(&cfg.state_file, "ip_changer", false);
                let _ = KillSwitch.start(cfg);
                warn(t(
                    "Network locked down to prevent leaks. Please check your Tor connection and restart.",
                    "Sızıntıları önlemek için ağ kilitlendi. Lütfen Tor bağlantınızı kontrol edip yeniden başlatın."
                ));
                return Err("Tor connection failed".to_string());
            }
        }

        Ok(())
    }

    fn stop(&self, cfg: &AppConfig) -> Result<(), String> {
        let state = AnonState::load(&cfg.state_file);
        let _ = service_ctrl("stop", "tor.service", None);

        let rules_bak = cfg.backup_dir.join("iptables.rules.bak");
        if let Some(rules) = fs::read_to_string(&rules_bak).ok().filter(|s| !s.is_empty()) {
            let _ = run_stdin("iptables-restore", &[], &rules);
        } else {
            for args in [
                &["-F"][..],
                &["-X"],
                &["-t", "nat", "-F"],
                &["-t", "nat", "-X"],
                &["-P", "INPUT", "ACCEPT"],
                &["-P", "FORWARD", "ACCEPT"],
                &["-P", "OUTPUT", "ACCEPT"],
            ] {
                let _ = run_quiet("iptables", args);
            }
        }

        if state.is_enabled("sandbox_isolation") && is_netns_active("anon_jail") {
            apply_sandbox_tor_rules();
        }

        let rules6_bak = cfg.backup_dir.join("ip6tables.rules.bak");
        if cmd_exists("ip6tables-restore")
            && fs::metadata(&rules6_bak).map(|m| m.len() > 0).unwrap_or(false)
        {
            if let Ok(rules6) = fs::read_to_string(&rules6_bak) {
                let _ = run_stdin("ip6tables-restore", &[], &rules6);
            }
        } else if cmd_exists("ip6tables") {
            for args in [
                &["-F"][..],
                &["-P", "INPUT", "ACCEPT"],
                &["-P", "FORWARD", "ACCEPT"],
                &["-P", "OUTPUT", "ACCEPT"],
            ] {
                let _ = run_quiet("ip6tables", args);
            }
        }

        if !is_wsl() {
            let _ = sysctl_write("net.ipv6.conf.all.disable_ipv6", "0");
            let _ = sysctl_write("net.ipv6.conf.default.disable_ipv6", "0");
        }

        let torrc_bak = cfg.backup_dir.join("torrc.bak");
        if let Ok(orig) = fs::read_to_string(&torrc_bak) {
            let cleaned = if !state.is_enabled("sandbox_isolation") {
                strip_sandbox_marker_lines(&orig)
            } else {
                orig
            };
            let _ = fs::write(&cfg.torrc, cleaned);
        }

        unlock_nm_dns(cfg);
        restore_resolv_conf(cfg, "tor");
        unpin_snowflake_hosts(cfg);

        let _ = fs::remove_file(torrc_bak);
        let _ = fs::remove_file(rules_bak);
        let _ = fs::remove_file(rules6_bak);

        set_module_status(&cfg.state_file, "ip_changer", false)?;
        info(t(
            "Ip changer successfully disabled",
            "IP değiştirici başarıyla devre dışı bırakıldı"
        ));
        Ok(())
    }
}

pub fn check_tor_exit_ok(max_time_secs: u64) -> bool {
    let t = max_time_secs.to_string();
    if let Some(out) = run_capture("curl", &["-s", "--max-time", &t, "https://check.torproject.org/api/ip"]) {
        return out.contains("\"IsTor\":true");
    }
    false
}

fn wait_for_tor_bootstrap(max_wait_secs: u64) -> bool {
    info(&format!(
        "{} ({} {max_wait_secs}s)...",
        t("Waiting for Tor to bootstrap", "Tor bağlantısının kurulması bekleniyor"),
        t("up to", "azami")
    ));
    let mut elapsed = 0u64;
    let mut last: Option<u32> = None;
    let mut unreachable = 0u32;

    while elapsed < max_wait_secs {
        if let Some(prog) = tor_bootstrap_progress() {
            unreachable = 0;
            if last != Some(prog) {
                info(&format!(
                    "{}: {prog}%",
                    t("Tor bootstrap", "Tor bağlantı durumu")
                ));
                last = Some(prog);
            }
            if prog >= 100 {
                return true;
            }
        } else {
            unreachable += 1;
            if unreachable >= 5 {
                warn(t(
                    "Tor ControlPort unreachable; skipping bootstrap wait.",
                    "Tor ControlPort erişilemiyor; başlatma beklemesi atlanıyor."
                ));
                return false;
            }
        }
        thread::sleep(Duration::from_secs(3));
        elapsed += 3;
    }
    let last_str = last
        .map(|v| v.to_string())
        .unwrap_or_else(|| t("unknown", "bilinmiyor").to_string());
    warn(&format!(
        "{} {max_wait_secs}s ({}: {last_str}%).",
        t(
            "Tor did not finish bootstrapping within",
            "Tor belirtilen sürede başlatılamadı:"
        ),
        t("last", "son")
    ));
    false
}

pub fn unpin_snowflake_hosts(cfg: &AppConfig) {
    let Ok(content) = fs::read_to_string(&cfg.hosts_file) else {
        return;
    };
    if !content.lines().any(|l| l.trim_end().ends_with(ANON_PT_PIN_TAG)) {
        return;
    }
    let filtered: String = content
        .lines()
        .filter(|l| !l.trim_end().ends_with(ANON_PT_PIN_TAG))
        .map(|l| format!("{l}\n"))
        .collect();
    let _ = fs::write(&cfg.hosts_file, filtered);
}

fn pin_snowflake_hosts(cfg: &AppConfig) -> bool {
    let mut names: Vec<String> = cfg
        .snowflake_fronts
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    for entry in cfg.snowflake_ice.split(',') {
        let trimmed = entry.trim().strip_prefix("stun:").unwrap_or(entry.trim());
        let host = trimmed.split(':').next().unwrap_or("");
        if !host.is_empty() {
            names.push(host.to_string());
        }
    }

    unpin_snowflake_hosts(cfg);

    let mut append_buf = String::new();
    let mut pinned = 0usize;

    for name in names {
        if !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
        {
            continue;
        }
        let ip = resolve_ipv4_host(&name);
        if let Some(ip_str) = ip {
            append_buf.push_str(&format!("{ip_str}\t{name} {ANON_PT_PIN_TAG}\n"));
            pinned += 1;
        }
    }

    if pinned > 0 {
        if let Ok(mut existing) = fs::read_to_string(&cfg.hosts_file) {
            if !existing.is_empty() && !existing.ends_with('\n') {
                existing.push('\n');
            }
            existing.push_str(&append_buf);
            if fs::write(&cfg.hosts_file, existing).is_ok() {
                info(&format!(
                    "Pinned {pinned} Snowflake rendezvous host(s) for pre-bootstrap resolution."
                ));
                return true;
            }
        }
    }

    warn("Could not pre-resolve Snowflake front/STUN hosts (no clearnet DNS?). Snowflake may fail to bootstrap.");
    false
}

fn resolve_ipv4_host(name: &str) -> Option<String> {
    if let Some(out) = run_capture("getent", &["ahostsv4", name]) {
        if let Some(first) = out.lines().next().and_then(|l| l.split_whitespace().next()) {
            if is_valid_ipv4(first) {
                return Some(first.to_string());
            }
        }
    }
    use std::net::ToSocketAddrs;
    if let Ok(addrs) = (name, 443u16).to_socket_addrs() {
        for addr in addrs {
            if let std::net::SocketAddr::V4(v4) = addr {
                return Some(v4.ip().to_string());
            }
        }
    }
    None
}

fn is_valid_ipv4(s: &str) -> bool {
    s.parse::<std::net::Ipv4Addr>().is_ok()
}

fn append_bridges_to_string(path: &Path, out: &mut String) {
    if let Ok(content) = fs::read_to_string(path) {
        for raw in content.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line.starts_with("Bridge ") {
                out.push_str(line);
            } else {
                out.push_str("Bridge ");
                out.push_str(line);
            }
            out.push('\n');
        }
    }
}

fn apply_tor_iptables_atomic(tor_uid: u32, sandbox_active: bool) -> bool {
    let (sandbox_nat, sandbox_in) = if sandbox_active {
        (
            "-A PREROUTING -i veth-anon -p tcp --dport 9050 -j ACCEPT\n\
             -A PREROUTING -i veth-anon -p tcp -j REDIRECT --to-ports 9040\n\
             -A PREROUTING -i veth-anon -p udp --dport 53 -j REDIRECT --to-ports 5353\n\
             -A PREROUTING -i veth-anon -p tcp --dport 53 -j REDIRECT --to-ports 5353\n",
            "-A INPUT -i veth-anon -s 10.200.200.0/24 -p tcp -m multiport --dports 9040,9050,5353 -j ACCEPT\n\
             -A INPUT -i veth-anon -s 10.200.200.0/24 -p udp --dport 5353 -j ACCEPT\n",
        )
    } else {
        ("", "")
    };

    let rules = format!(
        "*nat\n\
         :PREROUTING ACCEPT [0:0]\n\
         :INPUT ACCEPT [0:0]\n\
         :OUTPUT ACCEPT [0:0]\n\
         :POSTROUTING ACCEPT [0:0]\n\
         {sandbox_nat}\
         -A OUTPUT -d 10.192.0.0/10 -p tcp -m tcp --tcp-flags FIN,SYN,RST,ACK SYN -j REDIRECT --to-ports 9040\n\
         -A OUTPUT -d 127.0.0.1/32 -p udp -m udp --dport 53 -j REDIRECT --to-ports 5353\n\
         -A OUTPUT -d 127.0.0.1/32 -p tcp -m tcp --dport 53 --tcp-flags FIN,SYN,RST,ACK SYN -j REDIRECT --to-ports 5353\n\
         -A OUTPUT -m owner --uid-owner {tor_uid} -j RETURN\n\
         -A OUTPUT -o lo -j RETURN\n\
         -A OUTPUT -d 127.0.0.0/8 -j RETURN\n\
         -A OUTPUT -d 10.0.0.0/8 -j RETURN\n\
         -A OUTPUT -d 172.16.0.0/12 -j RETURN\n\
         -A OUTPUT -d 192.168.0.0/16 -j RETURN\n\
         -A OUTPUT -p udp -m udp --dport 53 -j REDIRECT --to-ports 5353\n\
         -A OUTPUT -p tcp -m tcp --tcp-flags FIN,SYN,RST,ACK SYN -j REDIRECT --to-ports 9040\n\
         COMMIT\n\
         *filter\n\
         :INPUT DROP [0:0]\n\
         :FORWARD DROP [0:0]\n\
         :OUTPUT DROP [0:0]\n\
         -A INPUT -m state --state ESTABLISHED,RELATED -j ACCEPT\n\
         -A INPUT -i lo -j ACCEPT\n\
         {sandbox_in}\
         -A INPUT -j DROP\n\
         -A FORWARD -j DROP\n\
         -A OUTPUT -m conntrack --ctstate INVALID -j DROP\n\
         -A OUTPUT -m state --state INVALID -j DROP\n\
         -A OUTPUT -m state --state ESTABLISHED -j ACCEPT\n\
         -A OUTPUT -m owner --uid-owner {tor_uid} -j ACCEPT\n\
         -A OUTPUT -d 10.0.0.0/8 -p udp --dport 53 -j DROP\n\
         -A OUTPUT -d 10.0.0.0/8 -p tcp --dport 53 -j DROP\n\
         -A OUTPUT -d 172.16.0.0/12 -p udp --dport 53 -j DROP\n\
         -A OUTPUT -d 172.16.0.0/12 -p tcp --dport 53 -j DROP\n\
         -A OUTPUT -d 192.168.0.0/16 -p udp --dport 53 -j DROP\n\
         -A OUTPUT -d 192.168.0.0/16 -p tcp --dport 53 -j DROP\n\
         -A OUTPUT -d 127.0.0.0/8 -j ACCEPT\n\
         -A OUTPUT -d 10.0.0.0/8 -j ACCEPT\n\
         -A OUTPUT -d 172.16.0.0/12 -j ACCEPT\n\
         -A OUTPUT -d 192.168.0.0/16 -j ACCEPT\n\
         -A OUTPUT -d 127.0.0.1/32 -o lo -j ACCEPT\n\
         -A OUTPUT -d 127.0.0.1/32 -p udp -m udp --dport 5353 -j ACCEPT\n\
         -A OUTPUT -d 127.0.0.1/32 -p tcp -m tcp --dport 9040 --tcp-flags FIN,SYN,RST,ACK SYN -j ACCEPT\n\
         -A OUTPUT -j DROP\n\
         COMMIT\n"
    );

    if !run_stdin("iptables-restore", &[], &rules) {
        return false;
    }

    if cmd_exists("ip6tables-restore") {
        let ip6_rules = "*filter\n:INPUT DROP [0:0]\n:FORWARD DROP [0:0]\n:OUTPUT DROP [0:0]\n-A INPUT -i lo -j ACCEPT\n-A OUTPUT -o lo -j ACCEPT\nCOMMIT\n";
        let _ = run_stdin("ip6tables-restore", &[], ip6_rules);
    }
    true
}

pub fn apply_sandbox_tor_rules() {
    let nat_checks: [&[&str]; 4] = [
        &["-t", "nat", "-C", "PREROUTING", "-i", "veth-anon", "-p", "tcp", "--dport", "9050", "-j", "ACCEPT"],
        &["-t", "nat", "-C", "PREROUTING", "-i", "veth-anon", "-p", "tcp", "-j", "REDIRECT", "--to-ports", "9040"],
        &["-t", "nat", "-C", "PREROUTING", "-i", "veth-anon", "-p", "udp", "--dport", "53", "-j", "REDIRECT", "--to-ports", "5353"],
        &["-t", "nat", "-C", "PREROUTING", "-i", "veth-anon", "-p", "tcp", "--dport", "53", "-j", "REDIRECT", "--to-ports", "5353"],
    ];
    let nat_adds: [&[&str]; 4] = [
        &["-t", "nat", "-A", "PREROUTING", "-i", "veth-anon", "-p", "tcp", "--dport", "9050", "-j", "ACCEPT"],
        &["-t", "nat", "-A", "PREROUTING", "-i", "veth-anon", "-p", "tcp", "-j", "REDIRECT", "--to-ports", "9040"],
        &["-t", "nat", "-A", "PREROUTING", "-i", "veth-anon", "-p", "udp", "--dport", "53", "-j", "REDIRECT", "--to-ports", "5353"],
        &["-t", "nat", "-A", "PREROUTING", "-i", "veth-anon", "-p", "tcp", "--dport", "53", "-j", "REDIRECT", "--to-ports", "5353"],
    ];
    for (chk, add) in nat_checks.iter().zip(nat_adds.iter()) {
        if !run_quiet("iptables", chk) {
            let _ = run_quiet("iptables", add);
        }
    }

    if !run_quiet("iptables", &["-C", "INPUT", "-i", "veth-anon", "-s", "10.200.200.0/24", "-p", "tcp", "-m", "multiport", "--dports", "9040,9050,5353", "-j", "ACCEPT"]) {
        let _ = run_quiet("iptables", &["-I", "INPUT", "1", "-i", "veth-anon", "-s", "10.200.200.0/24", "-p", "tcp", "-m", "multiport", "--dports", "9040,9050,5353", "-j", "ACCEPT"]);
    }
    if !run_quiet("iptables", &["-C", "INPUT", "-i", "veth-anon", "-s", "10.200.200.0/24", "-p", "udp", "--dport", "5353", "-j", "ACCEPT"]) {
        let _ = run_quiet("iptables", &["-I", "INPUT", "1", "-i", "veth-anon", "-s", "10.200.200.0/24", "-p", "udp", "--dport", "5353", "-j", "ACCEPT"]);
    }
}

// ═══════════════════════════════════════════════════════════════════
// 2. Kill Switch — Atomic IPv4 + IPv6 Lockdown
// ═══════════════════════════════════════════════════════════════════
pub struct KillSwitch;

impl AnonModule for KillSwitch {
    fn name(&self) -> &'static str {
        "kill_switch"
    }

    fn is_live(&self, cfg: &AppConfig) -> bool {
        cfg.backup_dir.join("iptables_killswitch.bak").is_file()
    }

    fn start(&self, cfg: &AppConfig) -> Result<(), String> {
        let state = AnonState::load(&cfg.state_file);
        if state.is_enabled("kill_switch") {
            warn(t(
                "Network Kill Switch is already running.",
                "Ağ Kill Switch zaten çalışıyor."
            ));
            return Ok(());
        }

        let ks_bak = cfg.backup_dir.join("iptables_killswitch.bak");
        let ks6_bak = cfg.backup_dir.join("ip6tables_killswitch.bak");
        if let Some(rules) = run_capture("iptables-save", &[]) {
            let _ = fs::write(&ks_bak, rules);
        }
        if cmd_exists("ip6tables-save") {
            if let Some(rules6) = run_capture("ip6tables-save", &[]) {
                let _ = fs::write(&ks6_bak, rules6);
            }
        }

        let mut tor_rules = String::new();
        if is_tor_running() {
            if let Some(tor_uid) = resolve_uid_any(&["debian-tor", "tor", "_tor"]) {
                tor_rules = format!(
                    "-A OUTPUT -m owner --uid-owner {tor_uid} -j ACCEPT\n\
                     -A OUTPUT -d 127.0.0.1/32 -p tcp --dport 9040 -j ACCEPT\n\
                     -A OUTPUT -d 127.0.0.1/32 -p udp --dport 5353 -j ACCEPT\n\
                     -A OUTPUT -d 127.0.0.1/32 -p tcp --dport 9050 -j ACCEPT\n"
                );
            }
        }

        let mut i2p_rules = String::new();
        if state.is_enabled("i2p_integration") {
            if let Some(i2p_uid) = resolve_uid_any(&["_i2pd", "i2pd"]) {
                i2p_rules = format!(
                    "-A OUTPUT -m owner --uid-owner {i2p_uid} -j ACCEPT\n\
                     -A OUTPUT -d 127.0.0.1/32 -p tcp -m multiport --dports 4444,4446,7070 -j ACCEPT\n"
                );
            }
        }

        let mut doh_rules = String::new();
        if state.is_enabled("doh_shield") {
            if let Some(doh_uid) = resolve_uid_any(&["_dnscrypt-proxy", "dnscrypt-proxy"]) {
                doh_rules.push_str(&format!("-A OUTPUT -m owner --uid-owner {doh_uid} -j ACCEPT\n"));
            }
            doh_rules.push_str(
                "-A OUTPUT -d 127.0.2.1/32 -p udp --dport 53 -j ACCEPT\n\
                 -A OUTPUT -d 127.0.2.1/32 -p tcp --dport 53 -j ACCEPT\n",
            );
        }

        let sandbox_rules = if state.is_enabled("sandbox_isolation") {
            "-A INPUT -i veth-anon -s 10.200.200.0/24 -p tcp -m multiport --dports 9040,9050,5353 -j ACCEPT\n\
             -A INPUT -i veth-anon -s 10.200.200.0/24 -p udp --dport 5353 -j ACCEPT\n"
        } else {
            ""
        };

        if !is_wsl() {
            let _ = sysctl_write("net.ipv6.conf.all.disable_ipv6", "1");
            let _ = sysctl_write("net.ipv6.conf.default.disable_ipv6", "1");
        }

        if cmd_exists("ip6tables-restore") {
            let ip6 = "*filter\n:INPUT DROP [0:0]\n:FORWARD DROP [0:0]\n:OUTPUT DROP [0:0]\n-A INPUT -i lo -j ACCEPT\n-A OUTPUT -o lo -j ACCEPT\nCOMMIT\n";
            let _ = run_stdin("ip6tables-restore", &[], ip6);
        }

        let rules = format!(
            "*filter\n\
             :INPUT DROP [0:0]\n\
             :FORWARD DROP [0:0]\n\
             :OUTPUT DROP [0:0]\n\
             -A INPUT -i lo -j ACCEPT\n\
             -A OUTPUT -o lo -j ACCEPT\n\
             {sandbox_rules}\
             -A INPUT -m state --state ESTABLISHED,RELATED -j ACCEPT\n\
             -A OUTPUT -m state --state ESTABLISHED -j ACCEPT\n\
             {tor_rules}\
             {i2p_rules}\
             {doh_rules}\
             -A OUTPUT -d 10.0.0.0/8 -p udp --dport 53 -j DROP\n\
             -A OUTPUT -d 10.0.0.0/8 -p tcp --dport 53 -j DROP\n\
             -A OUTPUT -d 172.16.0.0/12 -p udp --dport 53 -j DROP\n\
             -A OUTPUT -d 172.16.0.0/12 -p tcp --dport 53 -j DROP\n\
             -A OUTPUT -d 192.168.0.0/16 -p udp --dport 53 -j DROP\n\
             -A OUTPUT -d 192.168.0.0/16 -p tcp --dport 53 -j DROP\n\
             -A OUTPUT -d 127.0.0.0/8 -j ACCEPT\n\
             -A OUTPUT -d 10.0.0.0/8 -j ACCEPT\n\
             -A OUTPUT -d 172.16.0.0/12 -j ACCEPT\n\
             -A OUTPUT -d 192.168.0.0/16 -j ACCEPT\n\
             COMMIT\n"
        );

        if run_stdin("iptables-restore", &[], &rules) {
            set_module_status(&cfg.state_file, "kill_switch", true)?;
            info(t(
                "Network Kill Switch enabled (atomic IPv4+IPv6 iptables-restore). All non-Tor traffic blocked.",
                "Ağ Kill Switch etkinleştirildi (atomik IPv4+IPv6 iptables-restore). Tor dışı tüm trafik engellendi."
            ));
            notify(
                t("ANON Kill Switch", "ANON Kill Switch"),
                t(
                    "Network Kill Switch active. All non-Tor traffic is blocked.",
                    "Ağ Kill Switch etkin. Tor dışı tüm trafik engellendi."
                ),
                "normal",
            );
            Ok(())
        } else {
            warn(t(
                "Failed to apply Kill Switch rules. Restoring previous state...",
                "Kill Switch kuralları uygulanamadı. Önceki durum geri yükleniyor..."
            ));
            if let Some(r) = fs::read_to_string(&ks_bak).ok().filter(|s| !s.is_empty()) {
                let _ = run_stdin("iptables-restore", &[], &r);
            }
            if cmd_exists("ip6tables-restore") {
                if let Some(r6) = fs::read_to_string(&ks6_bak).ok().filter(|s| !s.is_empty()) {
                    let _ = run_stdin("ip6tables-restore", &[], &r6);
                }
            }
            Err("Kill switch iptables-restore failed".to_string())
        }
    }

    fn stop(&self, cfg: &AppConfig) -> Result<(), String> {
        let ks_bak = cfg.backup_dir.join("iptables_killswitch.bak");
        let ks6_bak = cfg.backup_dir.join("ip6tables_killswitch.bak");

        if let Some(r) = fs::read_to_string(&ks_bak).ok().filter(|s| !s.is_empty()) {
            let _ = run_stdin("iptables-restore", &[], &r);
        } else {
            warn(t(
                "No non-empty Kill Switch backup found; resetting filter policy to ACCEPT.",
                "Boş olmayan Kill Switch yedeği bulunamadı; filtre politikası ACCEPT olarak sıfırlanıyor."
            ));
            for args in [
                &["-F"][..],
                &["-P", "INPUT", "ACCEPT"],
                &["-P", "FORWARD", "ACCEPT"],
                &["-P", "OUTPUT", "ACCEPT"],
            ] {
                let _ = run_quiet("iptables", args);
            }
        }
        let _ = fs::remove_file(ks_bak);

        if cmd_exists("ip6tables-restore")
            && fs::metadata(&ks6_bak).map(|m| m.len() > 0).unwrap_or(false)
        {
            if let Ok(r6) = fs::read_to_string(&ks6_bak) {
                let _ = run_stdin("ip6tables-restore", &[], &r6);
            }
        } else if cmd_exists("ip6tables") {
            for args in [
                &["-F"][..],
                &["-P", "INPUT", "ACCEPT"],
                &["-P", "FORWARD", "ACCEPT"],
                &["-P", "OUTPUT", "ACCEPT"],
            ] {
                let _ = run_quiet("ip6tables", args);
            }
        }
        let _ = fs::remove_file(ks6_bak);

        if !is_wsl() {
            let _ = sysctl_write("net.ipv6.conf.all.disable_ipv6", "0");
            let _ = sysctl_write("net.ipv6.conf.default.disable_ipv6", "0");
        }

        set_module_status(&cfg.state_file, "kill_switch", false)?;
        info(t(
            "Network Kill Switch disabled — original firewall rules restored.",
            "Ağ Kill Switch devre dışı bırakıldı — orijinal güvenlik duvarı kuralları geri yüklendi."
        ));
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════
// 3. DNS Changer
// ═══════════════════════════════════════════════════════════════════
pub struct DnsChanger;

impl AnonModule for DnsChanger {
    fn name(&self) -> &'static str {
        "dns_changer"
    }

    fn is_live(&self, cfg: &AppConfig) -> bool {
        cfg.backup_dir.join("resolv.dns.bak").exists()
            || cfg.backup_dir.join("resolv.conf.bak").exists()
    }

    fn start(&self, cfg: &AppConfig) -> Result<(), String> {
        let state = AnonState::load(&cfg.state_file);
        if state.is_enabled("ip_changer") {
            warn(t(
                "The Dns changer is not available while Ip changer is active.",
                "IP değiştirici etkinken DNS değiştirici kullanılamaz."
            ));
            return Ok(());
        }
        if state.is_enabled("doh_shield") {
            warn(t(
                "The Dns changer is not available while DoH Shield is active.",
                "DoH Kalkanı etkinken DNS değiştirici kullanılamaz."
            ));
            return Ok(());
        }
        if state.is_enabled("dns_changer") {
            warn(t(
                "Dns changer is already running",
                "DNS değiştirici zaten çalışıyor"
            ));
            return Ok(());
        }

        backup_resolv_conf(cfg, "dns");
        lock_nm_dns(cfg);

        let content = "# generated by anon\n\
                       nameserver 194.242.2.2\n\
                       nameserver 9.9.9.9\n\
                       nameserver 84.200.69.80\n\
                       nameserver 84.200.70.40\n\
                       nameserver 91.239.100.100\n\
                       nameserver 89.233.43.71\n";
        fs::write(&cfg.resolv_conf, content).map_err(|e| format!("write resolv.conf: {e}"))?;
        let _ = run_quiet("chattr", &["+i", &cfg.resolv_conf.to_string_lossy()]);

        drop_ipv6_dns();
        set_module_status(&cfg.state_file, "dns_changer", true)?;
        info(t(
            "Dns changer successfully enabled (Mullvad/Quad9/DNS.WATCH/UncensoredDNS)",
            "DNS değiştirici başarıyla etkinleştirildi (Mullvad/Quad9/DNS.WATCH/UncensoredDNS)"
        ));
        Ok(())
    }

    fn stop(&self, cfg: &AppConfig) -> Result<(), String> {
        restore_ipv6_dns();
        unlock_nm_dns(cfg);
        restore_resolv_conf(cfg, "dns");
        set_module_status(&cfg.state_file, "dns_changer", false)?;
        info(t(
            "Dns changer successfully disabled",
            "DNS değiştirici başarıyla devre dışı bırakıldı"
        ));
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════
// 4. DoH Shield
// ═══════════════════════════════════════════════════════════════════
pub struct DohShield;

impl AnonModule for DohShield {
    fn name(&self) -> &'static str {
        "doh_shield"
    }

    fn is_live(&self, cfg: &AppConfig) -> bool {
        cfg.backup_dir.join("resolv.doh.bak").exists()
            || cfg.backup_dir.join("resolv.conf.doh.bak").exists()
    }

    fn start(&self, cfg: &AppConfig) -> Result<(), String> {
        let state = AnonState::load(&cfg.state_file);
        if state.is_enabled("ip_changer") {
            warn(t(
                "DoH Shield is not available while Ip changer is active.",
                "IP değiştirici etkinken DoH Kalkanı kullanılamaz."
            ));
            return Ok(());
        }
        if state.is_enabled("dns_changer") {
            warn(t(
                "DoH Shield is not available while Dns changer is active.",
                "DNS değiştirici etkinken DoH Kalkanı kullanılamaz."
            ));
            return Ok(());
        }
        if state.is_enabled("doh_shield") {
            warn(t(
                "DoH Shield is already running.",
                "DoH Kalkanı zaten çalışıyor."
            ));
            return Ok(());
        }

        info(t(
            "Starting DNS-over-HTTPS (DoH) Shield...",
            "DNS-over-HTTPS (DoH) Kalkanı başlatılıyor..."
        ));
        if !cmd_exists("dnscrypt-proxy") {
            info(t(
                "Installing dnscrypt-proxy...",
                "dnscrypt-proxy kuruluyor..."
            ));
            if cmd_exists("apt-get") {
                let ok = run_quiet("apt-get", &["update", "-qq"])
                    && Command::new("apt-get")
                        .env("DEBIAN_FRONTEND", "noninteractive")
                        .args(["install", "-y", "dnscrypt-proxy"])
                        .status()
                        .map(|s| s.success())
                        .unwrap_or(false);
                if !ok {
                    warn(t(
                        "Failed to install dnscrypt-proxy. Please install it manually.",
                        "dnscrypt-proxy kurulamadı. Lütfen manuel olarak kurun."
                    ));
                    return Err("dnscrypt-proxy install failed".to_string());
                }
            } else {
                warn(t(
                    "dnscrypt-proxy is missing and apt-get is unavailable.",
                    "dnscrypt-proxy eksik ve apt-get kullanılamıyor."
                ));
                return Err("dnscrypt-proxy missing".to_string());
            }
        }

        let _ = service_ctrl("unmask", "dnscrypt-proxy", None);
        if !service_ctrl("start", "dnscrypt-proxy", None) {
            warn(t(
                "Failed to start dnscrypt-proxy daemon. Check /etc/dnscrypt-proxy/dnscrypt-proxy.toml.",
                "dnscrypt-proxy servisi başlatılamadı. /etc/dnscrypt-proxy/dnscrypt-proxy.toml dosyasını kontrol edin."
            ));
            return Err("dnscrypt-proxy start failed".to_string());
        }

        thread::sleep(Duration::from_secs(1));
        if !service_ctrl("is-active", "dnscrypt-proxy", None) {
            warn(t(
                "dnscrypt-proxy is not active after startup; aborting resolv.conf override to prevent DNS outage.",
                "dnscrypt-proxy başlatma sonrası aktif değil; DNS kesintisini önlemek için resolv.conf değişikliği iptal ediliyor."
            ));
            return Err("dnscrypt-proxy not active".to_string());
        }

        backup_resolv_conf(cfg, "doh");
        lock_nm_dns(cfg);

        fs::write(
            &cfg.resolv_conf,
            "# DoH Shield (DNSCrypt) managed by Anon\nnameserver 127.0.2.1\nnameserver 127.0.0.1\noptions edns0\n",
        )
        .map_err(|e| format!("write resolv.conf: {e}"))?;
        let _ = run_quiet("chattr", &["+i", &cfg.resolv_conf.to_string_lossy()]);

        drop_ipv6_dns();
        set_module_status(&cfg.state_file, "doh_shield", true)?;
        msg(t(
            "DNS-over-HTTPS (DoH) Shield activated.",
            "DNS-over-HTTPS (DoH) Kalkanı etkinleştirildi."
        ));
        Ok(())
    }

    fn stop(&self, cfg: &AppConfig) -> Result<(), String> {
        info(t(
            "Stopping DNS-over-HTTPS (DoH) Shield...",
            "DNS-over-HTTPS (DoH) Kalkanı durduruluyor..."
        ));
        let _ = service_ctrl("stop", "dnscrypt-proxy", None);
        restore_ipv6_dns();
        unlock_nm_dns(cfg);
        restore_resolv_conf(cfg, "doh");
        set_module_status(&cfg.state_file, "doh_shield", false)?;
        msg(t(
            "DoH Shield deactivated.",
            "DoH Kalkanı devre dışı bırakıldı."
        ));
        Ok(())
    }
}

fn drop_ipv6_dns() {
    if cmd_exists("ip6tables") {
        if !run_quiet("ip6tables", &["-C", "OUTPUT", "-p", "udp", "--dport", "53", "-j", "DROP"]) {
            let _ = run_quiet("ip6tables", &["-I", "OUTPUT", "1", "-p", "udp", "--dport", "53", "-j", "DROP"]);
        }
        if !run_quiet("ip6tables", &["-C", "OUTPUT", "-p", "tcp", "--dport", "53", "-j", "DROP"]) {
            let _ = run_quiet("ip6tables", &["-I", "OUTPUT", "1", "-p", "tcp", "--dport", "53", "-j", "DROP"]);
        }
    }
}

fn restore_ipv6_dns() {
    if cmd_exists("ip6tables") {
        let _ = run_quiet("ip6tables", &["-D", "OUTPUT", "-p", "udp", "--dport", "53", "-j", "DROP"]);
        let _ = run_quiet("ip6tables", &["-D", "OUTPUT", "-p", "tcp", "--dport", "53", "-j", "DROP"]);
    }
}

// ═══════════════════════════════════════════════════════════════════
// 5. Tor Bridges
// ═══════════════════════════════════════════════════════════════════
pub struct TorBridges;

pub fn is_valid_bridge_line(line: &str) -> bool {
    let mut parts = line.split_whitespace();
    let first = match parts.next() {
        Some(v) => v,
        None => return false,
    };
    let transport = if first == "Bridge" {
        match parts.next() {
            Some(t) => t,
            None => return false,
        }
    } else {
        first
    };
    matches!(transport, "obfs4" | "meek_lite" | "snowflake" | "webtunnel") && parts.next().is_some()
}

impl AnonModule for TorBridges {
    fn name(&self) -> &'static str {
        "tor_bridges"
    }

    fn is_live(&self, cfg: &AppConfig) -> bool {
        AnonState::load(&cfg.state_file).is_enabled("tor_bridges")
    }

    fn start(&self, cfg: &AppConfig) -> Result<(), String> {
        let state = AnonState::load(&cfg.state_file);
        if state.is_enabled("tor_bridges") {
            warn(t(
                "Tor bridges is already enabled",
                "Tor köprüleri zaten etkin"
            ));
            return Ok(());
        }
        if !cmd_exists("obfs4proxy") {
            warn(t(
                "obfs4proxy is not installed. Please run: sudo apt-get install obfs4proxy",
                "obfs4proxy kurulu değil. Lütfen çalıştırın: sudo apt-get install obfs4proxy"
            ));
            return Err("obfs4proxy missing".to_string());
        }

        if is_stdin_tty() {
            println!(
                "\n {}ℹ {}:{} {}",
                crate::ui::CYAN,
                t("INFO", "BİLGİ"),
                crate::ui::RESET,
                t(
                    "Please paste your Tor Bridge lines (obfs4/webtunnel/snowflake) below.",
                    "Lütfen Tor Köprü (obfs4/webtunnel/snowflake) satırlarınızı aşağıya yapıştırın."
                )
            );
            println!(
                " {DIM}{}{RESET}\n",
                t(
                    "Press [ENTER] on an empty line when done (or immediately to use saved/default bridges).",
                    "Tamamlandığında boş satırda [ENTER]'a basın (veya kayıtlı/varsayılan köprüleri kullanmak için hemen basın)."
                )
            );
            let stdin = io::stdin();
            let mut valid = String::new();
            let mut invalid_count = 0usize;
            for line_res in stdin.lock().lines() {
                let Ok(raw) = line_res else { break };
                let line = raw.trim_end_matches('\r');
                if line.is_empty() {
                    break;
                }
                if is_valid_bridge_line(line) {
                    valid.push_str(line);
                    valid.push('\n');
                } else {
                    invalid_count += 1;
                    warn(&format!(
                        "{}: {line}",
                        t("Ignored malformed bridge line", "Geçersiz köprü satırı yoksayıldı")
                    ));
                }
            }
            if !valid.is_empty() {
                let target = cfg.src_dir.join("sources/bridges.txt");
                if let Some(p) = target.parent() {
                    let _ = fs::create_dir_all(p);
                }
                let _ = fs::write(target, valid);
            } else if invalid_count > 0 {
                warn(t(
                    "No valid bridge lines provided.",
                    "Geçerli köprü satırı girilmedi."
                ));
            }
        }

        // Ensure default fallback file exists if user hasn't created one
        let default_path = cfg.src_dir.join("sources/bridges.default.txt");
        if !default_path.exists() {
            if let Some(p) = default_path.parent() {
                let _ = fs::create_dir_all(p);
            }
            let _ = fs::write(&default_path, DEFAULT_BRIDGES_EMBEDDED);
        }

        if let Some(resolved) = resolve_bridges_file(cfg) {
            set_module_status(&cfg.state_file, "tor_bridges", true)?;
            let src_name = resolved
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("bridges");
            info(&format!(
                "{} ({}: {src_name})",
                t(
                    "Tor bridges configured and enabled",
                    "Tor köprüleri yapılandırıldı ve etkinleştirildi"
                ),
                t("Source", "Kaynak")
            ));
            Ok(())
        } else {
            warn(t(
                "No bridges were found or saved. Tor bridges module was NOT enabled.",
                "Hiçbir köprü bulunamadı veya kaydedilmedi. Tor köprüleri modülü etkinleştirilmedi."
            ));
            Err("No bridges available".to_string())
        }
    }

    fn stop(&self, cfg: &AppConfig) -> Result<(), String> {
        set_module_status(&cfg.state_file, "tor_bridges", false)?;
        info(t(
            "Tor bridges successfully disabled (Bridges saved for next use)",
            "Tor köprüleri başarıyla devre dışı bırakıldı (Köprüler sonraki kullanım için saklandı)"
        ));
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════
// 6. DPI Spoofing (Snowflake WebRTC Camouflage)
// ═══════════════════════════════════════════════════════════════════
pub struct DpiSpoofing;

impl AnonModule for DpiSpoofing {
    fn name(&self) -> &'static str {
        "dpi_spoofing"
    }

    fn is_live(&self, cfg: &AppConfig) -> bool {
        AnonState::load(&cfg.state_file).is_enabled("dpi_spoofing")
    }

    fn start(&self, cfg: &AppConfig) -> Result<(), String> {
        let state = AnonState::load(&cfg.state_file);
        if state.is_enabled("dpi_spoofing") {
            warn(t(
                "DPI Spoofing is already enabled.",
                "DPI Yanıltma zaten etkin."
            ));
            return Ok(());
        }
        if !cmd_exists("snowflake-client") {
            warn(t(
                "snowflake-client is not installed. Attempting to install...",
                "snowflake-client kurulu değil. Kurulum deneniyor..."
            ));
            if cmd_exists("apt-get") {
                let ok = run_quiet("apt-get", &["update", "-qq"])
                    && Command::new("apt-get")
                        .env("DEBIAN_FRONTEND", "noninteractive")
                        .args(["install", "-y", "snowflake-client"])
                        .status()
                        .map(|s| s.success())
                        .unwrap_or(false);
                if !ok {
                    warn(t(
                        "snowflake-client installation failed. Please run: sudo apt-get install snowflake-client",
                        "snowflake-client kurulumu başarısız oldu. Lütfen çalıştırın: sudo apt-get install snowflake-client"
                    ));
                    return Err("snowflake-client install failed".to_string());
                }
            } else {
                warn(t(
                    "snowflake-client is not installed and apt-get is unavailable.",
                    "snowflake-client kurulu değil ve apt-get kullanılamıyor."
                ));
                return Err("snowflake-client missing".to_string());
            }
        }

        set_module_status(&cfg.state_file, "dpi_spoofing", true)?;
        info(t(
            "DPI Spoofing (Snowflake WebRTC Camouflage) successfully enabled.",
            "DPI Yanıltma (Snowflake WebRTC Kamufilajı) başarıyla etkinleştirildi."
        ));
        if state.is_enabled("ip_changer") {
            info(t(
                "Refreshing Tor transparent proxy configuration...",
                "Tor şeffaf vekil sunucu yapılandırması yenileniyor..."
            ));
            let _ = IpChanger.stop(cfg);
            let _ = IpChanger.start(cfg);
        }
        Ok(())
    }

    fn stop(&self, cfg: &AppConfig) -> Result<(), String> {
        let state = AnonState::load(&cfg.state_file);
        set_module_status(&cfg.state_file, "dpi_spoofing", false)?;
        info(t(
            "DPI Spoofing successfully disabled.",
            "DPI Yanıltma başarıyla devre dışı bırakıldı."
        ));
        if state.is_enabled("ip_changer") {
            info(t(
                "Refreshing Tor transparent proxy configuration...",
                "Tor şeffaf vekil sunucu yapılandırması yenileniyor..."
            ));
            let _ = IpChanger.stop(cfg);
            let _ = IpChanger.start(cfg);
        }
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════
// 7. I2P Integration
// ═══════════════════════════════════════════════════════════════════
pub struct I2pIntegration;

impl AnonModule for I2pIntegration {
    fn name(&self) -> &'static str {
        "i2p_integration"
    }

    fn is_live(&self, _cfg: &AppConfig) -> bool {
        service_ctrl("is-active", "i2pd", None)
    }

    fn start(&self, cfg: &AppConfig) -> Result<(), String> {
        let state = AnonState::load(&cfg.state_file);
        if state.is_enabled("i2p_integration") {
            warn(t(
                "I2P integration is already running",
                "I2P entegrasyonu zaten çalışıyor"
            ));
            return Ok(());
        }
        info(t(
            "Starting I2P Router daemon (i2pd)...",
            "I2P Yönlendirici servisi (i2pd) başlatılıyor..."
        ));
        let _ = service_ctrl("start", "i2pd", None);
        thread::sleep(Duration::from_secs(2));

        if service_ctrl("is-active", "i2pd", None) {
            set_module_status(&cfg.state_file, "i2p_integration", true)?;
            info(t(
                "I2P Router successfully started.",
                "I2P Yönlendirici başarıyla başlatıldı."
            ));
            info(t(
                "I2P SOCKS5 proxy available at 127.0.0.1:4446",
                "I2P SOCKS5 vekil sunucusu 127.0.0.1:4446 adresinde hazır"
            ));
            info(t(
                "I2P HTTP proxy available at 127.0.0.1:4444",
                "I2P HTTP vekil sunucusu 127.0.0.1:4444 adresinde hazır"
            ));
            info(t(
                "I2P Web Console available at http://127.0.0.1:7070",
                "I2P Web Konsolu http://127.0.0.1:7070 adresinde hazır"
            ));
        } else {
            warn(t(
                "Failed to start i2pd service. Is it installed?",
                "i2pd servisi başlatılamadı. Kurulu olduğundan emin misiniz?"
            ));
        }
        Ok(())
    }

    fn stop(&self, cfg: &AppConfig) -> Result<(), String> {
        let _ = service_ctrl("stop", "i2pd", None);
        set_module_status(&cfg.state_file, "i2p_integration", false)?;
        info(t(
            "I2P Router successfully disabled",
            "I2P Yönlendirici başarıyla devre dışı bırakıldı"
        ));
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════
// 8. Proxychains Integration
// ═══════════════════════════════════════════════════════════════════
pub struct ProxychainsIntegration;

impl AnonModule for ProxychainsIntegration {
    fn name(&self) -> &'static str {
        "proxychains_integration"
    }

    fn is_live(&self, cfg: &AppConfig) -> bool {
        cfg.backup_dir.join("proxychains4.conf.bak").is_file()
    }

    fn start(&self, cfg: &AppConfig) -> Result<(), String> {
        let state = AnonState::load(&cfg.state_file);
        if state.is_enabled("proxychains_integration") {
            warn(t(
                "Proxychains integration is already running",
                "Proxychains entegrasyonu zaten çalışıyor"
            ));
            return Ok(());
        }
        if !cfg.proxychains_conf.is_file() {
            warn(t(
                "proxychains4.conf not found. Is proxychains4 installed?",
                "proxychains4.conf bulunamadı. proxychains4 kurulu mu?"
            ));
            return Ok(());
        }
        let orig = fs::read_to_string(&cfg.proxychains_conf)
            .map_err(|e| format!("read proxychains4.conf: {e}"))?;
        let _ = fs::write(cfg.backup_dir.join("proxychains4.conf.bak"), &orig);

        let mut updated = String::with_capacity(orig.len() + 32);
        for line in orig.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("socks4 ")
                || trimmed.starts_with("socks4\t")
                || trimmed.starts_with("socks5 ")
                || trimmed.starts_with("socks5\t")
            {
                updated.push_str("socks5  127.0.0.1 9050\n");
            } else if line.starts_with("#quiet_mode") {
                updated.push_str("quiet_mode");
                updated.push_str(&line["#quiet_mode".len()..]);
                updated.push('\n');
            } else {
                updated.push_str(line);
                updated.push('\n');
            }
        }
        fs::write(&cfg.proxychains_conf, updated)
            .map_err(|e| format!("write proxychains4.conf: {e}"))?;
        set_module_status(&cfg.state_file, "proxychains_integration", true)?;
        info(t(
            "Proxychains-ng successfully integrated with Tor Socks5",
            "Proxychains-ng, Tor Socks5 ile başarıyla entegre edildi"
        ));
        Ok(())
    }

    fn stop(&self, cfg: &AppConfig) -> Result<(), String> {
        let bak = cfg.backup_dir.join("proxychains4.conf.bak");
        if bak.is_file() && cfg.proxychains_conf.is_file() {
            if let Ok(orig) = fs::read(&bak) {
                let _ = fs::write(&cfg.proxychains_conf, orig);
            }
            let _ = fs::remove_file(bak);
        }
        set_module_status(&cfg.state_file, "proxychains_integration", false)?;
        info(t(
            "Proxychains-ng integration successfully disabled",
            "Proxychains-ng entegrasyonu başarıyla devre dışı bırakıldı"
        ));
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════
// 9. MAC Changer
// ═══════════════════════════════════════════════════════════════════
pub struct MacChanger;

const VENDOR_OUIS: [&str; 31] = [
    "00:17:f2", "3c:07:54", "a4:83:e7", "f0:18:98",
    "00:1b:21", "00:21:6a", "34:13:e8", "68:05:ca",
    "00:14:22", "18:66:da", "ec:f4:bb", "f8:bc:12",
    "00:26:37", "34:23:ba", "50:56:bf", "e4:58:b8",
    "00:1a:a1", "00:25:84", "58:97:bd",
    "00:1e:0b", "3c:d9:2b", "70:5a:0f",
    "00:59:07", "60:99:d1", "80:2b:f9",
    "04:92:26", "2c:4d:54", "70:8b:cd",
    "00:e0:4c", "52:54:00", "b8:8a:ec",
];

fn is_vm_or_wsl() -> bool {
    if is_wsl() {
        return true;
    }
    if cmd_exists("dmidecode") {
        let mfg = run_capture("dmidecode", &["-s", "system-manufacturer"]).unwrap_or_default();
        let prod = run_capture("dmidecode", &["-s", "system-product-name"]).unwrap_or_default();
        for pat in [
            "innotek",
            "VMware",
            "QEMU",
            "Xen",
            "Microsoft Corporation",
            "Parallels",
        ] {
            if mfg.contains(pat) {
                return true;
            }
        }
        for pat in ["VirtualBox", "VMware", "KVM", "Bochs", "HVM", "Virtual Machine"] {
            if prod.contains(pat) {
                return true;
            }
        }
    }
    false
}

impl AnonModule for MacChanger {
    fn name(&self) -> &'static str {
        "mac_changer"
    }

    fn is_live(&self, cfg: &AppConfig) -> bool {
        cfg.backup_dir.join("mac_addresses").is_dir()
    }

    fn start(&self, cfg: &AppConfig) -> Result<(), String> {
        let state = AnonState::load(&cfg.state_file);
        if state.is_enabled("mac_changer") {
            warn(t(
                "Mac changer is already running",
                "MAC değiştirici zaten çalışıyor"
            ));
            return Ok(());
        }

        let mac_bak_dir = cfg.backup_dir.join("mac_addresses");
        let _ = fs::create_dir_all(&mac_bak_dir);

        let mut ifaces = Vec::new();
        for iface in list_interfaces() {
            if is_virtual_iface(&iface) {
                continue;
            }
            ifaces.push(iface.clone());
            let sys_addr = format!("/sys/class/net/{iface}/address");
            if let Ok(addr) = fs::read_to_string(&sys_addr) {
                let _ = fs::write(mac_bak_dir.join(&iface), addr);
            }
        }

        let mut changed = false;
        let spoof_iface = |target: &str, changed_flag: &mut bool| {
            let sys_addr = format!("/sys/class/net/{target}/address");
            let orig_mac = fs::read_to_string(&sys_addr)
                .map(|s| s.trim().to_string())
                .unwrap_or_default();

            let mut out = String::new();
            let mut spoof_ok = false;

            for _ in 0..10 {
                let r = rand_u64();
                let oui = VENDOR_OUIS[(r as usize) % VENDOR_OUIS.len()];
                let b1 = ((r >> 8) & 0xff) as u8;
                let b2 = ((r >> 16) & 0xff) as u8;
                let b3 = ((r >> 24) & 0xff) as u8;
                out = format!("{oui}:{b1:02x}:{b2:02x}:{b3:02x}");

                let _ = run_quiet("ip", &["link", "set", target, "down"]);
                if run_quiet("ip", &["link", "set", target, "address", &out]) {
                    let _ = run_quiet("ip", &["link", "set", target, "up"]);
                    spoof_ok = true;
                    break;
                }

                // Fallback locally-administered unicast MAC
                let nibbles = ['0','1','2','3','4','5','6','7','8','9','a','b','c','d','e','f'];
                let second_opts = ['2', '6', 'a', 'e'];
                let r2 = rand_u64();
                let first_hex = nibbles[((r2 >> 4) as usize) % 16];
                let second_hex = second_opts[((r2 >> 8) as usize) % 4];
                let e1 = ((r2 >> 16) & 0xff) as u8;
                let e2 = ((r2 >> 24) & 0xff) as u8;
                out = format!("{first_hex}{second_hex}:{b1:02x}:{b2:02x}:{b3:02x}:{e1:02x}:{e2:02x}");

                if run_quiet("ip", &["link", "set", target, "address", &out]) {
                    let _ = run_quiet("ip", &["link", "set", target, "up"]);
                    spoof_ok = true;
                    break;
                }
                let _ = run_quiet("ip", &["link", "set", target, "up"]);
            }

            if spoof_ok {
                let new_mac = fs::read_to_string(&sys_addr)
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();
                if orig_mac.is_empty() || new_mac != orig_mac || new_mac == out {
                    *changed_flag = true;
                    info(&format!(
                        "{} {target} ({}: {out})",
                        t(
                            "Mac changer enabled for",
                            "MAC değiştirici etkinleştirildi:"
                        ),
                        t("Spoofed to", "Sahte MAC")
                    ));
                } else {
                    warn(&format!(
                        "{} {target} ({new_mac})",
                        t(
                            "Driver ignored MAC change on",
                            "Sürücü bu arayüzde MAC değişikliğini yoksaydı:"
                        )
                    ));
                }
            } else {
                warn(&format!(
                    "{} {target}",
                    t(
                        "Failed to assign spoofed MAC on",
                        "Sahte MAC ataması başarısız:"
                    )
                ));
            }
        };

        if is_vm_or_wsl() {
            let nm_wifi = run_capture("nmcli", &["dev"])
                .map(|s| s.to_ascii_lowercase().contains("wifi"))
                .unwrap_or(false);
            if nm_wifi {
                for iface in &ifaces {
                    if iface != "eth0" && iface != "ens33" && iface != "enp0s3" {
                        spoof_iface(iface, &mut changed);
                    }
                }
            } else {
                warn(t(
                    "Virtual machine (or WSL) detected. No suitable wireless interface found for safe MAC spoofing.",
                    "Sanal makine (veya WSL) tespit edildi. Güvenli MAC değişikliği için uygun kablosuz arayüz bulunamadı."
                ));
            }
        } else {
            for iface in &ifaces {
                spoof_iface(iface, &mut changed);
            }
        }

        if changed {
            let _ = wait_for_network(4);
            set_module_status(&cfg.state_file, "mac_changer", true)?;
        } else {
            warn(t(
                "MAC Changer could not modify any physical interface.",
                "MAC Değiştirici hiçbir fiziksel arayüzü değiştiremedi."
            ));
        }
        Ok(())
    }

    fn stop(&self, cfg: &AppConfig) -> Result<(), String> {
        let mac_bak_dir = cfg.backup_dir.join("mac_addresses");
        if mac_bak_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(&mac_bak_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_file() {
                        continue;
                    }
                    if let Some(devname) = path.file_name().and_then(|s| s.to_str()) {
                        if let Ok(orig_mac) = fs::read_to_string(&path) {
                            let orig = orig_mac.trim();
                            if !orig.is_empty() {
                                let _ = run_quiet("ip", &["link", "set", devname, "down"]);
                                let _ = run_quiet("ip", &["link", "set", devname, "address", orig]);
                                let _ = run_quiet("ip", &["link", "set", devname, "up"]);
                            }
                        }
                    }
                }
            }
            let _ = fs::remove_dir_all(&mac_bak_dir);
            let _ = wait_for_network(4);
        }
        set_module_status(&cfg.state_file, "mac_changer", false)?;
        info(t(
            "Mac changer successfully disabled",
            "MAC değiştirici başarıyla devre dışı bırakıldı"
        ));
        Ok(())
    }
}

pub fn strip_sandbox_marker_lines(raw: &str) -> String {
    raw.lines()
        .filter(|l| !l.trim_end().ends_with("# ANON_SANDBOX"))
        .map(|l| format!("{l}\n"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_validation_matches_bash() {
        assert!(is_valid_bridge_line("obfs4 1.2.3.4:443 ABC"));
        assert!(is_valid_bridge_line("Bridge snowflake 192.0.2.3:80 FP"));
        assert!(!is_valid_bridge_line("bad line"));
        assert!(!is_valid_bridge_line("obfs4"));
    }

    #[test]
    fn strip_sandbox_marker_removes_only_anon_sandbox_lines() {
        let sample = "SocksPort 9050\nSandbox 1 # ANON_SANDBOX\nTransPort 9040\n";
        let stripped = strip_sandbox_marker_lines(sample);
        assert_eq!(stripped, "SocksPort 9050\nTransPort 9040\n");
    }
}
