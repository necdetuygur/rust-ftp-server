#!/usr/bin/env bash
set -e

RELEASE_FILE="release"

if [ ! -f "$RELEASE_FILE" ]; then
  echo "Hata: '$RELEASE_FILE' dosyası bulunamadı."
  exit 1
fi

# Mevcut sürümü oku ve boşluk/satır sonlarını temizle (örn: v0.2.2)
CURRENT_VERSION=$(tr -d '[:space:]' < "$RELEASE_FILE")

# vX.Y.Z formatını doğrula ve parçala
if [[ ! "$CURRENT_VERSION" =~ ^v([0-9]+)\.([0-9]+)\.([0-9]+)$ ]]; then
  echo "Hata: Geçersiz sürüm formatı ('$CURRENT_VERSION'). Beklenen format: v0.2.2"
  exit 1
fi

MAJOR="${BASH_REMATCH[1]}"
MINOR="${BASH_REMATCH[2]}"
PATCH="${BASH_REMATCH[3]}"

# Patch sürümünü 1 artır
NEW_PATCH=$((PATCH + 1))
NEW_VERSION="v${MAJOR}.${MINOR}.${NEW_PATCH}"

echo "Sürüm güncelleniyor: $CURRENT_VERSION -> $NEW_VERSION"

# release dosyasını yeni sürümle güncelle
printf "%s\n" "$NEW_VERSION" > "$RELEASE_FILE"

# Git işlemlerini çalıştır
git add . && git commit -m "Release" && git push
git tag "$NEW_VERSION" && git push origin "$NEW_VERSION"

echo "Tamamlandı: $NEW_VERSION yayınlandı."
