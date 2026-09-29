<div align="center">
  <img src="assets/anon_hero.jpg" alt="ANON Hero Banner" width="100%">
  
  <h1>ANON: Gelişmiş OPSEC ve Anonimlik Çerçevesi</h1>
  <p><b>Linux Sistemler için Hükümet Seviyesinde Anonimlik, Anti-Adli Bilişim ve Trafik Karartma Aracı</b></p>
  
  <p>
    <a href="https://github.com/egnake/anon/blob/main/LICENSE"><img src="https://img.shields.io/badge/Lisans-MIT-blue.svg?style=for-the-badge" alt="License"></a>
    <a href="https://github.com/egnake/anon/releases"><img src="https://img.shields.io/badge/Sürüm-3.0-green.svg?style=for-the-badge" alt="Version"></a>
    <img src="https://img.shields.io/badge/Platform-Linux%20(Debian%2FKali)-orange.svg?style=for-the-badge" alt="Platform">
    <img src="https://img.shields.io/badge/Dil-Bash_100%25-yellow.svg?style=for-the-badge" alt="Language">
  </p>
</div>

<br>

## 🚀 Genel Bakış

**ANON**, siber güvenlik araştırmacıları, gazeteciler ve aktivistler için tasarlanmış son teknoloji bir OPSEC (Operasyonel Güvenlik) ve anonimlik çerçevesidir (framework). Sadece basit bir Tor yönlendirmesi yapmakla kalmaz; **17 bağımsız modülü** sayesinde kesin ağ izolasyonu sağlar, sistem parmak izini değiştirir, trafik analizi saldırılarını (zamanlama saldırıları) engeller ve paranoyak düzeyde anti-adli bilişim (anti-forensics) tedbirleri alır.

Sıradan scriptlerin aksine ANON, **Whonix tarzı Kernel Ağ Yalıtımı (Kernel Network Namespace Isolation - `anon_jail`)** sunar. Bu sanal kum havuzu (sandbox), sıfır gün (0-day) tarayıcı açıklarına karşı koruma sağlar ve tüm ağın fiziksel olarak Tor'dan geçmesini zorunlu kılar.

---

## ⚡ Özellikler ve Modüller

ANON, üç temel taktiksel alana ayrılmış 17 yüksek düzeyde uzmanlaşmış modül sunar:

### 🌐 Ağ ve Yönlendirme (Network & Routing)
- **[01] Tor Bridges (obfs4):** İnternet Servis Sağlayıcı (ISS) engellemelerini ve Derin Paket İncelemesini (DPI) atlatmak için özel köprü düğümleri (bridges) kullanır.
- **[02] IP Changer (Tor TransPort):** Sistemdeki tüm TCP/DNS trafiğini şeffaf (transparent) bir şekilde Tor ağına yönlendirir.
- **[03] DNS Changer:** DNS çözümlemelerini doğrudan Tor'un güvenli DNS ağı üzerinden yaparak IP/DNS sızıntılarını kesin olarak engeller.
- **[04] MAC Changer:** Yerel ağda (LAN) izlenmeyi önlemek için ağ kartınızın (NIC) fiziksel MAC adresini rastgele değiştirir.
- **[05] Proxychains Integration:** Terminal komutlarınızı otomatik olarak Tor tünelinden geçirir.
- **[06] I2P Integration:** Trafiğinizi Görünmez İnternet Projesi (I2P) ağı üzerinden yönlendirmeyi destekler.
- **[07] Traffic Obfuscation (Zamanlama Saldırısı Koruması):** Linux `tc` (Traffic Control) modülünü kullanarak dışarı çıkan paketlere rastgele gecikme (jitter/latency) ekler ve arka planda sahte (dummy) Tor trafiği oluşturur. Bu sayede düşmanların paket boyutlarına ve zamanlamalarına bakarak hangi siteye girdiğinizi bulmasını (Website Fingerprinting) engeller.

### 🛡️ Sistem Gizleme ve İzolasyon (System Obfuscation & Isolation)
- **[08] Timezone Changer:** Konum tabanlı tarayıcı parmak izi (fingerprinting) izlemesini engellemek için sistem saat dilimini sahtesiyle (spoof) değiştirir.
- **[09] Hostname Changer:** Halka açık ağlarda (kafe vb.) bilgisayarınızın adını gizlemek için Hostname değerini rastgele bir isimle değiştirir.
- **[10] Browser Anonymization:** Doğrudan sisteminizin geçici hafızasında (RAM / `/dev/shm`) çalışan, son derece sertleştirilmiş, tek kullanımlık (disposable) bir Firefox profili oluşturur.
- **[11] OS Obfuscation (TTL Spoofing):** Giden paketlerin "Time-To-Live" (TTL) değerini manipüle ederek ağ tarayıcılarına işletim sisteminizi yanlış tanıtır (örneğin Linux kullanırken Windows gibi görünmenizi sağlar).
- **[12] Process Obfuscation:** Arka planda çalışan OPSEC script süreçlerini standart sistem izleme araçlarından (ps, top) gizler.
- **[13] Sandbox Isolation (Kernel netns):** İzole edilmiş bir kernel seviyesi ağ katmanı (`anon_jail`) yaratır. Buradan başlatılan uygulamalar asla gerçek ağ kartlarına erişemez. Ekstra güvenlik için **Firejail / AppArmor** süreç izolasyonu ile tam entegre çalışır.

### 🔥 Aktif Savunma ve Anti-Adli Bilişim (Active Defense & Anti-Forensics)
- **[14] Network Kill Switch:** `iptables` kurallarını kullanarak Tor ağına yönlendirilmeyen **TÜM** ağ paketlerini anında engeller (DROP). Cihazdan tek bir byte verinin doğrudan çıkması fiziksel olarak imkansız hale gelir.
- **[15] Anti-MITM (ARP Spoofing Protection):** ARP tablosunu sürekli izleyerek bulunduğunuz ağdaki Ortadaki Adam (Man-in-the-Middle) saldırılarını tespit edip engeller.
- **[16] Anti-Cold Boot (sdmem):** Kapatma (shutdown) işleminden önce bilgisayarınızın RAM ve Swap bölümlerini kriptografik olarak (shredding) silerek "Cold-Boot" adli bilişim saldırılarını engeller.
- **[17] Bluetooth Disabler:** Kablosuz izlenmeyi ve Blueborne gibi saldırıları engellemek için `rfkill` üzerinden tüm Bluetooth radyolarını donanımsal seviyede kapatır.

---

## 🖥️ Etkileşimli Kullanıcı Arayüzü (TUI)

ANON v3.0, doğrudan Bash içerisinde kodlanmış, yön tuşlarıyla yönetilebilen tamamen yeni bir Arayüz (TUI) ile gelir.

- **Live Dashboard (Canlı Panel):** Tor ağ durumunu, çıkış IP'nizi ve aktif modülleri gerçek zamanlı izleyin.
- **Operations Menu (Operasyon Menüsü):** IP Sızıntı testleri yapın, yeni bir Tor Devresi (Exit Node) isteyin veya Tek Kullanımlık Tarayıcıyı tek tuşla başlatın.
- **Dinamik ASCII Bannerlar:** Terminalinizin boyutuna tam uyum sağlayan ve her açılışta değişen dinamik hacker temalı arayüz grafikleri.

---

## ⚙️ Kurulum

ANON, Debian tabanlı bir Linux dağıtımı gerektirir (Kali Linux, Parrot OS, Ubuntu, Debian veya WSL2).

```bash
git clone https://github.com/egnake/anon.git
cd anon
sudo chmod +x setup.sh
sudo ./setup.sh
```

## 🛠️ Kullanım

Etkileşimli OPSEC çerçevesini başlatmak için:
```bash
sudo anon
```

**CLI Parametreleri:**
- `sudo anon --start` : Kaydedilen son profili kullanarak modülleri başlatır.
- `sudo anon --stop` : Tüm modülleri devre dışı bırakarak sistemi eski, normal haline geri döndürür.
- `sudo anon --status` : Çalışan modüllerin mevcut durumunu yazdırır.
- `sudo anon --dashboard` : Gerçek zamanlı ağ izleme panelini açar.
- `sudo anon --browser` : Sadece RAM üzerinde çalışan tek kullanımlık güvenli tarayıcıyı başlatır.
- `sudo anon --update` : Aracın en güncel sürümünü GitHub üzerinden otomatik indirip kurar.

---

## ⚠️ Sorumluluk Reddi (Disclaimer)

ANON **sadece eğitim ve güvenlik araştırmaları amacıyla** geliştirilmiştir. Geliştiriciler, bu programın yasa dışı kullanımından veya yol açabileceği herhangi bir zarardan sorumlu tutulamaz. Anonimlik sadece bir yazılım değil, bir süreçtir. OPSEC (Operasyonel Güvenlik) kurallarınızı mutlaka sağlam tutun.

**Hata Bildirimi (Report Issues):** egcaem000@gmail.com | [GitHub Issues](https://github.com/egnake/anon/issues)
