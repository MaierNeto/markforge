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

---

## Camada operacional do A-SDLC — contrato de rastreabilidade 🌐

> Incorporado em 12/08/2026 (A-SDLC v0.8.0+, `processo/INICIALIZACAO_DE_PROJETO.md` Passo 4).

**Contrato:** a **diretiva deste projeto** e o **`settings.json` que carrega o hook de teste**
são **arquivos rastreados pelo git**. Não são conveniência de quem configurou a máquina — são
o que faz o gate existir. Não rastreados, o projeto tem gate em **uma máquina só**, e nada dá
erro: o próximo clone, colaborador ou CI fica sem diretiva e sem gate.

**Caminho canônico:** `.claude/CLAUDE.md` e `.claude/settings.json`.

**Prova (presença se prova, não se presume):**

```bash
git ls-files | grep -E "^(CLAUDE\.md|\.claude/CLAUDE\.md)$"   # a diretiva viaja com o repo?
git ls-files | grep -E "^\.claude/settings\.json$"            # o hook viaja com o repo?
```

Saída vazia em qualquer uma das duas = **falha de fundação**, não pendência menor.

**Se um `.gitignore` (frequentemente herdado de upstream OSS) engolir `.claude/`:**

1. A diretiva vai para a **raiz** (`CLAUDE.md`) — caminho igualmente suportado, fora da
   convenção de ignore herdada. Declarar o desvio aqui, para que sessão futura não "corrija"
   de volta e reintroduza o problema em silêncio.
2. O `settings.json` **não tem alternativa de caminho** — só resta corrigir a regra:

   ```gitignore
   .claude/*                      # excluir o CONTEÚDO, não o diretório
   !.claude/settings.json
   .claude/settings.local.json
   ```

   `.claude/` + `!.claude/settings.json` **não funciona**: o git não re-inclui arquivo cujo
   diretório-pai está excluído. A negação sozinha não faz nada e parece ter feito.
