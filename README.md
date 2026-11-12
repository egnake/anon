<div align="center">
  <img src="assets/anon_logo.jpg" alt="ANON Logo" width="200" style="border-radius: 20px;">
  
  <h1>ANON: Advanced OPSEC & Anonymity Framework</h1>
  <p><b>A comprehensive, modular anonymity and anti-forensics toolkit for Linux environments.</b></p>
  
  <p>
    <a href="https://github.com/egnake/anon/blob/main/LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg?style=for-the-badge" alt="License"></a>
    <a href="https://github.com/egnake/anon/releases"><img src="https://img.shields.io/badge/Version-3.0-green.svg?style=for-the-badge" alt="Version"></a>
    <img src="https://img.shields.io/badge/Platform-Linux%20(Debian%2FKali)-orange.svg?style=for-the-badge" alt="Platform">
    <img src="https://img.shields.io/badge/Rust-Zero_Deps-yellow.svg?style=for-the-badge" alt="Language">
    <img src="https://img.shields.io/badge/Architecture-Modular-purple.svg?style=for-the-badge" alt="Architecture">
  </p>

  <h3>
    <a href="README.md">🇺🇸 English</a> | 
    <a href="README-TR.md">🇹🇷 Türkçe</a>
  </h3>
</div>

<br>

## 🚀 Overview

**ANON** is a rigorous, open-source anonymity and Operations Security (OPSEC) framework written in **zero-dependency Rust** for researchers, developers, and privacy advocates. Compiled to a fast native binary, ANON provides a deeply integrated, 22-module architecture (alongside native Netlink/inotify daemons, parallel metadata sanitization, disposable browser, emergency panic, and system recovery utilities) that enforces strict network isolation, randomizes system fingerprints, and applies active anti-forensics measures.

By utilizing Linux kernel namespaces (`netns`), traffic control (`tc`), and dynamic `iptables` configurations, ANON transforms a standard Debian-based installation into a highly restricted environment. It ensures that all outbound traffic is strictly routed through the Tor network while mitigating common timing attacks and local network threats.

---

## 🛡️ Threat Model & Philosophy

ANON is built around the concept of "Defense in Depth." It assumes that adversaries may exist at multiple points in your communication chain. The framework attempts to mitigate the following specific vectors:

1. **Traffic Analysis & ISP Monitoring:** Mitigated by forcing all TCP and DNS traffic through Tor, while utilizing `obfs4` bridges, Snowflake WebRTC camouflage, and active traffic obfuscation (dummy packets and jitter) to disrupt Website Fingerprinting (WFP).
2. **Local Area Network (LAN) Attacks:** Mitigated via MAC address randomization, ARP spoofing detection, hostname spoofing, and strict isolation from the physical host interfaces.
3. **Endpoint Vulnerabilities & Forensics:** Mitigated through the use of isolated kernel network namespaces (`anon_jail`), temporary RAM-based filesystems for browsers and system logs (`volatile_logging`), process masquerading (`execution_stealth`), and cryptographic memory wiping (`sdmem`) to counter cold-boot data extraction.

---

## ⚙️ The 22 OPSEC Modules & Utilities (Detailed Breakdown)

ANON is highly modular. You can toggle all 22 stateful modules individually via the interactive Terminal User Interface (TUI) or preset security profiles, alongside built-in disposable browser, metadata cleaning, emergency panic, and system recovery utilities.

### 🌐 Network & Routing Subsystem

**[01] Tor Bridges (obfs4)**
Standard Tor traffic is easily detectable via Deep Packet Inspection (DPI) by ISPs and firewalls. This module configures the Tor daemon to utilize `obfs4` bridges. By doing so, it encrypts the Tor handshake and alters the traffic signature, making it indistinguishable from random noise, which is essential for bypassing strict censorship.

**[02] IP Changer (Tor TransPort)**
Relying solely on SOCKS5 proxies can lead to fatal data leaks if an application is misconfigured. This module leverages strict `iptables` `REDIRECT` rules at the kernel level. It captures all outbound TCP traffic originating from your machine and physically forces it into Tor's transparent proxy port (9040), ensuring absolute compliance regardless of application settings. Outbound DNS requests are automatically redirected to Tor's DNSPort (5353).

**[03] DNS Changer (Privacy DNS Fallback)**
For scenarios where Tor transparent proxying is inactive, this module configures privacy-hardened, non-logging DNS resolvers (uncensored DNS servers) directly into system resolution, disabling ISP-based DNS hijacking.

**[04] MAC Changer**
Before a network connection is fully established, this module alters the Media Access Control (MAC) address of your Network Interface Card (NIC) to an authentic vendor-spoofed OUI value. This breaks historical tracking by routers, public Wi-Fi hotspots, and local network administrators.

**[05] Proxychains Integration**
Automatically configures and aligns `proxychains4` with your active Tor circuits. This allows you to effortlessly tunnel specific terminal commands, scripts, or external tools (such as `curl`, `wget`, or custom python scripts) through the Tor network without manual configuration tweaking.

**[06] I2P Integration**
Provides an alternative routing mechanism. For tasks that require interaction with the Invisible Internet Project (I2P) network rather than the Tor network, this module bridges the gap, allowing secure access to decentralized eepsites.

**[07] Traffic Obfuscation (Timing Attack Defense)**
Low-latency networks like Tor remain susceptible to end-to-end timing correlation and Website Fingerprinting (WFP). This advanced module utilizes the Linux Traffic Control utility (`tc`) to inject randomized latency (jitter) into your outbound packets. Furthermore, it generates continuous, randomized background Tor traffic (chaffing) to obscure your actual data volume, heavily complicating packet-size and timing analysis.

**[08] DPI Spoofing (Snowflake WebRTC Camouflage)**
Bypasses advanced Deep Packet Inspection by encapsulating Tor traffic inside ephemeral WebRTC connections via Tor Snowflake, disguising Tor traffic as ordinary peer-to-peer video/audio conference streams.

**[09] DoH Shield (DNS-over-HTTPS via DNSCrypt)**
Enforces encrypted DNS queries over HTTPS/DNSCrypt using a local `dnscrypt-proxy` daemon, ensuring cryptographic authenticity and confidentiality for DNS lookups when Tor DNSPort is not in use.

### 🧩 System Obfuscation & Isolation

**[10] Timezone Changer**
Modern web tracking often correlates your system's local timezone (leaked via JavaScript or system headers) with your IP address. This module temporarily spoofs your system timezone to random global coordinates (e.g., UTC or Asia/Tokyo), disrupting correlation algorithms.

**[11] Hostname Changer**
Your computer's hostname (e.g., `user-laptop`) is frequently broadcasted across local networks via DHCP requests. This module randomizes the hostname to generic OEM/Windows-style identifiers to blend into crowded networks seamlessly.

**[12] Browser Anonymization (Hardened Configuration)**
Installs hardened privacy preferences (`anon.js`) across system-wide browser configuration directories (`/etc/firefox`, `/etc/firefox-esr`), strictly disabling WebRTC, WebGL, Canvas fingerprinting, telemetry, and disk caching.

**[13] Disposable Browser (RAM-based Ephemeral Session)**
Provisions a hardened Firefox profile mounted entirely in volatile memory (`/dev/shm`). SOCKS5 proxying with remote DNS is strictly enforced. The instant the browser window closes, the memory profile is cryptographically wiped (`shred`), leaving zero forensic artifacts on disk.

**[14] OS Obfuscation (TTL & TCP Window Spoofing)**
Passive network scanners (such as `p0f` or `nmap`) guess your OS by analyzing default TTL values and TCP window parameters. This module intercepts outbound packets and rewrites TTL to 128 (Windows 10/11 emulation), disables TCP timestamps and window scaling, and adjusts ephemeral port ranges.

**[15] Process Obfuscation (hidepid=2 Kernel Isolation)**
Remounts `/proc` with `hidepid=2` so unprivileged users and malware cannot inspect running processes or background daemons. Creates RAM-based stealth execution wrappers for sensitive tools.

**[16] Sandbox Isolation (Kernel netns anon_jail)**
Creates an isolated Kernel Network Namespace (`anon_jail`) wired via a virtual ethernet pair (`veth-anon`/`veth-jail`). Outbound traffic is physically redirected into the host's Tor TransPort (9040) and DNSPort (5353). Integrates with AppArmor and Firejail for full containerized containment.

**[17] Execution Stealth (Fileless RAM Masquerading)**
Runs the entire framework and its tools inside ephemeral RAM storage (`/dev/shm`), while masquerading processes under innocent kernel worker names (`[kworker/u4:2]`). Ephemeral runtime state is shredded on exit while persistent configs and backups are preserved.

### 🔥 Active Defense & Anti-Forensics

**[18] Network Kill Switch (Atomic iptables-restore)**
Deploys strict firewall rules loaded atomically via `iptables-restore` and `ip6tables-restore`, preventing microsecond packet leaks. Only loopback and established Tor traffic are accepted; all clear web leaks are instantly dropped.

**[19] Anti-MITM (ARP Spoofing Protection)**
Continuously monitors gateway MAC addresses in the kernel ARP table. If an ARP cache poisoning attempt is detected, the network interface is immediately severed (`DOWN`) and the attacker's MAC is blocked.

**[20] Anti-Cold Boot (sdmem & Swap Lock)**
Disables swap, locks memory core dumps, and executes `sdmem` (Secure Delete Memory) to cryptographically wipe free RAM and swap space, neutralizing cold-boot attacks against memory keys.

**[21] Bluetooth Disabler (rfkill Hardware Block)**
Interfaces with Linux `rfkill` to enforce a hardware-level block on all Bluetooth controllers, eliminating proximity-based attacks and hardware beacon tracking.

**[22] Background Leak Auditor (Continuous Sentry)**
Runs an asynchronous watchdog daemon that continuously monitors active network egress, `/etc/resolv.conf` resolvers, and Tor circuits. If an unexpected clearnet connection or DNS leak is detected, it dispatches a critical desktop notification and instantly engages the Network Kill Switch.

**[23] Desktop Notifications (notify-send GUI Alerts)**
Bridges root/daemon events to active unprivileged desktop sessions, delivering high-priority GUI notifications for ARP spoofing attempts, Tor connection changes, leak detections, and panic lockdown triggers.

**[24] Metadata Cleaner (MAT2 & ExifTool Integration)**
Forensic sanitization tool to strip EXIF tags, GPS coordinates, author history, and device serial numbers from images, PDFs, office documents, and media using MAT2 (Tails OS standard) with seamless fallback to ExifTool.

**[Bonus] Volatile Logging (Tails OS RAM Overlay)**
Mounts an in-memory `tmpfs` disk over `/var/log` and journal directories during the session. All logs are written to volatile RAM; upon stopping, historical logs reappear with only a clean time gap and zero suspicious deletion traces.

---

## 🖥️ Interactive Terminal User Interface (TUI)

ANON replaces clunky command-line arguments with an elegant, flicker-free responsive TUI compiled natively in Rust.

* **Dynamic Aesthetic Rendering:** Centered ASCII banners that change dynamically on launch, providing a clean, distraction-free environment.
* **Live Dashboard:** Monitor your Tor exit IP, network status, traffic throughput, and active OPSEC modules in real-time without clear web leaks.
* **Operations Hub:** Execute DNS/IP leak tests, request a new Tor identity (ControlPort `SIGNAL NEWNYM` with `SIGHUP` fallback), launch the sandboxed disposable browser, or trigger Emergency Panic Lockdown directly from the interface.

---

## 📦 System Requirements & Dependencies

ANON is heavily integrated with the Linux networking stack. It requires:

- **Operating System:** Kali Linux, Parrot Security OS, Debian, Ubuntu, or Windows 10/11 via WSL2 (`systemd=true` in `/etc/wsl.conf` recommended; Docker supported with `--cap-add=NET_ADMIN --cap-add=SYS_ADMIN`).
- **Permissions:** Root privileges (`sudo`) are required to manipulate `iptables`, kernel namespaces, and network interfaces.
- **Build & Runtime Dependencies (Automatically installed via `setup.sh`):** `cargo`, `rustc`, `make`, `tor`, `obfs4proxy`, `snowflake-client`, `dnscrypt-proxy`, `macchanger`, `proxychains4`, `secure-delete` (`sdmem`), `rfkill`, `firejail`, `apparmor-utils`, `iproute2`, `iptables`, `i2pd`, `mat2`, `libimage-exiftool-perl`, `libnotify-bin`, `psmisc`, `unzip`, `curl`, `ca-certificates`.
- **Windows (WSL2) & Docker:** Run `setup.bat` on Windows to install/configure WSL2 and add `anon.bat` to your user `PATH` (supports Windows-to-WSL path translation for `--clean-meta`). In Docker, build with `docker build -t anon .` and run with `docker run --rm -it --cap-add=NET_ADMIN --cap-add=SYS_ADMIN anon`.

---

## 🛠️ Installation Guide

The installation process is fully automated via the `setup.sh` script.

```bash
# 1. Clone the repository
git clone https://github.com/egnake/anon.git

# 2. Enter the directory
cd anon

# 3. Make the setup script executable
sudo chmod +x setup.sh

# 4. Run the installer
sudo ./setup.sh
```

---

## 🚀 Usage & Commands

To launch the main interactive TUI, simply run:
```bash
sudo anon
```

### Advanced Command-Line Flags
For users who prefer automation or scripting, ANON supports direct command-line execution:

| Command | Description |
| :--- | :--- |
| `sudo anon --start` | Starts the framework non-interactively using the last saved module configuration (or default Tor + Kill Switch). |
| `sudo anon --profile <name>` | Applies and starts a preset security profile (`paranoia`, `quick`, `pentest`, `darknet`, `lockdown`, `ghost`) non-interactively. |
| `sudo anon --stop` | Immediately disables all active modules, flushes iptables, and restores the original network state. |
| `sudo anon --status` | Prints a comprehensive visual report of all active and inactive OPSEC modules. |
| `sudo anon --status --json` | Outputs current version, stealth state, and all 22 module statuses in machine-readable JSON. |
| `sudo anon --dashboard` | Opens the live monitoring dashboard (auto-refreshes every 3 seconds). |
| `sudo anon --browser` | Directly launches the RAM-based disposable browser inside the isolated network namespace. |
| `sudo anon --audit` | Performs an instant deep IP, DNS, and socket leak audit (append `--json` for machine-readable output). |
| `sudo anon --clean-meta <path>` | Sanitizes forensic metadata (EXIF/GPS/tags) from files or directories via MAT2/ExifTool. |
| `sudo anon --panic` | Emergency Duress Protocol: immediately cuts all network interfaces, drops all firewall traffic, terminates processes, and sanitizes RAM. |
| `sudo anon --fix` | Restores default system network configurations from backup in the event of a crash or abrupt shutdown. |
| `sudo anon --update` | Fetches the latest release tarball (routed via Tor when active), verifies `SHA256SUMS`, and reinstalls the framework. |
| `anon --version` | Displays the installed ANON version (`ANON v3.0.0`) without modifying system state. |

---

## ❓ FAQ & Troubleshooting

**Q: I have no internet connection after running ANON!**
A: If Tor fails to connect while the `Kill Switch` is active, all traffic is dropped by `iptables`. Ensure your network is working before starting the script, check your system clock (Tor needs an accurate clock), or verify your Tor bridges are functioning. You can run `sudo anon --stop` to reset your network, or `sudo anon --fix` to restore from physical backups.

**Q: Can I use this on a Virtual Machine (VM)?**
A: Yes. However, some modules (like MAC Changer) might require your hypervisor (VMware, VirtualBox) to explicitly allow MAC address spoofing in the network adapter settings. Without it, the VM might lose internet connectivity when the MAC is changed.

**Q: Does the Disposable Browser save my bookmarks or passwords?**
A: No. By design, the browser profile exists only in volatile RAM (`/dev/shm`). Everything—history, cache, extensions, and bookmarks—is destroyed the moment the browser window is closed. It is intended for highly sensitive, single-use sessions.

**Q: Is it safe to log into my personal accounts (like Google or Facebook) while using ANON?**
A: Absolutely not. This is a critical OPSEC failure. Even if your IP and MAC address are hidden, logging into an account tied to your real identity instantly correlates your anonymous traffic with you. ANON protects your machine, but it cannot protect you from behavioral mistakes.

**Q: Why does the "Traffic Obfuscation" module slow down my connection?**
A: This module intentionally injects artificial latency (jitter) into your packets to defeat timing correlation and Website Fingerprinting (WFP) attacks. The minor slowdown is a trade-off for significantly increased anonymity against sophisticated network observers.

**Q: How does the Kernel netns Isolation differ from normal Proxychains?**
A: Proxychains intercepts standard library calls (like `connect()`) to route applications through Tor. However, some applications can bypass this (e.g., static binaries, custom DNS resolution). Kernel `netns` creates a literal, hard-walled sandbox. Inside the jail, the only physical path out of the container is wired to the Tor TransPort. It is impossible to bypass it.

**Q: What happens if the script crashes or my computer loses power abruptly?**
A: Upon installation, ANON backs up your original `iptables`, `resolv.conf`, and `NetworkManager` states. If the system reboots unexpectedly, simply run `sudo anon --fix` to instantly restore all network settings to their default factory state.

---

## ⚠️ Security Notice & Limitations

ANON provides advanced technical mitigation against network analysis and forensics, but it is **not a magic bullet**. 

* **Operational Security (OPSEC):** Software cannot protect you from human error. Logging into personal accounts (Facebook, Google) while using ANON will immediately deanonymize you.
* **HTTPS/TLS:** Tor encrypts traffic up to the Exit Node. If you send unencrypted HTTP traffic, the Exit Node can monitor and intercept it. Always ensure you are using HTTPS.
* **Firmware/Hardware Risks:** ANON operates at the OS layer. It cannot protect against compromised BIOS/UEFI firmware, Intel ME/AMD PSP backdoors, or hardware keyloggers.

**Disclaimer:** This tool is provided for educational, academic, and security research purposes only. The developers are not responsible for any misuse, illegal activities, or damage caused by this software. Use it responsibly and legally within your jurisdiction.

---

<div align="center">
  <b>Report Issues & Contribute</b><br>
  egcaem000@gmail.com | <a href="https://github.com/egnake/anon/issues">GitHub Issues</a>
</div>
