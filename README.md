<div align="center">
  <img src="assets/anon_hero.jpg" alt="ANON Hero Banner" width="100%">
  
  <h1>ANON: Advanced OPSEC & Anonymity Framework</h1>
  <p><b>Government-Level Anonymity, Anti-Forensics, and Traffic Obfuscation Framework for Linux</b></p>
  
  <p>
    <a href="https://github.com/egnake/anon/blob/main/LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg?style=for-the-badge" alt="License"></a>
    <a href="https://github.com/egnake/anon/releases"><img src="https://img.shields.io/badge/Version-3.0-green.svg?style=for-the-badge" alt="Version"></a>
    <img src="https://img.shields.io/badge/Platform-Linux%20(Debian%2FKali)-orange.svg?style=for-the-badge" alt="Platform">
    <img src="https://img.shields.io/badge/Bash-100%25-yellow.svg?style=for-the-badge" alt="Language">
  </p>
</div>

<br>

## 🚀 Overview

**ANON** is a state-of-the-art anonymity and OPSEC (Operations Security) framework designed for security researchers, journalists, and activists. It goes beyond simple Tor routing by providing **17 independent modules** that enforce strict network isolation, obfuscate system fingerprints, mitigate traffic analysis (timing attacks), and execute paranoid anti-forensics measures.

Unlike traditional scripts, ANON introduces **Whonix-style Kernel Network Namespace Isolation (`anon_jail`)**, creating a virtual sandbox that forces all traffic through Tor and protects against 0-day browser exploits.

---

## ⚡ Features & Modules

ANON provides 17 highly specialized modules categorized into three tactical domains:

### 🌐 Network & Routing
- **[01] Tor Bridges (obfs4):** Bypass Deep Packet Inspection (DPI) and ISP blocking using custom bridge nodes.
- **[02] IP Changer (Tor TransPort):** Routes all TCP/DNS traffic through the Tor network transparently.
- **[03] DNS Changer:** Prevents DNS leaks by forcing name resolution through Tor's secure DNS.
- **[04] MAC Changer:** Randomizes your network interface controller's (NIC) MAC address to prevent LAN tracking.
- **[05] Proxychains Integration:** Automatically tunnels any terminal command through Tor.
- **[06] I2P Integration:** Supports routing traffic through the Invisible Internet Project (I2P) network.
- **[07] Traffic Obfuscation (Timing Attack Defense):** Uses Linux `tc` (Traffic Control) to add randomized jitter/latency to packets, combined with background dummy Tor traffic to defeat Website Fingerprinting (WFP) and correlation attacks.

### 🛡️ System Obfuscation & Isolation
- **[08] Timezone Changer:** Spoofs your system timezone to prevent location-based browser fingerprinting.
- **[09] Hostname Changer:** Randomizes your computer's hostname to blend into public networks.
- **[10] Browser Anonymization:** Deploys a highly-hardened, disposable Firefox profile directly into RAM (`/dev/shm`).
- **[11] OS Obfuscation (TTL Spoofing):** Modifies IP packet Time-To-Live (TTL) values to mask your true OS from network scanners (e.g., hiding Linux as Windows).
- **[12] Process Obfuscation:** Hides background OPSEC processes from standard monitoring tools.
- **[13] Sandbox Isolation (Kernel netns):** Creates an isolated kernel-level network namespace (`anon_jail`). Applications launched here have no access to the real host network interfaces and are strictly routed through Tor. Integrates with **Firejail / AppArmor**.

### 🔥 Active Defense & Anti-Forensics
- **[14] Network Kill Switch:** Implements strict `iptables` rules that immediately DROP all packets that are not destined for the Tor network. Absolutely zero data leaves the machine directly.
- **[15] Anti-MITM (ARP Spoofing Protection):** Continuously monitors the ARP table and blocks Man-in-the-Middle attacks on your local network.
- **[16] Anti-Cold Boot (sdmem):** Cryptographically wipes the system RAM (Random Access Memory) and Swap partitions before shutdown to prevent cold-boot forensics.
- **[17] Bluetooth Disabler:** Hard-blocks all Bluetooth radios via `rfkill` to prevent wireless tracking and Blueborne attacks.

---

## 🖥️ Interactive User Interface (TUI)

ANON v3.0 features a completely redesigned, arrow-key navigable TUI built directly in Bash. 

- **Live Dashboard:** Monitor Tor network status, exit IP, and active modules in real-time.
- **Operations Menu:** Execute IP Leak Tests, request new Tor Circuits (`SIGHUP`), or launch the Disposable Browser directly from the interface.
- **Dynamic ASCII Banners:** Hacker-themed UI that centers perfectly regardless of your terminal size.

---

## ⚙️ Installation

ANON requires a Debian-based Linux distribution (Kali Linux, Parrot OS, Ubuntu, Debian, or WSL2).

```bash
git clone https://github.com/egnake/anon.git
cd anon
sudo chmod +x setup.sh
sudo ./setup.sh
```

## 🛠️ Usage

To launch the interactive framework, simply run:
```bash
sudo anon
```

**CLI Flags:**
- `sudo anon --start` : Starts the tool immediately using the last saved profile.
- `sudo anon --stop` : Disables all modules and restores the system to its original state.
- `sudo anon --status` : Prints the status of all OPSEC modules.
- `sudo anon --dashboard` : Opens the live monitoring dashboard.
- `sudo anon --browser` : Launches the RAM-based disposable browser.
- `sudo anon --update` : Fetches and installs the latest version from GitHub.

---

## ⚠️ Disclaimer

ANON is provided for **educational and research purposes only**. The developers assume no liability and are not responsible for any misuse or damage caused by this program. Anonymity is not a product; it is a process. Always combine software tools with strong operational security practices.

**Report Issues:** egcaem000@gmail.com | [GitHub Issues](https://github.com/egnake/anon/issues)
