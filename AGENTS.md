# AGENTS.md — Markforge

Regras deste repositório. Coexistem com o `~/.qoder/agents.md` global (regras do
titular, Walter), que manda nos atos transversais; aqui fica o específico do
Markforge.

## O que é este repo

- Produto **Navyra**: editor visual (WYSIWYG) de Markdown, com exportação para
  DOCX e PDF a partir de templates com capa, cabeçalho e rodapé.
- Stack: **React + Vite** (frontend, `src/`), **Tauri 2 + Rust** (app desktop,
  `src-tauri/`), com os sidecars **Pandoc** e **Typst** embutidos.

## Remote e branch de trabalho

- Remote: `https://github.com/MaierNeto/markforge.git` (origin). Branch de
  trabalho: **`main`**.
- **GitHub é só armazenamento: nenhuma automação neste repositório** (RFC-013,
  diretiva do titular, 25/10/2026). Não se cria nem se mantém
  `.github/workflows/`, `.github/dependabot.yml`, config de bot nem badge de CI.
  Fica apenas o conteúdo humano de `.github/` (templates de issue/PR, CODEOWNERS,
  docs). Os workflows `ci.yml` (deps de sistema + testes) e `release.yml`
  (publicava Release no GitHub em push de tag `v*`) foram **aposentados em
  25/10/2026**; o gate `scripts/check_sem_ci_github.py` confere que nenhum
  arquivo de automação reapareça.

## A prova de entrega é o portão local (não a nuvem)

Antes de todo commit/push, rodar no ambiente dev desta estação:

```
bash scripts/ci_local.sh                 # todos os jobs
bash scripts/ci_local.sh --apenas <nome> # recorta um job
bash scripts/ci_local.sh --dry-run       # inventario dos jobs
```

O portão transcreve os jobs úteis do workflow aposentado **sem reimplementar
gate nenhum**: cada job delega no mesmo instrumento que o step da nuvem já
rodava (gatilho é só gatilho):

| Job | Delega em |
|---|---|
| `sem-automacao-github` | `scripts/check_sem_ci_github.py` (selftest + varredura) |
| `higiene` | `scripts/higiene.sh` |
| `secret-sweep` | `scripts/secret_sweep.py` (selftest + varredura) |
| `front-end-build` | `npm run build` (tsc + vite build) |
| `front-end-tests` | `npm test` (vitest run) |
| `tauri-icon` | `npm run tauri icon build/icon.png` |
| `smoke-export` | `scripts/smoke-test-export.sh` com os sidecars do alvo atual |
| `rust-check` | `cargo check` em `src-tauri/` |
| `pdf-fixtures` | `scripts/generate_pdf_fixtures.py` |
| `rust-tests-lib` | `cargo test --lib` em `src-tauri/` |
| `docker-linux-validation` | `docker/Dockerfile` (paridade Linux) |

Veredito honesto (itens 5 e 9 do AGENTS global): cada job é **PASS / FAIL /
NAO-EXECUTADO / LAUDO-MONITORADO**. Ferramenta ausente ou serviço que não sobe
**nunca** é verde — o job nomeia a causa e fica pendência lavrada no registro.
**FAIL bloqueia a entrega.** LAUDO-MONITORADO é o modo baseline do item 13
("portão novo nasce com baseline"): só vale para achado **pré-existente**,
contado no arquivo versionado `scripts/ci_local.baseline` com commit de origem e
decisão pendente do dono; sítio além do contado, arquivo novo ou candidato novo é
**FAIL**; baseline ilegível com job vermelho é **VIGIA_CEGO**, nunca "limpo".
Nunca `--no-verify`, nunca enfraquecer gate para passar; higienizar o baseline é
ato do titular. O veredito de cada corrida é lavrado no registro do projeto —
`registro/portao-local.md` (hora GMT-3 + hash conferido + resumo real do
runner).

### Baseline vigente (medido 25/10, corrida 1 — decisão pendente do titular)

- `front-end-build` / `front-end-tests`: o tip `975cafc` (RFC link-handler)
  chegou ao `main` quebrando o `tsc` (4 erros) e o vitest (15 suítes — o
  `vitest.config.ts` novo ficou sem o alias `@` e um teste usa URL que o Windows
  recusa). Conserto é da sessão do link-handler ou reversão decidida pelo
  titular — o portão **monitora**, não conserta produto alheio por cima.
- `sem-automacao-github`: 24 achados em **pontas de ref** — os workflows
  aposentados seguem vivos nas árvores de ramos mortos/feature (locais e
  remotos, incl. o ramo que o Dependabot criou). Apagar/promover ramo é higiene
  do titular (item 9). Automação na **árvore viva** (tree) nunca entra no
  baseline: bloqueia sempre.
- `rust-check` / `rust-tests-lib` / `docker-linux-validation`: **NÃO-EXECUTADO**
  com causa nomeada — a estação não tem a cadeia C/MSVC (`cl.exe` ausente; não
  se instala stack no desktop) e o `docker/Dockerfile` está incompleto (falta
  `docker/validar-linux.sh`, trabalho parado pelo titular em 13/08). A retomada
  do container de paridade Linux é o que devolve esses três jobs ao verde.

## Instrumentos viajantes (divergência intencional lavrada)

- `scripts/check_sem_ci_github.py` é **cópia como instrumento deste repo** do
  instrumento do Core (`navyra_dock/core/scripts/check_sem_ci_github.py`). O motivo:
  o Markforge não mora dentro do Dock (está em `C:\dev_walter\navyra_markForge\`,
  e o caminho de irmão que o Estudo Fácil usa — `../navyra_dock/...` — não alcança
  daqui); instrumento que só existe em outro repo é promessa, não controle (item 5
  do AGENTS global). Precedente: `estacao_memorias` (25/10).
- **Sincronia medida em 25/10/2026:** a cópia está byte-a-byte igual ao Core (diff
  vazio). Quando o pai evoluir, transplanta-se por cópia e registra-se a sincronia
  aqui, no mesmo commit.

## Ambiente em Docker (paridade Linux)

- O portão roda na estação e usa a cadeia de ferramentas **já presente no ambiente
  dev** deste app desktop (Node, Rust, Python, sidecars em `src-tauri/binaries`).
  **Não se instala stack no desktop só para um job passar** (diretiva 25/10).
- O que precisa de ambiente que não existe nativo aqui — a **compilação/link do
  Rust** (falta a cadeia MSVC: `link.exe` / "C++ build tools") e a **validação de
  paridade em Linux** — mora no Docker deste ambiente:
  - Subir: `docker build -t markforge-linux -f docker/Dockerfile docker` e, da
    raiz do repo, `docker run --rm -v "$PWD":/repo:ro markforge-linux`.
  - O `docker/Dockerfile` está **INCOMPLETO**: falta `docker/validar-linux.sh`
    (o ENTRYPOINT), parado em 13/08/2026 a pedido do titular
    (ver `docs/_interno/STATUS_HANDOFF.md`). Enquanto incompleto, ou com o daemon
    do Docker desligado, os jobs `rust-check`, `rust-tests-lib` e
    `docker-linux-validation` são **NAO-EXECUTADO com a causa nomeada** — nunca
    presumidos verdes.
- Container que não sobe é NAO-EXECUTADO com a causa; um job que não precisa de
  serviço não ganha container por estética.

## Publicação de Release — ato do titular

- O `release.yml` aposentado era o **publicador**: montava os instaladores
  (Windows + Linux) e **abria a Release no GitHub** em push de tag `v*.*.*`.
- Com o GitHub reduzido a armazenamento, **publicar Release e criar tag `v*`
  passaram a ser ATO MANUAL DO TITULAR** — o portão não publica, não taggeia, não
  abre release. A conferência de coerência de versão (tag == `Cargo.toml` ==
  `package.json`) continua regra válida de homologação, mas é decisão do titular,
  não job do portão.

## Segurança e higiene (resumo; detalhe no AGENTS global)

- Nada de segredo nem PII no texto: credencial só em `.env` gitignored ou env de
  deploy, nunca hardcoded. `scripts/secret_sweep.py` varre os rastreados e morde
  padrão de credencial (selftest com iscas).
- **Trava de compartimento**: os acessos AWS desta estação nunca entram neste
  repo nem em projeto corporativo; dado corporativo nunca aparece aqui.
