// Template Pandoc -> Typst do Markforge.
// Recebido via: pandoc --template=pdf-template.typ --pdf-engine=typst
// Os campos de titulo, subtitulo, autor e data vem do front-matter YAML do
// documento. A flag "has-cover" e passada explicitamente via
// --metadata has-cover:true pelo comando export_document (Rust) quando a
// opcao "Incluir capa" esta marcada -- assim o comportamento fica identico
// ao pipeline DOCX.
//
// A linha abaixo traz os helpers padrao do Pandoc (blockquote,
// horizontalrule, endnote, etc.) que o writer typst pressupoe existirem.
$definitions.typst()$

#let ACCENT = rgb("#1F3A5F")
#let MUTED = rgb("#6B7280")

// Sobrescreve o "blockquote" padrão do Pandoc com o estilo do Markforge
// (borda esquerda âmbar em vez do recuo simples).
#let blockquote(body) = block(
  inset: (left: 1em),
  stroke: (left: 3pt + rgb("#E8A33D")),
)[#body]

#set text(size: 11pt, fill: rgb("#232830"), lang: "pt")
#set page(
  paper: "a4",
  margin: (top: 2.2cm, bottom: 2.2cm, left: 2.5cm, right: 2.5cm),
)
#set heading(numbering: none)

#show heading.where(level: 1): it => block(above: 1.4em, below: 0.8em)[
  #set text(size: 20pt, weight: "bold", fill: ACCENT)
  #it.body
]
#show heading.where(level: 2): it => block(above: 1.2em, below: 0.6em)[
  #set text(size: 15pt, weight: "bold", fill: ACCENT)
  #it.body
]
#show heading.where(level: 3): it => block(above: 1em, below: 0.5em)[
  #set text(size: 13pt, weight: "bold", fill: ACCENT)
  #it.body
]
#show heading.where(level: 4): it => block(above: 0.8em, below: 0.4em)[
  #set text(size: 11.5pt, weight: "bold", fill: ACCENT)
  #it.body
]

#show raw.where(block: true): it => block(
  fill: rgb("#F7F8FA"),
  inset: 10pt,
  radius: 4pt,
  width: 100%,
)[#it]

#set table(stroke: 0.5pt + rgb("#D0D4DA"))
#show table.cell.where(y: 0): it => block(fill: ACCENT, inset: 6pt)[
  #set text(fill: white, weight: "bold")
  #it
]

$if(has-cover)$
#set page(header: none, footer: none)
#v(6cm)
#align(center)[
  #block(width: 85%)[
    #text(size: 30pt, weight: "bold", fill: ACCENT)[$title$]
    #line(length: 100%, stroke: 1pt + ACCENT)
    $if(subtitle)$
    #v(0.4em)
    #text(size: 16pt, style: "italic", fill: MUTED)[$subtitle$]
    $endif$
    #v(0.6em)
    $if(author)$
    #text(size: 13pt, fill: MUTED)[$author$] \
    $endif$
    $if(date)$
    #text(size: 12pt, fill: MUTED)[$date$]
    $endif$
  ]
]
#pagebreak()
$endif$

#set page(
  header: context [
    #align(right)[#text(size: 8pt, weight: "bold", fill: MUTED)[MARKFORGE]]
    #line(length: 100%, stroke: 0.4pt + rgb("#D0D4DA"))
  ],
  footer: context {
    let cur = counter(page).get().first()
    let total = counter(page).final().first()
    [#align(center)[#text(size: 8pt, fill: MUTED)[Página #cur de #total]]]
  },
)

$body$
