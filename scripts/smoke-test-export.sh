#!/usr/bin/env bash
# Smoke test do pipeline de exportação (DOCX via reference.docx, PDF via
# Pandoc+Typst). Roda os mesmos comandos que o Markforge executa em Rust,
# usando os binários "sidecar" baixados no workflow, para pegar erros de
# template ANTES de gastar minutos compilando o app inteiro.
#
# Uso: scripts/smoke-test-export.sh <pandoc_bin> <typst_bin>
set -euo pipefail

PANDOC_BIN="$1"
TYPST_BIN="$2"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURE="$ROOT_DIR/scripts/smoke-test-fixture.md"
OUT_DIR="$(mktemp -d)"

# O Pandoc só aceita --pdf-engine quando o *nome do arquivo* (basename) do
# binário é exatamente um dos motores conhecidos (ex.: "typst"). Os binários
# baixados pelos scripts fetch-sidecars-*.sh mantêm o sufixo de target triple
# no nome (ex.: typst-x86_64-unknown-linux-gnu), exatamente como o Tauri
# exige em src-tauri/binaries/ — mas isso faz o Pandoc rejeitar o binário
# (exit code 6: "pdf-engine must be one of ..."). No app final o Tauri
# remove esse sufixo ao empacotar o sidecar, então esse problema não existe
# em produção — mas aqui, testando o binário baixado cru, precisamos criar
# um link com o nome "puro" antes de invocar o Pandoc.
TYPST_LINK="$OUT_DIR/typst"
ln -s "$(cd "$(dirname "$TYPST_BIN")" && pwd)/$(basename "$TYPST_BIN")" "$TYPST_LINK"

echo "==> Testando exportação DOCX (Pandoc + reference.docx)"
"$PANDOC_BIN" "$FIXTURE" \
  --from markdown+yaml_metadata_block+raw_attribute-citations \
  --reference-doc="$ROOT_DIR/templates/default/reference.docx" \
  --standalone \
  -o "$OUT_DIR/smoke.docx"

if [ ! -s "$OUT_DIR/smoke.docx" ]; then
  echo "FALHA: smoke.docx não foi gerado ou está vazio"
  exit 1
fi
echo "OK: $(du -h "$OUT_DIR/smoke.docx" | cut -f1) — $OUT_DIR/smoke.docx"

echo "==> Testando exportação PDF (Pandoc + Typst + pdf-template.typ)"
"$PANDOC_BIN" "$FIXTURE" \
  --from markdown+yaml_metadata_block-citations \
  --template="$ROOT_DIR/templates/default/pdf-template.typ" \
  --pdf-engine="$TYPST_LINK" \
  --metadata has-cover:true \
  -o "$OUT_DIR/smoke.pdf"

if [ ! -s "$OUT_DIR/smoke.pdf" ]; then
  echo "FALHA: smoke.pdf não foi gerado ou está vazio"
  exit 1
fi
echo "OK: $(du -h "$OUT_DIR/smoke.pdf" | cut -f1) — $OUT_DIR/smoke.pdf"

echo "==> Smoke test passou. Limpando $OUT_DIR"
rm -rf "$OUT_DIR"
