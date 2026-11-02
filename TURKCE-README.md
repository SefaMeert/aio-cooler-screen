# AIO Cooler Screen

Bu depo, [Thnorty/aio-cooler-screen](https://github.com/Thnorty/aio-cooler-screen) projesinin **SefaMeert** tarafından ek geliştirmeler yapılan bir fork'udur. Uyumlu Monster AIO sıvı soğutucuların LCD ekranları (USB HID `1D6B:0130`) için hazırlanmıştır.

[English README](README.md) · [Orijinal proje](https://github.com/Thnorty/aio-cooler-screen) · [Derleme rehberi](FORK-BUILD-README.md)

> **Proje durumu:** Windows derlemesi oluşturulmuş, kurulum dosyası bir test bilgisayarında açılmış ve uygulama denenmiştir. Tüm donanımlarda, fontlarda ve LCD yapılandırmalarında tam uyumluluk henüz doğrulanmamıştır. **Dağıtılan uygulama dosyaları şu anda dijital imzalı değildir.** Windows Smart App Control engelleyebilir; kurulum için Windows güvenlik korumalarını kapatmayın.

![Ekran tasarımları](docs/designs.png)

## Bu fork'taki geliştirmeler

- **Genişletilmiş donanım sensörleri:** Donanımın desteklediği fan/pompa RPM, voltaj, güç, akım ve frekans gibi LibreHardwareMonitor değerleri.
- **Daha anlaşılır sensör seçimi:** Sensör türü/kategorisi etiketleri; mevcut tasarımların sensör kimliklerini koruma hedefi.
- **İngilizce ve Türkçe arayüz:** Varsayılan İngilizce; dil tercihi kaydedilebilir. Donanımdan gelen sensör isimleri kendi dilinde kalabilir.
- **Ek fontlar:** Google Sans, Google Sans Flex, Inter, Rajdhani, Orbitron, JetBrains Mono, Roboto Condensed ve Oswald. Fontlar lisanslarıyla birlikte kullanıcı kurulumunda değil, Windows kurulum paketi derlenirken indirilir. Ayrıntılar: [FONT-README.md](FONT-README.md).
- **GitHub Actions:** Tam Windows kurulum paketi ve ileri düzey kullanıcılar için tek başına EXE dosyası üretimi.

## Orijinal projeden gelen özellikler

- 480×480 LCD için hazır ve özel görsel, video ve GIF arka planları.
- Hazır tasarımlar, özel düzenleyici, bilgi katmanları, göstergeler, yazılar ve ilerleme çubukları.
- Sıcaklık/yük verileri ve HWiNFO ile LibreHardwareMonitor entegrasyonu.
- LCD ekranını güncelleyen Windows arka plan hizmeti.

## Kurulum

64 bit Windows 10/11 gerekir. Yeni kurulumda **tam kurulum paketini** kullanın. Bağımsız `aio-screen.exe`, arka plan hizmetini ve yardımcı bileşenleri kurmaz. LCD'yi iki programın aynı anda kontrol etmemesi için orijinal Monster LCD uygulamasını kapatın.

**Windows güvenliği:** İmzasız kurulum dosyaları Smart App Control tarafından engellenebilir. Kaynak kodunun açık olması veya SHA-256 değerinin yayımlanması tek başına ikili dosyanın güvenli olduğunu kanıtlamaz. İndirdiğiniz sürümün kaynağını doğrulayın ve Windows güvenlik özelliklerini açık tutun.

[Derleme ve çıktı dosyaları rehberi](FORK-BUILD-README.md)

## Donanım sensörleri

LibreHardwareMonitor uygulamasını yönetici olarak çalıştırın; **Options → Remote Web Server → Run** özelliğini etkinleştirin ve AIO Cooler Screen'de `8085` portunu kullanın. Yalnızca donanım ve izleme yazılımının sunduğu sensörler listelenir. Fan RPM değerinin gösterilmesi, **fan hızı kontrolü** anlamına gelmez.

## Katkılar ve lisanslar

Bu depo, **Thnorty'nin AIO Cooler Screen** projesi temel alınarak geliştirilmiştir. Orijinal telif hakları ve [MIT lisansı](LICENSE) korunmuştur. Yardımcı bileşenler ve fontlar için [üçüncü taraf bildirimlerine](THIRD-PARTY-NOTICES.md) ve [font rehberine](FONT-README.md) bakın.

Bu fork orijinal geliştiricinin veya Monster Notebook'un resmî sürümü değildir; Monster Notebook ya da Google tarafından desteklenmez. İlgili marka adları sahiplerine aittir.
