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

**ANON**, siber güvenlik araştırmacıları, geliştiriciler ve gizlilik savunucuları için tasarlanmış titiz, açık kaynaklı bir anonimlik ve Operasyonel Güvenlik (OPSEC) çerçevesidir. Temel proxy betiklerinin çok ötesine geçen ANON; sıkı ağ izolasyonu uygulayan, sistem parmak izlerini rastgele değiştiren ve aktif anti-adli bilişim (anti-forensics) tedbirleri alan derinlemesine entegre edilmiş 17 modüllü bir mimari sunar.

ANON; Linux çekirdek ağ alanlarını (`netns`), trafik kontrolünü (`tc`) ve dinamik `iptables` yapılandırmalarını kullanarak standart bir Debian kurulumunu son derece kısıtlanmış ve güvenli bir ortama dönüştürür. Çıkan tüm trafiğin kesinlikle Tor ağı üzerinden yönlendirilmesini sağlarken, yaygın zamanlama saldırılarını ve yerel ağ tehditlerini hafifletir.

---

## 🛡️ Tehdit Modeli ve Felsefe

ANON, "Derinlemesine Savunma" (Defense in Depth) kavramı etrafında inşa edilmiştir. Düşmanların, iletişim zincirinizin birden fazla noktasında bulunabileceğini varsayar. Çerçeve, aşağıdaki spesifik vektörleri hafifletmeye çalışır:

1. **Trafik Analizi ve İSS (ISP) Gözetimi:** Tüm TCP ve DNS trafiğini şeffaf olarak Tor'a zorlar. `obfs4` köprülerini ve Website Fingerprinting (WFP) saldırılarını bozmak için aktif trafik karartma (sahte paketler ve jitter) kullanarak savunma yapar.
2. **Yerel Ağ (LAN) Saldırıları:** MAC adresi rastgeleleştirme, ARP zehirlenmesi tespiti, bilgisayar adı (hostname) gizleme ve fiziksel ağ arayüzlerinden sıkı izolasyon yoluyla azaltılır.
3. **Uç Nokta Zafiyetleri ve Adli Bilişim:** İzole edilmiş kernel ağ alanları (`anon_jail`), tarayıcılar için geçici RAM tabanlı dosya sistemleri ve Cold-Boot veri çıkarımına karşı kriptografik bellek temizleme (`sdmem`) kullanımı yoluyla engellenir.

---

## ⚙️ 17 Modül (Detaylı İnceleme)

ANON son derece modülerdir. Etkileşimli Terminal Kullanıcı Arayüzünü (TUI) kullanarak bireysel özellikleri belirli operasyonel gereksinimlerinize göre açıp kapatabilirsiniz.

### 🌐 Ağ ve Yönlendirme Alt Sistemi

**[01] Tor Bridges (obfs4)**
Standart Tor trafiği, İSS'ler ve güvenlik duvarları tarafından Derin Paket İnceleme (DPI) yoluyla kolayca tespit edilebilir. Bu modül, Tor hizmetini `obfs4` köprülerini kullanacak şekilde yapılandırır. Böylece Tor el sıkışmasını (handshake) şifreler ve trafik imzasını değiştirerek rastgele veri gürültüsünden ayırt edilemez hale getirir. Sıkı sansürü atlatmak için esastır.

**[02] IP Changer (Tor TransPort)**
Sadece SOCKS5 proxy'lerine güvenmek, bir uygulamanın yanlış yapılandırılması durumunda ölümcül veri sızıntılarına yol açabilir. Bu modül, kernel seviyesinde sıkı `iptables` `REDIRECT` kurallarından yararlanır. Makinenizden kaynaklanan tüm dışarı çıkan TCP trafiğini yakalar ve fiziksel olarak Tor'un şeffaf proxy portuna (9040) zorlar. Uygulama ayarlarından bağımsız olarak mutlak uyumluluk sağlar.

**[03] DNS Changer (Tor DNSPort)**
DNS istekleri (alan adlarını IP'lere çözümleme), genellikle proxy tünellerinin dışına sızar. Bu modül tüm UDP/TCP 53. port trafiğini yakalar ve Tor'un güvenli DNSPort'una (5353) yönlendirir. Bu, yerel İSS'nizin ziyaret ettiğiniz web sitelerinin geçmişini oluşturamayacağını garanti eder.

**[04] MAC Changer**
Bir ağ bağlantısı tam olarak kurulmadan önce, bu modül Ağ Arayüz Kartınızın (NIC) Medya Erişim Kontrolü (MAC) adresini tamamen rastgele bir değere değiştirir. Bu, yönlendiriciler, halka açık Wi-Fi noktaları ve yerel ağ yöneticileri tarafından yapılan tarihsel takibi kırar.

**[05] Proxychains Integration**
`proxychains4` yapılandırmasını otomatik olarak düzenler ve aktif Tor devrelerinizle hizalar. Bu sayede terminal komutlarını, betikleri veya harici araçları (`curl`, `wget` veya özel python betikleri gibi) manuel yapılandırma ayarlarına gerek kalmadan doğrudan Tor ağı üzerinden tünellemenizi sağlar.

**[06] I2P Integration**
Alternatif bir yönlendirme mekanizması sağlar. Tor ağı yerine Görünmez İnternet Projesi (I2P) ağıyla etkileşim gerektiren görevler için bu modül köprü kurarak merkezi olmayan eepsite'lara güvenli erişim sağlar.

**[07] Traffic Obfuscation (Zamanlama Saldırısı Savunması)**
Tor gibi düşük gecikmeli ağlar, uçtan uca zamanlama korelasyonuna ve Website Fingerprinting (WFP) saldırılarına karşı hassastır. Bu gelişmiş modül, dışarı çıkan paketlerinize rastgele gecikme (jitter) enjekte etmek için Linux Trafik Kontrolü yardımcı programını (`tc`) kullanır. Ayrıca, gerçek veri hacminizi gizlemek için sürekli, rastgele arka plan Tor trafiği (chaffing) üreterek paket boyutu ve zamanlama analizini büyük ölçüde zorlaştırır.

### 🧩 Sistem Gizleme ve İzolasyon

**[08] Timezone Changer**
Modern web takibi, sisteminizin yerel saat dilimini (JavaScript veya sistem başlıkları aracılığıyla sızdırılan) genellikle IP adresinizle ilişkilendirir. Bu modül, sisteminizin saat dilimini geçici olarak rastgele küresel koordinatlara (örneğin UTC veya Asya/Tokyo) değiştirerek korelasyon algoritmalarını bozar.

**[09] Hostname Changer**
Bilgisayarınızın adı (ör. `kullanici-laptop`), DHCP istekleri aracılığıyla yerel ağlarda sıkça yayınlanır. Bu modül, kalabalık ağlara sorunsuzca karışmak için hostname değerini tanımlanamayan jenerik kelimelerle rastgeleleştirir.

**[10] Browser Anonymization (Disposable RAM Profile)**
Geçmişi, çerezleri ve önbelleği sabit diskinize kaydeden standart bir tarayıcı profili başlatmak yerine, ANON WebRTC, WebGL ve Canvas izlemelerini devre dışı bırakan son derece sıkılaştırılmış bir Firefox yapılandırması (`anon.js`) oluşturur. Bu profili tamamen geçici (volatile) bir bellek dosya sistemine (`/dev/shm`) bağlar. Tarayıcı işlemi sonlandığında, profil anında yok edilir ve kalıcı depolamada sıfır adli iz bırakır.

**[11] OS Obfuscation (TTL Spoofing)**
Pasif ağ tarayıcıları (`p0f` veya `nmap` gibi), ağ paketlerinizin varsayılan Time-To-Live (TTL) değerlerini analiz ederek İşletim Sisteminizi tahmin edebilir (Linux genellikle 64, Windows 128 kullanır). Bu modül, dışarı çıkan paketleri engeller ve TTL değerlerini farklı işletim sistemlerini taklit edecek şekilde yeniden yazarak OS parmak izi çıkarma girişimlerini boşa çıkarır.

**[12] Process Obfuscation**
ANON'un arka plan betiklerini ve OPSEC süreçlerini standart kullanıcı alanı izleme araçlarından (`ps` veya `top` gibi) gizlemek için teknikler yürütür. Bu, makinenin savunma yeteneklerinin profilini çıkarmaya çalışan ayrıcalıksız kötü amaçlı betiklere karşı ikincil bir savunma katmanı görevi görür.

**[13] Sandbox Isolation (Kernel netns)**
Gelişmiş izolasyon mimarilerini taklit eden bu modül, yalıtılmış bir Çekirdek Ağ Alanı (`anon_jail`) yaratır. İçinden çıkan tüm trafiğin kesinlikle host'un Tor TransPort'u üzerinden geçmesini sağlayan sanal bir ethernet çifti (`veth`) sağlar. Bu sanal alanın (sandbox) içinde başlatılan uygulamaların (Tek Kullanımlık Tarayıcı gibi) host'un gerçek ağ arayüzleri, gerçek IP'si veya MAC adresi üzerinde kesinlikle hiçbir görünürlüğü yoktur. Olası 0-day tarayıcı açıklarını kontrol altına almak için Firejail/AppArmor ile entegre çalışır.

### 🔥 Aktif Savunma ve Anti-Adli Bilişim

**[14] Network Kill Switch**
Tor hizmeti çökerse, beklenmedik yeniden başlatmalar olursa veya köprüler başarısız olursa, işletim sistemleri genellikle trafiği şifresiz ağ (clear web) üzerinden yönlendirmeye geçer. Kill Switch, döngü (loopback - `lo`) arayüzüne veya kurulu Tor bağlantı noktalarına yönelik olmayan **tüm paketleri açıkça DROP eden** acımasız `iptables` kuralları uygular. Tor kapanırsa, tüm internet bağlantısı kapanır. İstisna ve sızıntı yoktur.

**[15] Anti-MITM (ARP Spoofing Protection)**
Yerel Adres Çözümleme Protokolü (ARP) tablolarını sürekli olarak izler. Aynı yerel ağdaki bir saldırgan bir ARP Spoofing/Zehirlenme saldırısı aracılığıyla trafiği engellemeye çalışırsa, bu modül MAC adresi anomalisini algılar, kullanıcıyı anında uyarır ve veri dinlenmesini önlemek için bağlantıyı keser.

**[16] Anti-Cold Boot (sdmem)**
Fiziksel cihaza el konulması sırasında, düşmanlar RAM modüllerini dondurarak şifreleme anahtarlarını ve hassas oturum belirteçlerini çıkarabilir (Cold-Boot Saldırısı). Etkinleştirildiğinde, ANON çerçevesinden çıkmak, sistem kapanmadan önce tüm boş RAM ve Swap (Takas) alanının rastgele verilerle kriptografik olarak üzerine yazıldığı `sdmem` (Güvenli Bellek Silme) işlemini tetikler.

**[17] Bluetooth Disabler**
Bluetooth radyoları benzersiz donanım kimlikleri yayınlar ve tarihsel olarak yakınlığa dayalı istismarlara (Blueborne gibi) karşı savunmasızdır. Bu modül, cihazdaki tüm Bluetooth denetleyicilerine sıkı bir donanım seviyesi bloğu uygulamak için `rfkill` ile etkileşime girer.

---

## 🖥️ Etkileşimli Terminal Kullanıcı Arayüzü (TUI)

ANON, hantal komut satırı parametrelerini tamamen Bash ile yazılmış zarif, duyarlı bir TUI ile değiştirir.

* **Dinamik Estetik Oluşturma:** TUI, her başlatıldığında dinamik olarak değişen, güzel biçimlendirilmiş, ortalanmış ASCII banner'lara sahiptir ve temiz, dikkat dağıtmayan bir ortam sağlar.
* **Canlı Kontrol Paneli (Live Dashboard):** Entegre bir gösterge paneli, Tor çıkış IP'nizi, aktif devre durumunuzu, ağ yapılandırmanızı ve aktif OPSEC modüllerinizi gerçek zamanlı olarak izlemenize olanak tanır.
* **Operasyon Merkezi:** Yön tuşlarınızı kullanarak doğrudan arayüzden DNS/IP sızıntı testleri yapın, yeni bir Tor kimliği (`SIGHUP` ile devreyi yeniden kurma) talep edin veya korumalı (sandboxed) tek kullanımlık tarayıcıyı başlatın.

---

## 📦 Sistem Gereksinimleri ve Bağımlılıklar

ANON, Linux ağ yığınına derinden entegredir. Gereksinimleri:

- **İşletim Sistemi:** Kali Linux, Parrot Security OS, Debian, Ubuntu veya WSL2 (netns desteği için özel Kernel önerilir).
- **İzinler:** `iptables`, kernel ağ alanlarını (namespaces) ve ağ arayüzlerini manipüle etmek için Root ayrıcalıkları (`sudo`) gereklidir.
- **Bağımlılıklar (Otomatik kurulur):** `tor`, `obfs4proxy`, `macchanger`, `proxychains4`, `secure-delete` (sdmem), `rfkill`, `firejail`, `iproute2`.

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

*Kurulum betiği, paket yöneticinizi güncelleyecek, gerekli tüm bağımlılıkları yükleyecek, `anon` çalıştırılabilir dosyasını `/usr/bin/` yolunuza yerleştirecek ve gerekli yedekleme dizinlerini oluşturacaktır.*

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
| `sudo anon --start` | Aracı, en son kaydedilen modül yapılandırmasını kullanarak arka planda sessizce başlatır. |
| `sudo anon --stop` | Tüm aktif modülleri anında devre dışı bırakır, iptables'ı temizler ve orijinal ağ durumunu geri yükler. |
| `sudo anon --status` | Tüm aktif ve pasif OPSEC modüllerinin kapsamlı bir görsel raporunu yazdırır. |
| `sudo anon --dashboard` | Canlı izleme panelini açar (her 3 saniyede bir otomatik yenilenir). |
| `sudo anon --browser` | RAM tabanlı tek kullanımlık tarayıcıyı doğrudan izole edilmiş ağ alanı (namespace) içinde başlatır. |
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

**S: Bu beni NSA'den veya devlet destekli aktörlerden gizler mi?**
C: Hiçbir yazılım mutlak bir dokunulmazlık sağlamaz. ANON belirli teknik vektörleri (DPI, DNS sızıntıları, OS parmak izi, yerel adli bilişim) hafifletir. Ancak istihbarat seviyesindeki bir düşman küresel trafik korelasyonuna, sıfır gün (zero-day) uç nokta açıklarına ve fiziksel gözetime dayanır. ANON'u, kapsamlı bir operasyonel güvenlik stratejinizin bir katmanı olarak kullanın.

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
