#!/usr/bin/env bash
# ci_local — o portao do Markforge roda nesta estacao, no ambiente dev.
#
# RFC-013 (diretiva do titular, 25/10/2026): o GitHub e SO ARMAZENAMENTO — sem
# Actions, sem CI na nuvem, sem bot. A prova de entrega passou a ser esta
# corrida, conferida aqui, e NAO um run na nuvem.
#
# Este portao transcreve os jobs uteis do workflow aposentado
# (.github/workflows/ci.yml) SEM reimplementar gate nenhum: cada job delega no
# MESMO instrumento que o step da nuvem rodava (gatilho e so gatilho). O release
# (.github/workflows/release.yml, que publicava Release no GitHub em push de tag
# v*) foi aposentado — publicacao fora do repo e ato do titular, nao do portao.
#
# Veredito honesto (item 5 e 9 do AGENTS global): cada job e PASS (0), FAIL (1),
# NAO-EXECUTADO (2) ou LAUDO-MONITORADO (3). Ferramenta ausente, servico que nao
# sobe ou consulta que falha NUNCA e verde — o job nomeia a causa. FAIL bloqueia a
# entrega. NAO-EXECUTADO nunca vira verde presumido: fica pendencia lavrada no
# registro. LAUDO-MONITORADO e o modo baseline do item 13 (portao novo nasce com
# baseline): achado PRE-EXISTENTE, contado no arquivo versionado
# scripts/ci_local.baseline com commit de origem e decisao pendente do dono, nao
# trava para frente; sitio alem do contado, arquivo novo ou candidato novo = FAIL;
# job vermelho sem baseline legivel = VIGIA_CEGO, nunca "limpo". Nunca
# --no-verify, nunca enfraquecer gate para passar.
#
# Codigo e dados ficam na estacao; a cadeia de ferramentas do desktop (Node,
# Python, sidecars Pandoc/Typst em src-tauri/binaries) ja e o ambiente dev
# ordinario deste app desktop — nao se instala stack no desktop para um job
# passar (diretiva do titular, 25/10). O que precisa de ambiente que nao existe
# aqui — o compilador C (cl.exe, ausente nesta estacao) para os crates que o
# exigem, e o Linux de paridade via docker/Dockerfile (INCOMPLETO: falta
# validar-linux.sh, parado 13/08 a pedido do titular) — e NAO-EXECUTADO com a
# causa nomeada, nunca verde por imaginacao.
#
# Uso:
#   bash scripts/ci_local.sh                 # todos os jobs
#   bash scripts/ci_local.sh --apenas <nome> # so jobs cujo nome casa com <nome>
#   bash scripts/ci_local.sh --dry-run       # so o inventario dos jobs
#   bash scripts/ci_local.sh --list          # lista os nomes e sai
set -u
cd "$(git rev-parse --show-toplevel)" || { echo "[ci_local] nao estamos dentro de um repo git"; exit 2; }
export PYTHONIOENCODING=utf-8

BASELINE="scripts/ci_local.baseline"
DECISAO_BASELINE="decisao pendente do dono, ver cabecalho de $BASELINE"

# ---------------------------------------------------------------- argparse
DRY=0
APS=""
LIST=0
while [ $# -gt 0 ]; do
  case "$1" in
    --dry-run) DRY=1 ;;
    --list) LIST=1 ;;
    --apenas) shift; [ $# -gt 0 ] || { echo "[ci_local] --apenas sem valor"; exit 2; }; APS="$1" ;;
    --apenas=*) APS="${1#--apenas=}" ;;
    --help|-h)
      echo "uso: ci_local.sh [--dry-run|--list] [--apenas <trecho-do-nome>]"; exit 0 ;;
    *) echo "[ci_local] argumento desconhecido: $1"; exit 2 ;;
  esac
  shift
done

# ---------------------------------------------------------------- achado de ferramentas
if command -v python3 >/dev/null 2>&1; then PY=python3
elif command -v python >/dev/null 2>&1; then PY=python
else PY=""; fi

need() {  # need <tool> -> rc 2 se ausente; NAO pulamos silencioso: ferramenta ausente e causa
  command -v "$1" >/dev/null 2>&1 || { echo "  NAO-EXECUTADO: falta '$1' nesta estacao"; return 2; }
  return 0
}

need_py() {  # python3 ou python; a estacao so tem `python`
  [ -n "$PY" ] || { echo "  NAO-EXECUTADO: falta interpretador python (python3/python)"; return 2; }
  return 0
}

# alvo do rustc (host triple) e o sufixo .exe dos sidecars no Windows
rust_host() { need rustc || return 2; rustc -vV 2>/dev/null | sed -n 's/^host: //p'; }
sidecar_exe() { case "$1" in *windows*) echo ".exe" ;; *) echo "" ;; esac; }

# ---------------------------------------------------------------- baseline (item 13)
# baseline_contido <job> <chave...>: todas as chaves do achado medido estao
# contadas no baseline versionado? 0 = sim (monitorado); 1 = ha sitio novo ou
# baseline ilegivel (VIGIA_CEGO) — nestes casos o job e FAIL, nunca silencio.
baseline_contido() {
  local job="$1"; shift
  if [ $# -eq 0 ]; then
    echo "  o job falhou sem Produzir assinatura reconhecivel — FAIL puro, nada a monitorar"
    return 1
  fi
  if [ ! -r "$BASELINE" ]; then
    echo "  VIGIA_CEGO: job vermelho e baseline ilegivel/ausente ($BASELINE) — nunca 'limpo'"
    return 1
  fi
  local k novos=0
  for k in "$@"; do
    if ! grep -qF -- "$job|$k" "$BASELINE"; then
      echo "  ACHADO NOVO FORA DO BASELINE: $job :: $k"
      novos=$((novos + 1))
    fi
  done
  [ "$novos" -eq 0 ] || return 1
  echo "  LAUDO-MONITORADO: $# achado(s) pre-existente(s), todos contados no baseline"
  echo "  ($DECISAO_BASELINE)"
  return 0
}

# ---------------------------------------------------------------- jobs (ordem espelha o ci.yml)

# Gate anti-retorno da RFC-013: nenhum arquivo de automacao pode reaparecer no
# tree; achados em pontas de ref (ramos mortos/feature que ainda carregam os
# workflows aposentados) sao conferidos contra o baseline — apagar/promover ramo
# e higiene decidida pelo titular (item 9), nao pelo portao.
# O instrumento e o do Core, copiado para este repo (markforge nao vive dentro
# do Dock; ver AGENTS.md "Instrumentos viajantes") — selftest com iscas prova
# que morde antes de conferir.
job_sem_automacao_github() {
  need_py || return 2
  "$PY" scripts/check_sem_ci_github.py --selftest >/dev/null || { echo "  selftest do gate falhou"; return 1; }
  local out rc
  out=$("$PY" scripts/check_sem_ci_github.py . 2>&1); rc=$?
  printf '%s\n' "$out" | tail -8
  [ $rc -eq 0 ] && return 0
  if printf '%s\n' "$out" | grep -q "^ACHADO: "; then
    echo "  automacao no TREE — bloqueia sempre, baseline nao cobre arvore viva (item 13: para frente a regra e trava)"
    return 1
  fi
  local keys
  keys=$(printf '%s\n' "$out" | sed -nE 's/^ACHADO-REF: ([^ ]+) :: ([^ ]+) ::.*$/\1::\2/p' | sort -u)
  [ -n "$keys" ] || { echo "  gate falhou sem achado classificado — VIGIA_CEGO"; return 1; }
  local -a ks=()
  while IFS= read -r line; do ks+=("$line"); done <<< "$keys"
  baseline_contido "sem-automacao-github" "${ks[@]}" || return 1
  return 3
}

# scripts/higiene.sh — staging/disco sem segredo nem PII (contrato A-SDLC).
job_higiene() {
  need bash || return 2
  bash scripts/higiene.sh || return 1
}

# scripts/secret_sweep.py — guarda de credencial (diretiva 13/10). Selftest com
# iscas primeiro; depois a varredura real dos rastreados.
job_secret_sweep() {
  need_py || return 2
  "$PY" scripts/secret_sweep.py --selftest || { echo "  selftest do secret_sweep falhou"; return 1; }
  "$PY" scripts/secret_sweep.py || return 1
}

# Type-check + build do frontend (tsc && vite build). O tsc devolve rc != 0 em
# erro de tipo — isso e o gate RODANDO e reprovando. Quebra pre-existente do tip
# (commit de origem no baseline) e laudo monitorado; erro novo = FAIL.
job_front_end_build() {
  need node || return 2
  need npm || return 2
  local out rc keys
  out=$(npm run build 2>&1); rc=$?
  [ $rc -eq 0 ] && { printf '%s\n' "$out" | tail -3; return 0; }
  printf '%s\n' "$out" | grep -E "error TS[0-9]+" | head -20
  keys=$(printf '%s\n' "$out" | grep -E "^[^ ].*error TS[0-9]+: " | sort -u)
  [ -n "$keys" ] || { printf '%s\n' "$out" | tail -10; return 1; }
  local -a ks=()
  while IFS= read -r line; do ks+=("$line"); done <<< "$keys"
  baseline_contido "front-end-build" "${ks[@]}" || return 1
  return 3
}

# Testes do frontend (vitest run). Mesma disciplina do build: suites que já
# quebravam na origem do baseline ficam monitoradas; suite nova quebrando = FAIL.
job_front_end_tests() {
  need node || return 2
  need npm || return 2
  local out rc keys
  out=$(npm test 2>&1); rc=$?
  [ $rc -eq 0 ] && { printf '%s\n' "$out" | tail -3; return 0; }
  keys=$(printf '%s\n' "$out" | sed -E 's/\x1b\[[0-9;]*m//g' \
        | grep -oE "FAIL +src/[A-Za-z0-9._/-]+\.test\.ts" | awk '{print $2}' | sort -u)
  printf '%s\n' "$out" | sed -E 's/\x1b\[[0-9;]*m//g' | grep -E "Test Files|Tests " | head -4
  [ -n "$keys" ] || { printf '%s\n' "$out" | tail -15; return 1; }
  local -a ks=()
  while IFS= read -r line; do ks+=("$line"); done <<< "$keys"
  baseline_contido "front-end-tests" "${ks[@]}" || return 1
  return 3
}

# Gerar icones do app a partir de build/icon.png.
job_tauri_icon() {
  need npm || return 2
  [ -f build/icon.png ] || { echo "  NAO-EXECUTADO: falta build/icon.png"; return 2; }
  npm run tauri icon build/icon.png || return 1
}

# Smoke do pipeline de exportacao (Pandoc + Typst) nos sidecars do alvo atual.
# Roda no Windows com os sidecars ja presentes em src-tauri/binaries; se o alvo
# nao tem sidecar baixado, e NAO-EXECUTADO com a causa (nao fingimos verde).
job_smoke_export() {
  need bash || return 2
  local target exe bin_p bin_t
  target=$(rust_host) || return 2
  [ -n "$target" ] || { echo "  NAO-EXECUTADO: rustc nao devolveu o host triple"; return 2; }
  exe=$(sidecar_exe "$target")
  bin_p="src-tauri/binaries/pandoc-$target$exe"
  bin_t="src-tauri/binaries/typst-$target$exe"
  if [ ! -f "$bin_p" ] || [ ! -f "$bin_t" ]; then
    echo "  NAO-EXECUTADO: sidecars ausentes para o alvo '$target' ($bin_p / $bin_t)."
    echo "  Baixe com scripts/ci/fetch-sidecars-windows.ps1 (ou -linux.sh) — precisa de rede."
    return 2
  fi
  bash scripts/smoke-test-export.sh "$bin_p" "$bin_t"
}

# O que e "ferramenta ausente" no mundo Rust: linker/compilador nativo que a
# estacao nao tem (MSVC cl.exe / link.exe ausentes — medido 25/10). Nada de
# instalar stack no desktop: a cobertura desses jobs mora no container de
# paridade (job docker-linux-validation).
rust_tool_missing_regex="failed to find tool|ToolNotFound|cl\.exe|link\.exe|C\+\+ build tools|linker.*not found|could not find .*link"

# cargo check (src-tauri).
job_rust_check() {
  need cargo || return 2
  local out rc
  out=$( cd src-tauri && cargo check --message-format short 2>&1 ); rc=$?
  printf '%s\n' "$out" | tail -20
  [ $rc -eq 0 ] && return 0
  if printf '%s\n' "$out" | grep -qiE "$rust_tool_missing_regex"; then
    echo "  NAO-EXECUTADO: falta a cadeia C/MSVC nativa nesta estacao (cl.exe/link.exe ausentes — medido 25/10)."
    echo "  Nao se instala stack no desktop (diretiva 25/10); a cobertura desses jobs e o container de"
    echo "  paridade Linux do projeto (job docker-linux-validation) — pendencia lavrada no registro."
    return 2
  fi
  echo "  cargo check falhou por erro real de compilacao (nao por ferramenta ausente)."
  return 1
}

# Gerar os fixtures de PDF (reportlab + pillow) que NAO vivem no repositorio —
# alimentam os testes de importacao Rust.
job_pdf_fixtures() {
  need_py || return 2
  "$PY" scripts/generate_pdf_fixtures.py || return 1
}

# Testes unitarios Rust (cargo test --lib). Mesma condicao de ferramenta do check.
job_rust_tests_lib() {
  need cargo || return 2
  local out rc
  out=$( cd src-tauri && cargo test --lib 2>&1 ); rc=$?
  printf '%s\n' "$out" | tail -30
  [ $rc -eq 0 ] && return 0
  if printf '%s\n' "$out" | grep -qiE "$rust_tool_missing_regex"; then
    echo "  NAO-EXECUTADO: falta a cadeia C/MSVC nativa nesta estacao — cargo test nao linka aqui."
    return 2
  fi
  echo "  cargo test --lib falhou por erro real."
  return 1
}

# Validacao de paridade em Linux — o job que o runner ubuntu-22.04 fazia de ponta
# a ponta. Espelha docker/Dockerfile. A imagem ainda esta INCOMPLETA (falta
# docker/validar-linux.sh, parado 13/08 a pedido do titular) e o daemon pode estar
# desligado: em qualquer desses casos e NAO-EXECUTADO com a causa, nunca verde.
job_docker_linux_validation() {
  need docker || return 2
  docker info >/dev/null 2>&1 || { echo "  NAO-EXECUTADO: daemon do Docker indisponivel nesta estacao"; return 2; }
  [ -f docker/validar-linux.sh ] || {
    echo "  NAO-EXECUTADO: docker/validar-linux.sh ausente — o Dockerfile do projeto esta INCOMPLETO"
    echo "  (parado 13/08/2026 a pedido do titular; ver docs/_interno/STATUS_HANDOFF.md)."
    return 2
  }
  docker build -t markforge-linux -f docker/Dockerfile docker || return 1
  docker run --rm -v "$PWD":/repo:ro markforge-linux || return 1
}

# ---------------------------------------------------------------- fila de jobs
JOBS="sem-automacao-github higiene secret-sweep front-end-build front-end-tests \
tauri-icon smoke-export rust-check pdf-fixtures rust-tests-lib docker-linux-validation"

fn_do_job() { echo "job_$1" | tr '-' '_'; }

if [ "$LIST" = "1" ]; then
  for j in $JOBS; do echo "$j"; done
  exit 0
fi

selecionados=""
for j in $JOBS; do
  [ -z "$APS" ] || [[ "$j" == *"$APS"* ]] || continue
  selecionados="$selecionados $j"
done
if [ -z "$selecionados" ]; then
  echo "[ci_local] nenhum job casa com --apenas '$APS' (recusado; nada-a-fazer nunca e verde)"
  exit 2
fi

if [ "$DRY" = "1" ]; then
  for j in $selecionados; do echo "[ci_local] job=$j veredito=DRY"; done
  exit 0
fi

PASS=0; FAIL=0; NAO=0; MON=0
for j in $selecionados; do
  echo "[ci_local] job=$j iniciando"
  rc=0
  "$(fn_do_job "$j")" || rc=$?
  case "$rc" in
    0) echo "[ci_local] job=$j veredito=PASS"; PASS=$((PASS+1)) ;;
    2) echo "[ci_local] job=$j veredito=NAO-EXECUTADO"; NAO=$((NAO+1)) ;;
    3) echo "[ci_local] job=$j veredito=LAUDO-MONITORADO"; MON=$((MON+1)) ;;
    *) echo "[ci_local] job=$j veredito=FAIL"; FAIL=$((FAIL+1)) ;;
  esac
done

TOTAL=$(echo $selecionados | wc -w | tr -d ' ')
echo "[ci_local] resumo: $PASS/$TOTAL verdes ($MON monitorado, $NAO nao-executado, $FAIL vermelho)"
if [ "$FAIL" -gt 0 ]; then
  echo "[ci_local] VERMELHO: vermelho bloqueia a entrega (item 5/9 do AGENTS global)"
  exit 1
fi
if [ "$NAO" -gt 0 ] || [ "$MON" -gt 0 ]; then
  echo "[ci_local] ATENCAO: portao sem vermelho, mas com pendencias NOMEADAS (nao-execitado/monitorado)"
  echo "[ci_local] lavradas no registro do projeto — decisao pendente do dono; nunca verde presumido."
fi
echo "[ci_local] CI LOCAL VERDE (com as pendencias nomeadas acima)"
exit 0
