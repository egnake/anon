<div align="center">
  <img src="assets/anon_logo.jpg" alt="ANON Logo" width="200" style="border-radius: 20px;">
  
  <h1>ANON: Advanced OPSEC & Anonymity Framework</h1>
  <p><b>A comprehensive, modular anonymity and anti-forensics toolkit for Linux environments.</b></p>
  
  <p>
    <a href="https://github.com/egnake/anon/blob/main/LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg?style=for-the-badge" alt="License"></a>
    <a href="https://github.com/egnake/anon/releases"><img src="https://img.shields.io/badge/Version-3.0-green.svg?style=for-the-badge" alt="Version"></a>
    <img src="https://img.shields.io/badge/Platform-Linux%20(Debian%2FKali)-orange.svg?style=for-the-badge" alt="Platform">
    <img src="https://img.shields.io/badge/Bash-100%25-yellow.svg?style=for-the-badge" alt="Language">
    <img src="https://img.shields.io/badge/Architecture-Modular-purple.svg?style=for-the-badge" alt="Architecture">
  </p>

  <h3>
    <a href="README.md">🇺🇸 English</a> | 
    <a href="README-TR.md">🇹🇷 Türkçe</a>
  </h3>
</div>

<br>

## 🚀 Overview

**ANON** is a rigorous, open-source anonymity and Operations Security (OPSEC) framework designed for researchers, developers, and privacy advocates. Moving beyond basic proxy scripts, ANON provides a deeply integrated, 17-module architecture that enforces strict network isolation, randomizes system fingerprints, and applies active anti-forensics measures.

By utilizing Linux kernel namespaces (`netns`), traffic control (`tc`), and dynamic `iptables` configurations, ANON transforms a standard Debian-based installation into a highly restricted environment. It ensures that all outbound traffic is strictly routed through the Tor network while mitigating common timing attacks and local network threats.

---

## 🛡️ Threat Model & Philosophy

ANON is built around the concept of "Defense in Depth." It assumes that adversaries may exist at multiple points in your communication chain. The framework attempts to mitigate the following specific vectors:

1. **Traffic Analysis & ISP Monitoring:** Mitigated by forcing all TCP and DNS traffic through Tor, while utilizing `obfs4` bridges and active traffic obfuscation (dummy packets and jitter) to disrupt Website Fingerprinting (WFP).
2. **Local Area Network (LAN) Attacks:** Mitigated via MAC address randomization, ARP spoofing detection, hostname spoofing, and strict isolation from the physical host interfaces.
3. **Endpoint Vulnerabilities & Forensics:** Mitigated through the use of isolated kernel network namespaces (`anon_jail`), temporary RAM-based filesystems for browsers, and cryptographic memory wiping (`sdmem`) to counter cold-boot data extraction.

---

## ⚙️ The 17 Modules (Detailed Breakdown)

ANON is highly modular. You can toggle individual features on or off based on your specific operational requirements using the interactive Terminal User Interface (TUI).

### 🌐 Network & Routing Subsystem

**[01] Tor Bridges (obfs4)**
Standard Tor traffic is easily detectable via Deep Packet Inspection (DPI) by ISPs and firewalls. This module configures the Tor daemon to utilize `obfs4` bridges. By doing so, it encrypts the Tor handshake and alters the traffic signature, making it indistinguishable from random noise, which is essential for bypassing strict censorship.

**[02] IP Changer (Tor TransPort)**
Relying solely on SOCKS5 proxies can lead to fatal data leaks if an application is misconfigured. This module leverages strict `iptables` `REDIRECT` rules at the kernel level. It captures all outbound TCP traffic originating from your machine and physically forces it into Tor's transparent proxy port (9040), ensuring absolute compliance regardless of application settings.

**[03] DNS Changer (Tor DNSPort)**
DNS requests (resolving domains to IPs) often leak outside of proxy tunnels. This module captures all UDP/TCP port 53 traffic and redirects it to Tor's secure DNSPort (5353). This guarantees that your local ISP cannot build a history of the websites you visit.

**[04] MAC Changer**
Before a network connection is fully established, this module alters the Media Access Control (MAC) address of your Network Interface Card (NIC) to a completely randomized value. This breaks historical tracking by routers, public Wi-Fi hotspots, and local network administrators.

**[05] Proxychains Integration**
Automatically configures and aligns `proxychains4` with your active Tor circuits. This allows you to effortlessly tunnel specific terminal commands, scripts, or external tools (such as `curl`, `wget`, or custom python scripts) through the Tor network without manual configuration tweaking.

**[06] I2P Integration**
Provides an alternative routing mechanism. For tasks that require interaction with the Invisible Internet Project (I2P) network rather than the Tor network, this module bridges the gap, allowing secure access to decentralized eepsites.

**[07] Traffic Obfuscation (Timing Attack Defense)**
Low-latency networks like Tor remain susceptible to end-to-end timing correlation and Website Fingerprinting (WFP). This advanced module utilizes the Linux Traffic Control utility (`tc`) to inject randomized latency (jitter) into your outbound packets. Furthermore, it generates continuous, randomized background Tor traffic (chaffing) to obscure your actual data volume, heavily complicating packet-size and timing analysis.

### 🧩 System Obfuscation & Isolation

**[08] Timezone Changer**
Modern web tracking often correlates your system's local timezone (leaked via JavaScript or system headers) with your IP address. This module temporarily spoofs your system timezone to random global coordinates (e.g., UTC or Asia/Tokyo), disrupting correlation algorithms.

**[09] Hostname Changer**
Your computer's hostname (e.g., `user-laptop`) is frequently broadcasted across local networks via DHCP requests. This module randomizes the hostname to generic, non-identifiable strings to blend into crowded networks seamlessly.

**[10] Browser Anonymization (Disposable RAM Profile)**
Rather than launching a standard browser profile that saves history, cookies, and cache to your hard drive, ANON creates a highly hardened Firefox configuration (`anon.js`) that disables WebRTC, WebGL, and Canvas fingerprinting. It mounts this profile entirely in a volatile memory filesystem (`/dev/shm`). Once the browser process terminates, the profile is instantly annihilated, leaving zero forensic trace on permanent storage.

**[11] OS Obfuscation (TTL Spoofing)**
Passive network scanners (such as `p0f` or `nmap`) can guess your Operating System by analyzing the default Time-To-Live (TTL) values of your network packets (Linux typically uses 64, Windows 128). This module intercepts outbound packets and rewrites their TTL values to mimic different operating systems, frustrating OS fingerprinting attempts.

**[12] Process Obfuscation**
Executes techniques to hide ANON's background scripts and OPSEC processes from standard user-space monitoring tools (like `ps` or `top`). This acts as a secondary layer of defense against non-privileged malicious scripts trying to profile the machine's defensive capabilities.

**[13] Sandbox Isolation (Kernel netns)**
Mimicking advanced isolation architectures, this module creates an isolated Kernel Network Namespace (`anon_jail`). It provisions a virtual ethernet pair (`veth`), routing all traffic from within the namespace explicitly through the host's Tor TransPort. Applications launched inside this sandbox (such as the Disposable Browser) have absolutely zero visibility into the host's real network interfaces, real IP, or MAC address. It integrates with Firejail/AppArmor for robust process isolation, containing potential 0-day browser exploits.

### 🔥 Active Defense & Anti-Forensics

**[14] Network Kill Switch**
If the Tor daemon crashes, unexpected reboots occur, or bridges fail, operating systems usually default to routing traffic over the clear web. The Kill Switch deploys draconian `iptables` rules that explicitly `DROP` all packets that are not destined for the loopback interface (`lo`) or established Tor ports. If Tor goes down, the entire internet connection goes down. There are no exceptions and no leaks.

**[15] Anti-MITM (ARP Spoofing Protection)**
Continuously monitors the local Address Resolution Protocol (ARP) tables. If an attacker on the same local network attempts to intercept traffic via an ARP Spoofing/Poisoning attack, this module detects the MAC address anomaly, instantly alerts the user, and severs the connection to prevent data interception.

**[16] Anti-Cold Boot (sdmem)**
During physical device seizure, adversaries can extract encryption keys and sensitive session tokens by freezing RAM modules (Cold-Boot Attack). When enabled, exiting the ANON framework triggers `sdmem` (Secure Delete Memory), which cryptographically overwrites all free RAM and Swap space with random data before the system shuts down.

**[17] Bluetooth Disabler**
Bluetooth radios broadcast unique hardware identifiers and are historically vulnerable to proximity-based exploits (such as Blueborne). This module interfaces with `rfkill` to implement a strict hardware-level block on all Bluetooth controllers.

---

## 🖥️ Interactive Terminal User Interface (TUI)

ANON completely replaces clunky command-line arguments with an elegant, responsive TUI built entirely in Bash.

* **Dynamic Aesthetic Rendering:** The TUI features beautifully formatted, centered ASCII banners that change dynamically on launch, providing a clean, distraction-free environment.
* **Live Dashboard:** An integrated dashboard allows you to monitor your Tor exit IP, active circuit status, network configuration, and active OPSEC modules in real-time.
* **Operations Hub:** Execute DNS/IP leak tests, request a new Tor identity (`SIGHUP` circuit rebuild), or launch the sandboxed disposable browser directly from the interface using your arrow keys.

---

## 📦 System Requirements & Dependencies

ANON is heavily integrated with the Linux networking stack. It requires:

- **Operating System:** Kali Linux, Parrot Security OS, Debian, Ubuntu, or WSL2 (Custom Kernel recommended for netns support).
- **Permissions:** Root privileges (`sudo`) are required to manipulate `iptables`, kernel namespaces, and network interfaces.
- **Dependencies (Automatically installed):** `tor`, `obfs4proxy`, `macchanger`, `proxychains4`, `secure-delete` (sdmem), `rfkill`, `firejail`, `iproute2`.

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

*The setup script will update your package manager, install all required dependencies, place the `anon` executable in your `/usr/bin/` path, and create necessary backup directories.*

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
| `sudo anon --start` | Silently starts the framework in the background using the last saved module configuration. |
| `sudo anon --stop` | Immediately disables all active modules, flushes iptables, and restores the original network state. |
| `sudo anon --status` | Prints a comprehensive visual report of all active and inactive OPSEC modules. |
| `sudo anon --dashboard` | Opens the live monitoring dashboard (auto-refreshes every 3 seconds). |
| `sudo anon --browser` | Directly launches the RAM-based disposable browser inside the isolated network namespace. |
| `sudo anon --fix` | Restores default system network configurations from backup in the event of a crash or abrupt shutdown. |
| `sudo anon --update` | Fetches the latest source code from GitHub and reinstalls the framework automatically. |

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

**Q: Will this hide me from the NSA or state-level adversaries?**
A: No software provides absolute immunity. ANON mitigates specific technical vectors (DPI, DNS leaks, OS fingerprinting, local forensics). However, a state-level adversary relies heavily on global traffic correlation, zero-day endpoint exploits, and physical surveillance. Use ANON as one layer in a comprehensive operational security strategy.

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
