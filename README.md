# Rust FTP Server

`config.json` ile yönetilen, **kullanıcı başına ayrı kök dizini** olan çalışan bir FTP sunucusu.
[libunftp](https://docs.rs/libunftp) + `unftp-sbe-fs` kullanır.

## Neler düzeltildi?

- Eskiden `main.rs` sadece config doğrulayıp çıkıyordu; artık gerçekten dinleyici açıyor.
- Çoklu kullanıcı desteği eklendi. Her kullanıcı kendi dizinine hapsedilir:
  `UserDetail::home()` + `Filesystem::enter()` mekanizması kullanılır.
- Config yolu artık esnek: `--config`, `FTP_CONFIG` ortam değişkeni, çalışma dizini,
  executable yanı sırasıyla denenir (Docker'da `/app/config.json` bulunur).
- Docker imajı ayrıcalıksız kullanıcı (UID 1000) ile çalışır, Fedora/SELinux için `:z`
  montajları ve healthcheck vardır.
- Portlar varsayılan olarak sadece `127.0.0.1`'e bağlanır.

## Hızlı başlangıç (Fedora + Docker)

```bash
cd ~/Downloads/rust-ftp-server
docker compose build
docker compose up -d
docker compose ps          # healthy olmalı
docker compose logs -f ftp # CTRL+C ile çık
```

Örnek hesaplar:

| Kullanıcı | Şifre               | Salt gezinen dizin | Host dizini |
|-----------|---------------------|--------------------|-------------|
| `media`   | `media-demo-2026`   | `/data/MOV`        | `./MOV`     |
| `public`  | `public-demo-2026`  | `/data/public`     | `./public`  |

Bağlantı (passive mod varsayılandır):

```bash
ftp localhost 2121
# veya
lftp -p 2121 -u media,media-demo-2026 localhost
```

Python ile hızlı doğrulama:

```bash
python3 - <<'PY'
from ftplib import FTP
f = FTP(); f.connect('127.0.0.1', 2121); f.login('media', 'media-demo-2026')
print(f.pwd(), f.nlst())
f.storbinary('STOR deneme.txt', b'merhaba\n')
print(f.retrbinary('RETR deneme.txt', print))
f.delete('deneme.txt'); f.quit()
PY
```

## Yapılandırma (`config.json`)

```json
{
  "host": "0.0.0.0",
  "port": 2121,
  "passive_host": "127.0.0.1",
  "paths": [
    { "path": "/data/MOV",    "user": "media",  "password": "media-demo-2026" },
    { "path": "/data/public", "user": "public", "password": "public-demo-2026" }
  ],
  "passive_port_start": 50000,
  "passive_port_end": 50100
}
```

| Alan | Açıklama |
|------|----------|
| `host`, `port` | Dinlenecek adres. Konteyner içinde `0.0.0.0` olmalı. |
| `paths[].path` | Kullanıcının kök dizini. Başlatma sırasında var olmalı. |
| `paths[].user` | Benzersiz kullanıcı adı. |
| `paths[].password` | Düz metin şifre; değiştirin. |
| `passive_host` | PASV yanıtında bildirilen adres. Docker/NAT için `127.0.0.1`; LAN'dan bağlanacaksanız makinenin IP'sini yazın veya alanı silin (bağlantı IP'si kullanılır). |
| `passive_port_start/end` | Veri bağlantıları için port aralığı; compose dosyasında aynı aralık publish edilir. |

Doğrulama modu (sunucuyu başlatmadan config'i kontrol eder):

```bash
docker compose exec ftp rust-ftp-server --check
# veya konteyner dışında: cargo run -- --check
```

## Yerel derleme

Rust 1.85+ gerekir (edition 2024):

```bash
cargo build --release
./target/release/rust-ftp-server --config ./config.json --check
./target/release/rust-ftp-server --config ./config.json
```

## Güvenlik notları

- Düz FTP şifreleri ve veriyi şifrelemez. Şu an FTPS kapalı; sadece localhost veya güvenilir LAN kullanın.
- compose portları `127.0.0.1` ile sınırlar. Dışarı açmak için bilinçli olarak `2121:2121` yapın ve mutlaka FTPS ekleyin.
- Örnek şifreleri değiştirin. Yeni kullanıcı/dizin eklediğinizde compose'a ilgili volume satırını da ekleyin.
- Kullanıcı dizinleri `/` (filesystem root) olamaz; sunucu bunu reddeder.
- Her oturumun depolama kökü girişten sonra kullanıcının dizinine daraltılır, böylece kullanıcılar birbirinin dosyalarını göremez.

## Sorun giderme

Bu makinede varsayılan buildx sürücüsü (`kiri-build`) hata veriyorsa
(`no valid drivers found: unable to parse docker host 'kiri'`), derlemeyi şöyle tetikleyin:

```bash
BUILDX_BUILDER=default docker compose build
```

## Yayın workflow'u

`v*` tag'i gönderildiğinde `.github/workflows/release.yml` Windows, Linux ve macOS
(x86_64 + aarch64) için ikili dosyaları `config.json` ile birlikte paketler.
