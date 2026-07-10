#!/usr/bin/env bash
# Baixa os binários oficiais de Pandoc e Typst (releases mais recentes do
# GitHub) e os coloca em src-tauri/binaries/ com o nome de sidecar que o
# Tauri espera (<nome>-<target-triple>). Roda em runners Linux (x86_64).
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BIN_DIR="$ROOT_DIR/src-tauri/binaries"
mkdir -p "$BIN_DIR"

TARGET="$(rustc -vV | grep 'host:' | sed 's/host: //')"
WORK="$(mktemp -d)"
cd "$WORK"

echo "==> Target triple: $TARGET"

echo "==> Resolvendo última release do Pandoc"
PANDOC_URL=$(curl -sL https://api.github.com/repos/jgm/pandoc/releases/latest \
  | jq -r '.assets[] | select(.name | test("linux-amd64\\.tar\\.gz$")) | .browser_download_url')
if [ -z "$PANDOC_URL" ]; then
  echo "FALHA: não encontrei o asset linux-amd64.tar.gz do Pandoc"
  exit 1
fi
echo "    $PANDOC_URL"
curl -sL "$PANDOC_URL" -o pandoc.tar.gz
tar xzf pandoc.tar.gz
PANDOC_BIN="$(find . -maxdepth 3 -type f -path '*/bin/pandoc' | head -n1)"
cp "$PANDOC_BIN" "$BIN_DIR/pandoc-$TARGET"
chmod +x "$BIN_DIR/pandoc-$TARGET"
echo "==> Pandoc instalado em $BIN_DIR/pandoc-$TARGET"
"$BIN_DIR/pandoc-$TARGET" --version | head -1

echo "==> Resolvendo última release do Typst"
TYPST_URL=$(curl -sL https://api.github.com/repos/typst/typst/releases/latest \
  | jq -r '.assets[] | select(.name | test("x86_64-unknown-linux-musl\\.tar\\.xz$")) | .browser_download_url')
if [ -z "$TYPST_URL" ]; then
  echo "FALHA: não encontrei o asset x86_64-unknown-linux-musl.tar.xz do Typst"
  exit 1
fi
echo "    $TYPST_URL"
curl -sL "$TYPST_URL" -o typst.tar.xz
tar xJf typst.tar.xz
TYPST_BIN="$(find . -maxdepth 3 -type f -name 'typst' | head -n1)"
cp "$TYPST_BIN" "$BIN_DIR/typst-$TARGET"
chmod +x "$BIN_DIR/typst-$TARGET"
echo "==> Typst instalado em $BIN_DIR/typst-$TARGET"
"$BIN_DIR/typst-$TARGET" --version

cd "$ROOT_DIR"
rm -rf "$WORK"
