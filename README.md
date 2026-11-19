# Rust FTP Server

Küçük, ayar dosyasıyla yönetilen bir FTP sunucusu. Her kullanıcı yalnızca kendi
klasörünü görür; birbirinin dosyalarına erişemez.

## Özellikler

- Kullanıcı adı / şifre doğrulama
- Kullanıcı başına ayrı, kilitli kök klasör
- Aktarım için ayarlanabilir pasif port aralığı
- Tek dosyada (`config.json`) basit yapılandırma
- Windows, Linux ve macOS için hazır derlemler

---

## 1. Derlemi indirin

Projenin GitHub deposundaki **Releases** bölümüne gidin ve işletim sisteminize
uygun dosyayı indirin:

| İşletim sistemi | Dosya |
|---|---|
| Windows 64-bit | `rust-ftp-server-x86_64-pc-windows-msvc.zip` |
| Linux 64-bit | `rust-ftp-server-x86_64-unknown-linux-gnu.tar.gz` |
| macOS (Apple Silicon) | `rust-ftp-server-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `rust-ftp-server-x86_64-apple-darwin.tar.gz` |

Arşivi bir klasöre çıkarın. İçinde şunlar olmalı:

```
rust-ftp-server        (Windows'ta rust-ftp-server.exe)
config.json
README.md
LICENSE
```

## 2. Klasörleri oluşturun ve `config.json` dosyasını düzenleyin

Sunucuyu çalıştırmadan önce, ayarlarda yazan klasörleri oluşturun. Varsayılan
ayarlar `data/media` ve `data/public` klasörlerini kullanır:

```bash
mkdir -p data/media data/public
```

Windows'ta aynı klasörleri **Dosya Gezgini**'nden oluşturabilirsiniz.

Birlikte gelen `config.json` şablonu aşağıdaki gibidir:

```json
{
  "host": "0.0.0.0",
  "port": 2121,
  "paths": [
    {
      "path": "./data/media",
      "user": "CHANGE_ME_USER",
      "password": "CHANGE_ME_PASSWORD"
    },
    {
      "path": "./data/public",
      "user": "CHANGE_ME_USER_2",
      "password": "CHANGE_ME_PASSWORD_2"
    }
  ],
  "passive_port_start": 50000,
  "passive_port_end": 50100
}
```

| Alan | Ne yapmalısınız? |
|---|---|
| `host` | `0.0.0.0` tüm ağ arayüzlerinde dinler. Yalnızca yerel bağlantılara izin vermek için `127.0.0.1` yapın. |
| `port` | FTP kontrol portu. Varsayılan `2121`; değiştirirseniz FTP istemcisinde de değiştirin. |
| `paths[].path` | Kullanıcının kök klasörü. Göreli yol verirseniz programın çalıştırıldığı klasöre göre çözülür. Klasör önceden var olmalı. |
| `paths[].user` | Kullanıcı adı. Her kullanıcı için farklı olmalı. |
| `paths[].password` | Şifre. `CHANGE_ME` yazanları mutlaka değiştirin. |
| `passive_port_start` / `passive_port_end` | Dosya aktarımında kullanılan veri portları. Başka bir makineden bağlanıyorsanız güvenlik duvarında bu aralığı açın. |
| `passive_host` *(isteğe bağlı)* | PASV yanıtında bildirilen adres. Docker/NAT kullanıyorsanız erişilebilir IP'yi yazın; yazmazsanız bağlantı adresi kullanılır. |

Daha fazla kullanıcı eklemek için `paths` dizisine yeni bir nesne ekleyin. Aynı
kullanıcı adını iki kez kullanamazsınız.

Ayarları doğrulamak için (sunucuyu başlatmadan):

```bash
./rust-ftp-server --check
```

## 3. Çalıştırın

**Linux / macOS**

```bash
chmod +x ./rust-ftp-server
./rust-ftp-server
```

**Windows**

`rust-ftp-server.exe` dosyasına çift tıklayın. Konsol penceresi açık kalır;
sunucuyu kapatmak için pencereyi kapatın veya `Ctrl+C`'ye basın.
PowerShell kullanmayı tercih ederseniz:

```powershell
.\rust-ftp-server.exe
```

Başarılı başlatmada şuna benzer bir çıktı görürsünüz:

```
Listening on: ftp://0.0.0.0:2121
Storage root: /.../data
  media       -> /.../data/media
  public      -> /.../data/public
```

Farklı bir ayar dosyası kullanmak için: `./rust-ftp-server --config /yol/config.json`

## 4. Bağlanın

| Yöntem | Ayar |
|---|---|
| FileZilla | Host: `127.0.0.1`, Port: `2121`, Kullanıcı/Şifre: `config.json` içindekiler, Şifreleme: düz FTP |
| Komut satırı | `ftp 127.0.0.1 2121` |
| lftp | `lftp -p 2121 -u KULLANICI,SIFRE 127.0.0.1` |

Bağlandıktan sonra `ls` ile kendi klasörünüzü görebilir, `put`/`get` ile dosya
yükleyip indirebilirsiniz.

---

## Docker ile çalıştırma (isteğe bağlı)

Depoyu klonladıktan sonra:

```bash
docker compose up -d --build
docker compose logs -f ftp
```

Compose dosyası portları yalnızca `127.0.0.1`'e bağlar, pasif port aralığını
yayınlar ve `config.json` dosyasını okur. LAN'a açmak için port eşlemelerini,
`host` ayarını ve `FTP_PASSIVE_HOST` değerini birlikte güncelleyin.

Durdurmak için: `docker compose down`

## Güvenlik

- Düz FTP şifreleri ve dosyaları şifrelemez. Yalnızca güvendiğiniz bir ağda kullanın.
- Şablon şifrelerini değiştirin; uzun ve benzersiz şifreler seçin.
- Başka makinelerden erişecekseniz pasif port aralığını güvenlik duvarında açın.
- Kök klasör olarak `/` (diskin tamamı) kullanılamaz; sunucu bunu reddeder.
- Her oturum girişten sonra kullanıcının kendi klasörüne daraltılır.

## Geliştiriciler için

Rust 1.85+ gerekir:

```bash
cargo build --release
./target/release/rust-ftp-server --check
```

Sürüm etiketi (`v*`) gönderildiğinde GitHub Actions Windows, Linux ve macOS
derlemlerini otomatik olarak Releases bölümüne yükler.

## Lisans

MIT — bkz. `LICENSE`.
