# Roadmap — Markforge

O que vem por aí, em **Agora / Depois / Mais tarde**. Sem datas: data em roadmap
é promessa que o software não costuma cumprir.

O que já foi entregue está no [CHANGELOG.md](CHANGELOG.md).

## Agora

* **Importar PDF, convertendo para Markdown.** Abra um `.pdf` no Markforge e ele
  vira um `.md` editável, com títulos, parágrafos e listas reconhecidos, na
  ordem de leitura do documento — inclusive em arquivos de várias páginas.
  Conversão automática pede revisão: tabelas ainda chegam como texto corrido, e
  figuras e links não são trazidos (veja *Depois*). PDF digitalizado, feito de
  imagem, avisa que não há texto para importar em vez de gerar um arquivo
  ilegível.

## Depois

* **Aviso de nova versão.** O Markforge passa a avisar quando há uma atualização
  disponível — você não precisa conferir manualmente. Você continua decidindo
  quando instalar.

* **Figuras e imagens na importação de PDF.** As figuras, gráficos e imagens do
  PDF passam a vir junto: cada uma é salva ao lado do `.md`, numa pasta `media/`,
  e referenciada no texto no ponto certo da leitura — do mesmo jeito que já
  acontece ao importar um `.docx`. Assim o documento reexportado volta com as
  figuras no lugar.

* **Template de PDF próprio.** Hoje o template `.docx` importado define a
  identidade visual do DOCX, mas o PDF usa sempre o desenho padrão. Unificar
  isso exige definir o que é um template no Markforge — é decisão de produto,
  ainda em aberto.

## Mais tarde

* **Tabelas e links na importação de PDF.** Trazer tabela como tabela, e não
  como texto corrido, e preservar os links do documento original.

## Não previsto

* **Reconhecimento de texto em imagem (OCR).** Importar um PDF digitalizado
  transformando a imagem em texto não está no plano: exige um mecanismo de
  reconhecimento pesado, que contraria a proposta do Markforge de vir inteiro no
  instalador e funcionar sem internet — e o texto reconhecido é sempre um palpite
  da máquina, que se mistura ao texto real sem o leitor perceber. Se você precisa
  disso, [abra uma issue](https://github.com/MaierNeto/markforge/issues)
  contando o caso de uso.

* **Outros idiomas.** O Markforge é em português. Multi-idioma não está no
  plano — se você precisa disso,
  [abra uma issue](https://github.com/MaierNeto/markforge/issues) contando o
  caso de uso.
