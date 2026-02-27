#!/bin/sh
set -e

REPO="JOBOYA/ghostline"
VERSION=$(curl -sf "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)

if [ -z "$VERSION" ]; then
  echo "Error: could not determine latest version"
  exit 1
fi

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS-$ARCH" in
  linux-x86_64)   FILE="ghostline-linux-x86_64" ;;
  darwin-arm64)   FILE="ghostline-macos-arm64" ;;
  darwin-x86_64)  FILE="ghostline-macos-x86_64" ;;
  *) echo "Unsupported platform: $OS-$ARCH"; exit 1 ;;
esac

URL="https://github.com/$REPO/releases/download/$VERSION/$FILE"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

echo "Installing ghostline $VERSION for $OS/$ARCH..."
curl -fL "$URL" -o "$INSTALL_DIR/ghostline"
chmod +x "$INSTALL_DIR/ghostline"
echo "✓ Installed ghostline $VERSION to $INSTALL_DIR/ghostline"
