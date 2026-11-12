use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::Duration;

use crate::config::AppConfig;
use crate::helpers::{
    dep_check, extract_json_string_field, get_default_iface, is_tor_running, run_capture,
    tor_signal_newnym,
};
use crate::modules::utils::{
    clean_meta_target, get_backups, self_update, start_disposable_browser, trigger_panic,
};
use crate::modules::start_module;
use crate::registry::{
    profile_indices, ALL_MODULES, DEF_CAT_INDICES, EXEC_ORDER, NET_CAT_INDICES, OBF_CAT_INDICES,
};
use crate::state::AnonState;
use crate::sys::{poll_fd, read_fd_byte, TuiTermGuard, STDIN_FILENO};
use crate::ui::{
    banner, boxline, clear_screen, draw_end, draw_line, draw_sep, emptybox, hide_cursor, info,
    msg, show_cursor, warn, BLUE, BOLD, CYAN, DIM, GREEN, MAGENTA, RED, RESET, WHITE, YELLOW,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum NavKey {
    Up,
    Down,
    Select,
    All,
    Esc,
    Quit,
    Other,
}

fn read_nav_key() -> Option<NavKey> {
    if !poll_fd(STDIN_FILENO, -1) {
        return None;
    }
    let b = read_fd_byte(STDIN_FILENO)?;
    if b == 0x1b {
        if poll_fd(STDIN_FILENO, 50) {
            if let Some(b1) = read_fd_byte(STDIN_FILENO) {
                if b1 == b'[' && poll_fd(STDIN_FILENO, 50) {
                    if let Some(b2) = read_fd_byte(STDIN_FILENO) {
                        return Some(match b2 {
                            b'A' => NavKey::Up,
                            b'B' => NavKey::Down,
                            _ => NavKey::Esc,
                        });
                    }
                }
            }
        }
        return Some(NavKey::Esc);
    }
    Some(match b {
        b'\n' | b'\r' | b' ' => NavKey::Select,
        b'k' | b'K' => NavKey::Up,
        b'j' | b'J' => NavKey::Down,
        b'a' | b'A' => NavKey::All,
        b'q' | b'Q' => NavKey::Quit,
        _ => NavKey::Other,
    })
}

fn wait_any_key(prompt: &str) {
    print!("{prompt}");
    let _ = io::stdout().flush();
    let _guard = TuiTermGuard::enter();
    let _ = read_nav_key();
}

fn interactive_menu(cfg: &AppConfig, title: &str, opts: &[&str], stat_msg: &str) -> Option<usize> {
    let mut selected = 0usize;
    let _guard = TuiTermGuard::enter();
    hide_cursor();

    loop {
        banner(cfg);
        let mut buf = String::with_capacity(2048);
        draw_line(&mut buf);
        emptybox(&mut buf);
        boxline(&mut buf, &format!("{BOLD}{CYAN}  {title}{RESET}"));
        emptybox(&mut buf);
        draw_sep(&mut buf);

        for (i, opt) in opts.iter().enumerate() {
            if i == selected {
                boxline(&mut buf, &format!("{BOLD}{GREEN}  ▸ {opt}{RESET}"));
            } else {
                boxline(&mut buf, &format!("{DIM}    {opt}{RESET}"));
            }
        }

        draw_sep(&mut buf);
        emptybox(&mut buf);
        if !stat_msg.is_empty() {
            boxline(&mut buf, &format!("{YELLOW}  ◆ {stat_msg}{RESET}"));
        }
        let active_count = AnonState::load(&cfg.state_file).count_active();
        if active_count > 0 {
            boxline(
                &mut buf,
                &format!("{GREEN}  ● {active_count} module(s) active{RESET}"),
            );
        }
        boxline(&mut buf, &format!("{DIM}  ↑↓/jk Navigate  ⏎ Select  q Quit{RESET}"));
        emptybox(&mut buf);
        draw_end(&mut buf);

        print!("{buf}");
        let _ = io::stdout().flush();

        let Some(key) = read_nav_key() else {
            continue;
        };
        match key {
            NavKey::Up => {
                if selected == 0 {
                    selected = opts.len().saturating_sub(1);
                } else {
                    selected -= 1;
                }
            }
            NavKey::Down => {
                selected = (selected + 1) % opts.len().max(1);
            }
            NavKey::Select => {
                show_cursor();
                return Some(selected);
            }
            NavKey::Quit => {
                show_cursor();
                clear_screen();
                return None;
            }
            _ => {}
        }
    }
}

fn interactive_toggles(cfg: &AppConfig, choices: &mut [bool; 22]) -> bool {
    let mut visual_order = Vec::with_capacity(22);
    visual_order.extend_from_slice(&NET_CAT_INDICES);
    visual_order.extend_from_slice(&OBF_CAT_INDICES);
    visual_order.extend_from_slice(&DEF_CAT_INDICES);
    let total_items = visual_order.len();
    let mut cursor = 0usize;

    let _guard = TuiTermGuard::enter();
    hide_cursor();

    loop {
        let current_idx = if cursor < total_items {
            Some(visual_order[cursor])
        } else {
            None
        };

        banner(cfg);
        let mut buf = String::with_capacity(4096);
        draw_line(&mut buf);
        emptybox(&mut buf);
        boxline(
            &mut buf,
            &format!("{BOLD}{CYAN}  CUSTOM MODULE CONFIGURATION ({total_items} Modules){RESET}"),
        );
        emptybox(&mut buf);
        draw_sep(&mut buf);

        let render_cat = |buf: &mut String, heading: &str, indices: &[usize]| {
            boxline(buf, heading);
            for &idx in indices {
                let mark = if choices[idx] {
                    format!("{GREEN}✓{RESET}")
                } else {
                    format!("{RED}✗{RESET}")
                };
                let label = ALL_MODULES[idx].label;
                if current_idx == Some(idx) {
                    boxline(
                        buf,
                        &format!("{BOLD}{CYAN}  ▸ [{mark}{BOLD}{CYAN}] {label}{RESET}"),
                    );
                } else {
                    boxline(buf, &format!("    [{mark}] {label}"));
                }
            }
        };

        render_cat(
            &mut buf,
            &format!("{BOLD}{BLUE}  ── Network & Routing ──{RESET}"),
            &NET_CAT_INDICES,
        );
        emptybox(&mut buf);
        render_cat(
            &mut buf,
            &format!("{BOLD}{MAGENTA}  ── System Obfuscation ──{RESET}"),
            &OBF_CAT_INDICES,
        );
        emptybox(&mut buf);
        render_cat(
            &mut buf,
            &format!("{BOLD}{RED}  ── Active Defense ──{RESET}"),
            &DEF_CAT_INDICES,
        );

        emptybox(&mut buf);
        if cursor == total_items {
            boxline(&mut buf, &format!("{BOLD}{YELLOW}  ▸ ◀ BACK TO MAIN MENU{RESET}"));
        } else {
            boxline(&mut buf, &format!("{DIM}    ◀ BACK TO MAIN MENU{RESET}"));
        }

        draw_sep(&mut buf);
        emptybox(&mut buf);
        boxline(
            &mut buf,
            &format!("{DIM}  ↑↓/jk Navigate  ⏎/Space Toggle  a All/None  q Quit{RESET}"),
        );
        emptybox(&mut buf);
        draw_end(&mut buf);

        print!("{buf}");
        let _ = io::stdout().flush();

        let Some(key) = read_nav_key() else {
            continue;
        };
        match key {
            NavKey::Up => {
                if cursor == 0 {
                    cursor = total_items;
                } else {
                    cursor -= 1;
                }
            }
            NavKey::Down => {
                cursor = if cursor >= total_items { 0 } else { cursor + 1 };
            }
            NavKey::Select => {
                if cursor == total_items {
                    show_cursor();
                    return true;
                }
                let mod_i = visual_order[cursor];
                choices[mod_i] = !choices[mod_i];
            }
            NavKey::All => {
                let any_unchecked = visual_order.iter().any(|&i| !choices[i]);
                for &i in &visual_order {
                    choices[i] = any_unchecked;
                }
            }
            NavKey::Esc => {
                show_cursor();
                return true;
            }
            NavKey::Quit => {
                show_cursor();
                clear_screen();
                return false;
            }
            _ => {}
        }
    }
}

pub fn print_status_pretty(cfg: &AppConfig) {
    let state = AnonState::load(&cfg.state_file);
    println!(" {BOLD}{MAGENTA}ANON SYSTEM STATUS{RESET}");
    println!(" {DIM}─────────────────────────────────────────────────────────{RESET}");
    for m in ALL_MODULES {
        if state.is_enabled(m.name) {
            println!(" {CYAN}●{RESET} {:<25} : {GREEN}{BOLD}ACTIVE{RESET}", m.label);
        } else {
            println!(" {DIM}○ {:<25} : INACTIVE{RESET}", m.label);
        }
    }
    println!(" {DIM}─────────────────────────────────────────────────────────{RESET}\n");
}

pub fn run_dashboard(cfg: &AppConfig) {
    let _guard = TuiTermGuard::enter();
    hide_cursor();

    let net_status = Arc::new(Mutex::new((
        "WAITING".to_string(),
        "Detecting...".to_string(),
    )));
    let stop_flag = Arc::new(AtomicBool::new(false));

    let bg_cfg = cfg.clone();
    let bg_status = Arc::clone(&net_status);
    let bg_stop = Arc::clone(&stop_flag);

    let handle = thread::spawn(move || {
        while !bg_stop.load(Ordering::Relaxed) {
            let state = AnonState::load(&bg_cfg.state_file);
            let next = if state.is_enabled("ip_changer") {
                if let Some(tor_check) = run_capture(
                    "curl",
                    &[
                        "-s",
                        "--max-time",
                        "10",
                        "--socks5-hostname",
                        "127.0.0.1:9050",
                        "https://check.torproject.org/api/ip",
                    ],
                ) {
                    if tor_check.contains("\"IsTor\":true") {
                        let ip = extract_json_string_field(&tor_check, "IP")
                            .unwrap_or_else(|| "Hidden".to_string());
                        ("SECURE_TOR".to_string(), ip)
                    } else {
                        ("SECURE_SLOW".to_string(), "Hidden".to_string())
                    }
                } else {
                    ("SECURE_SLOW".to_string(), "Hidden".to_string())
                }
            } else if state.is_enabled("i2p_integration") {
                ("SECURE_I2P".to_string(), "Hidden".to_string())
            } else if state.is_enabled("kill_switch") {
                (
                    "LOCKED".to_string(),
                    "Blocked (no anonymity network up)".to_string(),
                )
            } else {
                ("UNSECURE".to_string(), "Direct (Unprotected)".to_string())
            };

            if let Ok(mut guard) = bg_status.lock() {
                *guard = next;
            }
            for _ in 0..50 {
                if bg_stop.load(Ordering::Relaxed) {
                    return;
                }
                thread::sleep(Duration::from_millis(100));
            }
        }
    });

    loop {
        banner(cfg);
        let mut buf = String::with_capacity(2048);
        draw_line(&mut buf);
        emptybox(&mut buf);
        boxline(&mut buf, &format!("{BOLD}{MAGENTA}  ◈ LIVE DASHBOARD{RESET}"));
        emptybox(&mut buf);
        draw_sep(&mut buf);

        let (net_state, net_ip) = net_status
            .lock()
            .map(|g| g.clone())
            .unwrap_or_else(|_| ("WAITING".to_string(), "Detecting...".to_string()));

        match net_state.as_str() {
            "SECURE_TOR" => {
                boxline(&mut buf, &format!("{GREEN}  ● Network    {BOLD}SECURE{RESET}{GREEN} (Tor Active){RESET}"));
                boxline(&mut buf, &format!("{CYAN}  ● Exit IP    {YELLOW}{net_ip}{RESET}"));
            }
            "SECURE_SLOW" => {
                boxline(&mut buf, &format!("{YELLOW}  ● Network    {BOLD}SECURE{RESET}{YELLOW} (Routing / High Latency){RESET}"));
                boxline(&mut buf, &format!("{CYAN}  ● Exit IP    {YELLOW}Hidden (Tor Encrypted){RESET}"));
            }
            "SECURE_I2P" => {
                boxline(&mut buf, &format!("{GREEN}  ● Network    {BOLD}SECURE{RESET}{GREEN} (I2P Router Active){RESET}"));
                boxline(&mut buf, &format!("{CYAN}  ● Exit IP    {YELLOW}Hidden (I2P){RESET}"));
            }
            "WAITING" => {
                boxline(&mut buf, &format!("{CYAN}  ● Network    {BOLD}SCANNING...{RESET}"));
                boxline(&mut buf, &format!("{CYAN}  ● Exit IP    {YELLOW}Detecting...{RESET}"));
            }
            "LOCKED" => {
                boxline(&mut buf, &format!("{YELLOW}  ● Network    {BOLD}LOCKED{RESET}{YELLOW} (Kill Switch — Tor offline){RESET}"));
                boxline(&mut buf, &format!("{CYAN}  ● IP         {YELLOW}{net_ip}{RESET}"));
            }
            _ => {
                boxline(&mut buf, &format!("{RED}  ○ Network    {BOLD}UNSECURE{RESET}{RED} (Direct){RESET}"));
                boxline(&mut buf, &format!("{CYAN}  ● IP         {YELLOW}{net_ip}{RESET}"));
            }
        }

        if let Some(iface) = get_default_iface() {
            let rx_p = format!("/sys/class/net/{iface}/statistics/rx_bytes");
            let tx_p = format!("/sys/class/net/{iface}/statistics/tx_bytes");
            if let (Ok(rx_s), Ok(tx_s)) = (fs::read_to_string(rx_p), fs::read_to_string(tx_p)) {
                let rx = rx_s.trim().parse::<f64>().unwrap_or(0.0) / 1_048_576.0;
                let tx = tx_s.trim().parse::<f64>().unwrap_or(0.0) / 1_048_576.0;
                boxline(
                    &mut buf,
                    &format!("{CYAN}  ● Traffic    {GREEN}↑{tx:.1}MB {CYAN}↓{rx:.1}MB{RESET}{DIM} on {iface}{RESET}"),
                );
            }
        }

        draw_sep(&mut buf);
        boxline(&mut buf, &format!("{BOLD}{WHITE}  ACTIVE MODULES:{RESET}"));
        emptybox(&mut buf);

        let state = AnonState::load(&cfg.state_file);
        let mut mod_line = String::new();
        let mut count = 0usize;
        let mut line_items = 0usize;
        for m in ALL_MODULES {
            if state.is_enabled(m.name) {
                mod_line.push_str(&format!("{GREEN}■{RESET}{}  ", m.tag));
                count += 1;
                line_items += 1;
                if line_items >= 7 {
                    boxline(&mut buf, &format!("  {mod_line}"));
                    mod_line.clear();
                    line_items = 0;
                }
            }
        }
        if count == 0 {
            boxline(&mut buf, &format!("{DIM}  No modules active{RESET}"));
        } else if !mod_line.is_empty() {
            boxline(&mut buf, &format!("  {mod_line}"));
        }

        emptybox(&mut buf);
        draw_sep(&mut buf);
        emptybox(&mut buf);
        boxline(
            &mut buf,
            &format!("{DIM}  Press {BOLD}q{RESET}{DIM} to return to menu  ·  Refreshes every 3s{RESET}"),
        );
        emptybox(&mut buf);
        draw_end(&mut buf);

        print!("{buf}");
        let _ = io::stdout().flush();

        if poll_fd(STDIN_FILENO, 3000) {
            if let Some(b) = read_fd_byte(STDIN_FILENO) {
                if b == b'q' || b == b'Q' {
                    break;
                }
            }
        }
    }

    stop_flag.store(true, Ordering::Relaxed);
    let _ = handle.join();
    show_cursor();
}

pub fn run_interactive_tui(cfg: &AppConfig, stop_fn: fn(&AppConfig, bool)) {
    dep_check(&[
        "tar",
        "tor",
        "curl",
        "sdmem",
        "obfs4proxy",
        "macchanger",
        "proxychains4",
        "rfkill",
        "i2pd",
    ]);

    if !cfg.backup_dir.join("anon_fix_backups.tar.gz").is_file() {
        get_backups(cfg);
    }

    let mut choices = [false; 22];
    let mut stat = String::new();

    let main_opts = [
        "Load Security Profile",
        "Custom Module Configuration",
        "Live Dashboard",
        "System Status",
        "System Update",
        "APPLY & START",
        "Exit",
    ];
    let prof_opts = [
        "Paranoia Mode  ─  Max Security",
        "Quick Surf  ─  Fast Anonymity",
        "Pentest Mode  ─  Offensive",
        "Darknet Explorer  ─  Tor + I2P",
        "Physical Lockdown  ─  Anti-Forensics",
        "The Ghost  ─  Ultimate (All Features)",
        "◀ Back",
    ];
    let ops_opts = [
        "Live Dashboard",
        "System Status",
        "New Tor Identity",
        "IP Leak Test",
        "Secure Browser",
        "Metadata Sanitizer",
        "Emergency Panic Lockdown",
        "Stop All Modules",
        "Back to Configuration",
        "Exit Anon",
    ];

    loop {
        let Some(sel) = interactive_menu(cfg, "MAIN MENU:", &main_opts, &stat) else {
            return;
        };
        stat.clear();

        match sel {
            0 => {
                let Some(psel) = interactive_menu(cfg, "SELECT SECURITY PROFILE:", &prof_opts, "")
                else {
                    return;
                };
                choices = [false; 22];
                let (pname, label) = match psel {
                    0 => ("paranoia", "◆ Loaded: Paranoia Mode"),
                    1 => ("quick", "◆ Loaded: Quick Surf"),
                    2 => ("pentest", "◆ Loaded: Pentest Mode"),
                    3 => ("darknet", "◆ Loaded: Darknet Explorer"),
                    4 => ("lockdown", "◆ Loaded: Physical Lockdown"),
                    5 => ("ghost", "◆ Loaded: The Ghost (Ultimate)"),
                    _ => ("", ""),
                };
                if !pname.is_empty() {
                    stat = label.to_string();
                    if let Some(idxs) = profile_indices(pname) {
                        for &i in idxs {
                            choices[i] = true;
                        }
                    }
                }
            }
            1 => {
                if !interactive_toggles(cfg, &mut choices) {
                    return;
                }
            }
            2 => run_dashboard(cfg),
            3 => {
                banner(cfg);
                let mut buf = String::new();
                draw_line(&mut buf);
                emptybox(&mut buf);
                boxline(&mut buf, &format!("{BOLD}{MAGENTA}  SYSTEM STATUS{RESET}"));
                emptybox(&mut buf);
                draw_sep(&mut buf);
                print!("{buf}");
                print_status_pretty(cfg);
                let mut end_buf = String::new();
                draw_end(&mut end_buf);
                println!("{end_buf}");
                wait_any_key(&format!("  {DIM}Press any key to return...{RESET}"));
            }
            4 => {
                self_update(cfg);
                return;
            }
            5 => {
                banner(cfg);
                let mut buf = String::new();
                draw_line(&mut buf);
                emptybox(&mut buf);
                boxline(&mut buf, &format!("{BOLD}{CYAN}  APPLYING CONFIGURATION...{RESET}"));
                emptybox(&mut buf);
                draw_end(&mut buf);
                println!("{buf}");

                for &idx in &EXEC_ORDER {
                    if choices[idx] {
                        let _ = start_module(cfg, ALL_MODULES[idx].name);
                    }
                }

                println!();
                let mut done_buf = String::new();
                draw_line(&mut done_buf);
                emptybox(&mut done_buf);
                boxline(
                    &mut done_buf,
                    &format!("{GREEN}{BOLD}  ✓ All selected modules have been activated.{RESET}"),
                );
                emptybox(&mut done_buf);
                draw_end(&mut done_buf);
                println!("{done_buf}");
                wait_any_key(&format!(
                    "  {DIM}Press any key to continue to Operations Menu...{RESET}"
                ));

                // Operations loop
                loop {
                    let Some(osel) = interactive_menu(cfg, "OPERATIONS:", &ops_opts, "") else {
                        return;
                    };
                    match osel {
                        0 => run_dashboard(cfg),
                        1 => {
                            banner(cfg);
                            let mut b = String::new();
                            draw_line(&mut b);
                            emptybox(&mut b);
                            boxline(&mut b, &format!("{BOLD}{MAGENTA}  SYSTEM STATUS{RESET}"));
                            emptybox(&mut b);
                            draw_sep(&mut b);
                            print!("{b}");
                            print_status_pretty(cfg);
                            let mut eb = String::new();
                            draw_end(&mut eb);
                            println!("{eb}");
                            wait_any_key(&format!("  {DIM}Press any key to return...{RESET}"));
                        }
                        2 => {
                            banner(cfg);
                            info("Requesting new Tor circuit...");
                            if is_tor_running() {
                                let _ = tor_signal_newnym();
                                thread::sleep(Duration::from_secs(2));
                                let new_ip = run_capture(
                                    "curl",
                                    &[
                                        "-s",
                                        "--max-time",
                                        "8",
                                        "--socks5-hostname",
                                        "127.0.0.1:9050",
                                        "https://api.ipify.org",
                                    ],
                                )
                                .filter(|s| !s.trim().is_empty())
                                .unwrap_or_else(|| "Unknown".to_string());
                                msg("New Tor circuit established!");
                                info(&format!("New Exit IP: {YELLOW}{}{RESET}", new_ip.trim()));
                            } else {
                                warn("Tor is not running.");
                            }
                            wait_any_key(&format!("\n  {DIM}Press any key to return...{RESET}"));
                        }
                        3 => {
                            banner(cfg);
                            info("Running IP Leak Test...");
                            if let Some(res) = run_capture(
                                "curl",
                                &[
                                    "-s",
                                    "--max-time",
                                    "10",
                                    "--socks5-hostname",
                                    "127.0.0.1:9050",
                                    "https://check.torproject.org/api/ip",
                                ],
                            ) {
                                if res.contains("\"IsTor\":true") {
                                    let ip = extract_json_string_field(&res, "IP")
                                        .unwrap_or_else(|| "Unknown".to_string());
                                    msg("No leaks detected! Routing through Tor.");
                                    info(&format!("Exit Node IP: {YELLOW}{ip}{RESET}"));
                                } else {
                                    warn("Possible leak or Tor not active!");
                                }
                            } else {
                                warn("Possible leak or Tor not active!");
                            }
                            info("Checking DNS via Mullvad...");
                            if let Some(m) = run_capture(
                                "curl",
                                &[
                                    "-s",
                                    "--max-time",
                                    "8",
                                    "--socks5-hostname",
                                    "127.0.0.1:9050",
                                    "https://am.i.mullvad.net/json",
                                ],
                            ) {
                                if !m.trim().is_empty() {
                                    let mip = extract_json_string_field(&m, "ip")
                                        .unwrap_or_else(|| "Unknown".to_string());
                                    let mcountry = extract_json_string_field(&m, "country")
                                        .unwrap_or_else(|| "Unknown".to_string());
                                    info(&format!(
                                        "Mullvad: IP={YELLOW}{mip}{RESET} Country={CYAN}{mcountry}{RESET}"
                                    ));
                                }
                            }
                            wait_any_key(&format!("\n  {DIM}Press any key to return...{RESET}"));
                        }
                        4 => {
                            banner(cfg);
                            let _ = start_disposable_browser(cfg);
                            wait_any_key(&format!("\n  {DIM}Press any key to return...{RESET}"));
                        }
                        5 => {
                            banner(cfg);
                            info("Metadata Sanitizer (MAT2 / ExifTool)");
                            print!("  {YELLOW}Enter file or directory path to sanitize:{RESET} ");
                            let _ = io::stdout().flush();
                            let mut target = String::new();
                            if io::stdin().read_line(&mut target).is_ok() {
                                let t = target.trim();
                                if !t.is_empty() {
                                    let _ = clean_meta_target(Path::new(t));
                                } else {
                                    warn("No target path entered.");
                                }
                            }
                            wait_any_key(&format!("\n  {DIM}Press any key to return...{RESET}"));
                        }
                        6 => {
                            trigger_panic(cfg);
                            return;
                        }
                        7 => {
                            banner(cfg);
                            stop_fn(cfg, true);
                            wait_any_key(&format!("\n  {DIM}Press any key to return...{RESET}"));
                        }
                        8 => break,
                        _ => {
                            clear_screen();
                            return;
                        }
                    }
                }
            }
            _ => {
                clear_screen();
                return;
            }
        }
    }
}
