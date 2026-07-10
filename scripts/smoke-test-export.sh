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

echo "==> Testando exportação DOCX (Pandoc + reference.docx)"
"$PANDOC_BIN" "$FIXTURE" \
  --from markdown+yaml_metadata_block+raw_attribute \
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
  --from markdown+yaml_metadata_block \
  --template="$ROOT_DIR/templates/default/pdf-template.typ" \
  --pdf-engine="$TYPST_BIN" \
  --metadata has-cover:true \
  -o "$OUT_DIR/smoke.pdf"

if [ ! -s "$OUT_DIR/smoke.pdf" ]; then
  echo "FALHA: smoke.pdf não foi gerado ou está vazio"
  exit 1
fi
echo "OK: $(du -h "$OUT_DIR/smoke.pdf" | cut -f1) — $OUT_DIR/smoke.pdf"

echo "==> Smoke test passou. Limpando $OUT_DIR"
rm -rf "$OUT_DIR"
