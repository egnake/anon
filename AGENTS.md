# AGENTS.md — ANON Project Guide

This document provides architecture context, strict engineering conventions, known pitfalls, and verification workflows for AI coding agents and contributors working on **ANON** (`egnake/anon`).

---

## 1. Project Overview

**ANON (v3.0)** is a modular Linux Operational Security (OPSEC), anonymity, and anti-forensics framework implemented as a zero-dependency **Rust** native binary (`src/`), with optional Windows/WSL2 wrappers and container packaging.

- **Target Platforms:** Debian, Kali Linux, Parrot OS, Ubuntu, and Windows 10/11 via WSL2.
- **Core Capabilities:** Tor transparent proxying (`TransPort` 9040 / `DNSPort` 5353), `obfs4` & Snowflake DPI circumvention, atomic `iptables-restore`/`ip6tables-restore` kill switch, kernel network namespace isolation (`anon_jail`) + AppArmor/Firejail, MAC/hostname/timezone/OS-fingerprint spoofing, RAM-based disposable Firefox & `tmpfs` volatile logging, event-driven Netlink/inotify ARP MITM and IP/DNS leak watchdogs, native `mlock`/volatile RAM wiping, and multi-threaded MAT2/ExifTool metadata sanitization.

---

## 2. Repository Structure & File Roles

```text
.
├── Cargo.toml                  # Zero-dependency Rust package manifest (opt-level 3, LTO, strip)
├── src/
│   ├── main.rs                 # CLI entrypoint, internal daemon dispatch, profiles, fix & panic
│   ├── config.rs               # Path resolution (BASE_DIR, BACKUPDIR, TORRC, env overrides)
│   ├── state.rs                # flock + atomic temp/chmod/rename runtime state (anon.state)
│   ├── sys.rs                  # Zero-crate Linux libc FFI (geteuid, flock, prctl, Netlink, inotify, mlock wipe)
│   ├── helpers.rs              # Service control, NM DNS lock, Tor ControlPort TCP, JSON, shredder, dep_check
│   ├── registry.rs             # Declarative 22-module registry, categories, EXEC_ORDER/STOP_ORDER, AnonModule trait
│   ├── ui.rs                   # ANSI palette, buffered box drawing, cached session banner(), notify()
│   ├── tui.rs                  # Raw/tty-guarded interactive menu, category toggles, 3s live dashboard
│   └── modules/
│       ├── mod.rs              # Module factory & trait dispatch
│       ├── network.rs          # ip_changer, dns_changer, doh_shield, kill_switch, tor_bridges, dpi_spoofing, i2p_integration, proxychains_integration, mac_changer
│       ├── obfuscation.rs      # timezone_changer, hostname_changer, browser_anonymization, os_obfuscation, process_obfuscation, sandbox_isolation, execution_stealth, traffic_obfuscation
│       ├── defense.rs          # anti_mitm (Netlink), leak_auditor (inotify/Netlink + --audit), volatile_logging, bluetooth_disabler, anti_cold_boot
│       └── utils.rs            # get_backups/restore_system (--fix), trigger_panic (--panic), start_disposable_browser, parallel clean_meta_target, self_update
├── anon.bat                    # Windows CMD/PowerShell -> WSL wrapper (`wsl -u root -- anon %*`)
├── setup.sh                    # Native Linux apt-get installer (incl. cargo/rustc) + `make install`
├── setup.bat                   # Windows WSL2 setup, user PATH updater, and WSL installer
├── Makefile                    # build / install / uninstall / reinstall / lint / test -> /usr/share/anon & /usr/bin/anon
├── Dockerfile                  # Multi-stage Kali Rolling image (builder compiles release binary -> slim runtime)
├── test_suite.sh               # Cargo unit tests + isolated fakeroot blackbox CLI test suite (41 assertions)
├── .gitattributes              # Enforces LF for Rust/shell/docs and CRLF for *.bat
├── .gitignore                  # Ignores target/, Cargo.lock, *.deb, *.tar.gz, backups/*, scratch/, bridges.txt
├── .github/workflows/
│   ├── ci.yml                  # cargo test -D warnings, cargo build --release, bash -n, shellcheck, ./test_suite.sh
│   └── release.yml             # Compiles release binary, builds arch .deb package and source tarball on v* tags
├── assets/
│   ├── anon.desktop            # Linux desktop launcher (Icon=/usr/share/anon/assets/icons/anon.jpg)
│   ├── anon_logo.jpg           # README logo
│   ├── icons/anon.jpg          # Desktop entry icon
│   ├── banners/1.txt..30.txt   # Randomized ASCII/Unicode TUI banners
│   └── sources/
│       ├── anon.js             # Hardened Firefox preferences (also embedded fallback in obfuscation.rs)
│       ├── bridges.default.txt # Default obfs4 bridge fallback list
│       └── bridges.txt         # (Gitignored) User-provided runtime obfs4 bridges
├── backups/                    # Runtime backup archive destination (.gitkeep tracked; contents gitignored)
└── scratch/                    # (Gitignored) Local banner generation Python helper scripts
```

### Module Execution Model (`src/modules/`)

1. **Trait Lifecycle Modules (`AnonModule` trait in `src/registry.rs`):**
   Every module implements `name()`, `start(&cfg)`, `stop(&cfg)`, and `is_live(&cfg)`.
2. **Internal Background Daemons (`anon --internal-daemon <name>`):**
   - `anti-mitm`: Event-driven Netlink `RTMGRP_NEIGH` + polling ARP/ND watchdog.
   - `leak-auditor`: Event-driven `inotify` (`resolv.conf`) + Netlink `RTMGRP_IPV6_ROUTE` watchdog (`run_audit_once` also powers `--audit`).
   - `traffic-chaff`: Native Tor SOCKS/Tor chaffing daemon for `traffic_obfuscation`.
   - When `execution_stealth` is active, child daemons set `arg0` and `prctl(PR_SET_NAME)` to `[kworker/u4:2]`.
3. **Utilities (`src/modules/utils.rs`):**
   `get_backups`, `restore_system`, `trigger_panic`, `start_disposable_browser`, parallel `clean_meta_target` (`std::thread::scope`), and `self_update`.

---

## 3. Critical Engineering Rules & Known Pitfalls

### 3.1 Zero External Crates & Safe Rust Discipline
- Keep `Cargo.toml` dependency-free (`std` + narrow POSIX/Linux FFI isolated in `src/sys.rs`) so offline builds, Docker builds, and security auditing stay fast and reproducible.
- Never bypass root checks in production binaries (use `fakeroot` inside isolated tests).
- Never silence warnings; compile with `RUSTFLAGS="-D warnings"`.

### 3.2 Canonical Installation & State Paths
- `Makefile` installs `target/release/anon` to `/usr/bin/anon` and `assets/` to `/usr/share/anon/assets/`.
- Mutable runtime state (`<module>_status`, `<module>_pid`) is stored in `ANON_STATE_FILE` (default: `${BACKUPDIR}/anon.state`) using `flock` + atomic temp/`chmod 600`/`rename` in `src/state.rs`.
- Always resolve paths through `AppConfig::load()` (`BASE_DIR`, `BACKUPDIR`, `ANON_STATE_FILE`, `TORRC`, `ANON_HOSTS_FILE`, `ANON_RESOLV_CONF`, `ANON_PROXYCHAINS_CONF`, `APPARMOR_TOR_PROFILE`, `NETNS_ETC_DIR`, `WRAPPER_DIR`).

### 3.3 Execution Stealth (`/dev/shm/.kworker_*`) & Cleanup
- When `execution_stealth_status="Enable"` and `ANON_SKIP_STEALTH` is not set, `maybe_reexec_stealth()` copies the binary into a `mkdtemp` `/dev/shm/.kworker_XXXXXX` directory and re-execs with `argv[0] = "[kworker/u4:2]"` + `ANON_PERSIST_DIR` pointing to the real directory. `StealthGuard` removes the temp copy on normal exit.
- In `cleanup_ephemeral_paths()`, never remove `cfg.base_dir` or `cfg.backup_dir` if an environment override places them under `/tmp/anon*`.

### 3.4 Security & Zero-Vulnerability Practices
- **No predictable temp paths in shared directories:** Always use PID + CSPRNG nonces or `mkdtemp` (`O_CREAT | O_EXCL` or `0700`/`0600` permissions) in `/dev/shm` and `/tmp`.
- **Pass `--` before user-supplied paths** to external tools (`mat2`, `exiftool`, `tar`, `shred`).
- **IPv6 & DNS leak parity:** Any firewall (`ip_changer`, `kill_switch`) or DNS (`dns_changer`, `doh_shield`) change must handle IPv6 (`ip6tables` + `disable_ipv6`), `NetworkManager`, and `systemd-resolved` / `nsswitch.conf`.
- **Mutual exclusion & Torrc backup:** `ip_changer`, `dns_changer`, and `doh_shield` are mutually exclusive; always strip `# ANON_SANDBOX` lines when saving/restoring `torrc.bak` if `sandbox_isolation` is disabled.
- **Privilege drop & GUI environment:** Derive `$HOME` / `XAUTHORITY` from `$SUDO_USER` (`/home/$SUDO_USER/.Xauthority`) when invoking `firefox` or `notify-send`.

### 3.5 Cross-Platform & Line-Ending Discipline
- **Line endings:** All Rust files, Bash scripts (`setup.sh`, `test_suite.sh`), `Makefile`, `Dockerfile`, `.github/workflows/*`, `assets/banners/*`, and `*.md` MUST use **LF** (`\n`). Windows `.bat` files (`setup.bat`, `anon.bat`) MUST use **CRLF** (`\r\n`).
- **WSL2 compatibility:** Preserve `is_wsl()` guards (`/proc/version` check) around `sdmem`, `wipe_free_ram_native`, swap operations, and kernel sysctls that can crash WSL2 hosts (`E_UNEXPECTED`).

### 3.6 Focused Changes & Existing Behavior
- Follow strict **SOLID** principles and **targeted edits**.
- Every new system modification must have a matching teardown/restore path in both `AnonModule::stop` and `restore_system` (`--fix`).

---

## 4. Checklist for Adding or Modifying a Module

1. Implement `AnonModule` in the appropriate submodule under `src/modules/` and register it in `get_module()` (`src/modules/mod.rs`).
2. Register metadata (`ModuleMeta`), category index (`NET_CAT_INDICES`, `OBF_CAT_INDICES`, or `DEF_CAT_INDICES`), `EXEC_ORDER`, `STOP_ORDER`, and `profile_indices()` in `src/registry.rs`. (`ALL_MODULES` invariants + `get_module()` parity are enforced by unit tests.)
3. Add cleanup/teardown handling in `restore_system()` / `trigger_panic()` (`src/modules/utils.rs`) if applicable.
4. Keep external package lists synced across `setup.sh`, `setup.bat`, `Dockerfile`, `.github/workflows/release.yml`, and `dep_check()`.
5. Sync `README.md` / `README-TR.md` and add unit + mocked blackbox tests in `src/` and `test_suite.sh`.

---

## 5. Verification, Linting & Testing Workflow

Always verify changes in Linux or WSL2 before finishing:

### Rust Tests & Release Build (Strict `-D warnings`)
```bash
RUSTFLAGS="-D warnings" cargo test
RUSTFLAGS="-D warnings" cargo build --release
```

### Shell Syntax & ShellCheck (Matches `.github/workflows/ci.yml` & `make lint`)
```bash
bash -n setup.sh test_suite.sh
shellcheck -S warning -e SC1090,SC1091,SC2034 setup.sh test_suite.sh
```

### Run Full Test Suite (Cargo Unit Tests + 62 Isolated Fakeroot Assertions)
```bash
chmod +x test_suite.sh
./test_suite.sh
```

### Check Line Endings
Ensure no `\r` (`CRLF`) exists in `Cargo.toml`, `src/**/*.rs`, `setup.sh`, `test_suite.sh`, `Makefile`, `Dockerfile`, `.github/workflows/*`, `assets/sources/*`, `assets/banners/*`, or `*.md`, and that `anon.bat` / `setup.bat` retain `CRLF`.

