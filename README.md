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

---

## 📖 Table of Contents
- [Overview](#-overview)
- [Key Features & Modules](#-key-features--modules)
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

Beyond network routing, Anon actively hardens your machine's footprint by randomizing hardware identifiers (MAC addresses), preventing DNS leaks, wiping volatile memory (Anti Cold Boot), and thwarting Man-in-the-Middle (MITM) attacks.

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
6. **⏱ Timezone Changer** 
   Spoofs your system's timezone to UTC (or a random timezone). This mitigates timing-based fingerprinting attacks.
7. **🏷 Hostname Changer** 
   Temporarily changes your machine's hostname to a randomized string, masking your device's identity on local networks.
8. **🕵️ Browser Anonymization** 
   Injects secure, anti-fingerprinting configurations into supported browsers to prevent canvas fingerprinting and WebRTC leaks.
9. **❄ Anti Cold Boot** 
   Wipes remaining RAM contents (`sdmem`) upon system shutdown to protect encryption keys and sensitive data from Cold Boot physical attacks.
10. **🛑 Kill Switch** 
    In an emergency, instantly drops all `iptables` traffic rules, completely cutting off your machine from the internet.

---

## 📦 Prerequisites

Anon relies on several low-level network utilities. If you use the provided `make install` or `setup.bat`, these will be installed automatically:

- `tor` - The core routing network.
- `iptables` - For forcing transparent proxies.
- `macchanger` - For MAC address randomization.
- `network-manager` - For network interface controls.
- `secure-delete` - For the Log Killer and Anti Cold Boot modules.
- `python3` & `python3-scapy` - For the Anti-MITM engine.
- `curl`, `make`, `tar`

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
   - It will boot into WSL, install `tor`, `iptables`, and compile the tool.
3. Open a new Command Prompt or PowerShell and type:
```cmd
anon
```

### Docker (Containerized)
To run Anon safely inside an isolated container while routing your host's network:
```bash
# Build the image
docker build -t anon-tool .

# Run with required network privileges
docker run -it --rm --privileged --net=host anon-tool
```
*(⚠️ **Note:** `--privileged` and `--net=host` are strictly required. Without them, Anon cannot manipulate the host's `iptables` or MAC addresses).*

---

## 🎮 Usage

Anon features an interactive, user-friendly Command Line Interface.

### Interactive Menu
Just type `anon` to open the interface. Type the number of the module you want to toggle (e.g., `1` for Anti-MITM). Green `[ ✔ ON ]` indicates the module is active. Once configured, press `0` to apply settings and route your traffic.
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
