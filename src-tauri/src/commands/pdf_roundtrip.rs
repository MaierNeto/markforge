//! Harness de ida-e-volta `MD → PDF → MD` — a régua da paridade de importação.
//!
//! **Por que existe.** O critério de "importação entregue" é: o Markdown gerado
//! precisa ser rico o bastante para que a exportação devolva um PDF com a mesma
//! navegabilidade e estrutura do original. Isso não se mede olhando o `.md` a
//! olho nu — mede-se fechando o ciclo.
//!
//! **Por que o oráculo é o próprio produto.** O PDF que o Markforge exporta
//! (Pandoc + Typst, com o template real) sai marcado e navegável: `/Outlines`
//! hierárquico, destinos nomeados por título, `/StructTreeRoot` com `H1..H3`,
//! `Table`, `L/LI`, `Code`. Então um `.md` conhecido, exportado por esse mesmo
//! caminho, é um PDF de teste **realista e sintético ao mesmo tempo** — sem
//! documento de terceiro, sem dado real, sem fixture binário no repositório.
//!
//! Isso importa também porque os fixtures do `reportlab` usam fonte base-14 de
//! uma página só: o caso mais fácil que existe. O PDF do Typst traz fonte
//! embutida com `ToUnicode` e várias páginas — o caso que o mundo real entrega.
//!
//! O texto do documento-ouro é sintético e neutro (regra de dado real).

use std::path::{Path, PathBuf};

use super::pdf_import::import_pdf_to_markdown;

/// Marcos espalhados pelo documento. São únicos, curtos e sobrevivem a
/// qualquer reflow — a **ordem** deles no Markdown de volta é o que prova que
/// nenhuma página foi embaralhada e nenhum trecho se perdeu.
const MARCOS: [&str; 8] = [
    "MARCOUM", "MARCODOIS", "MARCOTRES", "MARCOQUATRO", "MARCOCINCO", "MARCOSEIS", "MARCOSETE",
    "MARCOOITO",
];

/// Documento-ouro: um de cada estrutura que a diretiva exige preservar, com
/// enchimento suficiente para transbordar para uma segunda página — os quatro
/// últimos marcos caem depois da quebra.
fn documento_ouro() -> String {
    let enchimento: String = (1..=42)
        .map(|i| format!("Linha de preenchimento numero {i} para empurrar o texto adiante.\n\n"))
        .collect();

    format!(
        "---\n\
         title: Documento de Paridade\n\
         author: Autor Ficticio\n\
         ---\n\n\
         # {m1} Introducao\n\n\
         Paragrafo inicial com **negrito** e *italico*, e um [link externo](https://example.org). {m2}\n\n\
         ## {m3} Metodo\n\n\
         - item alfa\n\
         - item beta\n\n\
         | Coluna A | Coluna B |\n\
         |----------|----------|\n\
         | a1       | b1       |\n\n\
         ### {m4} Detalhe\n\n\
         ```text\n\
         codigo_exemplo(1)\n\
         ```\n\n\
         {enchimento}\
         ## {m5} Resultados\n\n\
         Paragrafo da segunda pagina. {m6}\n\n\
         # {m7} Conclusao\n\n\
         Ultimo paragrafo do documento. {m8}\n",
        m1 = MARCOS[0],
        m2 = MARCOS[1],
        m3 = MARCOS[2],
        m4 = MARCOS[3],
        m5 = MARCOS[4],
        m6 = MARCOS[5],
        m7 = MARCOS[6],
        m8 = MARCOS[7],
    )
}

/// Caminho do sidecar embutido cujo nome comece por `prefixo`, se este ambiente
/// já o tiver baixado (mesmo critério do harness de ida-e-volta do DOCX).
fn sidecar(prefixo: &str) -> Option<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("binaries");
    std::fs::read_dir(dir).ok()?.flatten().find_map(|entry| {
        let path = entry.path();
        let name = path.file_name()?.to_string_lossy().to_string();
        (name.starts_with(prefixo) && !name.ends_with(".sha256")).then_some(path)
    })
}

/// O Pandoc exige que o binário de `--pdf-engine` se chame exatamente
/// `typst[.exe]`; o sidecar cru carrega o sufixo de target-triple. Mesma cópia
/// para nome puro que `export::resolve_typst_for_pandoc` faz em `tauri dev`.
fn typst_com_nome_puro(dir: &Path) -> Option<PathBuf> {
    let bruto = sidecar("typst")?;
    let destino = dir.join(if cfg!(windows) { "typst.exe" } else { "typst" });
    std::fs::copy(&bruto, &destino).ok()?;
    Some(destino)
}

/// Exporta pelo caminho real do produto: mesmo template, mesmo dialeto e mesmo
/// motor de PDF que `export::run_pandoc_to_pdf` usa.
fn exportar_para_pdf(pandoc: &Path, typst: &Path, md: &Path, pdf: &Path) {
    let template = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("raiz do projeto")
        .join("templates/default/pdf-template.typ");
    let status = std::process::Command::new(pandoc)
        .arg(md)
        .args(["--from", "markdown+yaml_metadata_block-citations"])
        .arg("--template")
        .arg(&template)
        .arg("--pdf-engine")
        .arg(typst)
        .arg("-o")
        .arg(pdf)
        .status()
        .expect("falha ao executar o Pandoc embutido");
    assert!(status.success(), "Pandoc/Typst falhou ao gerar o PDF do documento-ouro");
}

/// Um nó de estrutura do Markdown — o suficiente para comparar riqueza, sem
/// virar um parser de Markdown completo.
#[derive(Debug, PartialEq)]
enum No {
    Titulo { nivel: usize, texto: String },
    Item,
    LinhaDeTabela,
    Codigo,
    Paragrafo(String),
}

fn estrutura(markdown: &str) -> Vec<No> {
    let mut nos = Vec::new();
    let mut em_codigo = false;
    for linha in markdown.lines() {
        let t = linha.trim();
        if t.starts_with("```") {
            if !em_codigo {
                nos.push(No::Codigo);
            }
            em_codigo = !em_codigo;
            continue;
        }
        if em_codigo || t.is_empty() {
            continue;
        }
        if let Some(resto) = t.strip_prefix('#') {
            let extras = resto.chars().take_while(|c| *c == '#').count();
            let texto = resto.trim_start_matches('#').trim().to_string();
            nos.push(No::Titulo { nivel: 1 + extras, texto });
        } else if t.starts_with("- ") || t.starts_with("* ") {
            nos.push(No::Item);
        } else if t.starts_with('|') {
            nos.push(No::LinhaDeTabela);
        } else {
            nos.push(No::Paragrafo(t.to_string()));
        }
    }
    nos
}

fn nivel_do_titulo_que_contem(nos: &[No], marco: &str) -> Option<usize> {
    nos.iter().find_map(|no| match no {
        No::Titulo { nivel, texto } if texto.contains(marco) => Some(*nivel),
        _ => None,
    })
}

/// Roda o ciclo completo e devolve o Markdown de volta. `None` quando os
/// sidecars ainda não foram baixados neste ambiente (mesma convenção do DOCX).
fn ida_e_volta() -> Option<String> {
    let pandoc = sidecar("pandoc")?;
    let dir = std::env::temp_dir().join(format!(
        "markforge-roundtrip-{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&dir).ok()?;
    let typst = typst_com_nome_puro(&dir)?;

    let md = dir.join("ouro.md");
    std::fs::write(&md, documento_ouro()).ok()?;
    let pdf = dir.join("ouro.pdf");
    exportar_para_pdf(&pandoc, &typst, &md, &pdf);

    let volta = import_pdf_to_markdown(&pdf.to_string_lossy()).expect("importação deveria ler o PDF");
    std::fs::remove_dir_all(&dir).ok();
    Some(volta)
}

/// **Nenhum trecho pode sumir.** Texto que entrou tem de voltar.
#[test]
fn ida_e_volta_nao_perde_nenhum_marco() {
    let Some(volta) = ida_e_volta() else {
        eprintln!("sidecars ausentes — harness de ida-e-volta ignorado");
        return;
    };
    let ausentes: Vec<&str> = MARCOS.iter().copied().filter(|m| !volta.contains(m)).collect();
    assert!(
        ausentes.is_empty(),
        "marcos perdidos na importação: {ausentes:?}\n--- markdown de volta ---\n{volta}"
    );
}

/// **A ordem do documento é a ordem da leitura.** Com marcos nas duas páginas,
/// qualquer mistura entre páginas quebra a monotonicidade.
#[test]
fn ida_e_volta_preserva_a_ordem_atraves_das_paginas() {
    let Some(volta) = ida_e_volta() else {
        eprintln!("sidecars ausentes — harness de ordem ignorado");
        return;
    };
    let posicoes: Vec<(usize, &str)> = MARCOS
        .iter()
        .filter_map(|m| volta.find(m).map(|p| (p, *m)))
        .collect();
    // Sem isto o teste passaria "verde" num documento vazio: lista sem
    // elemento nenhum está trivialmente em ordem.
    assert_eq!(
        posicoes.len(),
        MARCOS.len(),
        "ordem só é aferível com todos os marcos presentes\n--- markdown de volta ---\n{volta}"
    );
    let mut ordenados = posicoes.clone();
    ordenados.sort_by_key(|(p, _)| *p);
    let esperado: Vec<&str> = posicoes.iter().map(|(_, m)| *m).collect();
    let obtido: Vec<&str> = ordenados.iter().map(|(_, m)| *m).collect();
    assert_eq!(
        esperado, obtido,
        "a ordem do documento se perdeu (páginas embaralhadas?)\n--- markdown de volta ---\n{volta}"
    );
}

/// **A hierarquia de títulos vira a navegação do PDF exportado.** Nível errado
/// aqui é bookmark errado lá.
#[test]
fn ida_e_volta_preserva_o_nivel_dos_titulos() {
    let Some(volta) = ida_e_volta() else {
        eprintln!("sidecars ausentes — harness de títulos ignorado");
        return;
    };
    let nos = estrutura(&volta);
    for (marco, nivel) in [
        (MARCOS[0], 1),
        (MARCOS[2], 2),
        (MARCOS[3], 3),
        (MARCOS[4], 2),
        (MARCOS[6], 1),
    ] {
        assert_eq!(
            nivel_do_titulo_que_contem(&nos, marco),
            Some(nivel),
            "título de {marco} deveria voltar no nível {nivel}\n--- markdown de volta ---\n{volta}"
        );
    }
}

/// **Estruturas de bloco.** Tabela, lista e código são o que separa "texto
/// corrido" de "documento". Alvo das fatias de leitura de PDF marcado e do
/// fallback geométrico — enquanto lá não chega, fica declarado aqui.
#[test]
#[ignore = "alvo das próximas fatias: tabela, lista e código ainda não são reconhecidos na importação"]
fn ida_e_volta_preserva_tabela_lista_e_codigo() {
    let Some(volta) = ida_e_volta() else {
        eprintln!("sidecars ausentes — harness de blocos ignorado");
        return;
    };
    let nos = estrutura(&volta);
    assert!(nos.iter().any(|n| matches!(n, No::LinhaDeTabela)), "tabela não voltou:\n{volta}");
    assert!(nos.iter().any(|n| matches!(n, No::Item)), "lista não voltou:\n{volta}");
    assert!(nos.iter().any(|n| matches!(n, No::Codigo)), "bloco de código não voltou:\n{volta}");
}

/// **Links são navegabilidade.** Alvo da fatia de anotações (`/Annots`).
#[test]
#[ignore = "alvo da próxima fatia: anotações de link (/Annots) ainda não são lidas na importação"]
fn ida_e_volta_preserva_link() {
    let Some(volta) = ida_e_volta() else {
        eprintln!("sidecars ausentes — harness de link ignorado");
        return;
    };
    assert!(volta.contains("](https://example.org)"), "link não voltou:\n{volta}");
}

#[cfg(test)]
mod testes_do_harness {
    use super::*;

    /// O parser de estrutura é o instrumento de medida — ele mesmo precisa de
    /// teste, ou o harness mede errado sem avisar.
    #[test]
    fn estrutura_classifica_cada_bloco() {
        let nos = estrutura("# Titulo\n\ntexto\n\n- item\n\n| a | b |\n\n```\ncodigo\n```\n");
        assert_eq!(
            nos,
            vec![
                No::Titulo { nivel: 1, texto: "Titulo".into() },
                No::Paragrafo("texto".into()),
                No::Item,
                No::LinhaDeTabela,
                No::Codigo,
            ]
        );
    }

    #[test]
    fn estrutura_le_o_nivel_pelo_numero_de_cerquilhas() {
        let nos = estrutura("### Fundo\n");
        assert_eq!(nos, vec![No::Titulo { nivel: 3, texto: "Fundo".into() }]);
    }

    /// Dentro de bloco de código, `#` é código — não título.
    #[test]
    fn estrutura_nao_confunde_comentario_de_codigo_com_titulo() {
        let nos = estrutura("```sh\n# isto e um comentario\n```\n");
        assert_eq!(nos, vec![No::Codigo]);
    }

    #[test]
    fn documento_ouro_traz_todos_os_marcos() {
        let doc = documento_ouro();
        for marco in MARCOS {
            assert!(doc.contains(marco), "documento-ouro deveria conter {marco}");
        }
    }
}
