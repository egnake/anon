use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use crate::config::AppConfig;
use crate::helpers::rand_u64;
use crate::sys::{is_stdout_tty, terminal_cols};

pub const RESET: &str = "\x1b[0m";
pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";
pub const RED: &str = "\x1b[31m";
pub const GREEN: &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";
pub const BLUE: &str = "\x1b[34m";
pub const MAGENTA: &str = "\x1b[35m";
pub const CYAN: &str = "\x1b[36m";
pub const WHITE: &str = "\x1b[37m";
pub const BG_BLUE: &str = "\x1b[44m";

static CACHED_BANNER: OnceLock<Option<String>> = OnceLock::new();

pub fn err(msg_text: &str) -> ! {
    eprintln!("\n {BG_BLUE}{WHITE}{BOLD} ANON {RESET} {RED}✖ ERROR:{RESET} {msg_text}\n");
    crate::modules::obfuscation::cleanup_stealth_dir_if_active();
    std::process::exit(1);
}

pub fn warn(msg_text: &str) {
    println!(" {YELLOW}⚠ WARNING:{RESET} {msg_text}");
}

pub fn msg(msg_text: &str) {
    println!(" {GREEN}✔ SUCCESS:{RESET} {msg_text}");
}

pub fn info(msg_text: &str) {
    println!(" {CYAN}ℹ INFO:{RESET} {msg_text}");
}

pub fn clear_screen() {
    if is_stdout_tty() {
        print!("\x1b[2J\x1b[H");
        let _ = io::stdout().flush();
    }
}

pub fn hide_cursor() {
    if is_stdout_tty() {
        print!("\x1b[?25l");
        let _ = io::stdout().flush();
    }
}

pub fn show_cursor() {
    if is_stdout_tty() {
        print!("\x1b[?25h");
        let _ = io::stdout().flush();
    }
}

pub fn notify(title: &str, message: &str, urgency: &str) {
    if !crate::helpers::cmd_exists("notify-send") {
        return;
    }
    let user_disp = std::env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string());
    if user_disp.is_empty() {
        return;
    }

    let sudo_user = std::env::var("SUDO_USER").ok().filter(|u| !u.is_empty());
    let active_user = sudo_user.as_deref().unwrap_or("root");
    let active_home = crate::helpers::resolve_user_home(active_user);
    let user_xauth = std::env::var("XAUTHORITY")
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| format!("{active_home}/.Xauthority"));

    if let Some(su) = sudo_user.filter(|u| u != "root") {
        let _ = Command::new("sudo")
            .args([
                "-u",
                &su,
                &format!("DISPLAY={user_disp}"),
                &format!("XAUTHORITY={user_xauth}"),
                "notify-send",
                "-u",
                urgency,
                "-a",
                "ANON",
                title,
                message,
            ])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    } else {
        let _ = Command::new("notify-send")
            .env("DISPLAY", &user_disp)
            .env("XAUTHORITY", &user_xauth)
            .args(["-u", urgency, "-a", "ANON", title, message])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
}

fn load_session_banner(src_dir: &Path) -> Option<String> {
    CACHED_BANNER
        .get_or_init(|| {
            let banner_dir = src_dir.join("banners");
            let entries: Vec<PathBuf> = fs::read_dir(&banner_dir)
                .ok()?
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("txt") && p.is_file())
                .collect();
            if entries.is_empty() {
                return None;
            }
            let idx = (rand_u64() as usize) % entries.len();
            fs::read_to_string(&entries[idx]).ok()
        })
        .clone()
}

pub fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            for nc in chars.by_ref() {
                if nc.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

pub fn render_banner_into(cfg: &AppConfig, out: &mut String, include_art: bool) {
    let width = terminal_cols();
    let bar_len = (56usize).min(width.saturating_sub(2)).max(10);
    let bar_pad = width.saturating_sub(bar_len) / 2;

    if include_art {
        out.push('\n');
        out.push_str(BOLD);
        out.push_str(CYAN);

        if let Some(banner_txt) = load_session_banner(&cfg.src_dir) {
            for raw_line in banner_txt.lines() {
                let line = raw_line.trim_end_matches('\r');
                let stripped = strip_ansi(line);
                let line_len = stripped.chars().count();
                let line_pad = width.saturating_sub(line_len) / 2;
                for _ in 0..line_pad {
                    out.push(' ');
                }
                out.push_str(line);
                out.push('\n');
            }
        }
        out.push_str(RESET);
        out.push('\n');
    }

    out.push_str(DIM);
    for _ in 0..bar_pad {
        out.push(' ');
    }
    for _ in 0..bar_len {
        out.push('═');
    }
    out.push_str(RESET);
    out.push('\n');

    let sub = "Advanced OPSEC & Anonymity Framework";
    let ver = "v3.0";
    let sub_len = sub.len() + 2 + ver.len();
    let sub_pad = width.saturating_sub(sub_len) / 2;
    for _ in 0..sub_pad {
        out.push(' ');
    }
    out.push_str(BOLD);
    out.push_str(CYAN);
    out.push_str(sub);
    out.push_str("  ");
    out.push_str(DIM);
    out.push_str(ver);
    out.push_str(RESET);
    out.push('\n');

    let contact = "github.com/egnake | egcaem000@gmail.com | Report Issue";
    let contact_pad = width.saturating_sub(contact.len()) / 2;
    for _ in 0..contact_pad {
        out.push(' ');
    }
    out.push_str(DIM);
    out.push_str(contact);
    out.push_str(RESET);
    out.push('\n');

    out.push_str(DIM);
    for _ in 0..bar_pad {
        out.push(' ');
    }
    for _ in 0..bar_len {
        out.push('═');
    }
    out.push_str(RESET);
    out.push('\n');
}

pub fn banner(cfg: &AppConfig) {
    clear_screen();
    let mut out = String::with_capacity(2048);
    render_banner_into(cfg, &mut out, true);
    out.push('\n');

    let mut stdout = io::stdout().lock();
    let _ = stdout.write_all(out.as_bytes());
    let _ = stdout.flush();
}

/// Renders a complete TUI frame atomically in-place without clearing to blank first.
/// Prevents screen flashing on Windows Terminal / ConPTY / tmux during rapid arrow navigation.
pub fn render_tui_frame(frame: &str) {
    if !is_stdout_tty() {
        let mut stdout = io::stdout().lock();
        let _ = stdout.write_all(frame.as_bytes());
        if !frame.ends_with('\n') {
            let _ = stdout.write_all(b"\n");
        }
        let _ = stdout.flush();
        return;
    }

    let trimmed = frame.trim_end_matches('\n');
    let mut out = String::with_capacity(trimmed.len() + 256);
    out.push_str("\x1b[?2026h\x1b[H");
    let mut first = true;
    for line in trimmed.split('\n') {
        if !first {
            out.push('\n');
        }
        first = false;
        out.push_str(line);
        out.push_str("\x1b[K");
    }
    out.push_str("\x1b[J\x1b[?2026l");

    let mut stdout = io::stdout().lock();
    let _ = stdout.write_all(out.as_bytes());
    let _ = stdout.flush();
}

pub fn tui_dims() -> (usize, usize, usize) {
    let width = terminal_cols().max(20);
    let inner = width.saturating_sub(4).clamp(16, 72);
    let pad = width.saturating_sub(inner + 2) / 2;
    (width, inner, pad)
}

pub fn draw_line(buf: &mut String) {
    let (_, inner, pad) = tui_dims();
    for _ in 0..pad {
        buf.push(' ');
    }
    buf.push_str(DIM);
    buf.push('╔');
    for _ in 0..inner {
        buf.push('═');
    }
    buf.push_str("╗");
    buf.push_str(RESET);
    buf.push('\n');
}

pub fn draw_end(buf: &mut String) {
    let (_, inner, pad) = tui_dims();
    for _ in 0..pad {
        buf.push(' ');
    }
    buf.push_str(DIM);
    buf.push('╚');
    for _ in 0..inner {
        buf.push('═');
    }
    buf.push_str("╝");
    buf.push_str(RESET);
    buf.push('\n');
}

pub fn draw_sep(buf: &mut String) {
    let (_, inner, pad) = tui_dims();
    for _ in 0..pad {
        buf.push(' ');
    }
    buf.push_str(DIM);
    buf.push('║');
    for _ in 0..inner {
        buf.push('─');
    }
    buf.push_str("║");
    buf.push_str(RESET);
    buf.push('\n');
}

pub fn emptybox(buf: &mut String) {
    let (_, inner, pad) = tui_dims();
    for _ in 0..pad {
        buf.push(' ');
    }
    buf.push_str(DIM);
    buf.push('║');
    for _ in 0..inner {
        buf.push(' ');
    }
    buf.push('║');
    buf.push_str(RESET);
    buf.push('\n');
}

pub fn boxline(buf: &mut String, text: &str) {
    let (_, inner, pad) = tui_dims();
    let stripped = strip_ansi(text);
    let mut tlen = stripped.chars().count();
    let max_tlen = inner.saturating_sub(1);

    let rendered = if tlen > max_tlen {
        let trunc: String = stripped.chars().take(max_tlen.saturating_sub(1)).collect();
        tlen = trunc.chars().count() + 1;
        format!("{trunc}…")
    } else {
        text.to_string()
    };

    let rpad = inner.saturating_sub(tlen + 1);
    for _ in 0..pad {
        buf.push(' ');
    }
    buf.push_str(DIM);
    buf.push_str("║");
    buf.push_str(RESET);
    buf.push(' ');
    buf.push_str(&rendered);
    for _ in 0..rpad {
        buf.push(' ');
    }
    buf.push_str(DIM);
    buf.push_str("║");
    buf.push_str(RESET);
    buf.push('\n');
}

