<div align="center">
  <img src="assets/anon_logo_iconic.jpg" alt="ANON Logo" width="220">
  
  <h1>ANON: Advanced OPSEC & Anonymity Framework</h1>
  <p><b>Zero-dependency, high-performance Linux Operational Security (OPSEC), anti-forensics, and privacy engine written in native Rust.</b></p>
  
  <p>
    <a href="https://github.com/egnake/anon/blob/main/LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg?style=for-the-badge" alt="License"></a>
    <a href="https://github.com/egnake/anon/releases"><img src="https://img.shields.io/badge/Version-3.0.0-green.svg?style=for-the-badge" alt="Version"></a>
    <img src="https://img.shields.io/badge/Platform-Debian%20%7C%20Kali%20%7C%20Parrot%20%7C%20WSL2-orange.svg?style=for-the-badge" alt="Platform">
    <img src="https://img.shields.io/badge/Rust-Zero_External_Crates-yellow.svg?style=for-the-badge" alt="Language">
    <img src="https://img.shields.io/badge/Architecture-22_Modules-purple.svg?style=for-the-badge" alt="Architecture">
    <img src="https://img.shields.io/badge/i18n-English%20%7C%20T%C3%BCrk%C3%A7e-red.svg?style=for-the-badge" alt="i18n">
  </p>

  <h3>
    <a href="README.md">🇺🇸 English</a> | 
    <a href="README-TR.md">🇹🇷 Türkçe</a>
  </h3>
</div>

<br>

## 🚀 Overview

**ANON (v3.0)** is an enterprise-grade operational security (OPSEC) and counter-forensics framework engineered from the ground up as a **zero-dependency native Rust binary**. Designed for security researchers, penetration testers, investigative journalists, and privacy advocates, ANON transforms Debian, Kali Linux, Parrot OS, Ubuntu, and Windows 10/11 (via WSL2) installations into strictly isolated, hardened anonymous environments.

ANON combines **22 stateful security modules**, event-driven **Netlink and inotify kernel sentries**, hardware-level blocks, parallel multi-threaded metadata scrubbing, and fully volatile memory execution to guarantee complete confidentiality against active surveillance, ISP deep packet inspection (DPI), local network attacks, and cold-boot physical forensics.

---

## ⚡ Key Highlights (v3.0)

- **Pure Rust Native Engine (`src/`):** Zero external crates (`Cargo.toml` relies purely on `std` + isolated POSIX/Linux libc FFI in `src/sys.rs`). Lightning-fast compilation, zero dependency rot, and minimal binary footprint with full LTO and stripped symbols.
- **Bilingual Interface (`English` & `Türkçe`):** Seamless language switcher available directly inside the interactive TUI menu or via `--lang en` / `--lang tr`. Language preferences persist independently in `${BACKUPDIR}/anon.lang` across state resets.
- **Flicker-Free Interactive TUI:** Dual-buffered terminal rendering with dynamic box scaling, UTF-8 unicode borders, zero full-screen clear flashes (`\x1b[2J`), arrow key and Vim navigation (`j`/`k`), and a 3-second live auto-refresh dashboard.
- **Declarative Security Profiles:** Instantly apply hardened configurations non-interactively with `--profile <name>` (`paranoia`, `quick`, `pentest`, `darknet`, `lockdown`, `ghost`).
- **Atomic Firewall Routing:** Kernel-enforced `iptables-restore` / `ip6tables-restore` kill switches preventing microsecond packet leaks during transitions.
- **Kernel Namespace Sandbox (`anon_jail`):** Hardware-level process and network stack separation via Linux kernel network namespaces (`netns`), virtual ethernet pairs (`veth`), AppArmor MAC confinement, and Firejail seccomp sandboxing.
- **RAM-Only Ephemeral Execution:** Fileless browser sessions (`/dev/shm`), volatile logging overlays (`/var/log` on `tmpfs`), and process masquerading under kernel worker signatures (`[kworker/u4:2]`).
- **Cross-Platform Readiness:** Automated installer for Linux (`setup.sh`), Windows WSL2 wrapper (`setup.bat` & `anon.bat` with automated Windows-to-WSL path translation), and multi-stage Kali Rolling `Dockerfile`.

---

## 🛡️ Threat Model & Defense In Depth

ANON assumes adversaries may monitor network traffic, poison local ARP gateways, analyze packet timing, or seize physical equipment. The framework applies comprehensive mitigations across all four attack surfaces:

```
┌────────────────────────────────────────────────────────────────────────┐
│                        ANON DEFENSE MODEL                              │
├────────────────────────┬───────────────────────────────────────────────┤
│ Attack Vector          │ ANON Mitigation Technique                     │
├────────────────────────┼───────────────────────────────────────────────┤
│ ISP / DPI Surveillance │ Tor TransPort (9040), obfs4, Snowflake WebRTC, │
│ & Traffic Analysis     │ Traffic Chaffing, Jitter (tc netem)           │
├────────────────────────┼───────────────────────────────────────────────┤
│ Local Network Threats  │ Random MAC (OUI), Anti-MITM (Netlink ARP),     │
│ (Wi-Fi / LAN / DHCP)   │ Hostname Randomizer, RFKill Bluetooth Block   │
├────────────────────────┼───────────────────────────────────────────────┤
│ OS Fingerprinting &    │ TTL=128 Windows 10 Emulation, Timezone Spoof, │
│ Browser Profiling      │ Hardened anon.js Prefs, Anti-Canvas/WebGL     │
├────────────────────────┼───────────────────────────────────────────────┤
│ Host / RAM Forensics   │ Kernel netns Jail, hidepid=2, volatile tmpfs  │
│ & Physical Seizure     │ logging, sdmem RAM zeroization, --panic nuker │
└────────────────────────┴───────────────────────────────────────────────┘
```

---

## ⚙️ The 22 OPSEC Modules & Core Utilities

### 🌐 Network & Anonymity Subsystem

| # | Module Name | Description |
| :-: | :--- | :--- |
| **01** | **Tor Bridges (`tor_bridges`)** | Bypasses ISP censorship by wrapping Tor circuits in obfuscated `obfs4`, `webtunnel`, or custom bridge protocols. Includes interactive paste prompt and fallback persistence. |
| **02** | **IP Changer (`ip_changer`)** | Atomically forces all outbound TCP packets into Tor's transparent proxy (`TransPort` 9040) and DNS queries into `DNSPort` (5353) via kernel `iptables` redirection. |
| **03** | **DNS Changer (`dns_changer`)** | Overrides system resolvers with hardened, non-logging, uncensored DNS providers (Mullvad, Quad9, DNS.WATCH, UncensoredDNS) when Tor TransPort is inactive. |
| **04** | **DoH Shield (`doh_shield`)** | Establishes encrypted DNS-over-HTTPS / DNSCrypt resolution via a local `dnscrypt-proxy` daemon, preventing DNS tampering, interception, and ISP snooping. |
| **05** | **Network Kill Switch (`kill_switch`)** | Deploys an atomic firewall barrier (`iptables-restore` & `ip6tables-restore`). Drops all outbound and inbound packets not matching loopback or established Tor/I2P sockets. |
| **06** | **DPI Spoofing (`dpi_spoofing`)** | Camouflages Tor network traffic within ephemeral WebRTC connections via Tor Snowflake, disguising restricted traffic as ordinary peer-to-peer video conference streams. |
| **07** | **I2P Integration (`i2p_integration`)** | Manages the `i2pd` Invisible Internet Project router daemon, providing local SOCKS5 (`127.0.0.1:4446`) and HTTP (`127.0.0.1:4444`) proxies to decentralized eepsites. |
| **08** | **Proxychains Integration (`proxychains_integration`)** | Automatically configures `/etc/proxychains4.conf` to hook directly into active Tor SOCKS5 listeners with quiet mode enabled. |
| **09** | **MAC Changer (`mac_changer`)** | Spoofs physical network interface MAC addresses with authentic OEM/vendor-assigned OUIs, breaking historical router tracking while preserving wireless/VM stability. |

### 🧩 System Obfuscation & Sandboxing

| # | Module Name | Description |
| :-: | :--- | :--- |
| **10** | **Timezone Changer (`timezone_changer`)** | Spoofs `/etc/localtime` and `/etc/timezone` to randomized global coordinates (e.g. UTC, Asia/Tokyo, Europe/London), thwarting browser JavaScript timezone correlation. |
| **11** | **Hostname Changer (`hostname_changer`)** | Randomizes the system hostname (`/etc/hostname`, `/etc/hosts`, `dhclient.conf`) to generic OEM/Windows identifiers (`DESKTOP-*`, `LAPTOP-*`, `WIN-*`). |
| **12** | **Browser Anonymization (`browser_anonymization`)** | Injects hardened `anon.js` preferences across all installed Firefox, Firefox-ESR, and LibreWolf directories, strictly killing WebRTC, Canvas telemetry, and disk caches. |
| **13** | **OS Obfuscation (`os_obfuscation`)** | Emulates a Windows 10/11 TCP/IP fingerprint: rewrites outbound TTL to 128, disables TCP timestamps and window scaling, and tunes ephemeral port ranges (`p0f`/`nmap` deception). |
| **14** | **Process Obfuscation (`process_obfuscation`)** | Remounts `/proc` with `hidepid=2` so unprivileged users and untrusted software cannot view running processes. Provisions RAM-based stealth execution wrappers for sensitive tools. |
| **15** | **Sandbox Isolation (`sandbox_isolation`)** | Creates a virtual Kernel Network Namespace (`anon_jail`) connected via `veth` pair (`10.200.200.0/24`), AppArmor profiles for `/usr/bin/tor`, and strict Firejail seccomp sandboxing. |
| **16** | **Execution Stealth (`execution_stealth`)** | Automatically re-executes the framework from `/dev/shm` in volatile RAM, masquerading the process name and command-line arguments under `[kworker/u4:2]`. |
| **17** | **Traffic Obfuscation (`traffic_obfuscation`)** | Defeats Website Fingerprinting (WFP) and packet-size timing correlation attacks by injecting latency jitter via Linux `tc netem` and generating background dummy Tor chaff. |

### 🔥 Active Defense & Counter-Forensics

| # | Module Name | Description |
| :-: | :--- | :--- |
| **18** | **Anti-MITM Sentry (`anti_mitm`)** | Event-driven Netlink `RTMGRP_NEIGH` daemon combined with polling watchdog that detects ARP cache poisoning. Severely cuts compromised interfaces (`DOWN`) and blocks attacker MACs. |
| **19** | **Background Leak Auditor (`leak_auditor`)** | Event-driven `inotify` and Netlink sentry monitoring `/etc/resolv.conf`, Tor circuit liveness, and IPv6 routes. Automatically locks down the network and fires alerts upon detecting leaks. |
| **20** | **Volatile Logging (`volatile_logging`)** | Tails OS paradigm: mounts a dedicated `tmpfs` RAM disk over `/var/log`. Logs remain in volatile memory during the session; on stop, historical disk logs reappear with zero suspicious deletion gaps. |
| **21** | **Bluetooth Disabler (`bluetooth_disabler`)** | Interfaces with Linux `rfkill` to enforce a hardware-level block on all Bluetooth transmitters, closing proximity-based tracking and bluejacking vectors. |
| **22** | **Anti-Cold Boot (`anti_cold_boot`)** | Disables swap, dumps memory cores to `/bin/false`, and executes cryptographic RAM page wiping (`sdmem` or native multi-threaded Rust zeroization) to eliminate cold-boot memory recovery. |

### 🛠️ Built-in Operations & Utilities

- **🌐 Disposable Browser (`sudo anon --browser`):** Spawns an ephemeral Firefox instance mounted in `/dev/shm` routed strictly through Tor inside `anon_jail`. The moment the browser window closes, the entire RAM profile is cryptographically wiped (`shred`).
- **🧹 Parallel Metadata Cleaner (`sudo anon --clean-meta <path>`):** Multi-threaded forensic sanitizer utilizing `MAT2` (Tails OS standard) with automatic fallback to `ExifTool`. Strips EXIF metadata, GPS coordinates, author histories, and hardware serials across images, documents, and archives.
- **🚨 Emergency Panic Protocol (`sudo anon --panic`):** Duress protocol: instantly severs all physical/wireless interfaces, enforces total drop firewall policies, terminates all network processes, shreds session RAM directories, drops kernel caches, wipes bash/zsh history, and scrubs free memory.
- **🔧 System Recovery & Fix (`sudo anon --fix`):** Atomic recovery utility that inspects saved backups, restores baseline network configs, unmasks `resolv.conf`, restores original MAC addresses, and resets firewall tables after abrupt shutdowns.
- **🔄 Tor Circuit Refresh (`SIGNAL NEWNYM`):** Interactive Tor ControlPort integration allowing one-click identity changes and circuit rotation without restarting the daemon.

---

## 🎯 Security Profiles

ANON provides 6 pre-configured security profiles to match operational requirements non-interactively:

```bash
sudo anon --profile <name>
```

| Profile | Focus | Enabled Modules |
| :--- | :--- | :--- |
| `paranoia` | **Maximum OPSEC** | All 22 modules enabled simultaneously (Tor, Bridges, Kill Switch, Jail, Stealth, Anti-MITM, RAM wipe, etc.). |
| `quick` | **Fast Anonymous Browsing** | `ip_changer`, `kill_switch`, `sandbox_isolation` |
| `pentest` | **Offensive Security Work** | `ip_changer`, `proxychains_integration`, `traffic_obfuscation`, `anti_mitm` |
| `darknet` | **Deep Censorship Bypass** | `ip_changer`, `tor_bridges`, `i2p_integration`, `doh_shield` |
| `lockdown` | **Physical Device Defense** | `kill_switch`, `anti_cold_boot`, `volatile_logging`, `bluetooth_disabler` |
| `ghost` | **Maximum System Camouflage**| 19 non-overlapping modules emphasizing host/network obfuscation and stealth execution |

---

## 🖥️ Interactive Terminal User Interface (TUI)

Launch the interface by typing:
```bash
sudo anon
```

The responsive TUI automatically adapts to your terminal dimensions (`LINES` x `COLUMNS`):
- **Live Monitoring Dashboard:** Real-time throughput (TX/RX), Tor exit IP verification, network state indicators, and active module counters (refreshes every 3s).
- **Smooth Navigation:** Use Arrow keys or Vim keybindings (`j`/`k`), `Enter` to select, and `q` to return or quit without screen flicker.
- **Interactive Language Switcher:** Switch between English and Türkçe in real-time with instant UI re-rendering.

---

## 💻 Installation & Quick Start

### Native Linux (Debian / Kali / Parrot / Ubuntu)

```bash
# 1. Clone the repository
git clone https://github.com/egnake/anon.git
cd anon

# 2. Run the automated installer
sudo chmod +x setup.sh
sudo ./setup.sh

# 3. Launch ANON
sudo anon
```

### Windows 10/11 (via WSL2)

1. Open PowerShell or Command Prompt as Administrator in the repository folder:
   ```cmd
   setup.bat
   ```
2. Once installed, invoke ANON anywhere from Windows terminal:
   ```cmd
   anon
   anon --browser
   anon --clean-meta "D:\SensitiveDocs"
   ```

### Docker Container (Kali Rolling Runtime)

```bash
docker build -t anon .
docker run --rm -it --cap-add=NET_ADMIN --cap-add=SYS_ADMIN anon
```

---

## 📖 CLI Command Reference

| Command | Action |
| :--- | :--- |
| `sudo anon` | Launches the interactive Terminal User Interface (TUI). |
| `sudo anon --start` | Starts the framework non-interactively with saved or default modules. |
| `sudo anon --stop` | Immediately tears down all active modules and restores original networking. |
| `sudo anon --profile <name>` | Non-interactively applies a profile (`paranoia`, `quick`, `pentest`, `darknet`, `lockdown`, `ghost`). |
| `sudo anon --status` | Prints a colored visual status report of all 22 modules. |
| `sudo anon --status --json` | Outputs machine-readable JSON status of all modules and version info for scripting. |
| `sudo anon --dashboard` | Opens the 3-second live auto-refreshing network & throughput dashboard. |
| `sudo anon --browser` | Directly launches the ephemeral RAM-based disposable browser. |
| `sudo anon --audit` | Performs an instant comprehensive IP, DNS, and socket leak audit (append `--json` for JSON output). |
| `sudo anon --clean-meta <path>`| Recursively cleans forensic metadata from files or directories in parallel. |
| `sudo anon --lang <en|tr>` | Switches interface language between English (`en`) and Türkçe (`tr`). |
| `sudo anon --panic` | Triggers the Emergency Panic Protocol to instantly lockdown and wipe memory. |
| `sudo anon --fix` | Restores baseline system state and network connectivity from stored backups. |
| `sudo anon --update` | Fetches the latest signed release from GitHub via Tor, checks SHA256, and reinstalls. |
| `anon --version` | Displays current version (`ANON v3.0.0`) without requiring root privileges. |

---

## 🏗️ Repository Architecture

```text
.
├── Cargo.toml                  # Zero-dependency manifest (opt-level 3, LTO, strip)
├── src/
│   ├── main.rs                 # CLI entrypoint, internal daemon dispatcher, profile engine
│   ├── config.rs               # Environment-aware path resolution (BASE_DIR, BACKUPDIR, TORRC)
│   ├── i18n.rs                 # Multi-language engine (English / Türkçe, persistent anon.lang)
│   ├── state.rs                # POSIX flock + atomic temp/chmod/rename state persistence
│   ├── sys.rs                  # Linux libc FFI (geteuid, flock, Netlink, inotify, mlock RAM wiping)
│   ├── helpers.rs              # NetworkManager locks, Tor ControlPort TCP, JSON helpers, dependency checker
│   ├── registry.rs             # Declarative 22-module registry, execution orders, trait definitions
│   ├── ui.rs                   # ANSI color palette, buffered box drawing, dynamic banners, notifications
│   ├── tui.rs                  # Raw/tty-guarded interactive menu, category toggles, live dashboard
│   └── modules/
│       ├── mod.rs              # Trait dispatch & module factory
│       ├── network.rs          # IP, DNS, DoH, Kill Switch, Bridges, DPI, I2P, Proxychains, MAC
│       ├── obfuscation.rs      # Timezone, Hostname, Browser, OS, Process, Sandbox Jail, Stealth, Chaff
│       ├── defense.rs          # Anti-MITM, Leak Auditor, Volatile Logging, Bluetooth, Anti-Cold Boot
│       └── utils.rs            # Fix/Recovery, Panic, Disposable Browser, Parallel Metadata Sanitizer, Update
├── assets/                     # Desktop entries, icons, hardened anon.js, bridge lists, banners
├── setup.sh / setup.bat        # Automated installers for Linux and Windows WSL2
├── Makefile                    # Standard Linux packaging (build, install, uninstall, lint, test)
├── Dockerfile                  # Multi-stage Kali Rolling container build
└── test_suite.sh               # Cargo unit tests + 68 blackbox fakeroot integration tests
```

---

## 🧪 Testing & Verification

ANON maintains a strict **zero-warning policy** and continuous integration testing. Run the complete test suite locally:

```bash
# 1. Run cargo lint and unit tests
make lint
RUSTFLAGS="-D warnings" cargo test

# 2. Run full blackbox integration suite (68 assertions)
ANON_FORCE_BUILD=1 ./test_suite.sh
```

---

## ⚠️ Legal & Security Disclaimer

This software is developed strictly for educational purposes, security audits, authorized penetration testing, and digital privacy preservation. The developers assume no liability for misuse, unauthorized activities, or violations of local telecommunications regulations.

Always adhere to operational security principles: software cannot protect you from behavioral mistakes (e.g. logging into personal clearnet accounts over Tor).

---

<div align="center">
  <b>Developed & Maintained by <a href="https://github.com/egnake">egnake</a></b><br>
  For bug reports and feature requests, open a <a href="https://github.com/egnake/anon/issues">GitHub Issue</a>.
</div>
