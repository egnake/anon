mod config;
mod helpers;
mod modules;
mod registry;
mod state;
mod sys;
mod tui;
mod ui;

use std::env;
use std::io::{self, Write};
use std::path::Path;

use crate::config::{AppConfig, ANON_VERSION};
use crate::helpers::dep_check;
use crate::modules::defense::{run_anti_mitm_daemon, run_audit_once, run_leak_auditor_daemon};
use crate::modules::obfuscation::{
    cleanup_stealth_dir_if_active, maybe_reexec_stealth, run_traffic_chaff_daemon,
};
use crate::modules::utils::{
    clean_meta_target, get_backups, restore_system, self_update, start_disposable_browser,
    trigger_panic,
};
use crate::modules::{is_module_live, start_module, stop_module};
use crate::registry::{profile_indices, ALL_MODULES, EXEC_ORDER, STOP_ORDER};
use crate::state::{set_module_status, AnonState};
use crate::sys::effective_uid;
use crate::tui::{print_status_pretty, run_dashboard, run_interactive_tui};
use crate::ui::{banner, err, info, msg, strip_ansi, warn, GREEN, RED, RESET, YELLOW};

fn print_help() {
    msg(&format!(
        "Usage : sudo anon {GREEN}[option]{RESET}\n\n\
         \x20{GREEN}--start          :{RESET}   It will make backups and start the program.\n\
         \x20{GREEN}--stop           :{RESET}   Closes the program using a backup.\n\
         \x20{GREEN}--profile <name> :{RESET}   Apply profile headlessly (paranoia|quick|pentest|darknet|lockdown|ghost).\n\
         \x20{GREEN}--status [--json]:{RESET}   Provides system module & network status (optional JSON output).\n\
         \x20{GREEN}--dashboard      :{RESET}   Opens a live real-time monitoring dashboard.\n\
         \x20{GREEN}--browser        :{RESET}   Launches a disposable secure browser session.\n\
         \x20{GREEN}--audit  [--json]:{RESET}   Executes an instant deep IP and DNS leak audit (optional JSON).\n\
         \x20{GREEN}--clean-meta     :{RESET}   Sanitizes metadata from files/folders (EXIF/GPS/tags) via MAT2/ExifTool.\n\
         \x20{GREEN}--panic          :{RESET}   Emergency lockdown: immediately severs network, kills processes, and sanitizes RAM.\n\
         \x20{GREEN}--fix            :{RESET}   Used to repair the system in case of a possible bug.\n\
         \x20{GREEN}--update         :{RESET}   Automatically downloads and installs the latest version.\n\
         \x20{GREEN}--version        :{RESET}   Prints installed ANON version.\n\
         \x20{GREEN}--help           :{RESET}   This shows the menu."
    ));
}

fn stop_all(cfg: &AppConfig, non_fatal: bool) {
    let state = AnonState::load(&cfg.state_file);
    let active_cnt = state.count_active();

    if active_cnt > 0 {
        for &idx in &STOP_ORDER {
            let cur = AnonState::load(&cfg.state_file);
            let mod_name = ALL_MODULES[idx].name;
            if cur.is_enabled(mod_name) {
                let _ = stop_module(cfg, mod_name);
            }
        }
    } else if !is_module_live(cfg, "ip_changer") {
        if non_fatal {
            warn("No features are currently active.");
        } else {
            err(&format!(
                "No features are active. (Run to activate '{GREEN}sudo anon --start{RESET}')"
            ));
        }
    }

    // Tear down orphaned ip_changer state (e.g. failed bootstrap left torrc.bak)
    let after = AnonState::load(&cfg.state_file);
    if !after.is_enabled("ip_changer") && is_module_live(cfg, "ip_changer") {
        let _ = stop_module(cfg, "ip_changer");
    }
}

fn status_cmd(cfg: &AppConfig, sub_arg: Option<&str>) {
    let state = AnonState::load(&cfg.state_file);
    if sub_arg == Some("--json") {
        let active_cnt = state.count_active();
        print!("{{\"version\":\"{ANON_VERSION}\",\"active_count\":{active_cnt},\"modules\":{{");
        for (i, m) in ALL_MODULES.iter().enumerate() {
            if i > 0 {
                print!(",");
            }
            let b = if state.is_enabled(m.name) {
                "true"
            } else {
                "false"
            };
            print!("\"{}\":{b}", m.name);
        }
        println!("}}}}");
        return;
    }
    print_status_pretty(cfg);
}

fn cli_start(cfg: &AppConfig) {
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

    let state = AnonState::load(&cfg.state_file);
    let enabled_count = state.count_active();

    if enabled_count > 0 {
        info("Starting Anon with saved module configuration...");
        for &idx in &EXEC_ORDER {
            let cur = AnonState::load(&cfg.state_file);
            let mod_name = ALL_MODULES[idx].name;
            if cur.is_enabled(mod_name) {
                if is_module_live(cfg, mod_name) {
                    info(&format!(
                        "{mod_name} is already active; skipping duplicate start."
                    ));
                    continue;
                }
                let _ = set_module_status(&cfg.state_file, mod_name, false);
                let _ = start_module(cfg, mod_name);
            }
        }
    } else {
        info("No active modules found in saved config. Activating default security profile (Tor + Kill Switch)...");
        let _ = start_module(cfg, "ip_changer");
        let _ = start_module(cfg, "kill_switch");
    }

    msg("Anon started successfully in non-interactive mode.");
    status_cmd(cfg, None);
}

fn cli_profile(cfg: &AppConfig, prof_arg: Option<&str>) {
    let Some(prof_name) = prof_arg.filter(|s| !s.is_empty()) else {
        err("Missing profile name. Valid profiles: paranoia, quick, pentest, darknet, lockdown, ghost");
    };
    let Some(selected) = profile_indices(prof_name) else {
        err(&format!(
            "Unknown profile '{prof_name}'. Valid profiles: paranoia, quick, pentest, darknet, lockdown, ghost"
        ));
    };

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

    for &idx in &STOP_ORDER {
        if !selected.contains(&idx) {
            let cur = AnonState::load(&cfg.state_file);
            let mod_name = ALL_MODULES[idx].name;
            if cur.is_enabled(mod_name) || is_module_live(cfg, mod_name) {
                let _ = stop_module(cfg, mod_name);
            }
        }
    }

    for &idx in &EXEC_ORDER {
        if selected.contains(&idx) {
            let mod_name = ALL_MODULES[idx].name;
            if is_module_live(cfg, mod_name) {
                let _ = set_module_status(&cfg.state_file, mod_name, true);
            } else {
                let _ = set_module_status(&cfg.state_file, mod_name, false);
                let _ = start_module(cfg, mod_name);
            }
        }
    }

    msg(&format!("Profile '{prof_name}' applied successfully."));
    status_cmd(cfg, None);
}

fn audit_cmd(cfg: &AppConfig, sub_arg: Option<&str>) -> i32 {
    if sub_arg == Some("--json") {
        let (rc, raw) = match run_audit_once(cfg) {
            Ok(out) => (0, out),
            Err((rc, out)) => (rc, out),
        };
        let status_str = if rc == 0 { "clean" } else { "leak_detected" };
        let stripped = strip_ansi(&raw);
        let escaped = crate::helpers::json_escape(stripped.trim_end_matches('\n'));
        println!(
            "{{\"version\":\"{ANON_VERSION}\",\"status\":\"{status_str}\",\"exit_code\":{rc},\"details\":\"{escaped}\"}}"
        );
        return rc;
    }

    match run_audit_once(cfg) {
        Ok(out) => {
            print!("{out}");
            0
        }
        Err((rc, out)) => {
            eprint!("{out}");
            rc
        }
    }
}

fn clean_meta_cmd(args: &[String]) {
    if !args.is_empty() {
        let mut all_ok = true;
        for item in args {
            if !clean_meta_target(Path::new(item)) {
                all_ok = false;
            }
        }
        if !all_ok {
            std::process::exit(1);
        }
        return;
    }

    print!("  {YELLOW}Enter file or directory path to sanitize:{RESET} ");
    let _ = io::stdout().flush();
    let mut target = String::new();
    if io::stdin().read_line(&mut target).is_ok() {
        let t = target.trim();
        if !t.is_empty() {
            if !clean_meta_target(Path::new(t)) {
                std::process::exit(1);
            }
            return;
        }
    }
    err("No target path specified for metadata cleaning.");
}

fn fix_cmd(cfg: &AppConfig) {
    if cfg.backup_dir.join("anon_fix_backups.tar.gz").is_file() {
        restore_system(cfg);
        msg("System successfully repaired");
    } else {
        err("Anon backup file not found");
    }
}

struct StealthGuard;
impl Drop for StealthGuard {
    fn drop(&mut self) {
        cleanup_stealth_dir_if_active();
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if let Some(first) = args.first() {
        let lower = first.to_ascii_lowercase();
        if lower == "--version" || lower == "-v" {
            println!("ANON v{ANON_VERSION}");
            return;
        }
        if lower == "--help" || lower == "-h" {
            if let Ok(cfg) = AppConfig::load() {
                banner(&cfg);
            }
            print_help();
            return;
        }
    }

    let cfg = match AppConfig::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("\x1b[31m [-] FATAL ERROR: {e}\x1b[0m");
            std::process::exit(1);
        }
    };

    // Internal daemon subcommands (do not print banner)
    if args.len() >= 2 && args[0] == "--internal-daemon" {
        if effective_uid() != 0 {
            err("Anon must be run as root.");
        }
        match args[1].as_str() {
            "anti-mitm" => run_anti_mitm_daemon(&cfg),
            "leak-auditor" => run_leak_auditor_daemon(&cfg),
            "traffic-chaff" => run_traffic_chaff_daemon(&cfg),
            other => err(&format!("Unknown internal daemon: {other}")),
        }
    }

    let has_json = args.iter().any(|a| a == "--json");
    if !has_json {
        banner(&cfg);
    }

    if effective_uid() != 0 {
        let bin = env::args()
            .next()
            .and_then(|p| {
                Path::new(&p)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .map(str::to_string)
            })
            .unwrap_or_else(|| "anon".to_string());
        err(&format!("Anon must be run as root. {GREEN}'sudo {bin}'{RESET}"));
    }

    maybe_reexec_stealth(&cfg, &args);
    let _stealth_guard = StealthGuard;

    if args.is_empty() {
        run_interactive_tui(&cfg, stop_all);
        return;
    }

    let sub = args.get(1).map(String::as_str);
    match args[0].to_ascii_lowercase().as_str() {
        "--start" | "-st" => cli_start(&cfg),
        "--stop" | "-sp" => stop_all(&cfg, false),
        "--profile" => cli_profile(&cfg, sub),
        "--status" | "-ss" => status_cmd(&cfg, sub),
        "--dashboard" | "-d" => run_dashboard(&cfg),
        "--browser" | "-b" => {
            let _ = start_disposable_browser(&cfg);
        }
        "--audit" | "-a" => {
            let rc = audit_cmd(&cfg, sub);
            if rc != 0 {
                std::process::exit(rc);
            }
        }
        "--clean-meta" | "-cm" => clean_meta_cmd(&args[1..]),
        "--panic" | "-p" => trigger_panic(&cfg),
        "--fix" | "-f" => fix_cmd(&cfg),
        "--update" | "-u" => self_update(&cfg),
        "--version" | "-v" => println!("ANON v{ANON_VERSION}"),
        "--help" | "-h" => print_help(),
        invalid => {
            warn(&format!("Anon: Invalid option {RED}'{invalid}'{RESET}"));
            info(&format!(
                "Run {GREEN}'anon --help'{RESET} parameter for more information."
            ));
            std::process::exit(1);
        }
    }
}

