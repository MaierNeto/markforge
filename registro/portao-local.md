# Registro do portão local — Markforge

Instrumento: `scripts/ci_local.sh` (RFC-013; GitHub é só armazenamento — AGENTS global
itens 5, 11 e 12). Um verbete por corrida conferida: hora GMT-3, hash do commit conferido,
veredito por job e resumo real do runner. "Não medido" escrito explicitamente — nunca verde
por presumação.

---

## 2026-10-25 22:52 GMT-3 — corrida 3 (oficial) da fatia de conversão — commits conferidos `eb5b6e6` + `fa8aeaa` — **5/11 verdes (3 monitorado, 3 nao-executado, 0 vermelho)**

Conteúdo conferido: árvore dos dois commits da fatia (aposento `eb5b6e6` +
portão/registro/AGENTS `fa8aeaa`), igual ao testado desde a corrida 2; a
modificação preexistente de `package-lock.json` (sessão concorrente, deixei lá)
não afeta as assinaturas medidas.

| job | veredito | nota |
|---|---|---|
| sem-automacao-github | LAUDO-MONITORADO | selftest 12 casos OK (iscas mordem); tree: 0 arquivos de automação; 22 achados em pontas de ref (ramos mortos/feature que ainda carregam os workflows aposentados — 2 a menos que o baseline desde que `main` local ficou limpo; subset = monitorado). Higienizar ramo é ato do titular. |
| higiene | PASS | `scripts/higiene.sh` — staging/disco limpos. |
| secret-sweep | PASS | selftest com iscas + varredura dos rastreados (agora inclui os instrumentos novos da fatia) limpa. |
| front-end-build | LAUDO-MONITORADO | 4 erros tsc pré-existentes do tip `975cafc` (sessão link-handler), todos contados no baseline; erro novo seria FAIL. |
| front-end-tests | LAUDO-MONITORADO | 15 suítes falhando pré-existentes (mesma origem: `vitest.config.ts` sem alias `@` + URL de `tauri-config.test.ts`); os 22 testes que rodam passam. |
| tauri-icon | PASS | ícones gerados de `build/icon.png`. |
| smoke-export | PASS | pipeline Pandoc+Typst com os sidecars do alvo `x86_64-pc-windows-msvc`. |
| rust-check | NAO-EXECUTADO | falta a cadeia C/MSVC nativa na estação (`cl.exe` ausente — crate `vswhom-sys`); não se instala stack no desktop; cobertura = container de paridade. |
| pdf-fixtures | PASS | `generate_pdf_fixtures.py` (reportlab+pillow presentes). |
| rust-tests-lib | NAO-EXECUTADO | mesma causa do rust-check. |
| docker-linux-validation | NAO-EXECUTADO | `docker/validar-linux.sh` ausente — Dockerfile INCOMPLETO, trabalho parado pelo titular em 13/08/2026 (`docs/_interno/STATUS_HANDOFF.md`). |

**Antecedentes desta lavra:** corrida 1 (19:28–19:57, pré-correções) fechou
5/11 com 5 vermelhos; o instrumento foi corrigido no que era defeito dele —
ausência de `cl.exe` é NAO-EXECUTADO, não FAIL — e o baseline (item 13:
portão nascente nasce com laudo monitorado para pré-existência) foi medido
nessa corrida sobre `975cafc`. Corrida 2 validou a correção (5/11 + 3
monitorados, 0 vermelhos). Nada foi afrouxado: automação na árvore viva
bloqueia sempre, sítio fora do baseline é FAIL, baseline ilegível é VIGIA_CEGO.

**Pendências nomeadas (decisão do dono, cabeçalho do baseline):**
1. Tip do `main` quebra `tsc`/vitest desde `975cafc` — conserto pela sessão do
   link-handler ou reversão; o portão não conserta produto alheio por cima.
2. 22 achados em pontas de ref (ramos `feat/*`, resto do Dependabot,
   `origin/main` até o push) — apagar/promover ramo é ato do titular.
3. Retomar `docker/validar-linux.sh` (parado 13/08) devolve `rust-check`,
   `rust-tests-lib` e `docker-linux-validation` ao verde em Docker desta
   estação.

**Publicação:** o `release.yml` aposentado era o publicador de Release no
GitHub (tag `v*`); com ele aposentado, gerar instalador + publicar + taggear
passam a exigir ato explícito do titular. Nada foi publicado nem taggeado
nesta fatia.


## 2026-10-25 22:54 GMT-3 — reconferência do push da conversão

- **Empurrado:** `main -> main` (`975cafc..908e51b`, 3 commits: `eb5b6e6`
  aposento da automação, `fa8aeaa` portão local + baseline + AGENTS + registro,
  `908e51b` lavração da corrida 3).
- **Conferência lida de volta:** `git fetch` + `status -sb` → `main...origin/main`
  em par; `origin/main` = `908e51b1…` (releitura às 22:54 GMT-3). Fast-forward
  limpo, sem força, sem tag, sem release.
- **Pós-push, job anti-retorno reexecutado:** 18 achados em pontas de ref
  (eram 24; `main` local, `origin/main` e `origin/HEAD` ficaram limpos com o
  push) — todos contidos no baseline, subset válido, `LAUDO-MONITORADO`.
  Restam os ramos `feat/*` locais/remotos e o ramo órfão do Dependabot;
  apagá-los é ato do titular.
- **Árvore concorrente declarada (não é minha, deixei lá):**
  `package-lock.json` modificado por sessão irmã (3 linhas) — não viajou em
  nenhum commit desta fatia.
- **Ato seguinte (ordinário):** esta entrada de reconferência é commitada e
  empurrada em seguida; sua própria reconferência é o estado em par do terminal
  da fatia.


## 2026-10-25 23:52 GMT-3 — sync por merge e aposento das pontas velhas (ordem do titular)

- **O que se mediu antes de tocar:** o `main` estava limpo e em par (`2ecce7a`), mas o gate
  com varredura de pontas de ref dava **18 achados em 9 pontas** — os dois workflows
  continuavam na árvore de `feat/exportar-mapa-mental`, `feat/importacao-fidelidade-docx`,
  `feat/instalador-pt-en`, `feat/pdf-import-poc` (local e remoto) e do ramo órfão do
  Dependabot. O GitHub lê o workflow da árvore do ref empurrado: aposentar só no `main`
  deixava automação disparável em cada uma delas.
- **Divergência local×remoto medida (não era trabalho distinto):** por patch-id, as pontas
  do mesmo nome eram a mesma história com hashes diferentes (3/1, 1/0, 1/0, 2/0 commits
  únicos) e as árvores só diferiam por `.claude/CLAUDE.md` + `.claude/settings.json`,
  rastreados nas pontas velhas. Empurrar corte sobre ponta divergente é rejeitado
  (non-fast-forward) — por isso a sincronização veio antes, e por merge, não por força.
- **Executado por ponta (worktree descartável, zero `--force`, zero `--no-verify`):**
  `git merge origin/<ramo>` → commit aposentando `.github/workflows/ci.yml` e
  `release.yml` → `push` fast-forward. Pontas resultantes: `feat/exportar-mapa-mental`
  `3ca84e8`, `feat/importacao-fidelidade-docx` `9950408`, `feat/instalador-pt-en` `2a4e803`,
  `feat/pdf-import-poc` `1150392` — **local == remoto conferido em todas**.
- **`.claude/` não viajou para o público:** a diretiva do titular de 06/09/2026 manda a
  retaguarda ficar fora do repo público (o `main` cumpre — `.gitignore:93`). Como o sync
  publicaria esses dois arquivos, eles saíram do índice de cada ponta (`git rm --cached`,
  ficam em disco) e o bloco `.claude/` entrou no `.gitignore` de cada uma, como no `main`.
  Conteúdo lido antes: briefing de projeto + hook `PostToolUse` rodando `npm test` — zero
  segredo; o briefing ainda cita `release.yml` como existente (rot que a RFC-013 revoga,
  declarado, não corrigido a revelia em arquivo de ponta morta).
- **Ramo órfão do Dependabot podado** por ordem do titular: `origin/dependabot/npm_and_yarn/npm_and_yarn-62ce72d0a2`,
  hash lavrado para restauração `70da0f145e55a91cdcec223b4ccf87c1ae667a2b`. Nada se perdeu:
  o único commit dele era `bump vite 8.2.0`, já superado no `main` (`^8.2.1`, medido em
  `package.json`). `git fetch --prune` conferido.
- **Baseline encolhida de 24 para 0 chaves na seção `sem-automacao-github`** (consolidadas,
  não perdoadas — contagem total 43 → 19; as 4 + 15 dos jobs de frontend ficaram
  intactas, conferido por contagem no mesmo ato). **Prova de que o gate continua
  mordendo:** isca = workflow plantado numa ponta temporária → `ACHADO NOVO FORA DO
  BASELINE: sem-automacao-github :: refs/heads/tmp-isca-rfc013::.github/workflows/isca-rfc013.yml`,
  veredito FAIL, rc=1; contra-exemplo = isca removida → PASS, rc=0. Resíduo da bancada
  apagado (worktree e ramo `tmp-isca-rfc013` não existem mais).
- **Veredito lavrado:** `bash scripts/ci_local.sh --apenas sem-automacao` às 23:52 GMT-3
  sobre `main` == `2ecce7a` + esta fatia → **1/1 verdes (0 monitorado, 0 nao-executado,
  0 vermelho)**, 11 pontas de ref conferidas, 0 achados. Suíte inteira **não** reexecutada
  nesta rodada: a árvore está com sessão concorrente (`package-lock.json` modificado por
  irmã, fora deste commit) e os jobs de frontend escrevem `dist/` na mesma árvore — a
  corrida oficial da conversão segue lavrada na entrada de 22:54 acima.
- **Pendências que isto NÃO fecha:** o mesmo defeito existe em `estudo-facil` (o
  `.github/workflows/ci.yml` vive na ponta `master`, ramo de release) e em
  `Portais/claustrum` (vive na ponta `main`) — lá o corte precisa chegar ao ramo default,
  e é decisão do titular promover. E o `audita_estacao.sh` mede só o índice do branch
  checkout: enquanto não varrer pontas de ref, o `auto=0` dele é verde parcial.
