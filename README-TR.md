<div align="center">
  <img src="assets/anon_hero.jpg" alt="ANON Hero Banner" width="100%">
  
  <h1>ANON: Gelişmiş OPSEC ve Anonimlik Çerçevesi</h1>
  <p><b>Linux Sistemler için İstihbarat Seviyesinde Anonimlik, Anti-Adli Bilişim ve Trafik Karartma Aracı</b></p>
  
  <p>
    <a href="https://github.com/egnake/anon/blob/main/LICENSE"><img src="https://img.shields.io/badge/Lisans-MIT-blue.svg?style=for-the-badge" alt="License"></a>
    <a href="https://github.com/egnake/anon/releases"><img src="https://img.shields.io/badge/Sürüm-3.0-green.svg?style=for-the-badge" alt="Version"></a>
    <img src="https://img.shields.io/badge/Platform-Linux%20(Debian%2FKali)-orange.svg?style=for-the-badge" alt="Platform">
    <img src="https://img.shields.io/badge/Dil-Bash_100%25-yellow.svg?style=for-the-badge" alt="Language">
    <img src="https://img.shields.io/badge/Mimari-Modüler-purple.svg?style=for-the-badge" alt="Architecture">
  </p>

  <p>
    <a href="README.md">English</a> •
    <a href="README-TR.md">Türkçe</a>
  </p>
</div>

<br>

## 🚀 Genel Bakış

**ANON**, aşırı yüksek tehdit modelleri altında çalışan siber güvenlik araştırmacıları, bilgi sızdıranlar (whistleblowers), gazeteciler ve aktivistler için tasarlanmış son teknoloji bir OPSEC (Operasyonel Güvenlik) ve anonimlik çerçevesidir (framework). Sadece Tor ağını açıp IP değiştirmekle yetinmez; kesin ağ izolasyonu sağlayan, sistem parmak izini gizleyen, trafik analizi saldırılarını (zamanlama saldırıları) engelleyen ve paranoyak düzeyde anti-adli bilişim (anti-forensics) tedbirleri uygulayan **17 bağımsız modül** sunar.

Sıradan betiklerin aksine ANON, **Whonix tarzı Kernel Ağ Yalıtımı (Kernel Network Namespace Isolation - `anon_jail`)** mimarisini doğrudan ana makineye entegre eder. Bu sistem, tüm ağın donanımsal izolasyonla Tor'dan geçmesini zorunlu kılar ve tarayıcıya yönelik sıfır gün (0-day) açıklarından kaynaklanabilecek sızıntıları engeller. Sıradan bir Kali/Debian kurulumunu adeta bir kaleye çevirir.

---

## ⚡ Teknik Mimari ve Tehdit Modellemesi (Threat Modeling)

ANON, belirli düşman profillerine karşı savunma yapmak için üç temel taktiksel alanda çalışır:

### 1. Küresel Pasif İzleyici (GPA) Savunması
Trafik akışını ve meta verileri izleyen İSS'ler (İnternet Servis Sağlayıcıları), otonom sistemler ve devlet seviyesi gözetim mekanizmalarına karşı koruma.
- **Savunma Yöntemleri:** Şeffaf Tor Yönlendirmesi, obfs4 Köprüleri, DNS Sızıntı Koruması ve Aktif Trafik Karartma (Jitter & Sahte Paketler).

### 2. Yerel Ağ Düşmanı (LNA) Savunması
Fiziksel olarak bulunduğunuz yerel ağdaki (LAN) saldırganlara karşı (örneğin sahte erişim noktaları, ARP Zehirlenmesi, MAC izleme) koruma.
- **Savunma Yöntemleri:** MAC Adresi Rastgeleleştirme, ARP Spoofing Tespiti, Hostname Gizleme, Bluetooth Radyo Kapatıcı (Kill Switch).

### 3. Uç Nokta (Endpoint) ve Adli Bilişim Savunması
Fiziksel cihaza el konulması, Cold-Boot (Soğuk Önyükleme) saldırıları ve gerçek IP'yi sızdırmak için tarayıcıyı hedef alan zararlı yazılımlara karşı koruma.
- **Savunma Yöntemleri:** Sadece RAM'de çalışan tarayıcılar, Kernel Sandbox İzolasyonu, `sdmem` RAM Temizleme, İşletim Sistemi TTL Manipülasyonu.

---

## 🛠️ Detaylı 17 Modül İncelemesi

### 🌐 Ağ ve Yönlendirme (GPA Savunması)

**[01] Tor Bridges (obfs4)**
Standart Tor trafiği, Derin Paket İnceleme (DPI) cihazları tarafından kolayca tespit edilebilir. Bu modül, Tor trafiğini `obfs4` köprü düğümleri üzerinden geçirerek trafiğinizi anlamsız rastgele verilere benzetir. Güçlü sansür uygulanan ağlarda hayati öneme sahiptir.

**[02] IP Changer (Tor TransPort)**
Uygulamaların SOCKS5 vekillik ayarlarının yanlış yapılandırılması sonucu oluşan sızıntıları önlemek için `iptables` REDIRECT kurallarını kullanır. Makinedeki **tüm TCP trafiğini** fiziksel olarak Tor'un şeffaf proxy bağlantı noktasına (9040) zorlar.

**[03] DNS Changer (DNSCrypt / Tor DNSPort)**
DNS sızıntıları, deşifre olmanın 1 numaralı nedenidir. Bu modül, tüm 53. port (UDP/TCP) trafiğini ele geçirir ve Tor'un DNSPort'u (5353) üzerinden zorla çözümler. İSS'niz hangi alan adlarını sorguladığınızı asla göremez.

**[04] MAC Changer**
Herhangi bir ağa bağlanmadan önce Ağ Arayüz Kartınızın (NIC) Fiziksel Medya Erişim Kontrolü (MAC) adresini rastgele değiştirir. Bu, cihazınızın farklı kafelerde veya ortak ağlarda izlenmesini engeller.

**[05] Proxychains Integration**
Sisteme `proxychains4` yapılandırmasını otomatik entegre eder. Böylece `nmap`, `curl` veya `sqlmap` gibi komut satırı araçlarını tek komutla sorunsuz bir şekilde Tor üzerinden çalıştırabilirsiniz.

**[06] I2P Integration**
Tor ağının aşırı izlendiği veya engellendiği tehdit modelleri için, trafiğinizi doğrudan Görünmez İnternet Projesi (I2P) ağına yönlendirmeyi destekler.

**[07] Traffic Obfuscation (Zamanlama Saldırısı Savunması) 🆕**
*Ağ savunmasının başyapıtı.* Tor gibi düşük gecikmeli (low-latency) ağlar, Website Fingerprinting (WFP) ve uçtan uca zamanlama korelasyonu saldırılarına karşı savunmasızdır. Bu modül, Linux `tc` (Traffic Control) modülü ile dışarı çıkan her pakete 15ms-30ms arası **rastgele gecikme (jitter)** ekleyerek zamanlama analizini felç eder. Aynı zamanda arka planda sürekli olarak sahte Tor verisi (chaffing) indirerek trafiğinizin gerçek boyutunu maskeler.

### 🛡️ Sistem Gizleme ve İzolasyon (Uç Nokta Savunması)

**[08] Timezone Changer**
Tarayıcılar ve sistem araçları, JavaScript veya NTP üzerinden bulunduğunuz yerel saat dilimini sızdırır. Bu modül, sisteminizin saat dilimini rastgele küresel koordinatlara (ör. Asya/Tokyo veya UTC) alarak sizi milyonlarca kullanıcının arasına gizler.

**[09] Hostname Changer**
Bilgisayarınızın adı `Ahmet-MacBook` ise, bağlandığınız her modem bunu kaydeder. Bu modül, makinenizin adını tamamen jenerik kelimelerle değiştirerek DHCP loglarında sizi görünmez kılar.

**[10] Browser Anonymization (Disposable RAM Profile)**
Normal bir tarayıcı açmak yerine, ANON cihazınızın geçici belleğinde (RAM - `/dev/shm`) izole bir dosya sistemi oluşturur. WebRTC, WebGL ve Canvas izleyicileri kapatılmış özel `anon.js` dosyasını buraya kopyalar. Tarayıcıyı kapattığınız saniye profil tamamen silinir. Hardiskinize (SSD/HDD) tek bir byte bile yazılmaz.

**[11] OS Obfuscation (TTL Spoofing)**
Ağ tarayıcıları (`nmap`, `p0f`), paketlerinizin Time-To-Live (TTL) değerlerine bakarak hangi işletim sistemini kullandığınızı tahmin edebilir. Bu modül, Linux paketlerini manipüle ederek sizi ağda bir Windows veya macOS bilgisayar olarak gösterir.

**[12] Process Obfuscation**
Çalışan OPSEC scriptlerini ve arka plan koruma mekanizmalarını standart sistem izleme araçlarından (`ps`, `top`) gizler. Root olmayan zararlı yazılımların sistem savunmanızı analiz etmesini engeller.

**[13] Sandbox Isolation (Kernel netns) 🆕**
*APT Seviyesinde İzolasyon.* Whonix'in mimarisini kopyalayan bu modül, bilgisayarınızda yalıtılmış bir Çekirdek Ağ Alanı (`anon_jail`) yaratır. İçeriye sadece tek bir sanal kablo (`veth`) çeker ve bu kablonun tek çıkış yolu host makinedeki Tor ağıdır. Tarayıcıyı başlattığınızda, tarayıcı bu "hapishane" içerisinde, yetkileri kısıtlanmış (`sudo -u`) ve Firejail/AppArmor ile sınırlandırılmış şekilde açılır. Tarayıcıda bir RCE (Uzaktan Kod Çalıştırma) 0-day açığı kullanılsa bile, saldırgan gerçek IP adresinizi veya gerçek ethernet kartınızı göremez.

### 🔥 Aktif Savunma ve Anti-Adli Bilişim

**[14] Network Kill Switch**
En büyük güvenlik önlemi. Eğer Tor çökmezse veya bir şekilde kapanırsa, sisteminiz trafiği normal internetten (clear web) göndermeye çalışır. Kill Switch modülü, Tor ağı dışındaki **TÜM** paketleri fiziksel olarak DROP eden acımasız `iptables` kuralları uygular. Tor giderse, internetiniz kesilir. Sızıntıya taviz verilmez.

**[15] Anti-MITM (ARP Spoofing Protection)**
ARP tablosunu gerçek zamanlı analiz eder. Eğer yerel ağınızdaki bir saldırgan (kötü niyetli bir Wi-Fi veya Evil Twin) ARP Zehirlenmesi ile araya girmeye çalışırsa, anında tespit edip ağ bağlantısını keserek trafiğinizi kurtarır.

**[16] Anti-Cold Boot (sdmem)**
Fiziksel olarak ele geçirilme senaryolarında, saldırganlar RAM donanımını dondurarak (Cold-Boot Attack) cihazınızdaki şifreleri ve aktif oturumları okuyabilirler. Bu modül açıksa, ANON'u kapattığınızda sistem kapanmadan önce `sdmem` (Secure Delete Memory) aracı devreye girer ve tüm RAM ile Swap alanını kriptografik olarak siler.

**[17] Bluetooth Disabler**
Bluetooth radyo dalgaları, izleme cihazları tarafından yakalanabilir ve Blueborne tarzı uzaktan erişim açıklarına sahiptir. Bu modül `rfkill` kullanarak cihazdaki tüm Bluetooth donanımlarını fiziksel seviyede devreden çıkarır.

---

## 🖥️ Etkileşimli TUI (Kullanıcı Arayüzü)

Karmaşık komut parametrelerini unutun. ANON v3.0, tamamen Bash kullanılarak sıfırdan kodlanmış, yön tuşlarıyla gezilebilen modern bir arayüze sahiptir.

* **Dinamik ASCII Bannerlar:** Terminalinizin boyutuna tam uyum sağlayan ve her açılışta rastgele değişen hacker temalı arayüz grafikleri.
* **Live Dashboard (Canlı Panel):** Tor ağ durumunu, çıkış IP'nizi, aktif modülleri ve anlık ağ trafiğinizi takip edin.
* **Operations Menu (Operasyonlar):** Uygulamadan çıkmadan anında IP Sızıntı testleri yapın, IP değiştirmek için Tor'a `SIGHUP` gönderin veya izole edilmiş RAM Tarayıcıyı tek tuşla başlatın.

---

## ⚙️ Kurulum ve Gereksinimler

### Desteklenen İşletim Sistemleri
- Kali Linux (Önerilen)
- Parrot Security OS
- Debian / Ubuntu
- Windows Subsystem for Linux (WSL 2) - *İzolasyon ve iptables gibi çekirdek özellikleri için bazı özel WSL kernelleri gerektirebilir.*

### Hızlı Kurulum

```bash
git clone https://github.com/egnake/anon.git
cd anon
sudo chmod +x setup.sh
sudo ./setup.sh
```

`setup.sh` betiği, gereken tüm bağımlılıkları (Tor, Macchanger, secure-delete, proxychains4, firejail vb.) otomatik olarak kuracak ve ortamı hazırlayacaktır.

---

## 🛠️ Kullanım ve Komutlar

Arayüzü başlatmak için terminale yazmanız yeterlidir:
```bash
sudo anon
```

**Gelişmiş CLI Parametreleri:**
- `sudo anon --start` : Arayüzü açmadan, kaydedilen son profille aracı arka planda başlatır.
- `sudo anon --stop` : Tüm modülleri kapatır, iptables kurallarını temizler ve ağı normale döndürür.
- `sudo anon --status` : Hangi OPSEC modüllerinin aktif/pasif olduğunu liste halinde gösterir.
- `sudo anon --dashboard` : Her 3 saniyede bir güncellenen canlı izleme panelini açar.
- `sudo anon --browser` : Doğrudan anon_jail sandbox'ı içinde RAM tabanlı tarayıcıyı başlatır.
- `sudo anon --fix` : Cihazın ağ çökmesi veya yanlış kapanma durumlarında sistem yedeklerinden ağı onarır.
- `sudo anon --update` : En güncel sürümü GitHub deposundan çekip otomatik olarak kurar.

---

## ⚠️ Uyarı ve Sorumluluk Reddi (OPSEC Uyarıları)

ANON, **sadece eğitim, akademik araştırma ve güvenlik testleri amacıyla** sunulmuştur. Geliştiriciler, aracın kötüye kullanımı, yasa dışı faaliyetler veya sebep olacağı zararlardan sorumlu tutulamaz.

**Anonimlik bir yazılım ürünü değil, bir süreç ve zihniyettir.** 
ANON, cihazınıza askeri standartlarda bir ağ ve uç nokta (endpoint) savunması sağlasa da sizi şunlardan koruyamaz:
1. Operasyonel Güvenlik (OPSEC) hataları (örneğin anonim tarayıcıda kendi Facebook hesabınıza giriş yapmak).
2. Donanım seviyesindeki arka kapılar (firmware backdoors) veya BIOS implantları.
3. Çıkış (Exit) düğümlerinin şifrelenmemiş (HTTPS olmayan) HTTP trafiğini manipüle etmesi.

Yazılım araçlarını daima sıkı operasyonel güvenlik pratikleri ve bilinçle birleştirin.

---

<div align="center">
  <b>Hata Bildirimi & Katkı Sağlama</b><br>
  egcaem000@gmail.com | <a href="https://github.com/egnake/anon/issues">GitHub Issues</a>
</div>
