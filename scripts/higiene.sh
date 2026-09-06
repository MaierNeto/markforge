#!/usr/bin/env bash
# higiene.sh — prova que o staging está limpo (contrato mínimo A-SDLC)
# Uso: npm run higiene  OU  ./scripts/higiene.sh  (da raiz do projeto)

set -uo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "🔍 A-SDLC Higiene — verificando staging e disco..."

FS_PATTERN='(\.env$|\.env\.|\.pem$|\.key$|\.keystore$|\.jks$|\.p12$|service-account.*\.json$|_apagar/|credentials\.json$|secrets\.yaml$|secret.*\.json$)'

# Arquivos staged (já rastreados - não precisam de .gitignore)
STAGED=$(git status --porcelain 2>/dev/null | grep -E '^[AM]' | awk '{print $2}' || true)

# Arquivos untracked que casam padrão sensível (precisam de .gitignore)
UNTRACKED_SENSITIVE=$(git status --porcelain 2>/dev/null | grep -E '^\?\?' | awk '{print $2}' | grep -E "$FS_PATTERN" || true)

# Varredura de disco para padrões sensíveis
FS_SCAN=$(find . -path './.git' -prune -o -type f -print 2>/dev/null | grep -E "$FS_PATTERN" || true)

# Filtrar .example (intencionalmente versionados) do FS_SCAN
FS_SCAN=$(echo "$FS_SCAN" | grep -v '\.example$' || true)

# Combinar apenas untracked sensíveis + disk scan para verificar .gitignore
TO_CHECK=$(printf '%s\n%s\n' "$UNTRACKED_SENSITIVE" "$FS_SCAN" | sed '/^$/d' | sort -u)

FOUND=0

if [ -z "$TO_CHECK" ] && [ -z "$STAGED" ]; then
    echo -e "${GREEN}✓${NC} Nenhum arquivo sensível candidato no disco/staging"
else
    if [ -n "$STAGED" ]; then
        echo "Arquivos staged (já rastreados, OK):"
        echo "$STAGED" | sed 's/^/  /'
    fi
    if [ -n "$TO_CHECK" ]; then
        echo "Arquivos untracked/disco a verificar contra .gitignore:"
        echo "$TO_CHECK" | sed 's/^/  /'
    fi
fi

echo "🔍 Validando blindagem por .gitignore (apenas untracked/disco)..."
while IFS= read -r file; do
    [ -z "$file" ] && continue
    if git check-ignore --no-index "$file" >/dev/null 2>&1; then
        echo -e "  ${GREEN}✓${NC} $file → ignorado (protegido)"
    else
        echo -e "  ${RED}✗${NC} $file → NÃO protegido pelo .gitignore (risco)"
        FOUND=1
    fi
done <<< "$TO_CHECK"

echo "🔍 Validando configuração do .gitignore (padrões-chave)..."
IGNORE_TEST_FILES=(".env" "secrets/key.pem" "config/service-account.json" "tmp/dump.sql")
for test_file in "${IGNORE_TEST_FILES[@]}"; do
    if git check-ignore --no-index "$test_file" >/dev/null 2>&1; then
        echo -e "  ${GREEN}✓${NC} padrão cobre $test_file"
    else
        echo -e "  ${RED}✗${NC} .gitignore NÃO cobre '$test_file' (revisar)"
        FOUND=1
    fi
done

if [ -n "$STAGED" ]; then
    echo "🔍 Varrendo conteúdo por segredos em defaults..."
    while IFS= read -r file; do
        [ -z "$file" ] && continue
        [ -f "$file" ] || continue
        if grep -nE '\$\{[A-Z_]+:[^}]*\}' "$file" 2>/dev/null | grep -vE '(placeholder|exemplo|example|CHANGE_ME|your_|<.*>|^\s*#)' >/dev/null; then
            echo -e "  ${YELLOW}⚠${NC} $file: possíveis defaults com valor real em \${VAR:valor}"
            FOUND=1
        fi
    done <<< "$STAGED"
fi

if [ "$FOUND" -eq 0 ]; then
    echo -e "${GREEN}✓ HIGIENE OK${NC} — staging e disco limpos"
    exit 0
else
    echo -e "${RED}✗ HIGIENE FALHOU${NC} — corrija antes de commitar"
    exit 1
fi