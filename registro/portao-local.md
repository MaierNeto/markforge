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

