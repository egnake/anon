<div align="center">
  <img src="assets/anon_logo.jpg" alt="ANON Logo" width="200" style="border-radius: 20px;">
  
  <h1>ANON: Gelişmiş OPSEC ve Anonimlik Çerçevesi</h1>
  <p><b>Linux ortamları için kapsamlı, modüler bir anonimlik ve anti-adli bilişim (anti-forensics) araç seti.</b></p>
  
  <p>
    <a href="https://github.com/egnake/anon/blob/main/LICENSE"><img src="https://img.shields.io/badge/Lisans-MIT-blue.svg?style=for-the-badge" alt="License"></a>
    <a href="https://github.com/egnake/anon/releases"><img src="https://img.shields.io/badge/Sürüm-3.0-green.svg?style=for-the-badge" alt="Version"></a>
    <img src="https://img.shields.io/badge/Platform-Linux%20(Debian%2FKali)-orange.svg?style=for-the-badge" alt="Platform">
    <img src="https://img.shields.io/badge/Dil-Bash_100%25-yellow.svg?style=for-the-badge" alt="Language">
    <img src="https://img.shields.io/badge/Mimari-Modüler-purple.svg?style=for-the-badge" alt="Architecture">
  </p>

  <h3>
    <a href="README.md">🇺🇸 English</a> | 
    <a href="README-TR.md">🇹🇷 Türkçe</a>
  </h3>
</div>

<br>

## 🚀 Genel Bakış

**ANON**, siber güvenlik araştırmacıları, geliştiriciler ve gizlilik savunucuları için tasarlanmış titiz, açık kaynaklı bir anonimlik ve Operasyonel Güvenlik (OPSEC) çerçevesidir. Temel proxy betiklerinin çok ötesine geçen ANON; sıkı ağ izolasyonu uygulayan, sistem parmak izlerini rastgele değiştiren ve aktif anti-adli bilişim (anti-forensics) tedbirleri alan derinlemesine entegre edilmiş 21 modüllü bir mimari sunar.

ANON; Linux çekirdek ağ alanlarını (`netns`), trafik kontrolünü (`tc`) ve dinamik `iptables` yapılandırmalarını kullanarak standart bir Debian kurulumunu son derece kısıtlanmış ve güvenli bir ortama dönüştürür. Çıkan tüm trafiğin kesinlikle Tor ağı üzerinden yönlendirilmesini sağlarken, yaygın zamanlama saldırılarını ve yerel ağ tehditlerini hafifletir.

---

## 🛡️ Tehdit Modeli ve Felsefe

ANON, "Derinlemesine Savunma" (Defense in Depth) kavramı etrafında inşa edilmiştir. Düşmanların, iletişim zincirinizin birden fazla noktasında bulunabileceğini varsayar. Çerçeve, aşağıdaki spesifik vektörleri hafifletmeye çalışır:

1. **Trafik Analizi ve İSS (ISP) Gözetimi:** Tüm TCP ve DNS trafiğini şeffaf olarak Tor'a zorlar. `obfs4` köprülerini, Snowflake WebRTC gizlemesini ve Website Fingerprinting (WFP) saldırılarını bozmak için aktif trafik karartma (sahte paketler ve jitter) kullanarak savunma yapar.
2. **Yerel Ağ (LAN) Saldırıları:** MAC adresi rastgeleleştirme, ARP zehirlenmesi tespiti, bilgisayar adı (hostname) gizleme ve fiziksel ağ arayüzlerinden sıkı izolasyon yoluyla azaltılır.
3. **Uç Nokta Zafiyetleri ve Adli Bilişim:** İzole edilmiş kernel ağ alanları (`anon_jail`), tarayıcılar ve sistem logları için geçici RAM tabanlı dosya sistemleri (`volatile_logging`), süreç gizleme (`execution_stealth`) ve Cold-Boot veri çıkarımına karşı kriptografik bellek temizleme (`sdmem`) kullanımı yoluyla engellenir.

---

## ⚙️ 21 Modül (Detaylı İnceleme)

ANON son derece modülerdir. Etkileşimli Terminal Kullanıcı Arayüzünü (TUI) kullanarak bireysel özellikleri belirli operasyonel gereksinimlerinize göre açıp kapatabilirsiniz.

### 🌐 Ağ ve Yönlendirme Alt Sistemi

**[01] Tor Bridges (obfs4)**
Standart Tor trafiği, İSS'ler ve güvenlik duvarları tarafından Derin Paket İnceleme (DPI) yoluyla kolayca tespit edilebilir. Bu modül, Tor hizmetini `obfs4` köprülerini kullanacak şekilde yapılandırır. Böylece Tor el sıkışmasını (handshake) şifreler ve trafik imzasını değiştirerek rastgele veri gürültüsünden ayırt edilemez hale getirir. Sıkı sansürü atlatmak için esastır.

**[02] IP Changer (Tor TransPort)**
Sadece SOCKS5 proxy'lerine güvenmek, bir uygulamanın yanlış yapılandırılması durumunda ölümcül veri sızıntılarına yol açabilir. Bu modül, kernel seviyesinde sıkı `iptables` `REDIRECT` kurallarından yararlanır. Makinenizden kaynaklanan tüm dışarı çıkan TCP trafiğini yakalar ve fiziksel olarak Tor'un şeffaf proxy portuna (9040) zorlar. DNS istekleri otomatik olarak Tor DNSPort'una (5353) yönlendirilir.

**[03] DNS Changer (Gizlilik Odaklı DNS)**
Tor şeffaf proxy'sinin kullanılmadığı senaryolarda, İSS seviyesinde DNS takibini ve sansürünü engellemek için sistemi log tutmayan, şifresiz sansürsüz güvenli DNS sunucularına yönlendirir.

**[04] MAC Changer**
Bir ağ bağlantısı tam olarak kurulmadan önce, bu modül Ağ Arayüz Kartınızın (NIC) Medya Erişim Kontrolü (MAC) adresini üretici onaylı rastgele bir OUI değerine değiştirir. Bu, yönlendiriciler, halka açık Wi-Fi noktaları ve yerel ağ yöneticileri tarafından yapılan tarihsel takibi kırar.

**[05] Proxychains Integration**
`proxychains4` yapılandırmasını otomatik olarak düzenler ve aktif Tor devrelerinizle hizalar. Bu sayede terminal komutlarını, betikleri veya harici araçları (`curl`, `wget` veya özel python betikleri gibi) manuel yapılandırma ayarlarına gerek kalmadan doğrudan Tor ağı üzerinden tünellemenizi sağlar.

**[06] I2P Integration**
Alternatif bir yönlendirme mekanizması sağlar. Tor ağı yerine Görünmez İnternet Projesi (I2P) ağıyla etkileşim gerektiren görevler için bu modül köprü kurarak merkezi olmayan eepsite'lara güvenli erişim sağlar.

**[07] Traffic Obfuscation (Zamanlama Saldırısı Savunması)**
Tor gibi düşük gecikmeli ağlar, uçtan uca zamanlama korelasyonuna ve Website Fingerprinting (WFP) saldırılarına karşı hassastır. Bu gelişmiş modül, dışarı çıkan paketlerinize rastgele gecikme (jitter) enjekte etmek için Linux Trafik Kontrolü yardımcı programını (`tc`) kullanır. Ayrıca, gerçek veri hacminizi gizlemek için sürekli, rastgele arka plan Tor trafiği (chaffing) üretir.

**[08] DPI Spoofing (Snowflake WebRTC Gizlemesi)**
Gelişmiş DPI sansürünü atlatmak için Tor trafiğini geçici WebRTC bağlantıları içerisine kapsüller; Tor trafiğini sıradan eşler arası (P2P) görüntülü/sesli görüşme akışı gibi gösterir.

**[09] DoH Shield (DNSCrypt ile DNS-over-HTTPS)**
Sistem genelindeki DNS çözümlemesini yerel `dnscrypt-proxy` servisi üzerinden şifreli DoH / DNSCrypt protokollerine zorlar. Tor DNSPort aktif olmadığında dahi DNS sorgularının manipüle edilmesini ve dinlenmesini önler.

### 🧩 Sistem Gizleme ve İzolasyon

**[10] Timezone Changer**
Modern web takibi, sisteminizin yerel saat dilimini genellikle IP adresinizle ilişkilendirir. Bu modül, sisteminizin saat dilimini geçici olarak rastgele küresel koordinatlara (örneğin UTC veya Asya/Tokyo) değiştirerek korelasyon algoritmalarını bozar.

**[11] Hostname Changer**
Bilgisayarınızın adı (ör. `kullanici-laptop`), DHCP istekleri aracılığıyla yerel ağlarda sıkça yayınlanır. Bu modül, kalabalık ağlara sorunsuzca karışmak için hostname değerini tanımlanamayan jenerik OEM/Windows formatıyla rastgeleleştirir.

**[12] Browser Anonymization (Sıkılaştırılmış Firefox Yapılandırması)**
Sistem genelindeki Firefox yapılandırma dizinlerine WebRTC, WebGL, Canvas parmak izi, telemetri ve disk önbelleğini devre dışı bırakan son derece sıkılaştırılmış bir profil (`anon.js`) enjekte eder.

**[13] Disposable Browser (RAM Tabanlı Tek Kullanımlık Oturum)**
Tüm profilini geçici bellekte (`/dev/shm`) oluşturan tek kullanımlık bir Firefox oturumu başlatır. Uzak DNS çözümlü SOCKS5 proxy zorlanır. Tarayıcı kapatıldığı an profil kriptografik olarak silinir (`shred`) ve diskte sıfır adli iz bırakır.

**[14] OS Obfuscation (TTL & TCP Pencere Sahtelemesi)**
Pasif ağ tarayıcıları (`p0f` veya `nmap`), TTL değerlerini ve TCP parametrelerini analiz ederek İşletim Sisteminizi tahmin edebilir. Bu modül, giden paketlerin TTL değerini 128 (Windows 10/11 taklidi) yapar, TCP timestamps ve window scaling özelliklerini düzenler.

**[15] Process Obfuscation (hidepid=2 Kernel İzolasyonu)**
`/proc` dosya sistemini `hidepid=2` ile yeniden bağlayarak yetkisiz kullanıcıların ve zararlı yazılımların ANON arka plan süreçlerini veya diğer işlemleri görmesini engeller.

**[16] Sandbox Isolation (Kernel netns anon_jail)**
Yalıtılmış bir Çekirdek Ağ Alanı (`anon_jail`) yaratır. Sanal ethernet çifti (`veth-anon`/`veth-jail`) üzerinden içerideki tüm trafiği host makinenin Tor TransPort (9040) ve DNSPort (5353) portlarına yönlendirir. AppArmor ve Firejail ile entegre çalışır.

**[17] Execution Stealth (Dosyasız RAM Çalışması & Süreç Maskeleme)**
ANON çerçevesini doğrudan geçici RAM diskinde (`/dev/shm`) çalıştırır ve süreç adını kernel iş parçacığı formatında (`[kworker/u4:2]`) maskeler. Çıkışta bellek temizlenirken kalıcı konfigürasyon ve yedekler güvenle saklanır.

### 🔥 Aktif Savunma ve Anti-Adli Bilişim

**[18] Network Kill Switch (Atomik iptables-restore)**
Tor bağlantısı kesilirse trafiğin açık ağa sızmasını engellemek için `iptables-restore` ve `ip6tables-restore` aracılığıyla atomik olarak tüm trafiği düşüren (`DROP`) güvenlik duvarı kuralları uygular.

**[19] Anti-MITM (ARP Spoofing Koruması)**
Yerel ARP tablosunu sürekli izler. Ağ geçidi MAC adresinde olağan dışı bir değişiklik tespit edilirse ağ arayüzünü derhal kapatır (`DOWN`) ve saldırganı engeller.

**[20] Anti-Cold Boot (sdmem & Takas Kilidi)**
Cihaz kapatılırken veya oturum sonlandırılırken swap alanını kapatır, bellek dökümlerini kilitler ve tüm boş RAM alanını `sdmem` ile kriptografik olarak rastgele verilerle temizler.

**[21] Bluetooth Disabler (rfkill Donanım Bloğu)**
`rfkill` aracılığıyla cihazdaki tüm Bluetooth denetleyicilerine donanım düzeyinde tam blok uygular; yakınlık tabanlı izleme ve saldırıları önler.

**[22] Arka Plan Sızıntı Denetleyicisi (Background Leak Auditor)**
Arka planda asenkron olarak çalışan nöbetçi daemon; ağ çıkışlarını, `/etc/resolv.conf` DNS sunucularını ve Tor devresini periyodik denetler. Sızıntı durumunda masaüstü bildirimi basıp anında Kill Switch'i devreye sokar.

**[23] Masaüstü Bildirimleri (notify-send GUI Uyarıları)**
Root/daemon olaylarını aktif masaüstü oturumuna aktararak ARP spoofing tespiti, Tor bağlantı durumu, sızıntı uyarıları ve acil durum panik kilitlenmelerinde anlık GUI bildirimleri iletir.

**[24] Metadata Temizleyici (MAT2 & ExifTool Entegrasyonu)**
Resimler, PDF'ler, ofis belgeleri ve medyadan EXIF, GPS konumları, yazar bilgileri ve cihaz seri numaralarını MAT2 (Tails OS standardı) veya ExifTool ile adli bilişime karşı temizler.

**[Ekstra] Volatile Logging (Tails OS RAM Loglama)**
`/var/log` dizini üzerine bir RAM diski (`tmpfs`) bağlar. Oturum boyunca tüm loglar geçici RAM'e yazılır; oturum kapandığında eski loglar bozulmadan geri gelir ve şüpheli log silme izi bırakmaz.

---

## 🖥️ Etkileşimli Terminal Kullanıcı Arayüzü (TUI)

ANON, hantal komut satırı parametrelerini tamamen Bash ile yazılmış zarif, duyarlı bir TUI ile değiştirir.

* **Dinamik Estetik Oluşturma:** TUI, her başlatıldığında dinamik olarak değişen, güzel biçimlendirilmiş, ortalanmış ASCII banner'lara sahiptir.
* **Canlı Kontrol Paneli (Live Dashboard):** Çıkış IP'nizi, aktif devre durumunuzu, anlık ağ trafiğini ve çalışan OPSEC modüllerini sızıntı olmadan gerçek zamanlı izleyin.
* **Operasyon Merkezi:** Yön tuşlarınızı kullanarak doğrudan DNS/IP sızıntı testi yapın, yeni bir Tor kimliği (`SIGHUP`) talep edin veya tek kullanımlık tarayıcıyı başlatın.

---

## 📦 Sistem Gereksinimleri ve Bağımlılıklar

ANON, Linux ağ yığınına derinden entegredir. Gereksinimleri:

- **İşletim Sistemi:** Kali Linux, Parrot Security OS, Debian, Ubuntu veya WSL2 (netns desteği için özel Kernel önerilir).
- **İzinler:** `iptables`, kernel ağ alanlarını (namespaces) ve ağ arayüzlerini manipüle etmek için Root ayrıcalıkları (`sudo`) gereklidir.
- **Bağımlılıklar (Otomatik kurulur):** `tor`, `obfs4proxy`, `macchanger`, `proxychains4`, `secure-delete` (sdmem), `rfkill`, `firejail`, `iproute2`, `iptables`, `i2pd`, `mat2`, `libimage-exiftool-perl`.

---

## 🛠️ Kurulum Rehberi

Kurulum işlemi `setup.sh` betiği aracılığıyla tamamen otomatiktir.

```bash
# 1. Depoyu klonlayın
git clone https://github.com/egnake/anon.git

# 2. Dizine girin
cd anon

# 3. Kurulum betiğini çalıştırılabilir yapın
sudo chmod +x setup.sh

# 4. Yükleyiciyi çalıştırın
sudo ./setup.sh
```

---

## 🚀 Kullanım ve Komutlar

Ana etkileşimli arayüzü başlatmak için şunu çalıştırmanız yeterlidir:
```bash
sudo anon
```

### Gelişmiş Komut Satırı Bayrakları
Otomasyon veya komut dosyası oluşturmayı tercih eden kullanıcılar için ANON, doğrudan komut satırı yürütmesini destekler:

| Komut | Açıklama |
| :--- | :--- |
| `sudo anon --start` | Aracı, en son kaydedilen modül yapılandırmasını (veya varsayılan Tor + Kill Switch) kullanarak doğrudan başlatır. |
| `sudo anon --stop` | Tüm aktif modülleri anında devre dışı bırakır, iptables'ı temizler ve orijinal ağ durumunu geri yükler. |
| `sudo anon --status` | Tüm aktif ve pasif OPSEC modülünün kapsamlı bir görsel raporunu yazdırır. |
| `sudo anon --dashboard` | Canlı izleme panelini açar (her 3 saniyede bir otomatik yenilenir). |
| `sudo anon --browser` | RAM tabanlı tek kullanımlık tarayıcıyı doğrudan izole edilmiş ağ alanı (namespace) içinde başlatır. |
| `sudo anon --audit` | Anlık derinlemesine IP, DNS ve soket sızıntı denetimi yürütür. |
| `sudo anon --clean-meta <hedef>` | Dosya veya klasörlerdeki adli metaverileri (EXIF/GPS/etiketler) MAT2 veya ExifTool ile temizler. |
| `sudo anon --panic` | Acil Durum / Panik Protokolü: Tüm ağ arayüzlerini anında kapatır, güvenlik duvarını kilitler, süreçleri sonlandırır ve RAM'i temizler. |
| `sudo anon --fix` | Çökme veya ani kapanma durumunda varsayılan sistem ağ yapılandırmalarını yedeklerden geri yükler. |
| `sudo anon --update` | En son kaynak kodunu GitHub'dan getirir ve çerçeveyi otomatik olarak yeniden yükler. |

---

## ❓ SSS ve Sorun Giderme

**S: ANON'u çalıştırdıktan sonra internet bağlantım yok!**
C: `Kill Switch` aktifken Tor bağlanamazsa tüm trafik `iptables` tarafından düşürülür (DROP). Betiği başlatmadan önce internetinizin çalıştığından emin olun, sistem saatinizi kontrol edin (Tor doğru bir saate ihtiyaç duyar) veya Tor köprülerinizin çalıştığından emin olun. Ağınızı sıfırlamak için her zaman `sudo anon --stop` veya fiziksel yedeklerden geri yüklemek için `sudo anon --fix` çalıştırabilirsiniz.

**S: Bunu Sanal Makinede (VM) kullanabilir miyim?**
C: Evet. Ancak MAC Changer gibi bazı modüller, Hipervizörünüzün (VMware, VirtualBox) ağ bağdaştırıcısı ayarlarında MAC adresi sahtekarlığına (spoofing) açıkça izin vermesini gerektirebilir. Bu izin olmadan MAC değiştirildiğinde VM internet bağlantısını kaybedebilir.

**S: Tek Kullanımlık Tarayıcı yer imlerimi veya şifrelerimi kaydeder mi?**
C: Hayır. Tasarım gereği, tarayıcı profili yalnızca geçici RAM'de (`/dev/shm`) bulunur. Geçmiş, önbellek, eklentiler ve yer imleri dahil olmak üzere her şey, tarayıcı penceresi kapatıldığı an yok edilir. Yalnızca son derece hassas, tek kullanımlık oturumlar için tasarlanmıştır.

**S: ANON kullanırken kişisel hesaplarıma (Google veya Facebook gibi) giriş yapmam güvenli mi?**
C: Kesinlikle hayır. Bu çok kritik bir OPSEC (Operasyonel Güvenlik) hatasıdır. IP ve MAC adresiniz gizlenmiş olsa bile, gerçek kimliğinize bağlı bir hesaba giriş yapmak, anonim trafiğinizi anında sizinle ilişkilendirir. ANON makinenizi korur, ancak sizi davranışsal hatalarınızdan koruyamaz.

**S: "Traffic Obfuscation" modülü bağlantımı neden yavaşlatıyor?**
C: Bu modül, zamanlama korelasyonunu ve Website Fingerprinting (WFP) saldırılarını bozmak için paketlerinize bilinçli olarak yapay bir gecikme (jitter) enjekte eder. Bu küçük yavaşlama, gelişmiş ağ izleyicilerine karşı anonimliği büyük ölçüde artırmanın bir bedelidir.

**S: Kernel netns İzolasyonu normal Proxychains'ten nasıl farklıdır?**
C: Proxychains, uygulamaları Tor üzerinden yönlendirmek için standart kütüphane çağrılarını (örneğin `connect()`) ele geçirir. Ancak bazı uygulamalar (statik derlenmiş dosyalar veya özel DNS istekleri) bunu atlayabilir. Kernel `netns` ise kelimenin tam anlamıyla aşılmaz, fiziksel bir sanal alan (sandbox) yaratır. Bu hapishane içinden çıkmanın tek fiziksel yolu Tor TransPort'a bağlıdır. Atlanması imkansızdır.

**S: Script çökerse veya bilgisayarımın gücü aniden kesilirse ne olur?**
C: Kurulum sırasında ANON, orijinal `iptables`, `resolv.conf` ve `NetworkManager` durumlarınızı yedekler. Sistem beklenmedik şekilde yeniden başlarsa, tüm ağ ayarlarınızı saniyeler içinde fabrika varsayılanlarına geri döndürmek için `sudo anon --fix` komutunu çalıştırmanız yeterlidir.

---

## ⚠️ Güvenlik Uyarısı ve Sınırlamalar

ANON, ağ analizi ve adli bilişime karşı gelişmiş teknik azaltma sağlar, ancak **sihirli bir değnek değildir**.

* **Operasyonel Güvenlik (OPSEC):** Yazılım sizi insan hatasından koruyamaz. ANON kullanırken kişisel hesaplarınıza (Facebook, Google) giriş yapmak sizi anında deşifre edecektir.
* **HTTPS/TLS:** Tor, trafiği Çıkış Düğümüne (Exit Node) kadar şifreler. Şifrelenmemiş HTTP trafiği gönderirseniz, Çıkış Düğümü bunu izleyebilir ve engelleyebilir. Her zaman HTTPS kullandığınızdan emin olun.
* **Yazılım/Donanım Riskleri:** ANON, İşletim Sistemi katmanında çalışır. Güvenliği ihlal edilmiş BIOS/UEFI donanım yazılımına, Intel ME/AMD PSP arka kapılarına veya donanım keylogger'larına karşı koruma sağlayamaz.

**Sorumluluk Reddi:** Bu araç yalnızca eğitim, akademik ve güvenlik araştırması amacıyla sağlanmıştır. Geliştiriciler, bu yazılımın neden olduğu herhangi bir kötüye kullanım, yasa dışı faaliyet veya hasardan sorumlu değildir. Kendi yargı bölgeniz içinde sorumlu ve yasal bir şekilde kullanın.

---

<div align="center">
  <b>Hata Bildirimi & Katkı Sağlama</b><br>
  egcaem000@gmail.com | <a href="https://github.com/egnake/anon/issues">GitHub Issues</a>
</div>
