# language: pt-BR
Funcionalidade: Links externos abrem no navegador padrão (RFC-007)
  Como usuário do MarkForge
  Quero clicar em links de internet nos documentos Markdown
  Para que se abram no navegador padrão da estação
  E o MarkForge nunca trave

  Contexto:
    Dado que o MarkForge está aberto com um arquivo Markdown
    E o arquivo contém links de internet (https:// ou http://)

  # CA-01: Clicar em link `https://cloud.linode.com/` abre no navegador padrão
  Cenário: CA-01 Happy Path — Link HTTPS abre no navegador padrão (Firefox/Chrome/Edge)
    Quando clico em um link externo "https://cloud.linode.com/"
    Então o navegador padrão do SO abre aquela URL
    E nenhum erro é lançado na console
    E nenhuma trava ocorre

  Cenário: Link HTTP também abre no navegador padrão
    Quando clico em um link externo "http://example.com/"
    Então o navegador padrão do SO abre aquela URL
    E o MarkForge segue responsivo

  # CA-02: MarkForge mantém foco (ou pode Alt+Tab sem congelo)
  Cenário: CA-02 — MarkForge mantém foco após clicar em link externo
    Quando clico em um link externo "https://cloud.linode.com/"
    E o navegador abre em background
    Então posso fazer Alt+Tab e voltar ao MarkForge
    E a janela do MarkForge responde imediatamente (sem congelo)

  Cenário: MarkForge não perde foco enquanto navegador carrega
    Quando clico em um link externo
    E aguardo até 3 segundos (tempo de carregamento)
    Então consigo interagir com MarkForge (cliques, edição)
    E nenhuma trava é detectada

  # CA-03: Sem travamento em modo leitura
  Cenário: CA-03 — Sem travamento em modo leitura
    Dado que estou em modo leitura
    Quando clico em um link externo "https://cloud.linode.com/"
    Então o navegador abre
    E posso clicar em botões do MarkForge imediatamente após
    E posso fechar o MarkForge sem travamento

  # CA-04: Sem travamento em modo edição
  Cenário: CA-04 — Sem travamento em modo edição
    Dado que estou em modo edição
    E o cursor está posicionado em um parágrafo
    Quando clico em um link externo dentro do documento
    Então o navegador abre
    E posso continuar editando (digitando, deletando, etc.) imediatamente após
    E nenhuma trava ocorre

  # CA-05: Links internos funcionam normalmente
  Cenário: CA-05a — Link de anchor (#seção) navega para seção, não abre navegador
    Quando clico em um link interno "#seção-2"
    Então o editor navega para a seção (scroll ou focus)
    E o navegador externo NÃO é aberto
    E nenhuma trava ocorre

  Cenário: CA-05b — Link relativo (./outro.md) não abre navegador externo
    Quando clico em um link relativo "./outro-arquivo.md"
    Então o editor trata como navegação interna
    E o navegador externo NÃO é aberto
    E o comportamento segue a lógica padrão do editor

  Cenário: Link de path absoluto (/docs) não abre navegador
    Quando clico em um link "/docs/guide.md"
    Então o editor trata como navegação interna
    E o navegador externo NÃO é aberto

  # CA-06: npm test passa, npm run build OK, zero console errors
  Cenário: CA-06 — Testes automatizados passam
    Quando executo "npm test"
    Então todos os testes de link-handler.test.ts passam
    E nenhum erro é lançado

  Cenário: Build produção compila sem erros
    Quando executo "npm run build"
    Então o build completa com sucesso
    E nenhum erro TypeScript é reportado
    E o arquivo compilado contém o handler de links

  # Casos de erro e exceção
  Cenário: Navegador não disponível — Tauri usa fallback
    Quando clico em um link externo
    E o navegador padrão não consegue ser aberto
    Então o Tauri lida com o erro graciosamente
    E nenhuma exceção não-capturada é lançada
    E o MarkForge segue funcionando

  Cenário: Link sem href — não trava
    Quando clico em uma tag <a> sem atributo href
    Então nenhum erro é lançado
    E o MarkForge continua responsivo

  Cenário: Proteção contra cliques duplos
    Quando faço clique duplo em um link externo rapidamente
    Então o navegador abre apenas uma vez
    E nenhuma trava ocorre (sem múltiplas janelas)
