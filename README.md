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

## Instalação

Baixe o instalador da sua plataforma na [página de releases](../../releases/latest):

- **Windows:** `Markforge_x.y.z_x64-setup.exe` (ou `.msi`)
- **Linux:** `markforge_x.y.z_amd64.deb` ou `markforge_x.y.z_amd64.AppImage`

### Dependências para exportação

A exportação usa ferramentas de código aberto já consagradas, instaladas
separadamente no seu sistema (o Markforge detecta automaticamente e avisa se
algo estiver faltando):

| Ferramenta | Necessária para | Instalação |
|---|---|---|
| [Pandoc](https://pandoc.org/installing.html) | Exportar DOCX e PDF | Windows: instalador oficial ou `winget install pandoc` · Linux: `sudo apt install pandoc` |
| [LibreOffice](https://www.libreoffice.org/download/download/) | Exportar PDF (opcional) | Windows: instalador oficial · Linux: `sudo apt install libreoffice` |

Sem essas ferramentas, a edição visual dos arquivos `.md` funciona normalmente
— elas só são necessárias na hora de exportar para DOCX/PDF.

## Como funciona a exportação

1. O conteúdo do editor (mais os metadados de capa) é convertido para DOCX via
   Pandoc, usando um arquivo `.docx` de referência que define fontes, cores,
   cabeçalho e rodapé.
2. Se PDF for solicitado, o DOCX gerado é convertido para PDF via LibreOffice
   em modo headless — garantindo que o PDF final tenha exatamente a mesma
   capa, cabeçalho e rodapé do DOCX.
3. Você pode importar seus próprios templates `.docx` (Templates → Importar)
   para usar a identidade visual da sua empresa.

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

## Releases automáticas

Todo push de uma tag `vX.Y.Z` dispara o workflow `.github/workflows/release.yml`,
que compila os instaladores para Windows e Linux via
[tauri-action](https://github.com/tauri-apps/tauri-action) e publica uma
release no GitHub com os artefatos anexados.

## Licença

[Apache License 2.0](LICENSE).
