# Changelog

Todas as mudanças relevantes do Markforge, em linguagem de quem **usa** o
produto — não despejo de commits.

Formato baseado em [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/);
versionamento segue [SemVer](https://semver.org/lang/pt-BR/).

## [0.5.0]

### Adicionado

- **Trocar de pasta ou arquivo sem fechar o Markforge.** Menu "Abrir…" na
  barra superior — qualquer edição pendente é salva automaticamente antes de
  trocar, do mesmo jeito que já acontecia ao trocar de arquivo dentro da
  árvore.
- **"Salvar como…"**, para mover o documento aberto para outra pasta.
- **Lista de recentes na tela inicial**, com as últimas pastas e arquivos
  abertos, um clique para retomar.
- **Importar um `.docx` direto na pasta do projeto aberto** — antes o `.md`
  só era criado do lado do `.docx` de origem. Disponível pelo menu de cada
  pasta na árvore de arquivos e pelo menu "Abrir…".

### Corrigido

- **"Salvar como" recusava pastas que o Markforge nunca tinha aberto**, com
  "Acesso negado" — afetava tanto o comando novo quanto a resolução de
  conflito de arquivo externo (introduzida na 0.3.0).

## [0.4.0]

### Adicionado

- **Importar um `.docx` e convertê-lo para Markdown.** Botão "Importar
  .docx…" na tela inicial: escolha um documento Word já existente e o
  Markforge gera o `.md` correspondente (mesma pasta, mesmo nome) e abre
  para edição. Vale para o formato `.docx` moderno — o formato binário antigo
  do Word 97-2003 (`.doc`) não é suportado.

## [0.3.0]

### Adicionado

- **Abertura em modo leitura.** Os documentos passam a abrir protegidos contra
  alteração acidental. Um botão libera a edição quando você quiser, e é
  possível reverter para o conteúdo com que o arquivo foi aberto na sessão.
- **Aviso quando o arquivo é alterado por outro programa.** Ao voltar para o
  Markforge, se o arquivo aberto mudou por fora, você é avisado — pode
  recarregar a versão do disco, resolver mantendo sua edição (com backup
  automático do conteúdo anterior) ou salvar como um novo arquivo.
- **A lista de arquivos se atualiza sozinha** quando um arquivo é criado ou
  removido por fora do Markforge, e também pode ser atualizada manualmente a
  qualquer momento pelo botão ⟳.
- **Abrir a pasta do documento direto do diálogo de exportação**, sem precisar
  procurar o arquivo no Explorer.
- **Confirmação ao fechar com alterações pendentes.** Fechar a janela com
  edições não salvas passa a perguntar se você quer salvar, descartar ou
  cancelar — nada se perde por engano.

### Corrigido

- **Diálogos de nova pasta, novo arquivo, renomear, excluir e remover
  template mostravam "tauri.localhost diz" no lugar do nome do Markforge.**
  Passaram a usar diálogos nativos do aplicativo.

## [0.2.1]

### Corrigido

- **Textos com `@` (como `@escopo/pacote` ou um e-mail) agora exportam
  corretamente.** Antes, um `@` seguido de palavra travava a geração do PDF e,
  no DOCX, era removido silenciosamente — comum em documentação técnica que cita
  pacotes ou menções.

## [0.2.0]

### Adicionado

- **Abrir um arquivo `.md` direto**, sem precisar abrir a pasta do projeto:
  botão "Abrir arquivo .md" na tela inicial e associação de `.md`/`.markdown`
  no sistema — duplo-clique num arquivo abre no Markforge. Nesse modo a barra
  lateral mostra só o arquivo aberto, com a opção **"Incluir a pasta deste
  arquivo"** para carregar a árvore do projeto quando quiser.
- **Tela de Configurações** com duas abas: associação dos tipos `.md`/`.markdown`
  (mostrando qual aplicativo responde hoje por cada um) e integração ao **menu de
  contexto** do Windows — "Abrir com Markforge" em arquivos e "Abrir pasta no
  Markforge" em pastas. Tudo por usuário, sem exigir administrador.
  > No Windows 10/11 tornar o Markforge o aplicativo **padrão** exige uma
  > confirmação sua na tela de Apps padrão — a própria tela abre esse ajuste.

### Alterado

- **O Windows passa a ter um instalador único** (`.exe`), que instala para o seu
  usuário e não pede privilégios de administrador. O pacote `.msi` foi
  descontinuado: os dois instalavam em lugares diferentes, o que permitia ficar
  com duas cópias do Markforge e a associação de arquivos apontando para a
  errada.
- **Capa do PDF agora só é gerada com um template customizado.** No template
  padrão o PDF sai sem página de capa, porque o padrão não carrega identidade
  visual e a capa genérica ficava pior do que não ter. No DOCX a capa continua
  valendo também para o template padrão.

### Corrigido

- **Título com aspas era corrompido:** um título como `Ele disse "olá"` ganhava
  barras de escape a cada abertura e salvamento do arquivo, acumulando
  indefinidamente.
- **Linha em branco extra** era inserida no início do documento a cada leitura,
  por divergência entre como o arquivo era lido e como era gravado.
- **Exportação de PDF em ambiente de desenvolvimento** falhava por não localizar
  o Typst (o app instalado não era afetado).

### Segurança

- **O acesso a arquivos agora se limita às pastas abertas no Markforge.** Um
  documento não alcança nada fora do projeto que você abriu: qualquer caminho
  fora das pastas abertas é recusado.
- **A interface passa a rodar sob uma política de conteúdo restritiva**, de modo
  que o conteúdo de um documento não execute código no aplicativo.

Recomendamos atualizar a partir das versões 0.1.x.

## [0.1.1] - 2026-07-11

### Adicionado

- **Pandoc e Typst embutidos no instalador.** Não é mais preciso instalar nada
  separadamente para exportar DOCX/PDF.

### Corrigido

- Geração de PDF quebrava com versões recentes do Pandoc (3.10+), que deixaram
  de embutir um arquivo de dados interno usado pelo template Typst.
- Smoke test de exportação rejeitava o Typst por causa do sufixo no nome do
  binário.
- Resolução instável de versão do Vite e do alias de importação.

## [0.1.0] - 2026-07-10

### Adicionado

- Primeira versão: editor visual (WYSIWYG) para arquivos Markdown, exportação
  DOCX e PDF com capa, cabeçalho e rodapé, navegação pela árvore de arquivos do
  projeto, templates de exportação, landing page e CI/CD.

[0.5.0]: https://github.com/MaierNeto/markforge/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/MaierNeto/markforge/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/MaierNeto/markforge/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/MaierNeto/markforge/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/MaierNeto/markforge/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/MaierNeto/markforge/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/MaierNeto/markforge/releases/tag/v0.1.0
