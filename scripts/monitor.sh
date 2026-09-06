#!/usr/bin/env bash
# monitor.sh - markForge (React + Tauri desktop app)
# Monitora estado do projeto

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

echo "📊 Monitor markForge — React + Tauri Desktop App"

# Git status
if [ -d .git ]; then
    echo "  📦 Git: inicializado"
    git status --short | head -10 | sed 's/^/    /'
else
    echo "  📦 Git: não inicializado"
fi

# Node.js status
if [ -f package.json ]; then
    echo "  📦 Node.js: package.json presente"
    if [ -d node_modules ]; then
        echo "  📦 node_modules: instalado"
    else
        echo "  📦 node_modules: não instalado"
    fi
else
    echo "  📦 Node.js: package.json não encontrado"
fi

# Rust/Tauri status
if [ -f src-tauri/Cargo.toml ]; then
    echo "  🦀 Rust/Tauri: Cargo.toml presente"
    if command -v cargo >/dev/null 2>&1; then
        echo "  🦀 cargo: disponível"
    else
        echo "  🦀 cargo: não instalado"
    fi
else
    echo "  🦀 Rust/Tauri: não configurado"
fi

# Scripts
echo "  🔧 Scripts de higiene/smoke/monitor: disponíveis"

exit 0