# Markforge

**Editor visual (WYSIWYG) para arquivos Markdown, com exportação para DOCX e PDF prontos para entrega.**

Markforge foi criado para quem trabalha com documentação de projetos guiados por
agentes de IA (specs, planos, ADRs, changelogs) escrita em `.md` — e que também
precisa, de tempos em tempos, transformar esses arquivos em documentos formais
(DOCX/PDF) com capa, cabeçalho e rodapé, sem reescrever nada à mão.

- **Edição visual, tipo Notion.** Sem ver `##` ou `**negrito**` — você formata
  com atalhos, menu "/" e barra de ferramentas, e o Markdown por baixo continua
  limpo e compatível com Git, GitHub e qualquer outra ferramenta.
- **O `.md` é sempre a fonte da verdade.** Nada de formato proprietário: o
  Markforge lê e grava exatamente arquivos `.md`, com front-matter YAML para
  metadados (título, subtítulo, autor, data).
- **Exportação com template.** Gere DOCX e PDF com capa, cabeçalho e rodapé
  consistentes, a partir de um template padrão incluso ou de um `.docx` de
  referência com a identidade visual da sua empresa.
- **Projeto inteiro, não só um arquivo.** Abra a raiz do seu repositório e
  navegue pela árvore de arquivos `.md` — ideal para pastas de controle de
  projetos com Claude Code e agentes similares.
- **Zero dependências externas.** Pandoc e Typst já vêm embutidos no
  instalador — não é preciso instalar nada separadamente para exportar
  DOCX/PDF.

## Instalação

Baixe o instalador da sua plataforma na [página de releases](../../releases/latest)
e pronto — Pandoc e Typst já vêm dentro do instalador, não é preciso instalar
mais nada:

- **Windows:** `Markforge_x.y.z_x64-setup.exe` (ou `.msi`)
- **Linux:** `markforge_x.y.z_amd64.deb` ou `markforge_x.y.z_amd64.AppImage`

## Como funciona a exportação

1. **DOCX:** o conteúdo do editor (mais os metadados de capa) é convertido
   para DOCX via Pandoc (embutido), usando um arquivo `.docx` de referência
   que define fontes, cores, cabeçalho e rodapé.
2. **PDF:** o mesmo conteúdo é convertido diretamente para PDF via Pandoc +
   [Typst](https://typst.app) (também embutido), usando um template Typst
   próprio que replica a mesma capa, cabeçalho e rodapé do template DOCX.
3. Você pode importar seus próprios templates `.docx` (Templates → Importar)
   para usar a identidade visual da sua empresa no DOCX. O template de PDF
   (Typst) por enquanto é fixo.

## Desenvolvimento

Stack: [Tauri 2](https://tauri.app) (Rust) + React + TypeScript + [Milkdown/Crepe](https://milkdown.dev).

```bash
npm install
npm run tauri dev      # ambiente de desenvolvimento
npm run tauri build    # gera o instalador para a plataforma atual
```

Pré-requisitos: Node.js 20+, Rust estável, e as dependências nativas do Tauri
para a sua plataforma (veja o [guia oficial](https://tauri.app/start/prerequisites/)).

O template DOCX padrão (`templates/default/reference.docx`) é gerado a partir
de `scripts/generate_reference_docx.py` (requer `python-docx`):

```bash
pip install python-docx
python3 scripts/generate_reference_docx.py
```

O template de PDF (`templates/default/pdf-template.typ`) é um template Pandoc
para o formato Typst — edite diretamente esse arquivo `.typ` para ajustar a
aparência do PDF.

### Binários embutidos (Pandoc + Typst)

O build local (`npm run tauri dev` / `npm run tauri build`) espera encontrar
os binários "sidecar" em `src-tauri/binaries/pandoc-<target-triple>[.exe]` e
`src-tauri/binaries/typst-<target-triple>[.exe]`. Para baixá-los:

```bash
# Linux
bash scripts/ci/fetch-sidecars-linux.sh

# Windows (PowerShell)
scripts/ci/fetch-sidecars-windows.ps1
```

## Releases automáticas

Todo push de uma tag `vX.Y.Z` dispara o workflow `.github/workflows/release.yml`,
que primeiro roda um smoke test do pipeline de exportação (Pandoc + Typst
contra um documento de teste) e, se passar, compila os instaladores para
Windows e Linux via [tauri-action](https://github.com/tauri-apps/tauri-action),
publicando uma release no GitHub com os artefatos anexados.

## Licença

[Apache License 2.0](LICENSE).
