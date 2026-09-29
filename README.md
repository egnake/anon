<div align="center">
  <img src="assets/anon_hero.jpg" alt="ANON Hero Banner" width="100%">
  
  <h1>ANON: Advanced OPSEC & Anonymity Framework</h1>
  <p><b>State-Sponsored Level Anonymity, Anti-Forensics, and Traffic Obfuscation Framework for Linux</b></p>
  
  <p>
    <a href="https://github.com/egnake/anon/blob/main/LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg?style=for-the-badge" alt="License"></a>
    <a href="https://github.com/egnake/anon/releases"><img src="https://img.shields.io/badge/Version-3.0-green.svg?style=for-the-badge" alt="Version"></a>
    <img src="https://img.shields.io/badge/Platform-Linux%20(Debian%2FKali)-orange.svg?style=for-the-badge" alt="Platform">
    <img src="https://img.shields.io/badge/Bash-100%25-yellow.svg?style=for-the-badge" alt="Language">
    <img src="https://img.shields.io/badge/Architecture-Modular-purple.svg?style=for-the-badge" alt="Architecture">
  </p>

  <p>
    <a href="README.md">English</a> •
    <a href="README-TR.md">Türkçe</a>
  </p>
</div>

<br>

## 🚀 Overview

**ANON** is a state-of-the-art anonymity and OPSEC (Operations Security) framework designed for security researchers, whistleblowers, journalists, and activists who operate under extreme threat models. It goes beyond simple proxy routing by providing **17 independent modules** that enforce strict network isolation, obfuscate system fingerprints, mitigate traffic analysis (timing attacks), and execute paranoid anti-forensics measures.

Unlike traditional scripts that merely run `tor` and change your IP, ANON introduces **Whonix-style Kernel Network Namespace Isolation (`anon_jail`)**. This creates a virtual OS-level sandbox that forces all traffic through Tor and protects against severe 0-day browser exploits. It turns a standard Kali/Debian installation into a fortress.

---

## ⚡ Technical Architecture & Threat Modeling

ANON operates on three primary tactical domains to defend against specific adversary capabilities:

### 1. Global Passive Adversary (GPA) Defense
Defending against ISPs, autonomous systems, and state-level surveillance that monitor traffic flow and metadata.
- **Mitigation:** Transparent Tor routing, obfs4 bridges, DNS leak prevention, and active Traffic Obfuscation (chaffing & jitter).

### 2. Local Network Adversary (LNA) Defense
Defending against attackers on your physical LAN (e.g., rogue access points, ARP spoofing attacks, MAC tracking).
- **Mitigation:** MAC address randomization, ARP spoofing detection, Hostname spoofing, Bluetooth radio kill switch.

### 3. Endpoint Forensics Defense
Defending against physical device seizure, cold-boot attacks, and malware exploiting the browser to leak the real IP.
- **Mitigation:** RAM-only disposable browsers, Kernel Namespace Sandboxing, `sdmem` memory wiping, TTL OS obfuscation.

---

## 🛠️ The 17 Modules Explained in Detail

### 🌐 Network & Routing (GPA Defense)

**[01] Tor Bridges (obfs4)**
Standard Tor traffic is easily identifiable via Deep Packet Inspection (DPI). This module configures Tor to use `obfs4` bridges, disguising your traffic as random noise. Crucial for users in countries with strict censorship firewalls (e.g., Great Firewall of China, Roskomnadzor).

**[02] IP Changer (Tor TransPort)**
Instead of relying on SOCKS5 proxies (which can leak if an application is misconfigured), this module uses `iptables` `REDIRECT` rules to force all TCP traffic on the machine into Tor's transparent proxy port (9040). 

**[03] DNS Changer (DNSCrypt / Tor DNSPort)**
DNS leaks are the #1 cause of deanonymization. This module hijacks all outbound port 53 (UDP/TCP) traffic and forces it through Tor's DNSPort (5353). Your ISP will never see which domains you are querying.

**[04] MAC Changer**
Randomizes the Media Access Control (MAC) address of your Network Interface Card (NIC) before connecting to any network. This prevents routers and public Wi-Fi hotspots from tracking your physical device across different locations.

**[05] Proxychains Integration**
Automatically configures and integrates `proxychains4`, allowing you to run any terminal command (like `nmap`, `curl`, or `sqlmap`) seamlessly through the Tor circuit without manual configuration.

**[06] I2P Integration**
For threat models where Tor is highly monitored, ANON can route specific traffic through the Invisible Internet Project (I2P), a fully decentralized mixnet optimized for hidden services rather than out-proxying.

**[07] Traffic Obfuscation (Timing Attack Defense) 🆕**
*The crown jewel of network defense.* Low-latency networks like Tor are vulnerable to Website Fingerprinting (WFP) and end-to-end timing correlation attacks. This module uses Linux `tc` (Traffic Control) to inject randomized jitter (15ms-30ms) into outbound packets, destroying precise timing analysis. Furthermore, it generates continuous background dummy traffic (chaffing) to obscure your actual data volume.

### 🛡️ System Obfuscation & Isolation (Endpoint Defense)

**[08] Timezone Changer**
Browsers and system tools leak your local timezone via JavaScript or NTP requests. This module spoofs your system timezone to random global coordinates (e.g., UTC or Asia/Tokyo), blending your fingerprint with millions of other users.

**[09] Hostname Changer**
If your computer is named `Johns-MacBook-Pro`, every router you connect to logs it. This module randomizes your hostname to generic strings, making your device invisible in DHCP logs.

**[10] Browser Anonymization (Disposable RAM Profile)**
Instead of launching a normal browser, ANON mounts a temporary filesystem directly into your RAM (`/dev/shm`). It applies a custom, highly-hardened `anon.js` configuration file (disabling WebRTC, WebGL, Canvas tracking). When the browser is closed, the profile vanishes. It never touches your SSD/HDD.

**[11] OS Obfuscation (TTL Spoofing)**
Network scanners (like `p0f` or `nmap`) can guess your operating system by looking at the default Time-To-Live (TTL) of your packets (Linux is usually 64, Windows is 128). This module manipulates `iptables` to forge packet TTLs, making your Linux machine appear as a Windows or macOS device to network sniffers.

**[12] Process Obfuscation**
A technique used to hide the OPSEC scripts and background processes from standard process monitoring (`ps`, `top`), mitigating risks from non-root malware attempting to profile the system defense mechanisms.

**[13] Sandbox Isolation (Kernel netns) 🆕**
*APT-Level Isolation.* Mimicking the architecture of Whonix, this module creates an isolated Kernel Network Namespace (`anon_jail`). It spawns a virtual ethernet pair (`veth`), forcing the namespace to communicate *only* through the host's Tor TransPort. When you launch the Disposable Browser, it is executed inside this jail with dropped privileges (`sudo -u`) and AppArmor/Firejail profiles. Even a remote code execution (RCE) 0-day in the browser cannot discover your real IP, MAC, or network interfaces.

### 🔥 Active Defense & Anti-Forensics

**[14] Network Kill Switch**
The ultimate fail-safe. If the Tor daemon crashes or the connection drops, your computer might default to sending traffic over the clear web. The Kill Switch applies draconian `iptables` rules that `DROP` everything except `lo` (loopback) and established Tor connections. If Tor goes down, your internet goes down. Zero leaks.

**[15] Anti-MITM (ARP Spoofing Protection)**
Monitors your local ARP table for malicious manipulation. If an attacker on your local network (e.g., a rogue Wi-Fi pineapple or evil twin) attempts to intercept your traffic via ARP spoofing, this module detects the anomaly and instantly severs the network connection.

**[16] Anti-Cold Boot (sdmem)**
In a physical seizure scenario, adversaries can extract encryption keys and active sessions by freezing your RAM modules (Cold-Boot Attack). When you exit ANON with this module enabled, it executes `sdmem` (Secure Delete Memory), cryptographically wiping all free RAM and Swap space before shutting down.

**[17] Bluetooth Disabler**
Bluetooth is notorious for broadcasting tracking beacons and being vulnerable to remote exploits (e.g., Blueborne). This module uses `rfkill` to hardware-block all Bluetooth controllers on the device.

---

## 🖥️ The Interactive TUI (Terminal User Interface)

Forget messy command-line arguments. ANON v3.0 features a completely redesigned, arrow-key navigable TUI built purely in Bash.

* **Dynamic ASCII Banners:** Hacker-themed UI that dynamically centers itself perfectly regardless of your terminal size. Changes automatically on every launch.
* **Live Dashboard:** Monitor Tor network status, exit IP, active modules, and traffic statistics in real-time.
* **Operations Menu:** Execute IP Leak Tests, request new Tor Circuits (`SIGHUP` for a new Exit Node), or launch the Disposable Browser directly from the interface without restarting the script.

---

## ⚙️ Installation & Requirements

### Supported Operating Systems
- Kali Linux (Recommended)
- Parrot Security OS
- Debian / Ubuntu
- Windows Subsystem for Linux (WSL 2) - *Some kernel-level features like netns or iptables may require custom WSL kernels.*

### Quick Install

```bash
git clone https://github.com/egnake/anon.git
cd anon
sudo chmod +x setup.sh
sudo ./setup.sh
```

The `setup.sh` script will automatically install all required dependencies (Tor, Macchanger, secure-delete, proxychains4, firejail, etc.) and configure the environment.

---

## 🛠️ Usage & Commands

To launch the interactive framework, simply run:
```bash
sudo anon
```

**Advanced CLI Flags:**
- `sudo anon --start` : Starts the tool immediately using the last saved profile configuration.
- `sudo anon --stop` : Disables all modules, flushes iptables, and restores the system to its original state.
- `sudo anon --status` : Prints a detailed visual list of all active and inactive OPSEC modules.
- `sudo anon --dashboard` : Opens the live monitoring dashboard (refreshes every 3 seconds).
- `sudo anon --browser` : Directly launches the RAM-based disposable browser within the sandbox.
- `sudo anon --fix` : Restores system network configurations from backup in case of a critical failure or abrupt shutdown.
- `sudo anon --update` : Fetches and installs the latest version directly from the GitHub repository.

---

## ⚠️ Disclaimer & OPSEC Warning

ANON is provided for **educational, academic, and security research purposes only**. The developers assume no liability and are not responsible for any misuse, illegal activities, or damage caused by this program. 

**Anonymity is not a software product; it is a process and a mindset.** 
While ANON provides military-grade network and endpoint obfuscation, it cannot protect you from:
1. Operational Security (OPSEC) failures (e.g., logging into personal accounts).
2. Hardware-level implants or firmware backdoors.
3. Compromised exit nodes performing traffic manipulation (if HTTPS is not enforced).

Always combine software tools with strong operational security practices.

---

<div align="center">
  <b>Report Issues & Contribute</b><br>
  egcaem000@gmail.com | <a href="https://github.com/egnake/anon/issues">GitHub Issues</a>
</div>
