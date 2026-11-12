# AGENTS.md — ANON Project Guide

This document provides architecture context, strict engineering conventions, known pitfalls, and verification workflows for AI coding agents and contributors working on **ANON** (`egnake/anon`).

---

## 1. Project Overview

**ANON (v3.0)** is a modular Linux Operational Security (OPSEC), anonymity, and anti-forensics framework written primarily in **Bash**, with optional Windows/WSL2 wrappers and container packaging.

- **Target Platforms:** Debian, Kali Linux, Parrot OS, Ubuntu, and Windows 10/11 via WSL2.
- **Core Capabilities:** Tor transparent proxying (`TransPort` 9040 / `DNSPort` 5353), `obfs4` & Snowflake DPI circumvention, atomic `iptables-restore`/`ip6tables-restore` kill switch, kernel network namespace isolation (`anon_jail`) + AppArmor/Firejail, MAC/hostname/timezone/OS-fingerprint spoofing, RAM-based disposable Firefox & `tmpfs` volatile logging, background ARP MITM and IP/DNS leak watchdogs, and MAT2/ExifTool metadata sanitization.

---

## 2. Repository Structure & File Roles

```text
.
├── anon                        # Main CLI/TUI entrypoint and orchestrator (installed to /usr/bin/anon)
├── anon.bat                    # Windows CMD/PowerShell -> WSL wrapper (`wsl -u root -- anon %*`)
├── setup.sh                    # Native Linux apt-get installer + `make install`
├── setup.bat                   # Windows WSL2 setup, user PATH updater, and WSL installer
├── Makefile                    # install / uninstall / reinstall targets -> /usr/share/anon & /usr/bin/anon
├── Dockerfile                  # Kali Rolling image definition (ENTRYPOINT ["anon"], CMD ["--start"])
├── test_suite.sh               # Mocked whitebox + blackbox test suite (36 assertions)
├── .gitattributes              # Enforces LF for Linux scripts/configs and CRLF for *.bat
├── .gitignore                  # Ignores backups/*, scratch/, bridges.txt, logs, IDE caches
├── .github/workflows/
│   ├── ci.yml                  # bash -n, shellcheck, and ./test_suite.sh on push/PR to main
│   └── release.yml             # Builds .deb package and source tarball on v* tags
├── assets/
│   ├── anon.desktop            # Linux desktop launcher (Icon=/usr/share/anon/assets/icons/anon.jpg)
│   ├── anon_logo.jpg           # README logo
│   ├── icons/anon.jpg          # Desktop entry icon
│   ├── banners/1.txt..30.txt   # Randomized ASCII/Unicode TUI banners
│   ├── sources/
│   │   ├── config              # Shared colors, path resolution, notify(), banner(), and default module state vars
│   │   ├── helpers             # Shared flock/atomic state helpers (ANON_STATE_FILE), service_ctrl, DNS/NM lock, net/bridge helpers
│   │   ├── anon.js             # Hardened Firefox preferences injected by browser_anonymization & disposable_browser
│   │   ├── bridges.default.txt # Default obfs4 bridge fallback list
│   │   └── bridges.txt         # (Gitignored) User-provided runtime obfs4 bridges
│   └── scripts/                # 26 modular scripts (22 stateful modules + 4 helper/standalone scripts)
├── backups/                    # Runtime backup archive destination (.gitkeep tracked; contents gitignored)
└── scratch/                    # (Gitignored) Local banner generation Python helper scripts
```

### Module Contracts (`assets/scripts/`)

Keep these 4 distinct execution contracts in mind when editing or adding modules:

1. **Sourced Lifecycle Modules** (loaded via `src_check` in `anon`; expose `start_<name>` and `stop_<name>`):
   `anti_cold_boot`, `bluetooth_disabler`, `browser_anonymization`, `dns_changer`, `doh_shield`, `dpi_spoofing`, `execution_stealth`, `hostname_changer`, `i2p_integration`, `ip_changer`, `kill_switch`, `mac_changer`, `os_obfuscation`, `process_obfuscation`, `proxychains_integration`, `timezone_changer`, `tor_bridges`, `volatile_logging`.
2. **CLI-Subcommand Hybrid Modules** (also invoked with `start` / `stop` arguments by `anon` wrappers):
   `sandbox_isolation`, `traffic_obfuscation`.
3. **Background Watchdog Daemons** (started in background with `&` in `anon`, PID & state persisted in `ANON_STATE_FILE` via `assets/sources/helpers`, stopped via `kill` in `anon`'s `stop()`):
   `anti_mitm`, `leak_auditor` (`leak_auditor` also supports `--check-once` / `-c` for `--audit`).
4. **Standalone / Utility Scripts**:
   - `fix`: Provides `get_backups` and `restore_system` (sourced via `src_check`).
   - `emergency_panic`: Provides `trigger_panic` (sourced via `src_check`, invoked by `--panic` / TUI).
   - `disposable_browser`: Provides `start_disposable_browser` (sourced on demand by `--browser` / TUI).
   - `metadata_cleaner`: Standalone CLI executed directly with target paths (`--clean-meta`).

---

## 3. Critical Engineering Rules & Known Pitfalls

### 3.1 NEVER Put `set -e` / `set -euo pipefail` in Sourced Scripts
- `anon` calls `src_check` to `source` modules directly into the main shell process.
- Any `set -euo pipefail` inside a sourced script leaks into `anon`, causing arithmetic post-increments (`((count++))` from `0`), `read -t` timeouts, and non-zero `grep`/`systemctl` checks in the TUI/dashboard to crash the entire application.
- **Rule:** Only standalone executed scripts (`anti_mitm`, `leak_auditor`, `metadata_cleaner`) or explicit `if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then set -euo pipefail; fi` guards (`sandbox_isolation`, `traffic_obfuscation`) may enable `set -euo pipefail`.

### 3.2 Canonical Installation & Config Paths
- `Makefile` copies `assets/` into `/usr/share/anon/assets/`.
- The canonical config and helper paths when installed are **`/usr/share/anon/assets/sources/config`** and **`/usr/share/anon/assets/sources/helpers`** (NOT `/usr/share/anon/sources/config`).
- Mutable runtime state (`<module>_status`, daemon PIDs) is stored in `ANON_STATE_FILE` (default: `${BACKUPDIR}/anon.state`) using `set_module_status` / `set_state_var` from `assets/sources/helpers` with `flock` + atomic `mktemp`/`mv` replacement, keeping `assets/sources/config` immutable across upgrades.
- Always resolve paths through `BASE_DIR`, `SRCDIR` (`${BASE_DIR}/assets`), `BACKUPDIR` (`${ANON_PERSIST_DIR:-${BASE_DIR}}/backups`), `ANON_STATE_FILE`, and `TORRC` sourced from `assets/sources/config` and `assets/sources/helpers`. Never hardcode `/usr/share/anon/sources/config`.

### 3.3 Execution Stealth (`/dev/shm/.kworker_*`) & Traps
- When `execution_stealth_status="Enable"`, `anon` copies `anon`, `assets/scripts`, and `assets/banners` into `/dev/shm/.kworker_XXXXXX`, symlinks `assets/sources` and `backups` back to `$ANON_PERSIST_DIR`, sets `ANON_STEALTH_ACTIVE=1`, and `exec -a "[kworker/u4:2]"` re-execs itself.
- **Rule:** Do NOT overwrite global `EXIT`/`INT`/`TERM` `trap` handlers inside functions (e.g., `update()`, `disposable_browser`, `dashboard()`) without chaining or isolating in a subshell, or the `/dev/shm/.kworker_*` cleanup trap will be lost. Also gate `tput cnorm` on `[[ -t 1 ]]` so non-TTY/JSON commands stay clean.

### 3.4 Security & Zero-Vulnerability Practices
- **No predictable temp paths in shared directories:** `/dev/shm` and `/tmp` are world-writable (`1777`). Always use `mktemp` / `mktemp -d` instead of fixed paths (avoid patterns like `/dev/shm/anon_net_status` or `/dev/shm/.systemd-kworker`).
- **Quote all paths and variables:** Especially important when target filenames or Windows/WSL paths contain spaces (e.g., `d:\pc yedekleri\...`). Pass `--` before user-supplied paths to tools like `mat2`, `exiftool`, `rm`, `cp`. Strict ShellCheck (without `SC2086`) is enforced in CI.
- **Safe locked state updates:** Always mutate runtime module states and PIDs via `set_module_status` / `set_state_var` / `reset_all_module_states` in `assets/sources/helpers` (`flock` + `mktemp` + `mv`) rather than raw `sed -i` on `config`.
- **IPv6 & DNS leak parity:** Any firewall (`ip_changer`, `kill_switch`) or DNS (`dns_changer`, `doh_shield`) change must consider IPv6 (`ip6tables` + `disable_ipv6`), `NetworkManager`, and `systemd-resolved` / `nsswitch.conf`.
- **Mutual exclusion:** `ip_changer`, `dns_changer`, and `doh_shield` are mutually exclusive; keep checks consistent across all three modules and `leak_auditor` (e.g., `127.0.2.1` for `dnscrypt-proxy`).
- **Privilege drop & GUI environment:** When spawning GUI apps (`firefox`, `notify-send`) from `sudo`/root, derive `$HOME` / `XAUTHORITY` from `$SUDO_USER` (`/home/$SUDO_USER/.Xauthority`), not root's `$HOME`.

### 3.5 Cross-Platform & Line-Ending Discipline
- **Line endings:** All Bash scripts, configs, `Makefile`, `Dockerfile`, and `assets/banners/*` MUST use **LF** (`\n`). Windows `.bat` files (`setup.bat`, `anon.bat`) MUST use **CRLF** (`\r\n`).
- **WSL2 compatibility:** Preserve WSL detection guards (`grep -qi microsoft /proc/version`) around `sdmem`, swap operations, and kernel sysctls that can crash WSL2 hosts (`E_UNEXPECTED`). Note that `systemctl`, `timedatectl`, and `macchanger` may behave differently or be unavailable in non-systemd WSL2/Docker environments.

### 3.6 Focused Changes & Existing Behavior
- Follow strict **SOLID** principles and **targeted edits**: modify only what the task requires; do not perform unsolicited refactors or style rewrites on unrelated modules.
- Preserve existing backup/restore behavior (`get_backups` / `restore_system` in `assets/scripts/fix`) and ensure every new system modification has a matching teardown/restore path in both `stop_<module>` and `fix`.

---

## 4. Checklist for Adding or Modifying a Module

`anon` uses a declarative module registry (while teardown/reset lists also live in `fix`, `emergency_panic`, and `helpers`):

1. Script in `assets/scripts/<module_name>` (executable `755`, LF line endings, `bash -n` and strict `shellcheck` clean; calls `set_module_status "<module_name>" "Enable|Disable"`).
2. Default status/PID variables in `assets/sources/config` (`<module_name>_status="Disable"`).
3. `assets/sources/helpers`: include `<module_name>` in `reset_all_module_states()` (plus any PID keys if it is a daemon).
4. `anon`:
   - `src_check` call in `_load_module_scripts()` (sourced modules vs `standalone` check).
   - Registry arrays: `MOD_NAMES`, `MOD_LABELS`, `MOD_TAGS`, and category arrays (`NET_CAT_INDICES`, `OBF_CAT_INDICES`, or `DEF_CAT_INDICES`).
   - `EXEC_ORDER` and `STOP_ORDER` arrays (plus hybrid/daemon cases in `_start_module()`, `_stop_module()`, and `_is_module_live()` if not a standard `start_<name>`/`stop_<name>` module).
   - Note: `interactive_toggles()`, `status()`, `--status --json`, and `dashboard()` iterate automatically over the registry!
   - Security profiles (`_profile_indices()` cases) if applicable.
5. `assets/scripts/fix` (`restore_system` cleanup + `reset_all_module_states`).
6. `assets/scripts/emergency_panic` (process kill / teardown / `reset_all_module_states` if applicable).
7. Dependencies synced across `setup.sh`, `setup.bat`, `Dockerfile`, `.github/workflows/release.yml`, and `dep_check()` in `anon`.
8. Documentation synced in `README.md` and `README-TR.md`.
9. Tests added to `test_suite.sh` (runs isolated inside `$MOCK_DIR`, covering registry parity, whitebox, and blackbox CLI flows).

---

## 5. Verification, Linting & Testing Workflow

Always verify changes in Linux or WSL2 before finishing:

### Syntax Check (All Scripts + Config + Helpers)
```bash
for f in anon setup.sh test_suite.sh assets/sources/config assets/sources/helpers assets/scripts/*; do
  bash -n "$f"
done
```

### ShellCheck (Strict — Matches `.github/workflows/ci.yml` & `make lint`)
```bash
shellcheck -S warning -e SC1090,SC1091,SC2034 \
  anon assets/scripts/* setup.sh test_suite.sh assets/sources/config assets/sources/helpers
```

### Run Isolated Test Suite (63 Assertions)
```bash
chmod +x test_suite.sh
./test_suite.sh
```
`test_suite.sh` executes inside an isolated temporary `mktemp -d` copy with mocked root/system binaries, so it never mutates `assets/sources/config` or system files. Still check `git status` before tagging/releasing to avoid packaging untracked local files (`assets/sources/bridges.txt`, `backups/*.tar.gz`).

### Check Line Endings (from Windows or Linux)
Ensure no `\r` (`CRLF`) exists in `anon`, `setup.sh`, `test_suite.sh`, `Makefile`, `Dockerfile`, `.github/workflows/*`, `assets/sources/*`, `assets/banners/*`, `assets/scripts/*`, or `*.md`, and that `anon.bat` / `setup.bat` retain `CRLF`.

