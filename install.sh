#!/bin/sh
set -e

REPO="JOBOYA/ghostline"
VERSION=$(curl -sf "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)

if [ -z "$VERSION" ]; then
  echo "Error: could not determine latest version" >&2
  exit 1
fi

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS-$ARCH" in
  linux-x86_64)   FILE="ghostline-linux-x86_64" ;;
  darwin-arm64)   FILE="ghostline-macos-arm64" ;;
  darwin-x86_64)  FILE="ghostline-macos-x86_64" ;;
  *) echo "Unsupported platform: $OS-$ARCH" >&2; exit 1 ;;
esac

BASE_URL="https://github.com/$REPO/releases/download/$VERSION"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

echo "Downloading ghostline $VERSION for $OS/$ARCH..."
curl -fL "$BASE_URL/$FILE"         -o "$TMP_DIR/ghostline"
curl -fL "$BASE_URL/SHA256SUMS"    -o "$TMP_DIR/SHA256SUMS"

# Verify checksum
cd "$TMP_DIR"
EXPECTED=$(grep "$FILE" SHA256SUMS | cut -d' ' -f1)
if [ -z "$EXPECTED" ]; then
  echo "Error: no checksum found for $FILE in SHA256SUMS" >&2
  exit 1
fi

if command -v sha256sum >/dev/null 2>&1; then
  ACTUAL=$(sha256sum ghostline | cut -d' ' -f1)
elif command -v shasum >/dev/null 2>&1; then
  ACTUAL=$(shasum -a 256 ghostline | cut -d' ' -f1)
else
  echo "Warning: no sha256sum or shasum found — skipping checksum verification" >&2
  ACTUAL="$EXPECTED"
fi

if [ "$ACTUAL" != "$EXPECTED" ]; then
  echo "Error: checksum mismatch — binary may be corrupted or tampered" >&2
  echo "  expected: $EXPECTED" >&2
  echo "  got:      $ACTUAL" >&2
  exit 1
fi

echo "Checksum verified."
install -m 755 ghostline "$INSTALL_DIR/ghostline"
echo "Installed ghostline $VERSION to $INSTALL_DIR/ghostline"
