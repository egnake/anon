<div align="center">
  <img src="assets/icons/anon.jpg" alt="Anon Icon" width="128"/>
  <h1>Anon - The Ultimate Privacy & Anonymity Framework</h1>
  <p><b>An advanced, all-in-one operational security (OPSEC) utility designed for ethical hackers, researchers, and privacy advocates.</b></p>
  
  <p>
    <a href="https://github.com/egnake/anon/releases"><img src="https://img.shields.io/github/v/release/egnake/anon?color=blue&style=flat-square" alt="Release"></a>
    <img src="https://img.shields.io/badge/Platform-Linux%20%7C%20WSL-blue.svg?style=flat-square&logo=linux" alt="Platform">
    <img src="https://img.shields.io/badge/Language-Bash-green.svg?style=flat-square&logo=gnu-bash" alt="Bash">
    <img src="https://img.shields.io/github/license/egnake/anon?style=flat-square&color=orange" alt="License">
    <img src="https://img.shields.io/github/stars/egnake/anon?style=flat-square&color=yellow" alt="Stars">
  </p>
</div>

---

<div align="center">
  <img src="assets/preview.png" alt="Anon CLI Menu" width="700"/>
</div>

<div align="center">
  <img src="assets/dashboard.png" alt="Anon Live Dashboard" width="700"/>
  <br><i>Real-time monitoring of Tor routing status and network traffic.</i>
</div>

---

## 📖 Table of Contents
- [Overview](#-overview)
- [Key Features & Modules](#-key-features--modules)
- [Advanced OPSEC Hardening](#-advanced-opsec-hardening)
- [Prerequisites](#-prerequisites)
- [Installation](#-installation)
  - [Kali Linux / Debian / Ubuntu](#kali-linux--debian--ubuntu)
  - [Windows (via WSL 2)](#windows-via-wsl-2)
  - [Docker (Containerized)](#docker-containerized)
- [Usage](#-usage)
- [Under the Hood](#-under-the-hood)
- [Troubleshooting](#-troubleshooting)
- [Disclaimer](#-disclaimer)
- [License](#-license)

---

## 📌 Overview

In today's digital landscape, relying solely on a VPN is often insufficient for true operational security. **Anon** is a powerful Bash-based framework that forcibly routes your entire system's network traffic through the **Tor network** using strict `iptables` rules. 

Beyond network routing, Anon actively hardens your machine's footprint by randomizing hardware identifiers (MAC addresses), blocking hardware tracking (Bluetooth), wiping volatile memory (Anti Cold Boot), spoofing OS fingerprints, and thwarting Man-in-the-Middle (MITM) attacks.

---

## ✨ Key Features & Modules

Anon is modular. You can toggle any combination of the following security layers:

1. **🌐 Anti-MITM (Man-in-the-Middle)** 
   Detects and blocks ARP spoofing and malicious packet injections on your local network.
2. **🧹 Log Killer** 
   Uses `secure-delete` (`srm`) to overwrite and completely obliterate system logs (`/var/log/*`), ensuring no traces of your activities remain on the disk.
3. **🎭 IP Changer (Transparent Proxy)** 
   Configures `iptables` to drop all non-Tor traffic. Forces all TCP/UDP connections through Tor's transparent proxy, effectively masking your real IP address.
4. **📡 DNS Changer** 
   Prevents DNS leaks by modifying `/etc/resolv.conf` and forcing DNS resolution through Tor's isolated DNS port (Port 5300).
5. **📱 MAC Changer** 
   Randomizes the physical MAC addresses of all active Network Interface Cards (NICs) to prevent device tracking on local networks (e.g., public Wi-Fi).
6. **🏷️ Hostname Changer** 
   Temporarily changes your machine's hostname to a randomized string, masking your device's identity on local networks.
7. **🕵️ Browser Anonymization** 
   Injects secure, anti-fingerprinting configurations into supported browsers to prevent Canvas fingerprinting and WebRTC IP leaks.
8. **❄️ Anti Cold Boot** 
   Wipes remaining RAM contents (`sdmem`) upon system shutdown to protect encryption keys and sensitive data from Cold Boot physical attacks.
9. **🛑 Kill Switch** 
    In an emergency, instantly drops all `iptables` traffic rules, completely cutting off your machine from the internet.

---

## 🛡️ Advanced OPSEC Hardening

These modules elevate Anon from a simple proxy router to a state-of-the-art OPSEC utility designed to counter deep-packet inspection and advanced tracking:

10. **🌉 Tor Bridges (Obfs4)**
    Bypasses Tor censorship (e.g., in China or corporate networks) by obfuscating your Tor traffic as random, meaningless data using obfs4 pluggable transports.
    > **Note:** If you live in a censored region, you MUST use bridges. To get them, visit [bridges.torproject.org/options](https://bridges.torproject.org/options), select **obfs4**, get the bridges (ensure they are IPv4, not IPv6), and paste them into the Anon terminal when prompted. Anon will automatically remember your bridges for future sessions.
11. **💻 OS Obfuscation (TCP/IP Fingerprinting)**
    Modifies kernel `sysctl` parameters (such as TTL and TCP Window sizes) so that network scanners (like Nmap) identify your Linux machine as a generic Windows 10 client.
12. **📴 Bluetooth Disabler (Beacon Tracking Protection)**
    Actively blocks all Bluetooth receivers and transmitters using `rfkill`, preventing passive beacon tracking and Bluejacking attacks.
13. **👻 Process Obfuscation**
    Spoofs the process names of common hacking tools (e.g., `nmap`, `sqlmap`, `tor`) by wrapping them in `exec -a "[kworker/u4:2]"`. They will appear as benign system background tasks in the process tree, hiding them from local malware or monitoring.
14. **⛓️ Proxychains-ng Integration**
    Automatically configures `proxychains4.conf` to hook into Tor's Socks5 port (9050). This forces stubborn tools that ignore system proxies to route securely through the Tor network.
15. **⏳ Fake Timezone Synchronization**
    Defeats time-based fingerprinting by randomly selecting a global timezone (e.g., `Asia/Tokyo`, `America/New_York`) instead of just defaulting to UTC.

---

## 📦 Prerequisites

Anon features an **Interactive Dependency Checker**. If any core packages are missing upon launch, Anon will automatically prompt to install them for you using `apt-get`. Key dependencies include:

- `tor`, `obfs4proxy`, `proxychains4`
- `iptables`, `macchanger`, `network-manager`, `rfkill`
- `secure-delete`, `python3-scapy`, `curl`

---

## 🚀 Installation

### Kali Linux / Debian / Ubuntu
```bash
# Clone the repository
git clone https://github.com/egnake/anon.git
cd anon

# Install dependencies and global shortcut
sudo make install

# Run the tool
sudo anon
```

### Windows (via WSL 2)
Anon fully supports Windows environments by leveraging the Windows Subsystem for Linux (WSL).
1. Ensure **WSL 2** is installed.
2. Run the `setup.bat` file as a standard user.
   - It will automatically set up your global Windows `PATH`.
   - It will boot into WSL, install dependencies, and compile the tool.
3. Open a new Command Prompt or PowerShell and type:
```cmd
anon
```

---

## 🎮 Usage

Anon features an interactive, user-friendly Command Line Interface.

### Interactive Menu
Just type `anon` to open the interface. Type the number of the module you want to toggle. Green `[ ✔ ON ]` indicates the module is active. Once configured, press `0` to apply settings and route your traffic.
```bash
sudo anon
```

### Quick Commands
You can bypass the menu for rapid execution:
```bash
# Immediately start Anon with your last saved configuration
sudo anon --start

# Stop all anonymization and restore default network settings
sudo anon --stop

# Show system status and active modules
sudo anon --status

# Open the Live Real-time Dashboard
sudo anon --dashboard

# Launch a completely RAM-based disposable browser (Zero-Trace)
sudo anon --browser

# Show help menu
anon --help
```

---

## 🔧 Under the Hood
When you trigger the **IP Changer**, Anon does not just set a local proxy. It actively flushes your current `iptables` and establishes a **Transparent Tor Proxy**. It sets up `DNAT` rules to redirect DNS requests to Tor's DNS port, and uses `REDIRECT` rules for all TCP traffic to Tor's `TransPort`. Any traffic that bypasses Tor is strictly `DROPPED`, ensuring zero leakage even if an application ignores proxy settings.

---

## ⚠️ Disclaimer
**Anon is built for educational, research, and ethical hacking purposes only.** 
The developers and contributors of this project do not endorse, encourage, or support any illegal activities. You are solely responsible for your actions and ensuring that your use of this tool complies with all local, state, and federal laws.

---

## 📄 License
This project is licensed under the [MIT License](LICENSE).
