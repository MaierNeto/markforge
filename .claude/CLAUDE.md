# Markforge — Instruções de Projeto

## Identidade
Markforge — editor visual (WYSIWYG) para arquivos Markdown, com exportação para DOCX e
PDF a partir de templates com capa, cabeçalho e rodapé. O `.md` é sempre a fonte da
verdade. Stack: React 18 + Vite + TypeScript (frontend) · Tauri 2 / Rust (desktop).
Autor: Walter Maier Neto.

## Perfil do projeto (o seletor de aplicabilidade)

- [x] `[tem-remoto]` — repositório com remoto (`origin`, GitHub público)
- [x] `[compila]` — build de artefato (`tsc && vite build` + `cargo`/Tauri)
- [x] `[tem-frontend]` — UI React/Vite
- [x] `[deploy-formal]` — publica instaladores por release (workflow `release.yml`)
- [ ] `[tem-banco]` · [ ] `[tem-auth]` · [ ] `[multi-tenant]` · [ ] `[custodia-credencial]`
      · [ ] `[tem-seeds]` · [ ] `[dominio-regulado]`

> **Repositório público.** Todo texto versionado (README, CHANGELOG, mensagens de commit,
> docs, dados de amostra em teste) é comunicação pública: na voz do produto, sem detalhe de
> processo interno, sem falha de segurança não corrigida, sem dado real. Ver §Comunicação.

## Princípios inegociáveis

1. **TDD** — teste é a especificação executável. Teste falhando = código errado, não o
   teste. Não alterar um teste só para passar; exceção legítima é teste factualmente
   obsoleto (corrigir documentando o motivo).
2. **Evidência antes de status** — nada "pronto" sem rodar (`npm test`, build, app abrindo).
3. **`npm test` verde antes de qualquer commit.**
4. **Higiene de segurança** — nunca versionar segredo/PII; ver `.gitignore`.
5. **`.md` é a fonte da verdade** — sem formato proprietário; ler e gravar `.md` limpo,
   compatível com Git.
6. **Comunicação pública** — o que sai no repo é o produto refinado; processo, dívida
   técnica e falha não corrigida ficam fora do repo público.

## Comandos essenciais
```bash
npm test          # vitest — obrigatório verde antes de commit
npm run dev       # sobe o frontend (Vite) em desenvolvimento
npm run build     # tsc + vite build — gate de compilação
npm run tauri     # empacota o app desktop (Tauri/Rust)
```
> **Smoke automatizado:** `scripts/smoke-test-export.sh` exercita o pipeline de exportação
> (Pandoc + Typst) no CI e antes de cada release. **Smoke manual** (o que ele não cobre):
> o app desktop abre, carrega um `.md` e exporta DOCX/PDF pela interface.

## Convenções do repo
- Testes em `src/**/*.test.ts(x)` (Vitest), um arquivo por área.
- Dados de amostra em teste são **fictícios e neutros** (repo público — nada de nomes de
  projeto internos, dados reais ou segredos).
- **Fonte única da versão: `src-tauri/Cargo.toml`.** O `tauri.conf.json` herda dela (não
  tem campo `version`) e o `package.json` a espelha. O job `versao` do `release.yml` falha
  se tag, `Cargo.toml` e `package.json` divergirem. CHANGELOG por SemVer; tag = `v` +
  versão. Commits convencionais (`feat:`, `fix:`, `docs:`, `chore(release):`).

## Comunicação pública (cozinha × salão)
README/CHANGELOG/roadmap/commit descrevem **o que o produto faz e o que mudou para quem
usa**, na voz do produto. Falha de segurança **só** se anuncia depois de corrigida, na
seção *Segurança* do CHANGELOG (descreve o que foi resolvido, não como explorar).

## Continuidade
> **Repo público → continuidade fica na retaguarda, não versionada.** O estado real e o log
> de decisões vivem na **memória local do projeto** (Claude Code). Pesquisa, estratégia e
> hipótese não validada vivem no **`ROADMAP-ESTRATEGICO.md`**, que o `.gitignore` mantém
> fora do remoto. O `ROADMAP.md` público só recebe capacidade **já validada**.
