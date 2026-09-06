#!/usr/bin/env bash
# smoke.sh — checagem rápida "o sistema acende?" (A-SDLC) para markForge
# Uso: npm run smoke  OU  ./smoke.sh  (da raiz do projeto markforge)

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "💨 A-SDLC Smoke Test — markForge (React + Tauri)"

# Verificar se package.json existe
if [ ! -f package.json ]; then
    echo "  ✗ package.json não encontrado em $REPO_ROOT"
    exit 1
fi

# Verificar se node_modules existe
if [ ! -d node_modules ]; then
    echo "  ⚠ node_modules não encontrado — rodando npm install"
    npm ci 2>&1 | tail -10
fi

# Build check (TypeScript + Vite)
echo "  🔨 Verificando build (tsc + vite build)..."
if npm run build 2>&1 | tail -20; then
    echo -e "  \033[0;32m✓\033[0m Build OK"
else
    echo -e "  \033[0;31m✗\033[0m Build falhou"
    exit 1
fi

# Testes rápidos (Vitest)
echo "  🧪 Rodando testes (vitest)..."
if npm test 2>&1 | tail -20; then
    echo -e "  \033[0;32m✓\033[0m Testes OK"
else
    echo -e "  \033[0;31m✗\033[0m Testes falharam"
    exit 1
fi

# Verificar Rust/Tauri se Cargo.toml existe
if [ -f src-tauri/Cargo.toml ]; then
    echo "  🦀 Verificando Rust/Tauri..."
    if command -v cargo >/dev/null 2>&1; then
        cd src-tauri
        if cargo check 2>&1 | tail -10; then
            echo -e "  \033[0;32m✓\033[0m Cargo check OK"
        else
            echo -e "  \033[0;31m✗\033[0m Cargo check falhou"
            exit 1
        fi
        cd ..
    else
        echo "  ⚠ cargo não encontrado — pulando verificação Rust"
    fi
fi

echo -e "\033[0;32m✓ SMOKE OK\033[0m — markForge build e testes passando"
exit 0