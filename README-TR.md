<div align="center">
  <img src="assets/anon_logo_iconic.jpg" alt="ANON Logo" width="220">
  
  <h1>ANON: Gelişmiş OPSEC ve Anonimlik Çerçevesi</h1>
  <p><b>Yerel Rust ile geliştirilmiş sıfır bağımlılıklı, yüksek performanslı Linux Operasyonel Güvenlik (OPSEC), adli bilişim karşıtı (anti-forensics) ve gizlilik motoru.</b></p>
  
  <p>
    <a href="https://github.com/egnake/anon/blob/main/LICENSE"><img src="https://img.shields.io/badge/Lisans-MIT-blue.svg?style=for-the-badge" alt="License"></a>
    <a href="https://github.com/egnake/anon/releases"><img src="https://img.shields.io/badge/S%C3%BCr%C3%BCm-3.0.0-green.svg?style=for-the-badge" alt="Version"></a>
    <img src="https://img.shields.io/badge/Platform-Debian%20%7C%20Kali%20%7C%20Parrot%20%7C%20WSL2-orange.svg?style=for-the-badge" alt="Platform">
    <img src="https://img.shields.io/badge/Rust-S%C4%B1f%C4%B1r_Harici_Ba%C4%9F%C4%B1ml%C4%B1l%C4%B1k-yellow.svg?style=for-the-badge" alt="Language">
    <img src="https://img.shields.io/badge/Mimari-22_Mod%C3%BCl-purple.svg?style=for-the-badge" alt="Architecture">
    <img src="https://img.shields.io/badge/Dil_Deste%C4%9Fi-English%20%7C%20T%C3%BCrk%C3%A7e-red.svg?style=for-the-badge" alt="i18n">
  </p>

  <h3>
    <a href="README.md">🇺🇸 English</a> | 
    <a href="README-TR.md">🇹🇷 Türkçe</a>
  </h3>
</div>

<br>

## 🚀 Genel Bakış

**ANON (v3.0)**, güvenlik araştırmacıları, sızma testi uzmanları, araştırmacı gazeteciler ve gizlilik savunucuları için **sıfır harici bağımlılıklı yerel Rust ikili dosyası (native binary)** olarak sıfırdan tasarlanmış kurumsal düzeyde bir operasyonel güvenlik (OPSEC) ve adli bilişim karşıtı koruma çerçevesidir. Debian, Kali Linux, Parrot OS, Ubuntu ve Windows 10/11 (WSL2) sistemlerini katı bir şekilde yalıtılmış, sıkılaştırılmış anonim çalışma ortamlarına dönüştürür.

ANON; aktif ağ gözetimi, İSS derin paket incelemesi (DPI), yerel ağ saldırıları ve Cold-Boot fiziksel bellek analizlerine karşı tam gizlilik sağlamak için **22 durumlu güvenlik modülünü**, olay güdümlü **Netlink ve inotify çekirdek gözcülerini**, donanım engellemelerini, paralel çok iş parçacıklı metaveri temizliğini ve tamamen uçucu RAM tabanlı çalışmayı bir araya getirir.

---

## ⚡ Öne Çıkan Özellikler (v3.0)

- **Saf Yerel Rust Motoru (`src/`):** Harici crate bağımlılığı yoktur (`Cargo.toml` yalnızca standart kütüphane `std` ve `src/sys.rs` içindeki izole POSIX/Linux libc FFI arayüzüne dayanır). Tam LTO optimizasyonu ile ultra hızlı derlenir ve minimum ikili dosya boyutu sunar.
- **Çift Dil Desteği (`English` & `Türkçe`):** Varsayılan olarak İngilizce başlar; doğrudan etkileşimli TUI menüsünden veya `--lang tr` / `--lang en` parametresiyle tüm arayüz anında Türkçe veya İngilizceye geçirilebilir. Tercih, `--stop` veya `--panic` sıfırlamalarından bağımsız olarak `${BACKUPDIR}/anon.lang` dosyasında kalıcı tutulur.
- **Titreşimsiz (Flicker-Free) Etkileşimli TUI:** Terminal boyutlarına (`LINES` x `COLUMNS`) dinamik uyum sağlayan çift arabellekli UTF-8 çerçeve çizimi, tam ekran sıfırlamaları (`\x1b[2J`) olmadan pürüzsüz yön tuşu / Vim (`j`/`k`) navigasyonu ve 3 saniyede bir otomatik yenilenen canlı kontrol paneli.
- **Bildirimsel Güvenlik Profilleri:** Tek komutla hazır güvenlik profillerini uygulayın: `--profile <isim>` (`paranoia`, `quick`, `pentest`, `darknet`, `lockdown`, `ghost`).
- **Atomik Güvenlik Duvarı Yönlendirmesi:** Geçişler sırasında milisaniyelik paket sızıntılarını bile önleyen çekirdek düzeyinde `iptables-restore` / `ip6tables-restore` ağ kilidi (Kill Switch).
- **Çekirdek Ağ Ad Alanı İzolasyonu (`anon_jail`):** Linux çekirdeği `netns`, sanal ethernet çiftleri (`veth`), AppArmor MAC profili ve Firejail seccomp koruması ile donanım düzeyinde süreç ve ağ ayrımı.
- **Yalnızca RAM Üzerinde Uçucu Çalışma:** `/dev/shm` içerisinde iz bırakmayan tarayıcı oturumları, `tmpfs` tabanlı geçici günlük (log) katmanı ve `[kworker/u4:2]` çekirdek iş parçacığı imzasıyla süreç maskeleme.
- **Çoklu Platform Uyumu:** Linux için tam otomatik kurucu (`setup.sh`), Windows WSL2 için otomatik yol çeviricili sarmalayıcı (`setup.bat` ve `anon.bat`) ve çok aşamalı Kali Rolling `Dockerfile`.

---

## 🛡️ Tehdit Modeli ve Derinlemesine Savunma

ANON; saldırganların ağ trafiğini izleyebileceğini, yerel ağda ARP zehirlenmesi yapabileceğini, paket zamanlamasını analiz edebileceğini veya cihaza fiziksel erişim sağlayabileceğini varsayar. Tüm saldırı yüzeylerinde katmanlı savunma uygular:

```
┌────────────────────────────────────────────────────────────────────────┐
│                        ANON SAVUNMA MODELİ                             │
├────────────────────────┬───────────────────────────────────────────────┤
│ Saldırı Vektörü        │ ANON Savunma Tekniği                          │
├────────────────────────┼───────────────────────────────────────────────┤
│ İSS / DPI Gözetimi ve  │ Tor TransPort (9040), obfs4, Snowflake WebRTC,│
│ Trafik Analizi         │ Sahte Trafik (Chaffing), Jitter (tc netem)    │
├────────────────────────┼───────────────────────────────────────────────┤
│ Yerel Ağ Tehditleri    │ Rastgele MAC (OUI), Anti-MITM (Netlink ARP),  │
│ (Wi-Fi / LAN / DHCP)   │ Hostname Rastgeleleştirici, RFKill Bluetooth  │
├────────────────────────┼───────────────────────────────────────────────┤
│ İşletim Sistemi ve     │ TTL=128 Windows 10 Taklidi, Saat Dilimi,      │
│ Tarayıcı Parmak İzi    │ Sıkılaştırılmış anon.js, Anti-Canvas/WebGL   │
├────────────────────────┼───────────────────────────────────────────────┤
│ Sistem / RAM Adli      │ Kernel netns Jail, hidepid=2, Uçucu tmpfs     │
│ Analizi ve El Koyma    │ loglama, sdmem RAM silme, --panic protokolü   │
└────────────────────────┴───────────────────────────────────────────────┘
```

---

## ⚙️ 22 OPSEC Modülü ve Çekirdek Araçlar

### 🌐 Ağ ve Anonimlik Alt Sistemi

| # | Modül Adı | Açıklama |
| :-: | :--- | :--- |
| **01** | **Tor Köprüleri (`tor_bridges`)** | Tor trafiğini `obfs4`, `webtunnel` veya özel köprü protokolleriyle gizleyerek İSS sansürünü aşar. Etkileşimli köprü yapıştırma ve kalıcı yedekleme içerir. |
| **02** | **IP Değiştirici (`ip_changer`)** | Çıkan tüm TCP paketlerini atomik `iptables` kurallarıyla Tor şeffaf vekil sunucusuna (`TransPort` 9040) ve DNS sorgularını `DNSPort` (5353) portuna zorlar. |
| **03** | **DNS Değiştirici (`dns_changer`)** | Tor TransPort aktif değilken sistem DNS çözümlemesini kayıt tutmayan, sansürsüz güvenli sağlayıcılara (Mullvad, Quad9, DNS.WATCH, UncensoredDNS) kilitler. |
| **04** | **DoH Kalkanı (`doh_shield`)** | Yerel `dnscrypt-proxy` servisi üzerinden şifreli DNS-over-HTTPS / DNSCrypt koruması kurarak DNS manipülasyonunu ve dinlemeyi engeller. |
| **05** | **Ağ Kilidi (`kill_switch`)** | Atomik güvenlik duvarı bariyeri (`iptables-restore` & `ip6tables-restore`) kurar. Loopback ve Tor/I2P soketleri dışındaki tüm trafiği kesin olarak düşürür. |
| **06** | **DPI Atlama (`dpi_spoofing`)** | Tor Snowflake aracılığıyla trafiği geçici WebRTC bağlantıları içine kapsüller; kısıtlı trafiği sıradan eşler arası (P2P) görüntülü görüşme gibi gösterir. |
| **07** | **I2P Entegrasyonu (`i2p_integration`)** | `i2pd` Görünmez İnternet Projesi yönlendiricisini yönetir; merkezi olmayan `.i2p` siteleri için yerel SOCKS5 (`127.0.0.1:4446`) ve HTTP (`127.0.0.1:4444`) proxy sağlar. |
| **08** | **Proxychains Entegrasyonu (`proxychains_integration`)** | `/etc/proxychains4.conf` dosyasını sessiz modda doğrudan aktif Tor SOCKS5 dinleyicisine bağlanacak şekilde otomatik yapılandırır. |
| **09** | **MAC Değiştirici (`mac_changer`)** | Fiziksel ağ arayüzlerinin MAC adreslerini gerçekçi OEM/üretici OUI değerleriyle değiştirir; sanal makine ve kablosuz arayüz uyumluluğunu korur. |

### 🧩 Sistem Gizleme ve İzolasyon

| # | Modül Adı | Açıklama |
| :-: | :--- | :--- |
| **10** | **Saat Dilimi Değiştirici (`timezone_changer`)** | JavaScript saat dilimi korelasyonunu bozmak için `/etc/localtime` ve `/etc/timezone` ayarlarını rastgele küresel koordinatlara (UTC, Tokyo, Londra vb.) taşır. |
| **11** | **Cihaz Adı Değiştirici (`hostname_changer`)** | DHCP yayınlarında sızan cihaz adını (`/etc/hostname`, `/etc/hosts`, `dhclient.conf`) jenerik Windows/OEM adlarına (`DESKTOP-*`, `LAPTOP-*`, `WIN-*`) dönüştürür. |
| **12** | **Tarayıcı Anonimleştirme (`browser_anonymization`)** | Kurulu Firefox, Firefox-ESR ve LibreWolf dizinlerine WebRTC, Canvas telemetrisi ve disk önbelleğini kapatan sıkılaştırılmış `anon.js` ayarlarını uygular. |
| **13** | **İşletim Sistemi İz Gizleme (`os_obfuscation`)** | Pasif ağ analizlerini (`p0f`/`nmap`) yanıltmak için Windows 10/11 TCP/IP parmak izini taklit eder: çıkış TTL değerini 128 yapar, TCP zaman damgalarını kapatır. |
| **14** | **Süreç Gizleme (`process_obfuscation`)** | `/proc` dosya sistemini `hidepid=2` ile yeniden bağlayarak yetkisiz kullanıcıların çalışan süreçleri görmesini engeller ve hassas araçları RAM'den çalıştırır. |
| **15** | **Sandbox İzolasyonu (`sandbox_isolation`)** | Çekirdek düzeyinde izole `anon_jail` ağ ad alanı, sanal `veth` köprüsü (`10.200.200.0/24`), `/usr/bin/tor` için AppArmor profili ve Firejail seccomp kısıtlaması kurar. |
| **16** | **Çalışma Gizliliği (`execution_stealth`)** | Aracı geçici RAM belleği (`/dev/shm`) üzerinden dosyasız çalıştırır; süreç adını ve argümanlarını çekirdek iş parçacığı olarak (`[kworker/u4:2]`) maskeler. |
| **17** | **Trafik Karartma (`traffic_obfuscation`)** | Linux `tc netem` ile paketlere yapay gecikme/titreme (jitter) ekleyerek ve arka planda sahte Tor trafiği üreterek Website Fingerprinting (WFP) saldırılarını bozar. |

### 🔥 Aktif Savunma ve Adli Bilişim Karşıtı Koruma

| # | Modül Adı | Açıklama |
| :-: | :--- | :--- |
| **18** | **ARP/MITM Koruması (`anti_mitm`)** | Olay güdümlü Netlink `RTMGRP_NEIGH` gözcüsü ve yoklama motoru ile ARP önbellek zehirlenmesini yakalar; saldırı anında ağ arayüzünü kapatıp saldırgan MAC'i engeller. |
| **19** | **Sızıntı Denetçisi (`leak_auditor`)** | `/etc/resolv.conf`, Tor devre sağlığı ve IPv6 rotalarını `inotify`/Netlink ile izler. Açık ağ sızıntısı tespitinde anında ağı kilitler ve uyarı verir. |
| **20** | **Uçucu RAM Günlükleri (`volatile_logging`)** | Tails OS yaklaşımı: `/var/log` üzerine geçici RAM (`tmpfs`) bağlar. Oturum sonlandığında geçmiş sistem kayıtları şüpheli silme izi olmadan geri yüklenir. |
| **21** | **Bluetooth Devre Dışı (`bluetooth_disabler`)** | Linux `rfkill` ile tüm Bluetooth donanımını alt seviyede bloke ederek yakınlık tabanlı izleme ve saldırı vektörlerini kapatır. |
| **22** | **Cold-Boot Bellek Kilidi (`anti_cold_boot`)** | Takas alanını (swap) kapatır, core dump çıktılarını engeller ve bellek kurtarma saldırılarına karşı `sdmem` veya çok iş parçacıklı Rust sıfırlaması ile RAM'i temizler. |

### 🛠️ Yerleşik Operasyonlar ve Yardımcı Araçlar

- **🌐 Tek Kullanımlık Tarayıcı (`sudo anon --browser`):** Profilini tamamen `/dev/shm` RAM diskinde oluşturan ve `anon_jail` içinden Tor'a yönlendirilen Firefox oturumu başlatır. Pencere kapatıldığı an tüm profil kriptografik olarak silinir (`shred`).
- **🧹 Paralel Metaveri Temizleyici (`sudo anon --clean-meta <yol>`):** `MAT2` (Tails OS standardı) ve `ExifTool` yedeklemesi kullanan çok iş parçacıklı adli temizleyici. Görsellerden, PDF'lerden ve belgelerden EXIF, GPS ve cihaz izlerini topluca kazır.
- **🚨 Acil Panik Protokolü (`sudo anon --panic`):** Tüm fiziksel/kablosuz ağ arayüzlerini anında keser, tam DROP güvenlik duvarı uygular, ağ süreçlerini sonlandırır, oturum dizinlerini `shred` ile siler, önbellekleri boşaltır ve bash/zsh geçmişini yok eder.
- **🔧 Sistem Kurtarma ve Onarım (`sudo anon --fix`):** Çökme veya ani güç kesintisi sonrası alınan yedeklerden temel ağ yapılandırmasını, `resolv.conf` kilidini, orijinal MAC adreslerini ve güvenlik duvarını onarır.
- **🔄 Tor Devre Yenileme (`SIGNAL NEWNYM`):** Servisi yeniden başlatmadan Tor ControlPort üzerinden tek tuşla yeni çıkış IP'si ve kimlik talep eder.

---

## 🎯 Hazır Güvenlik Profilleri

Operasyon ihtiyacınıza göre tek komutla etkileşimsiz profil başlatabilirsiniz:

```bash
sudo anon --profile <isim>
```

| Profil | Odak Noktası | Etkinleştirilen Modüller |
| :--- | :--- | :--- |
| `paranoia` | **Maksimum OPSEC** | Tüm 22 modül aynı anda etkin (Tor, Köprüler, Kill Switch, Jail, Stealth, Anti-MITM, RAM silme vb.). |
| `quick` | **Hızlı Anonim Gezinme** | `ip_changer`, `kill_switch`, `sandbox_isolation` |
| `pentest` | **Ofansif Güvenlik Testi** | `ip_changer`, `proxychains_integration`, `traffic_obfuscation`, `anti_mitm` |
| `darknet` | **Derin Sansür Aşma** | `ip_changer`, `tor_bridges`, `i2p_integration`, `doh_shield` |
| `lockdown` | **Fiziksel Cihaz Kilidi** | `kill_switch`, `anti_cold_boot`, `volatile_logging`, `bluetooth_disabler` |
| `ghost` | **Maksimum Sistem Gizleme** | Çakışmayan 19 gizlilik, parmak izi sahteleme ve dosyasız çalışma modülü |

---

## 🖥️ Etkileşimli Terminal Kullanıcı Arayüzü (TUI)

Ana arayüzü başlatmak için:
```bash
sudo anon
```

Duyarlı TUI, terminal pencere boyutunuza (`LINES` x `COLUMNS`) otomatik uyum sağlar:
- **Canlı İzleme Paneli:** Anlık veri akışı (TX/RX), Tor çıkış IP doğrulaması, ağ durumu göstergeleri ve aktif modül sayısı (3 saniyede bir güncellenir).
- **Pürüzsüz Gezinme:** Yön tuşları veya Vim (`j`/`k`) ile gezinin, `Enter` ile seçin ve `q` ile geri dönün veya çıkın.
- **Anlık Dil Değiştirme:** Tek seçimle arayüzü İngilizce ve Türkçe arasında anında değiştirin.

---

## 💻 Kurulum ve Hızlı Başlangıç

### Yerel Linux (Debian / Kali / Parrot / Ubuntu)

```bash
# 1. Depoyu klonlayın
git clone https://github.com/egnake/anon.git
cd anon

# 2. Otomatik kurulumu başlatın
sudo chmod +x setup.sh
sudo ./setup.sh

# 3. ANON'u çalıştırın
sudo anon
```

### Windows 10/11 (WSL2 Üzerinden)

1. Depo klasöründe Yönetici yetkili PowerShell veya CMD açın:
   ```cmd
   setup.bat
   ```
2. Kurulum tamamlandıktan sonra herhangi bir Windows terminalinden çağırın:
   ```cmd
   anon
   anon --browser
   anon --clean-meta "D:\HassasBelgeler"
   ```

### Docker Konteyneri (Kali Rolling Ortamı)

```bash
docker build -t anon .
docker run --rm -it --cap-add=NET_ADMIN --cap-add=SYS_ADMIN anon
```

---

## 📖 Komut Satırı Referansı (CLI)

| Komut | İşlev |
| :--- | :--- |
| `sudo anon` | Etkileşimli Terminal Kullanıcı Arayüzünü (TUI) başlatır. |
| `sudo anon --start` | Kaydedilmiş son yapılandırma veya varsayılan kurallarla etkileşimsiz başlatır. |
| `sudo anon --stop` | Tüm aktif modülleri durdurur ve orijinal ağ yapılandırmasını geri yükler. |
| `sudo anon --profile <isim>` | Hazır güvenlik profillerinden birini (`paranoia`, `quick`, `pentest`, `darknet`, `lockdown`, `ghost`) uygular. |
| `sudo anon --status` | 22 modülün tamamı için renkli görsel durum raporu yazdırır. |
| `sudo anon --status --json` | Otomasyon ve betikler için tüm modül durumlarını JSON formatında döndürür. |
| `sudo anon --dashboard` | Her 3 saniyede bir otomatik güncellenen canlı ağ ve trafik panelini açar. |
| `sudo anon --browser` | İzole ağ ad alanı içinde RAM tabanlı tek kullanımlık tarayıcıyı başlatır. |
| `sudo anon --audit` | Anlık IP, DNS ve soket sızıntı denetimi yapar (JSON için `--json` eklenebilir). |
| `sudo anon --clean-meta <yol>` | Dosya veya klasörlerdeki adli metaverileri paralel olarak temizler. |
| `sudo anon --lang <en\|tr>` | Arayüz dilini İngilizce (`en`) veya Türkçe (`tr`) olarak ayarlar. |
| `sudo anon --panic` | Acil Panik Protokolünü tetikleyerek ağı keser ve RAM kalıntılarını temizler. |
| `sudo anon --fix` | Kayıtlı yedeklerden temel sistem durumunu ve ağ bağlantısını kurtarır. |
| `sudo anon --update` | GitHub'dan en güncel imzalı sürümü (Tor üzerinden) indirir, SHA256 doğrular ve günceller. |
| `anon --version` | Root yetkisi gerektirmeden sürüm bilgisini (`ANON v3.0.0`) görüntüler. |

---

## 🏗️ Depo Mimarisi

```text
.
├── Cargo.toml                  # Sıfır bağımlılıklı manifest (opt-level 3, LTO, strip)
├── src/
│   ├── main.rs                 # CLI giriş noktası, arka plan servis yöneticisi, profil motoru
│   ├── config.rs               # Ortam değişkeni destekli yol çözümleme (BASE_DIR, BACKUPDIR, TORRC)
│   ├── i18n.rs                 # Çoklu dil motoru (İngilizce / Türkçe, kalıcı anon.lang)
│   ├── state.rs                # POSIX flock + atomik geçici dosya/chmod/rename durum yönetimi
│   ├── sys.rs                  # Linux libc FFI (geteuid, flock, Netlink, inotify, mlock RAM silme)
│   ├── helpers.rs              # NetworkManager kilitleri, Tor ControlPort TCP, JSON ve bağımlılık kontrolü
│   ├── registry.rs             # Bildirimsel 22 modüllü kayıt defteri, çalışma sıraları ve trait yapısı
│   ├── ui.rs                   # ANSI renk paleti, çift arabellekli çerçeve çizimi, dinamik afişler
│   ├── tui.rs                  # Ham terminal korumalı etkileşimli menü, modül seçici, canlı panel
│   └── modules/
│       ├── mod.rs              # Trait yönlendirici ve modül fabrikası
│       ├── network.rs          # IP, DNS, DoH, Kill Switch, Köprüler, DPI, I2P, Proxychains, MAC
│       ├── obfuscation.rs      # Saat Dilimi, Hostname, Tarayıcı, OS, Süreç, Jail, Stealth, Chaff
│       ├── defense.rs          # Anti-MITM, Sızıntı Denetçisi, Uçucu Günlükler, Bluetooth, Anti-Cold Boot
│       └── utils.rs            # Kurtarma (--fix), Panik, Tek Kullanımlık Tarayıcı, Metaveri, Güncelleme
├── assets/                     # Masaüstü başlatıcı, simgeler, anon.js profili, köprü listeleri, afişler
├── setup.sh / setup.bat        # Linux ve Windows WSL2 için otomatik kurulum betikleri
├── Makefile                    # Standart Linux derleme hedefleri (build, install, uninstall, lint, test)
├── Dockerfile                  # Çok aşamalı Kali Rolling konteyner imajı
└── test_suite.sh               # Cargo birim testleri + 68 izolasyonlu fakeroot kara kutu testi
```

---

## 🧪 Test ve Doğrulama

ANON katı bir **sıfır uyarı (zero-warning)** politikası izler ve sürekli entegrasyon testleriyle doğrulanır:

```bash
# 1. Cargo lint ve birim testlerini çalıştırın
make lint
RUSTFLAGS="-D warnings" cargo test

# 2. Tam kara kutu entegrasyon paketini çalıştırın (68 doğrulama)
ANON_FORCE_BUILD=1 ./test_suite.sh
```

---

## ⚠️ Yasal ve Güvenlik Uyarısı

Bu yazılım yalnızca eğitim, yetkili güvenlik denetimleri, sızma testleri ve dijital gizliliğin korunması amacıyla geliştirilmiştir. Geliştiriciler; kötüye kullanımdan, yetkisiz faaliyetlerden veya yerel yasal düzenlemelerin ihlalinden sorumlu tutulamaz.

Operasyonel güvenlik kurallarını her zaman uygulayın: hiçbir yazılım sizi davranışsal hatalardan (örneğin Tor üzerinden gerçek kimliğinize bağlı kişisel hesaplara giriş yapmaktan) koruyamaz.

---

<div align="center">
  <b>Geliştiren ve Sürdüren: <a href="https://github.com/egnake">egnake</a></b><br>
  Hata bildirimleri ve özellik talepleri için <a href="https://github.com/egnake/anon/issues">GitHub Issues</a> sayfasını kullanabilirsiniz.
</div>
