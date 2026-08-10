//! Pré-passe no `.docx` antes do Pandoc: promove a título os parágrafos que só
//! têm nível de estrutura de tópicos.
//!
//! **Por que existe.** Documento montado por formatação direta marca o título
//! com `<w:outlineLvl w:val="N"/>` dentro do `w:pPr`, mantendo um estilo de
//! parágrafo qualquer (lista, "sem espaçamento", recuo). O Word monta sumário e
//! navegação a partir desse nível — mas o leitor `.docx` do Pandoc reconhece
//! título **só pelo nome do estilo** (`w:name` = `heading N`). Sem esta passagem
//! o Markdown sai sem nenhum `#`: os títulos viram item de lista ordenada ou
//! citação em negrito, e a exportação de volta não tem onde aplicar hierarquia.
//!
//! **Escopo.** Só mexe em `word/document.xml` e `word/styles.xml`, e só nos
//! parágrafos que já declaram nível de tópico. Documento que já usa estilo de
//! título de verdade passa intacto.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Caminhos das duas partes do pacote OOXML que a pré-passe toca.
const DOCUMENT_PART: &str = "word/document.xml";
const STYLES_PART: &str = "word/styles.xml";

/// Maior nível de tópico que vira título. `w:outlineLvl` vai de 0 a 8; Markdown
/// e os estilos `heading N` do Word vão até 6, então 0..=5 é a faixa útil. Acima
/// disso (e no valor 9, que o Word usa para "corpo de texto") o parágrafo fica
/// como está.
const MAX_OUTLINE_LEVEL: u8 = 5;

/// Prefixo dos estilos que criamos quando o documento não traz `heading N`.
/// Determinístico de propósito: reconverter o mesmo arquivo duas vezes produz o
/// mesmo `styles.xml`.
const INJECTED_STYLE_PREFIX: &str = "MarkForgeTitulo";

/// Saída da pré-passe. Os contadores existem para o chamador saber se valeu a
/// pena reempacotar o `.docx` — tudo zero significa "nada a fazer, use o
/// original".
#[derive(Debug, PartialEq)]
pub struct Prepass {
    pub document_xml: String,
    pub styles_xml: String,
    pub promoted: usize,
    pub toc_removed: usize,
    pub images_inlined: usize,
}

impl Prepass {
    fn changed(&self) -> bool {
        self.promoted > 0 || self.toc_removed > 0 || self.images_inlined > 0
    }
}

/// Reempacota o `.docx` com a pré-passe aplicada, dentro de `work_dir`.
///
/// Devolve `Ok(None)` quando não há nada a promover — documento que já usa
/// estilo de título de verdade, ou pacote sem `styles.xml` (sem ele não há onde
/// declarar o estilo, e promover só o parágrafo não faria o Pandoc enxergar
/// título). Nesse caso o chamador segue com o arquivo original: a pré-passe
/// nunca é obrigatória para a importação funcionar.
pub fn prepare_for_import(source: &Path, work_dir: &Path) -> Result<Option<PathBuf>, String> {
    let open = || {
        File::open(source)
            .map_err(|e| format!("Não foi possível abrir o .docx: {e}"))
            .and_then(|f| {
                zip::ZipArchive::new(f).map_err(|e| format!("O .docx não pôde ser lido: {e}"))
            })
    };

    let mut archive = open()?;
    let (Some(document_xml), Some(styles_xml)) = (
        read_part(&mut archive, DOCUMENT_PART)?,
        read_part(&mut archive, STYLES_PART)?,
    ) else {
        return Ok(None);
    };

    let prepass = run_prepass(&document_xml, &styles_xml);
    if !prepass.changed() {
        return Ok(None);
    }

    let target = work_dir.join("import-prepared.docx");
    let out = File::create(&target).map_err(|e| format!("Não foi possível gravar o .docx preparado: {e}"))?;
    let mut writer = zip::ZipWriter::new(out);
    for index in 0..archive.len() {
        let entry = archive
            .by_index_raw(index)
            .map_err(|e| format!("Falha ao ler uma parte do .docx: {e}"))?;
        let name = entry.name().to_string();
        let replacement = match name.as_str() {
            DOCUMENT_PART => Some(prepass.document_xml.as_str()),
            STYLES_PART => Some(prepass.styles_xml.as_str()),
            _ => None,
        };
        match replacement {
            // As demais partes são copiadas sem recompactar — preserva bytes e
            // evita reprocessar mídia embutida.
            None => writer
                .raw_copy_file(entry)
                .map_err(|e| format!("Falha ao copiar uma parte do .docx: {e}"))?,
            Some(content) => {
                writer
                    .start_file(&name, zip::write::SimpleFileOptions::default())
                    .map_err(|e| format!("Falha ao regravar {name}: {e}"))?;
                writer
                    .write_all(content.as_bytes())
                    .map_err(|e| format!("Falha ao regravar {name}: {e}"))?;
            }
        }
    }
    writer
        .finish()
        .map_err(|e| format!("Falha ao fechar o .docx preparado: {e}"))?;
    Ok(Some(target))
}

fn read_part(archive: &mut zip::ZipArchive<File>, name: &str) -> Result<Option<String>, String> {
    match archive.by_name(name) {
        Err(_) => Ok(None),
        Ok(mut part) => {
            let mut buf = String::new();
            part.read_to_string(&mut buf)
                .map_err(|e| format!("A parte {name} do .docx não pôde ser lida: {e}"))?;
            Ok(Some(buf))
        }
    }
}

/// A pré-passe completa: tira o sumário automático e promove os títulos.
///
/// Nessa ordem de propósito — o sumário do Word é um bloco de resultado
/// cacheado, não estrutura; removê-lo antes evita varrer parágrafos que vão
/// embora de qualquer jeito.
pub fn run_prepass(document_xml: &str, styles_xml: &str) -> Prepass {
    let (without_toc, toc_removed) = remove_toc_controls(document_xml);
    let (with_images, images_inlined) = inline_anchored_pictures(&without_toc);
    let mut prepass = promote_outline_headings(&with_images, styles_xml);
    prepass.toc_removed = toc_removed;
    prepass.images_inlined = images_inlined;
    prepass
}

/// Resgata as figuras que o leitor do Pandoc não alcança, reapresentando cada
/// uma como desenho embutido simples.
///
/// **Por que existe.** No OOXML a figura pode estar embutida no fluxo do texto
/// (`wp:inline`), ancorada numa posição da página (`wp:anchor`) ou — o caso que
/// realmente quebra — **combinada com formas dentro de um grupo**, embrulhada em
/// `mc:AlternateContent`. É o que o Word grava quando a imagem foi montada junto
/// com faixas e molduras, típico de capa. Nesse arranjo a figura fica pendurada
/// num `wpg:wgp`, e não no `a:graphicData` de imagem que o Pandoc procura:
/// o documento chega ao Markdown **sem imagem nenhuma**, sem erro nenhum.
///
/// A saída descarta o que é só posicionamento e moldura, e preserva o que é
/// conteúdo: a referência à mídia, o tamanho da própria figura e o texto
/// alternativo.
///
/// **Só mexe onde há figura de verdade.** Forma decorativa sem imagem
/// (retângulo, seta, faixa de cor) não vira nada no Markdown de qualquer jeito;
/// reescrevê-la seria risco sem retorno.
fn inline_anchored_pictures(document_xml: &str) -> (String, usize) {
    replace_each(document_xml, "w:p", |paragraph| {
        let orphans = unreachable_pictures(paragraph);
        if orphans.is_empty() {
            return None;
        }
        // **Acrescenta, não substitui.** O grupo original fica onde está: ele
        // costuma carregar caixas de texto cujo conteúdo o Pandoc já lê, e
        // trocá-lo por uma figura engoliria esse texto. Cada figura resgatada
        // vira um `w:r` próprio no fim do parágrafo — o leitor do Pandoc só
        // enxerga um desenho por run, então dois desenhos no mesmo run fariam o
        // segundo desaparecer.
        let runs: String = orphans
            .iter()
            .enumerate()
            .map(|(index, pic)| {
                format!(
                    "<w:r><w:drawing>{}</w:drawing></w:r>",
                    inline_drawing(paragraph, pic, index)
                )
            })
            .collect();
        let close = "</w:p>";
        paragraph
            .strip_suffix(close)
            .map(|body| format!("{}{runs}{close}", drop_picture_fallbacks(body)))
    })
}

/// Tira o desenho antigo (VML) que existe só como alternativa para versões
/// remotas do Word.
///
/// O `mc:Fallback` é, por definição, a **mesma** figura desenhada de outro jeito.
/// Depois do resgate, o Pandoc passa a enxergar as duas representações e a
/// imagem sai repetida no Markdown. Só se descarta o bloco que é puramente
/// figura: `mc:Fallback` que carregue texto fica onde está — pode ser a única
/// via pela qual aquele texto chega à conversão.
fn drop_picture_fallbacks(paragraph: &str) -> String {
    replace_each(paragraph, "mc:Fallback", |block| {
        let so_figura = block.contains("<v:imagedata") && !block.contains("<w:t>") && !block.contains("<w:t ");
        so_figura.then(String::new)
    })
    .0
}

/// Figuras do parágrafo que o Pandoc não vai alcançar.
///
/// O leitor dele só reconhece figura pendurada num `a:graphicData` de imagem.
/// Figura dentro de grupo de formas fica sob o `graphicData` do grupo — e é
/// justamente essa que se perde. As já alcançáveis ficam de fora daqui, senão
/// seriam duplicadas.
fn unreachable_pictures(paragraph: &str) -> Vec<&str> {
    const PICTURE_URI: &str = "drawingml/2006/picture";

    let mut reachable: Vec<(usize, usize)> = Vec::new();
    let mut from = 0usize;
    while let Some((start, end)) = find_element(paragraph, "a:graphicData", from) {
        let tag = &paragraph[start..end];
        if attr_value(&tag[..tag_end(tag)], "uri").is_some_and(|uri| uri.contains(PICTURE_URI)) {
            reachable.push((start, end));
        }
        from = end;
    }

    let mut orphans = Vec::new();
    let mut from = 0usize;
    while let Some((start, end)) = find_element(paragraph, "pic:pic", from) {
        let pic = &paragraph[start..end];
        let inside_reachable = reachable.iter().any(|(s, e)| start >= *s && end <= *e);
        if pic.contains("r:embed=") && !inside_reachable {
            orphans.push(pic);
        }
        from = end;
    }
    orphans
}

/// Monta o `wp:inline` na ordem que o schema exige: `wp:extent`, `wp:docPr`,
/// `a:graphic`. O tamanho sai da própria figura (`a:ext`), não do grupo que a
/// envolvia — é o tamanho dela que interessa.
fn inline_drawing(block: &str, pic: &str, index: usize) -> String {
    let extent = find_element(pic, "a:ext", 0)
        .and_then(|(s, e)| {
            let tag = &pic[s..e];
            Some((attr_value(tag, "cx")?, attr_value(tag, "cy")?))
        })
        .map(|(cx, cy)| format!(r#"<wp:extent cx="{cx}" cy="{cy}"/>"#))
        .or_else(|| find_element(block, "wp:extent", 0).map(|(s, e)| block[s..e].to_string()))
        .unwrap_or_else(|| r#"<wp:extent cx="914400" cy="914400"/>"#.to_string());

    // O texto alternativo é conteúdo — descreve a imagem para quem não a vê.
    let descr = find_element(block, "wp:docPr", 0)
        .and_then(|(s, e)| attr_value(&block[s..e], "descr"))
        .map(|d| format!(r#" descr="{d}""#))
        .unwrap_or_default();
    let id = index + 1;

    format!(
        r#"<wp:inline xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" distT="0" distB="0" distL="0" distR="0">{extent}<wp:docPr id="{id}" name="Imagem {id}"{descr}/><wp:cNvGraphicFramePr/><a:graphic xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture">{pic}</a:graphicData></a:graphic></wp:inline>"#
    )
}

/// Percorre todos os elementos `name` e troca cada um pelo que `rebuild`
/// devolver; `None` deixa o elemento intacto. Devolve quantos foram trocados.
fn replace_each(
    xml: &str,
    name: &str,
    rebuild: impl Fn(&str) -> Option<String>,
) -> (String, usize) {
    let mut out = xml.to_string();
    let mut replaced = 0usize;
    let mut from = 0usize;
    while let Some((start, end)) = find_element(&out, name, from) {
        match rebuild(&out[start..end]) {
            Some(new_block) => {
                let next = start + new_block.len();
                out.replace_range(start..end, &new_block);
                replaced += 1;
                from = next;
            }
            None => from = end,
        }
    }
    (out, replaced)
}

/// Remove o controle de conteúdo que embrulha o sumário automático do Word.
///
/// **Por que remover.** O campo `TOC` guarda no arquivo o *resultado* da última
/// atualização feita no Word: uma linha por entrada, com link de indicador e
/// **número de página congelado**. Isso não é estrutura — é uma fotografia. Ao
/// virar Markdown o retrato entra como texto literal, some da navegação e ainda
/// mente sobre a paginação do documento novo. Com os títulos promovidos
/// (`promote_outline_headings`), o sumário passa a ser derivável do próprio
/// documento e é regerado na exportação.
///
/// Cobre o sumário moderno, embrulhado em `w:sdt` com galeria
/// `Table of Contents`. Campo `TOC` solto no corpo, sem `w:sdt` — forma antiga —
/// não é tocado: preferimos deixar texto a mais do que arriscar remover conteúdo
/// que não seja sumário.
fn remove_toc_controls(document_xml: &str) -> (String, usize) {
    let mut xml = document_xml.to_string();
    let mut removed = 0usize;
    let mut from = 0usize;
    while let Some((start, end)) = find_element(&xml, "w:sdt", from) {
        if is_toc_control(&xml[start..end]) {
            xml.replace_range(start..end, "");
            removed += 1;
            from = start;
        } else {
            // Avança só o suficiente para entrar no bloco: um sumário pode estar
            // aninhado dentro de outro controle de conteúdo.
            from = start + "<w:sdt".len();
        }
    }
    (xml, removed)
}

/// O `w:sdt` é o do sumário? Olha a galeria declarada no `w:sdtPr` do próprio
/// bloco — o primeiro do trecho, e não o de um controle aninhado.
fn is_toc_control(sdt: &str) -> bool {
    find_element(sdt, "w:sdtPr", 0)
        .and_then(|(start, end)| child_val(&sdt[start..end], "w:docPartGallery"))
        .is_some_and(|gallery| gallery.eq_ignore_ascii_case("table of contents"))
}

/// Reescreve `document.xml` e `styles.xml` de modo que todo parágrafo com
/// `w:outlineLvl` de 0 a 5 passe a usar um estilo cujo `w:name` é `heading N+1`.
/// Reaproveita o estilo de título que o documento já tiver; só injeta definição
/// nova para o nível que faltar.
fn promote_outline_headings(document_xml: &str, styles_xml: &str) -> Prepass {
    let existing = existing_heading_styles(styles_xml);
    let mut to_inject: BTreeSet<u8> = BTreeSet::new();
    let mut promoted = 0usize;

    let mut rewritten = String::with_capacity(document_xml.len() + 256);
    let mut cursor = 0usize;
    while let Some((start, end)) = find_element(document_xml, "w:pPr", cursor) {
        let block = &document_xml[start..end];
        match outline_level(block) {
            Some(outline) if outline <= MAX_OUTLINE_LEVEL => {
                let level = outline + 1;
                let style_id = match existing.get(&level) {
                    Some(id) => id.clone(),
                    None => {
                        to_inject.insert(level);
                        format!("{INJECTED_STYLE_PREFIX}{level}")
                    }
                };
                rewritten.push_str(&document_xml[cursor..start]);
                rewritten.push_str(&apply_heading_style(block, &style_id));
                promoted += 1;
            }
            _ => rewritten.push_str(&document_xml[cursor..end]),
        }
        cursor = end;
    }
    rewritten.push_str(&document_xml[cursor..]);

    Prepass {
        document_xml: if promoted == 0 {
            document_xml.to_string()
        } else {
            rewritten
        },
        styles_xml: inject_heading_styles(styles_xml, &to_inject),
        promoted,
        toc_removed: 0,
        images_inlined: 0,
    }
}

/// Mapa nível de título (1..=6) -> `w:styleId` do estilo cujo `w:name` é
/// exatamente `heading N`. O `w:name` é o nome canônico do OOXML — sempre em
/// inglês, mesmo em Word localizado, que só traduz o rótulo exibido. Comparação
/// sem distinguir caixa porque o Word alterna entre `heading 1` e `Heading 1`.
fn existing_heading_styles(styles_xml: &str) -> BTreeMap<u8, String> {
    let mut found = BTreeMap::new();
    let mut cursor = 0usize;
    while let Some((start, end)) = find_element(styles_xml, "w:style", cursor) {
        cursor = end;
        let block = &styles_xml[start..end];
        let (Some(style_id), Some(name)) = (
            attr_value(&block[..tag_end(block)], "w:styleId"),
            child_val(block, "w:name"),
        ) else {
            continue;
        };
        if let Some(level) = heading_level_of(name) {
            found.entry(level).or_insert_with(|| style_id.to_string());
        }
    }
    found
}

/// `"heading 3"` -> `Some(3)`. Qualquer outro nome (inclusive parecidos como
/// `"Título1"`, que é estilo de corpo) -> `None`.
fn heading_level_of(style_name: &str) -> Option<u8> {
    let rest = style_name.trim().to_ascii_lowercase();
    let digits = rest.strip_prefix("heading ")?;
    match digits.parse::<u8>() {
        Ok(level) if (1..=MAX_OUTLINE_LEVEL + 1).contains(&level) => Some(level),
        _ => None,
    }
}

/// Reescreve um bloco `w:pPr`: tira a numeração automática (senão o parágrafo
/// continua sendo lido como item de lista) e põe o estilo de título como
/// primeiro filho — posição exigida pelo schema do OOXML.
fn apply_heading_style(ppr: &str, style_id: &str) -> String {
    let mut block = ppr.to_string();
    while let Some((start, end)) = find_element(&block, "w:numPr", 0) {
        block.replace_range(start..end, "");
    }

    let style_tag = format!(r#"<w:pStyle w:val="{style_id}"/>"#);
    match find_element(&block, "w:pStyle", 0) {
        Some((start, end)) => block.replace_range(start..end, &style_tag),
        None => {
            let after_open = tag_end(&block);
            block.insert_str(after_open, &style_tag);
        }
    }
    block
}

/// Acrescenta as definições dos níveis de título que o documento não tinha.
/// `styleId` determinístico: reconverter o mesmo arquivo produz o mesmo XML.
fn inject_heading_styles(styles_xml: &str, levels: &BTreeSet<u8>) -> String {
    if levels.is_empty() {
        return styles_xml.to_string();
    }
    let Some(close) = styles_xml.rfind("</w:styles>") else {
        return styles_xml.to_string();
    };
    let mut defs = String::new();
    for level in levels {
        defs.push_str(&format!(
            r#"<w:style w:type="paragraph" w:styleId="{INJECTED_STYLE_PREFIX}{level}"><w:name w:val="heading {level}"/><w:qFormat/><w:pPr><w:outlineLvl w:val="{}"/></w:pPr></w:style>"#,
            level - 1
        ));
    }
    let mut out = String::with_capacity(styles_xml.len() + defs.len());
    out.push_str(&styles_xml[..close]);
    out.push_str(&defs);
    out.push_str(&styles_xml[close..]);
    out
}

/// Valor de `w:outlineLvl` dentro do bloco, se houver.
fn outline_level(ppr: &str) -> Option<u8> {
    child_val(ppr, "w:outlineLvl")?.parse().ok()
}

/// Localiza o elemento `name` a partir de `from`, devolvendo o intervalo que vai
/// do `<` de abertura até logo depois do fechamento. Conta profundidade para
/// aguentar aninhamento (um `w:pPr` dentro de `w:pPrChange`, por exemplo) e
/// trata a forma abreviada `<w:tag/>`.
fn find_element(xml: &str, name: &str, from: usize) -> Option<(usize, usize)> {
    let open = format!("<{name}");
    let close = format!("</{name}>");
    let mut search = from;
    loop {
        let start = search + xml[search..].find(&open)?;
        // `<w:pPr` não pode casar com `<w:pPrChange`: o caractere seguinte ao
        // nome tem de encerrar a tag ou separar um atributo.
        if !ends_tag_name(xml, start + open.len()) {
            search = start + open.len();
            continue;
        }

        let first_tag_end = start + tag_end(&xml[start..]);
        if xml[start..first_tag_end].ends_with("/>") {
            return Some((start, first_tag_end));
        }

        let mut depth = 1usize;
        let mut scan = first_tag_end;
        while depth > 0 {
            let next_open = xml[scan..].find(&open).map(|i| scan + i);
            let next_close = xml[scan..].find(&close).map(|i| scan + i)?;
            match next_open {
                Some(o) if o < next_close => {
                    let tag_close = o + tag_end(&xml[o..]);
                    if ends_tag_name(xml, o + open.len()) && !xml[o..tag_close].ends_with("/>") {
                        depth += 1;
                    }
                    scan = tag_close;
                }
                _ => {
                    depth -= 1;
                    scan = next_close + close.len();
                }
            }
        }
        return Some((start, scan));
    }
}

/// O nome da tag termina em `at`? Impede que `<w:pPr` case com `<w:pPrChange`.
fn ends_tag_name(xml: &str, at: usize) -> bool {
    match xml[at..].chars().next() {
        Some('>') | Some('/') => true,
        Some(c) => c.is_whitespace(),
        None => false,
    }
}

/// Posição logo depois do `>` que encerra a tag de abertura em `xml[0..]`.
fn tag_end(xml: &str) -> usize {
    xml.find('>').map(|i| i + 1).unwrap_or(xml.len())
}

/// Valor do atributo `w:val` do filho `name` — os elementos do OOXML que nos
/// interessam (`w:name`, `w:outlineLvl`) carregam o dado nesse atributo.
fn child_val<'a>(block: &'a str, name: &str) -> Option<&'a str> {
    let (start, end) = find_element(block, name, 0)?;
    let tag = &block[start..end];
    attr_value(&tag[..tag_end(tag)], "w:val")
}

/// Valor de um atributo dentro de uma tag de abertura já delimitada.
fn attr_value<'a>(tag: &'a str, attr: &str) -> Option<&'a str> {
    let needle = format!("{attr}=\"");
    let start = tag.find(&needle)? + needle.len();
    let len = tag[start..].find('"')?;
    Some(&tag[start..start + len])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `styles.xml` sintético. `headings` lista os níveis que já existem como
    /// estilo de título, com um `styleId` fora do padrão inglês — é o caso real
    /// de documento gerado em Word localizado.
    fn styles_with(headings: &[u8]) -> String {
        let mut s = String::from(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style>"#,
        );
        for n in headings {
            s.push_str(&format!(
                r#"<w:style w:type="paragraph" w:styleId="Ttulo{n}"><w:name w:val="heading {n}"/><w:basedOn w:val="Normal"/></w:style>"#
            ));
        }
        s.push_str("</w:styles>");
        s
    }

    fn document_with(paragraphs: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><w:body>{paragraphs}</w:body></w:document>"#
        )
    }

    fn paragraph(ppr_inner: &str, text: &str) -> String {
        format!(r#"<w:p><w:pPr>{ppr_inner}</w:pPr><w:r><w:t>{text}</w:t></w:r></w:p>"#)
    }

    #[test]
    fn promove_paragrafo_com_outline_lvl_usando_o_estilo_de_titulo_existente() {
        let doc = document_with(&paragraph(
            r#"<w:pStyle w:val="SemEspacamento"/><w:outlineLvl w:val="0"/>"#,
            "Secao",
        ));

        let out = promote_outline_headings(&doc, &styles_with(&[1]));

        assert!(
            out.document_xml.contains(r#"<w:pStyle w:val="Ttulo1"/>"#),
            "esperava o pStyle trocado pelo estilo heading 1 do documento, veio: {}",
            out.document_xml
        );
        assert!(!out.document_xml.contains("SemEspacamento"));
        assert_eq!(out.promoted, 1);
    }

    #[test]
    fn nivel_do_titulo_e_o_outline_lvl_mais_um() {
        let doc = document_with(&paragraph(
            r#"<w:pStyle w:val="Qualquer"/><w:outlineLvl w:val="2"/>"#,
            "Subsecao",
        ));

        let out = promote_outline_headings(&doc, &styles_with(&[1, 2, 3]));

        assert!(out.document_xml.contains(r#"<w:pStyle w:val="Ttulo3"/>"#));
    }

    #[test]
    fn remove_a_numeracao_automatica_do_paragrafo_promovido() {
        // Título montado como item de lista numerada: se o numPr ficar, o Pandoc
        // continua lendo lista e o "1." da numeração automática do Word ainda
        // atrapalharia — a numeração passa a ser responsabilidade do template.
        let doc = document_with(&paragraph(
            r#"<w:pStyle w:val="PargrafodaLista"/><w:numPr><w:ilvl w:val="0"/><w:numId w:val="27"/></w:numPr><w:outlineLvl w:val="0"/>"#,
            "Secao",
        ));

        let out = promote_outline_headings(&doc, &styles_with(&[1]));

        assert!(!out.document_xml.contains("<w:numPr>"));
        assert!(!out.document_xml.contains("w:numId"));
        assert!(out.document_xml.contains(r#"<w:pStyle w:val="Ttulo1"/>"#));
    }

    #[test]
    fn insere_pstyle_como_primeiro_elemento_quando_o_paragrafo_nao_tem_nenhum() {
        // O schema do OOXML exige w:pStyle como primeiro filho de w:pPr —
        // inserir no fim gera arquivo que o Word recusa a abrir.
        let doc = document_with(&paragraph(
            r#"<w:spacing w:after="0"/><w:outlineLvl w:val="1"/>"#,
            "Secao",
        ));

        let out = promote_outline_headings(&doc, &styles_with(&[1, 2]));

        assert!(
            out.document_xml
                .contains(r#"<w:pPr><w:pStyle w:val="Ttulo2"/><w:spacing"#),
            "pStyle precisa abrir o pPr, veio: {}",
            out.document_xml
        );
    }

    #[test]
    fn nao_toca_paragrafo_sem_outline_lvl() {
        let doc = document_with(&paragraph(r#"<w:pStyle w:val="Normal"/>"#, "Corpo"));

        let out = promote_outline_headings(&doc, &styles_with(&[1]));

        assert_eq!(out.document_xml, doc);
        assert_eq!(out.promoted, 0);
    }

    #[test]
    fn ignora_outline_lvl_fora_da_faixa_de_titulo() {
        // 9 é o valor que o Word usa para "corpo de texto" — não é título.
        let doc = document_with(&paragraph(
            r#"<w:pStyle w:val="Normal"/><w:outlineLvl w:val="9"/>"#,
            "Corpo",
        ));

        let out = promote_outline_headings(&doc, &styles_with(&[1]));

        assert_eq!(out.document_xml, doc);
        assert_eq!(out.promoted, 0);
    }

    #[test]
    fn injeta_o_estilo_de_titulo_que_faltar_no_styles_xml() {
        let doc = document_with(&paragraph(
            r#"<w:pStyle w:val="Qualquer"/><w:outlineLvl w:val="1"/>"#,
            "Subsecao",
        ));

        let out = promote_outline_headings(&doc, &styles_with(&[]));

        assert!(
            out.styles_xml.contains(r#"<w:name w:val="heading 2"/>"#),
            "esperava a definicao do heading 2 injetada, veio: {}",
            out.styles_xml
        );
        let injected = format!("{INJECTED_STYLE_PREFIX}2");
        assert!(out.styles_xml.contains(&format!(r#"w:styleId="{injected}""#)));
        assert!(out
            .document_xml
            .contains(&format!(r#"<w:pStyle w:val="{injected}"/>"#)));
        assert!(out.styles_xml.trim_end().ends_with("</w:styles>"));
    }

    #[test]
    fn injeta_apenas_os_niveis_realmente_usados() {
        let doc = document_with(&paragraph(
            r#"<w:pStyle w:val="Qualquer"/><w:outlineLvl w:val="0"/>"#,
            "Secao",
        ));

        let out = promote_outline_headings(&doc, &styles_with(&[]));

        assert!(out.styles_xml.contains(r#"<w:name w:val="heading 1"/>"#));
        assert!(!out.styles_xml.contains(r#"<w:name w:val="heading 2"/>"#));
    }

    #[test]
    fn reaproveita_o_estilo_existente_sem_duplicar_a_definicao() {
        let doc = document_with(&paragraph(
            r#"<w:pStyle w:val="Qualquer"/><w:outlineLvl w:val="0"/>"#,
            "Secao",
        ));

        let out = promote_outline_headings(&doc, &styles_with(&[1]));

        assert_eq!(out.styles_xml.matches(r#"w:val="heading 1""#).count(), 1);
        assert!(!out.styles_xml.contains(INJECTED_STYLE_PREFIX));
    }

    #[test]
    fn nao_confunde_estilo_de_nome_parecido_com_titulo() {
        // "Título1" (sem espaço, nome de exibição localizado) é um estilo comum
        // de corpo — não é o "heading 1" canonico do OOXML.
        let styles = r#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:type="paragraph" w:styleId="Ttulo10"><w:name w:val="Título1"/></w:style></w:styles>"#;
        let doc = document_with(&paragraph(
            r#"<w:pStyle w:val="Qualquer"/><w:outlineLvl w:val="0"/>"#,
            "Secao",
        ));

        let out = promote_outline_headings(&doc, styles);

        assert!(out
            .document_xml
            .contains(&format!(r#"<w:pStyle w:val="{INJECTED_STYLE_PREFIX}1"/>"#)));
    }

    #[test]
    fn promove_todos_os_paragrafos_e_conta_quantos_foram() {
        let paragraphs = format!(
            "{}{}{}",
            paragraph(
                r#"<w:pStyle w:val="A"/><w:outlineLvl w:val="0"/>"#,
                "Um"
            ),
            paragraph(r#"<w:pStyle w:val="B"/>"#, "Corpo"),
            paragraph(
                r#"<w:pStyle w:val="C"/><w:outlineLvl w:val="1"/>"#,
                "Dois"
            ),
        );
        let doc = document_with(&paragraphs);

        let out = promote_outline_headings(&doc, &styles_with(&[1, 2]));

        assert_eq!(out.promoted, 2);
        assert!(out.document_xml.contains(r#"<w:pStyle w:val="Ttulo1"/>"#));
        assert!(out.document_xml.contains(r#"<w:pStyle w:val="Ttulo2"/>"#));
        assert!(out.document_xml.contains(r#"<w:pStyle w:val="B"/>"#));
    }

    #[test]
    fn preserva_o_texto_e_o_restante_do_paragrafo() {
        let doc = document_with(&paragraph(
            r#"<w:pStyle w:val="A"/><w:outlineLvl w:val="0"/><w:jc w:val="both"/>"#,
            "Titulo da secao",
        ));

        let out = promote_outline_headings(&doc, &styles_with(&[1]));

        assert!(out.document_xml.contains("<w:t>Titulo da secao</w:t>"));
        assert!(out.document_xml.contains(r#"<w:jc w:val="both"/>"#));
        assert!(out.document_xml.contains(r#"<w:outlineLvl w:val="0"/>"#));
    }

    #[test]
    fn documento_que_ja_usa_estilo_de_titulo_passa_intacto() {
        // Sem outlineLvl direto no paragrafo, nada a promover — o Pandoc ja le
        // esse documento corretamente.
        let doc = document_with(&paragraph(r#"<w:pStyle w:val="Ttulo1"/>"#, "Secao"));
        let styles = styles_with(&[1]);

        let out = promote_outline_headings(&doc, &styles);

        assert_eq!(out.document_xml, doc);
        assert_eq!(out.styles_xml, styles);
        assert_eq!(out.promoted, 0);
    }

    #[test]
    fn nao_atravessa_a_fronteira_de_paragrafos_vizinhos() {
        // Um paragrafo sem outlineLvl seguido de outro com — o primeiro nao pode
        // ser arrastado junto.
        let paragraphs = format!(
            "{}{}",
            paragraph(r#"<w:pStyle w:val="Corpo"/>"#, "Antes"),
            paragraph(
                r#"<w:pStyle w:val="A"/><w:outlineLvl w:val="0"/>"#,
                "Titulo"
            ),
        );
        let doc = document_with(&paragraphs);

        let out = promote_outline_headings(&doc, &styles_with(&[1]));

        assert_eq!(out.promoted, 1);
        assert!(out.document_xml.contains(r#"<w:pStyle w:val="Corpo"/>"#));
    }

    /// Monta um `.docx` mínimo e sintético. Amostra real de defeito nunca vira
    /// fixture — o padrão estrutural é o que importa, não o conteúdo.
    fn write_docx(path: &Path, parts: &[(&str, &str)]) {
        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        for (name, content) in parts {
            zip.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(content.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }

    fn read_part_of(path: &Path, name: &str) -> Option<String> {
        let mut zip = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
        let mut part = zip.by_name(name).ok()?;
        let mut buf = String::new();
        part.read_to_string(&mut buf).unwrap();
        Some(buf)
    }

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("markforge-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    const CONTENT_TYPES: &str = r#"<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"/>"#;

    #[test]
    fn reempacota_o_docx_com_os_titulos_promovidos() {
        let dir = temp_dir();
        let source = dir.join("entrada.docx");
        let doc = document_with(&paragraph(
            r#"<w:pStyle w:val="SemEspacamento"/><w:outlineLvl w:val="0"/>"#,
            "Secao",
        ));
        write_docx(
            &source,
            &[
                ("[Content_Types].xml", CONTENT_TYPES),
                (DOCUMENT_PART, &doc),
                (STYLES_PART, &styles_with(&[1])),
            ],
        );

        let prepared = prepare_for_import(&source, &dir).unwrap().expect("esperava um .docx preparado");

        let rewritten = read_part_of(&prepared, DOCUMENT_PART).unwrap();
        assert!(rewritten.contains(r#"<w:pStyle w:val="Ttulo1"/>"#));
        assert!(!rewritten.contains("SemEspacamento"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn preserva_as_demais_partes_do_pacote() {
        let dir = temp_dir();
        let source = dir.join("entrada.docx");
        let midia = "bytes-de-midia-ficticios";
        let doc = document_with(&paragraph(
            r#"<w:pStyle w:val="A"/><w:outlineLvl w:val="0"/>"#,
            "Secao",
        ));
        write_docx(
            &source,
            &[
                ("[Content_Types].xml", CONTENT_TYPES),
                (DOCUMENT_PART, &doc),
                (STYLES_PART, &styles_with(&[1])),
                ("word/media/image1.png", midia),
            ],
        );

        let prepared = prepare_for_import(&source, &dir).unwrap().unwrap();

        assert_eq!(
            read_part_of(&prepared, "word/media/image1.png").as_deref(),
            Some(midia),
            "midia embutida nao pode se perder no reempacotamento"
        );
        assert_eq!(
            read_part_of(&prepared, "[Content_Types].xml").as_deref(),
            Some(CONTENT_TYPES)
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn grava_o_estilo_injetado_no_styles_xml_do_pacote() {
        let dir = temp_dir();
        let source = dir.join("entrada.docx");
        let doc = document_with(&paragraph(
            r#"<w:pStyle w:val="A"/><w:outlineLvl w:val="1"/>"#,
            "Subsecao",
        ));
        write_docx(
            &source,
            &[
                ("[Content_Types].xml", CONTENT_TYPES),
                (DOCUMENT_PART, &doc),
                (STYLES_PART, &styles_with(&[])),
            ],
        );

        let prepared = prepare_for_import(&source, &dir).unwrap().unwrap();

        let styles = read_part_of(&prepared, STYLES_PART).unwrap();
        assert!(styles.contains(r#"<w:name w:val="heading 2"/>"#));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn nao_reempacota_documento_que_ja_tem_titulo_de_verdade() {
        let dir = temp_dir();
        let source = dir.join("entrada.docx");
        let doc = document_with(&paragraph(r#"<w:pStyle w:val="Ttulo1"/>"#, "Secao"));
        write_docx(
            &source,
            &[
                ("[Content_Types].xml", CONTENT_TYPES),
                (DOCUMENT_PART, &doc),
                (STYLES_PART, &styles_with(&[1])),
            ],
        );

        assert_eq!(prepare_for_import(&source, &dir).unwrap(), None);
        assert!(!dir.join("import-prepared.docx").exists());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn segue_sem_preparo_quando_o_pacote_nao_tem_styles_xml() {
        let dir = temp_dir();
        let source = dir.join("entrada.docx");
        let doc = document_with(&paragraph(
            r#"<w:pStyle w:val="A"/><w:outlineLvl w:val="0"/>"#,
            "Secao",
        ));
        write_docx(
            &source,
            &[("[Content_Types].xml", CONTENT_TYPES), (DOCUMENT_PART, &doc)],
        );

        assert_eq!(prepare_for_import(&source, &dir).unwrap(), None);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn erro_claro_quando_o_arquivo_nao_e_um_docx() {
        let dir = temp_dir();
        let source = dir.join("naozip.docx");
        std::fs::write(&source, "isto nao e um pacote zip").unwrap();

        let err = prepare_for_import(&source, &dir).unwrap_err();

        assert!(err.contains(".docx"), "mensagem pouco clara: {err}");

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Controle de conteúdo do Word. `gallery` vazio significa sem `docPartObj`.
    fn content_control(gallery: &str, inner: &str) -> String {
        let props = if gallery.is_empty() {
            String::from("<w:sdtPr><w:id w:val=\"1\"/></w:sdtPr>")
        } else {
            format!(
                r#"<w:sdtPr><w:id w:val="1"/><w:docPartObj><w:docPartGallery w:val="{gallery}"/><w:docPartUnique/></w:docPartObj></w:sdtPr>"#
            )
        };
        format!("<w:sdt>{props}<w:sdtContent>{inner}</w:sdtContent></w:sdt>")
    }

    /// Entrada de sumário como o Word grava: link de indicador com o número de
    /// página da última atualização — retrato, não estrutura.
    fn toc_entry(label: &str) -> String {
        format!(
            r#"<w:p><w:pPr><w:pStyle w:val="Sumrio1"/></w:pPr><w:hyperlink w:anchor="_Toc1"><w:r><w:t>{label}</w:t></w:r><w:r><w:t>3</w:t></w:r></w:hyperlink></w:p>"#
        )
    }

    #[test]
    fn remove_o_bloco_do_sumario_automatico() {
        let toc = content_control("Table of Contents", &toc_entry("Entrada do sumario"));
        let doc = document_with(&toc);

        let out = run_prepass(&doc, &styles_with(&[1]));

        assert_eq!(out.toc_removed, 1);
        assert!(!out.document_xml.contains("Entrada do sumario"));
        assert!(!out.document_xml.contains("<w:sdt>"));
    }

    #[test]
    fn preserva_o_conteudo_ao_redor_do_sumario() {
        let doc = document_with(&format!(
            "{}{}{}",
            paragraph(r#"<w:pStyle w:val="Normal"/>"#, "Antes do sumario"),
            content_control("Table of Contents", &toc_entry("Entrada")),
            paragraph(r#"<w:pStyle w:val="Normal"/>"#, "Depois do sumario"),
        ));

        let out = run_prepass(&doc, &styles_with(&[1]));

        assert!(out.document_xml.contains("Antes do sumario"));
        assert!(out.document_xml.contains("Depois do sumario"));
        assert!(out.document_xml.contains("</w:body></w:document>"));
    }

    #[test]
    fn nao_remove_controle_de_conteudo_que_nao_e_sumario() {
        // Caixa de seleção, campo de formulário, bloco de construção: tudo isso
        // é w:sdt e carrega conteúdo real do documento.
        let doc = document_with(&content_control(
            "Quick Parts",
            &paragraph(r#"<w:pStyle w:val="Normal"/>"#, "Conteudo real"),
        ));

        let out = run_prepass(&doc, &styles_with(&[1]));

        assert_eq!(out.toc_removed, 0);
        assert!(out.document_xml.contains("Conteudo real"));
    }

    #[test]
    fn nao_remove_controle_de_conteudo_sem_galeria_declarada() {
        let doc = document_with(&content_control(
            "",
            &paragraph(r#"<w:pStyle w:val="Normal"/>"#, "Conteudo real"),
        ));

        let out = run_prepass(&doc, &styles_with(&[1]));

        assert_eq!(out.toc_removed, 0);
        assert!(out.document_xml.contains("Conteudo real"));
    }

    #[test]
    fn acha_o_sumario_aninhado_dentro_de_outro_controle() {
        let inner = content_control("Table of Contents", &toc_entry("Entrada"));
        let outer = content_control(
            "Quick Parts",
            &format!(
                "{}{}",
                paragraph(r#"<w:pStyle w:val="Normal"/>"#, "Conteudo real"),
                inner
            ),
        );
        let doc = document_with(&outer);

        let out = run_prepass(&doc, &styles_with(&[1]));

        assert_eq!(out.toc_removed, 1);
        assert!(!out.document_xml.contains("Entrada"));
        assert!(
            out.document_xml.contains("Conteudo real"),
            "o controle externo nao pode ir junto"
        );
    }

    #[test]
    fn remove_todos_os_sumarios_quando_ha_mais_de_um() {
        let doc = document_with(&format!(
            "{}{}{}",
            content_control("Table of Contents", &toc_entry("Um")),
            paragraph(r#"<w:pStyle w:val="Normal"/>"#, "Meio"),
            content_control("Table of Contents", &toc_entry("Dois")),
        ));

        let out = run_prepass(&doc, &styles_with(&[1]));

        assert_eq!(out.toc_removed, 2);
        assert!(out.document_xml.contains("Meio"));
    }

    #[test]
    fn documento_sem_sumario_passa_intacto() {
        let doc = document_with(&paragraph(r#"<w:pStyle w:val="Normal"/>"#, "Corpo"));

        let out = run_prepass(&doc, &styles_with(&[1]));

        assert_eq!(out.toc_removed, 0);
        assert_eq!(out.document_xml, doc);
    }

    #[test]
    fn campo_toc_solto_sem_controle_de_conteudo_nao_e_tocado() {
        // Forma antiga do sumário. Fora do escopo desta passagem — o combinado é
        // deixar texto a mais, nunca arriscar remover o que nao e sumario.
        let doc = document_with(
            r#"<w:p><w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText> TOC \o "1-3" </w:instrText></w:r><w:r><w:fldChar w:fldCharType="end"/></w:r></w:p>"#,
        );

        let out = run_prepass(&doc, &styles_with(&[1]));

        assert_eq!(out.toc_removed, 0);
        assert_eq!(out.document_xml, doc);
    }

    #[test]
    fn remove_o_sumario_e_promove_os_titulos_na_mesma_passagem() {
        let doc = document_with(&format!(
            "{}{}",
            content_control("Table of Contents", &toc_entry("Entrada")),
            paragraph(
                r#"<w:pStyle w:val="A"/><w:outlineLvl w:val="0"/>"#,
                "Titulo"
            ),
        ));

        let out = run_prepass(&doc, &styles_with(&[1]));

        assert_eq!(out.toc_removed, 1);
        assert_eq!(out.promoted, 1);
        assert!(out.document_xml.contains(r#"<w:pStyle w:val="Ttulo1"/>"#));
        assert!(!out.document_xml.contains("Entrada"));
    }

    #[test]
    fn reempacota_quando_so_o_sumario_foi_removido() {
        let dir = temp_dir();
        let source = dir.join("entrada.docx");
        let doc = document_with(&format!(
            "{}{}",
            content_control("Table of Contents", &toc_entry("Entrada")),
            paragraph(r#"<w:pStyle w:val="Ttulo1"/>"#, "Titulo de verdade"),
        ));
        write_docx(
            &source,
            &[
                ("[Content_Types].xml", CONTENT_TYPES),
                (DOCUMENT_PART, &doc),
                (STYLES_PART, &styles_with(&[1])),
            ],
        );

        // Nada a promover — mas o sumário sozinho já justifica reempacotar.
        let prepared = prepare_for_import(&source, &dir)
            .unwrap()
            .expect("esperava .docx preparado só pela remocao do sumario");

        let rewritten = read_part_of(&prepared, DOCUMENT_PART).unwrap();
        assert!(!rewritten.contains("Entrada"));
        assert!(rewritten.contains("Titulo de verdade"));

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Partes que faltam para o pacote sintético ser um `.docx` que o Pandoc
    /// aceita abrir. Conteúdo é esqueleto do OOXML, sem dado nenhum.
    fn package_skeleton() -> Vec<(&'static str, String)> {
        vec![
            ("[Content_Types].xml", r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/></Types>"#.to_string()),
            ("_rels/.rels", r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#.to_string()),
            ("word/_rels/document.xml.rels", r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/></Relationships>"#.to_string()),
        ]
    }

    /// Caminho do Pandoc embutido, se este ambiente já tiver o sidecar baixado.
    fn pandoc_sidecar() -> Option<PathBuf> {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("binaries");
        std::fs::read_dir(dir).ok()?.flatten().find_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_string_lossy().to_string();
            (name.starts_with("pandoc") && !name.ends_with(".sha256")).then_some(path)
        })
    }

    /// Espelha o que `import_via_pandoc` faz de verdade — inclusive o diretório
    /// de trabalho e a extração de mídia, de que dependem os links relativos.
    fn pandoc_to_markdown(pandoc: &Path, docx: &Path, out: &Path) -> String {
        let status = std::process::Command::new(pandoc)
            .current_dir(out.parent().unwrap())
            .arg(docx)
            .args(["--from", "docx", "--to", super::super::import::IMPORT_MARKDOWN_DIALECT])
            .args(["--wrap", "none", "--extract-media", ".", "-o"])
            .arg(out)
            .status()
            .expect("falha ao executar o Pandoc embutido");
        assert!(status.success(), "Pandoc retornou erro no fixture sintetico");
        std::fs::read_to_string(out).unwrap()
    }

    /// O teste que carrega a missão: título marcado só por nível de tópico
    /// precisa chegar ao Markdown como `#`. Roda de ponta a ponta com o Pandoc
    /// real; é ignorado onde o sidecar ainda não foi baixado.
    #[test]
    fn ponta_a_ponta_o_pandoc_passa_a_enxergar_o_titulo() {
        let Some(pandoc) = pandoc_sidecar() else {
            eprintln!("sidecar do Pandoc ausente — teste de ponta a ponta ignorado");
            return;
        };
        let dir = temp_dir();

        // Padrão do defeito: estilo de lista + numeração automática + nível de
        // tópico, sem nenhum estilo de título.
        let doc = document_with(&format!(
            "{}{}",
            paragraph(
                r#"<w:pStyle w:val="PargrafodaLista"/><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr><w:outlineLvl w:val="0"/>"#,
                "Titulo Sintetico",
            ),
            paragraph(r#"<w:pStyle w:val="Normal"/>"#, "Corpo do paragrafo."),
        ));
        let mut parts = package_skeleton();
        parts.push((DOCUMENT_PART, doc));
        parts.push((STYLES_PART, styles_with(&[])));
        let borrowed: Vec<(&str, &str)> = parts.iter().map(|(n, c)| (*n, c.as_str())).collect();

        let source = dir.join("sintetico.docx");
        write_docx(&source, &borrowed);

        let antes = pandoc_to_markdown(&pandoc, &source, &dir.join("antes.md"));
        assert!(
            !antes.contains("# Titulo Sintetico"),
            "o fixture precisa reproduzir o defeito; sem a pre-passe nao pode haver titulo. Veio: {antes}"
        );

        let prepared = prepare_for_import(&source, &dir).unwrap().expect("esperava .docx preparado");
        let depois = pandoc_to_markdown(&pandoc, &prepared, &dir.join("depois.md"));

        assert!(
            depois.contains("# Titulo Sintetico"),
            "apos a pre-passe o Pandoc tem de emitir o titulo. Veio: {depois}"
        );
        assert!(depois.contains("Corpo do paragrafo."), "o corpo nao pode se perder");

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Tabela simples: uma linha de cabeçalho e uma de dados, um parágrafo por
    /// célula. Cabe em tabela de canos (GFM) — o editor renderiza.
    fn simple_table() -> String {
        let cell = |t: &str| {
            format!(r#"<w:tc><w:tcPr><w:tcW w:w="2000" w:type="dxa"/></w:tcPr><w:p><w:r><w:t>{t}</w:t></w:r></w:p></w:tc>"#)
        };
        format!(
            "<w:tbl><w:tblPr><w:tblW w:w=\"4000\" w:type=\"dxa\"/></w:tblPr><w:tblGrid><w:gridCol w:w=\"2000\"/><w:gridCol w:w=\"2000\"/></w:tblGrid><w:tr>{}{}</w:tr><w:tr>{}{}</w:tr></w:tbl>",
            cell("Criterio"),
            cell("Peso"),
            cell("Alinhamento"),
            cell("25%")
        )
    }

    /// O Markdown que sai da importação tem de ser legível pelo editor: nada de
    /// sintaxe que só o Pandoc entende. É o critério da missão do lado de quem
    /// abre o arquivo — humano na tela, e IA sem conversão adicional.
    #[test]
    fn o_markdown_gerado_nao_usa_sintaxe_fora_do_commonmark_e_gfm() {
        let Some(pandoc) = pandoc_sidecar() else {
            eprintln!("sidecar do Pandoc ausente — teste de dialeto ignorado");
            return;
        };
        let dir = temp_dir();

        let corpo = format!(
            "{}{}{}",
            // Título com indicador de sumário, como o Word grava.
            format!(
                r#"<w:p><w:pPr><w:pStyle w:val="A"/><w:outlineLvl w:val="0"/></w:pPr><w:bookmarkStart w:id="1" w:name="_Toc1"/><w:r><w:t>Titulo Sintetico</w:t></w:r><w:bookmarkEnd w:id="1"/></w:p>"#
            ),
            paragraph(r#"<w:pStyle w:val="Normal"/>"#, "Corpo do paragrafo."),
            simple_table(),
        );
        let mut parts = package_skeleton();
        parts.push((DOCUMENT_PART, document_with(&corpo)));
        parts.push((STYLES_PART, styles_with(&[])));
        let borrowed: Vec<(&str, &str)> = parts.iter().map(|(n, c)| (*n, c.as_str())).collect();
        let source = dir.join("dialeto.docx");
        write_docx(&source, &borrowed);

        let prepared = prepare_for_import(&source, &dir).unwrap().expect("esperava preparo");
        let md = pandoc_to_markdown(&pandoc, &prepared, &dir.join("saida.md"));

        assert!(
            !md.contains("]{#"),
            "span de indicador nao renderiza no editor. Veio:\n{md}"
        );
        assert!(
            !md.lines().any(|l| l.starts_with('#') && l.contains('{')),
            "atributo de titulo nao renderiza no editor. Veio:\n{md}"
        );
        assert!(
            !md.lines().any(|l| l.trim_start().starts_with("+--")),
            "tabela em grade nao renderiza no editor. Veio:\n{md}"
        );
        assert!(
            md.contains("| Criterio"),
            "tabela simples tem de sair como tabela de canos. Veio:\n{md}"
        );
        // Nada pode se perder no caminho.
        assert!(md.contains("# Titulo Sintetico"));
        assert!(md.contains("Corpo do paragrafo."));
        assert!(md.contains("Alinhamento") && md.contains("25%"));

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Tabela cuja célula tem bloco dentro (dois parágrafos) — não cabe em
    /// tabela de canos e força a saída a escolher entre HTML e perder o dado.
    fn table_with_block_cell() -> String {
        let plain = |t: &str| format!(r#"<w:tc><w:p><w:r><w:t>{t}</w:t></w:r></w:p></w:tc>"#);
        let multi = r#"<w:tc><w:p><w:r><w:t>Primeira linha da celula</w:t></w:r></w:p><w:p><w:r><w:t>Segunda linha da celula</w:t></w:r></w:p></w:tc>"#;
        format!(
            "<w:tbl><w:tblGrid><w:gridCol w:w=\"2000\"/><w:gridCol w:w=\"2000\"/></w:tblGrid><w:tr>{}{}</w:tr><w:tr>{}{}</w:tr></w:tbl>",
            plain("Item"),
            plain("Detalhe"),
            plain("Um"),
            multi
        )
    }

    /// Guarda contra a armadilha: limpar o dialeto não pode custar conteúdo.
    /// `gfm-raw_html` deixaria a saída mais bonita e apagaria esta tabela
    /// inteira, sem erro nenhum.
    #[test]
    fn tabela_complexa_sobrevive_mesmo_sem_caber_em_tabela_de_canos() {
        let Some(pandoc) = pandoc_sidecar() else {
            eprintln!("sidecar do Pandoc ausente — teste de nao-perda ignorado");
            return;
        };
        let dir = temp_dir();

        let mut parts = package_skeleton();
        parts.push((DOCUMENT_PART, document_with(&table_with_block_cell())));
        parts.push((STYLES_PART, styles_with(&[1])));
        let borrowed: Vec<(&str, &str)> = parts.iter().map(|(n, c)| (*n, c.as_str())).collect();
        let source = dir.join("tabela.docx");
        write_docx(&source, &borrowed);

        let md = pandoc_to_markdown(&pandoc, &source, &dir.join("saida.md"));

        for esperado in [
            "Item",
            "Detalhe",
            "Primeira linha da celula",
            "Segunda linha da celula",
        ] {
            assert!(
                md.contains(esperado),
                "\"{esperado}\" sumiu da conversao — dialeto esta descartando conteudo. Veio:\n{md}"
            );
        }

        std::fs::remove_dir_all(&dir).ok();
    }

    /// O critério da missão: importar e exportar de volta tem de devolver um
    /// documento com a mesma estrutura — hierarquia navegável, tabela e texto.
    /// Antes da pré-passe isto era impossível: sem `#` no Markdown, o DOCX de
    /// volta não tinha um único título, e sem título não há navegação nem
    /// sumário regerável.
    #[test]
    fn ida_e_volta_devolve_a_estrutura_do_documento() {
        let Some(pandoc) = pandoc_sidecar() else {
            eprintln!("sidecar do Pandoc ausente — teste de ida-e-volta ignorado");
            return;
        };
        let dir = temp_dir();

        let corpo = format!(
            "{}{}{}{}",
            content_control("Table of Contents", &toc_entry("Titulo Sintetico")),
            format!(
                r#"<w:p><w:pPr><w:pStyle w:val="PargrafodaLista"/><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr><w:outlineLvl w:val="0"/></w:pPr><w:r><w:t>Titulo Sintetico</w:t></w:r></w:p>"#
            ),
            paragraph(r#"<w:pStyle w:val="SemEspacamento"/><w:outlineLvl w:val="1"/>"#, "Subtitulo Sintetico"),
            format!(
                "{}{}",
                paragraph(r#"<w:pStyle w:val="Normal"/>"#, "Corpo do paragrafo."),
                simple_table()
            ),
        );
        let mut parts = package_skeleton();
        parts.push((DOCUMENT_PART, document_with(&corpo)));
        parts.push((STYLES_PART, styles_with(&[])));
        let borrowed: Vec<(&str, &str)> = parts.iter().map(|(n, c)| (*n, c.as_str())).collect();
        let source = dir.join("ida.docx");
        write_docx(&source, &borrowed);

        // Ida: importação (pré-passe + Pandoc no dialeto do editor).
        let prepared = prepare_for_import(&source, &dir).unwrap().expect("esperava preparo");
        let md_path = dir.join("intermediario.md");
        let md = pandoc_to_markdown(&pandoc, &prepared, &md_path);
        assert!(md.contains("# Titulo Sintetico"));
        assert!(md.contains("## Subtitulo Sintetico"));

        // Volta: exportação para DOCX, com o dialeto que o app usa de verdade.
        let volta = dir.join("volta.docx");
        let status = std::process::Command::new(&pandoc)
            .arg(&md_path)
            .args(["--from", super::super::export::EXPORT_MARKDOWN_DIALECT])
            .args(["--standalone", "-o"])
            .arg(&volta)
            .status()
            .expect("falha ao executar o Pandoc na volta");
        assert!(status.success(), "Pandoc falhou na exportação de volta");

        let doc_de_volta = read_part_of(&volta, DOCUMENT_PART).expect("DOCX de volta sem document.xml");

        // Compara pelo atributo, não pela tag inteira: a serialização exata
        // (espaço antes de `/>`) é detalhe do escritor, não contrato.
        assert!(
            doc_de_volta.contains(r#"w:pStyle w:val="Heading1""#),
            "o DOCX de volta precisa ter titulo de nivel 1 — e o que da navegacao e sumario"
        );
        assert!(
            doc_de_volta.contains(r#"w:pStyle w:val="Heading2""#),
            "o DOCX de volta precisa preservar a hierarquia de nivel 2"
        );
        assert!(doc_de_volta.contains("Titulo Sintetico"));
        assert!(doc_de_volta.contains("Subtitulo Sintetico"));
        assert!(doc_de_volta.contains("Corpo do paragrafo."));
        assert!(doc_de_volta.contains("<w:tbl>"), "a tabela nao pode se perder na volta");
        assert!(doc_de_volta.contains("Alinhamento") && doc_de_volta.contains("25%"));
        assert!(
            !doc_de_volta.contains("](#_Toc"),
            "o retrato do sumario antigo nao pode ressurgir no documento de volta"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// PNG 1x1 transparente — imagem de verdade, sem nada dentro.
    const PNG_1X1: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00,
        0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    fn write_docx_bin(path: &Path, parts: &[(&str, Vec<u8>)]) {
        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        for (name, content) in parts {
            zip.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(content).unwrap();
        }
        zip.finish().unwrap();
    }

    /// Corpo do desenho: o gráfico com a referência à mídia.
    fn picture_graphic(rel_id: &str) -> String {
        format!(
            r#"<a:graphic xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:pic xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:nvPicPr><pic:cNvPr id="1" name="Imagem 1"/><pic:cNvPicPr/></pic:nvPicPr><pic:blipFill><a:blip r:embed="{rel_id}"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill><pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="914400"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic>"#
        )
    }

    /// Desenho **ancorado** (flutuante) — a forma que o Pandoc não lê.
    fn anchored_drawing(rel_id: &str) -> String {
        format!(
            r#"<w:p><w:r><w:drawing><wp:anchor xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" distT="0" distB="0" distL="114300" distR="114300" simplePos="0" relativeHeight="1" behindDoc="0" locked="0" layoutInCell="1" allowOverlap="1"><wp:simplePos x="0" y="0"/><wp:positionH relativeFrom="column"><wp:posOffset>0</wp:posOffset></wp:positionH><wp:positionV relativeFrom="paragraph"><wp:posOffset>0</wp:posOffset></wp:positionV><wp:extent cx="914400" cy="914400"/><wp:effectExtent l="0" t="0" r="0" b="0"/><wp:wrapSquare wrapText="bothSides"/><wp:docPr id="1" name="Imagem 1" descr="Diagrama do processo"/><wp:cNvGraphicFramePr/>{}</wp:anchor></w:drawing></w:r></w:p>"#,
            picture_graphic(rel_id)
        )
    }

    /// Desenho ancorado **dentro de um grupo de formas**, embrulhado em
    /// `mc:AlternateContent` — é assim que o Word grava quando a figura foi
    /// combinada com formas (capa, faixa, moldura). A figura fica pendurada num
    /// `wpg:wgp`, e não diretamente no `a:graphicData` de imagem.
    fn grouped_drawing(rel_id: &str) -> String {
        format!(
            r#"<w:p><w:r><mc:AlternateContent xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"><mc:Choice Requires="wpg"><w:drawing><wp:anchor xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" distT="0" distB="0" distL="0" distR="0" simplePos="0" relativeHeight="1" behindDoc="1" locked="0" layoutInCell="1" allowOverlap="1"><wp:simplePos x="0" y="0"/><wp:positionH relativeFrom="page"><wp:posOffset>0</wp:posOffset></wp:positionH><wp:positionV relativeFrom="page"><wp:posOffset>0</wp:posOffset></wp:positionV><wp:extent cx="914400" cy="914400"/><wp:effectExtent l="0" t="0" r="0" b="0"/><wp:wrapNone/><wp:docPr id="33" name="Grupo 33" descr="Diagrama do processo"/><wp:cNvGraphicFramePr/><a:graphic xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:graphicData uri="http://schemas.microsoft.com/office/word/2010/wordprocessingGroup"><wpg:wgp xmlns:wpg="http://schemas.microsoft.com/office/word/2010/wordprocessingGroup"><wpg:cNvGrpSpPr/><wps:wsp xmlns:wps="http://schemas.microsoft.com/office/word/2010/wordprocessingShape"><wps:cNvSpPr/><wps:spPr><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:solidFill><a:srgbClr val="FEE4D2"/></a:solidFill></wps:spPr></wps:wsp><pic:pic xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:nvPicPr><pic:cNvPr id="34" name="Imagem 34"/><pic:cNvPicPr/></pic:nvPicPr><pic:blipFill><a:blip r:embed="{rel_id}"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill><pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="914400"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></wpg:wgp></a:graphicData></a:graphic></wp:anchor></w:drawing></mc:Choice><mc:Fallback><w:pict><v:rect xmlns:v="urn:schemas-microsoft-com:vml" style="width:72pt;height:72pt"/></w:pict></mc:Fallback></mc:AlternateContent></w:r></w:p>"#
        )
    }

    /// Grupo com duas figuras e um texto dentro de caixa — o arranjo de capa.
    /// O texto tem de sobreviver tanto quanto as imagens.
    fn grouped_drawing_two(rel_a: &str, rel_b: &str) -> String {
        let pic = |rel: &str, id: &str| {
            format!(
                r#"<pic:pic xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:nvPicPr><pic:cNvPr id="{id}" name="Imagem {id}"/><pic:cNvPicPr/></pic:nvPicPr><pic:blipFill><a:blip r:embed="{rel}"/></pic:blipFill><pic:spPr><a:xfrm><a:ext cx="914400" cy="914400"/></a:xfrm></pic:spPr></pic:pic>"#
            )
        };
        format!(
            r#"<w:p><w:r><mc:AlternateContent xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"><mc:Choice Requires="wpg"><w:drawing><wp:anchor xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"><wp:extent cx="914400" cy="914400"/><wp:docPr id="33" name="Grupo 33" descr="Capa"/><a:graphic xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:graphicData uri="http://schemas.microsoft.com/office/word/2010/wordprocessingGroup"><wpg:wgp xmlns:wpg="http://schemas.microsoft.com/office/word/2010/wordprocessingGroup"><wps:wsp xmlns:wps="http://schemas.microsoft.com/office/word/2010/wordprocessingShape"><wps:txbx><w:txbxContent><w:p><w:r><w:t>Texto dentro da caixa</w:t></w:r></w:p></w:txbxContent></wps:txbx></wps:wsp>{}{}</wpg:wgp></a:graphicData></a:graphic></wp:anchor></w:drawing></mc:Choice><mc:Fallback><w:pict><v:rect xmlns:v="urn:schemas-microsoft-com:vml"/></w:pict></mc:Fallback></mc:AlternateContent></w:r></w:p>"#,
            pic(rel_a, "34"),
            pic(rel_b, "35")
        )
    }

    /// Desenho **embutido** — a forma que o Pandoc já lê.
    fn inline_drawing(rel_id: &str) -> String {
        format!(
            r#"<w:p><w:r><w:drawing><wp:inline xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" distT="0" distB="0" distL="0" distR="0"><wp:extent cx="914400" cy="914400"/><wp:effectExtent l="0" t="0" r="0" b="0"/><wp:docPr id="1" name="Imagem 1"/><wp:cNvGraphicFramePr/>{}</wp:inline></w:drawing></w:r></w:p>"#,
            picture_graphic(rel_id)
        )
    }

    fn package_with_image(body: &str) -> Vec<(&'static str, Vec<u8>)> {
        let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Default Extension="png" ContentType="image/png"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/></Types>"#;
        let doc_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rId10" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image1.png"/></Relationships>"#;
        let package_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;
        vec![
            ("[Content_Types].xml", content_types.as_bytes().to_vec()),
            ("_rels/.rels", package_rels.as_bytes().to_vec()),
            ("word/_rels/document.xml.rels", doc_rels.as_bytes().to_vec()),
            (DOCUMENT_PART, document_with(body).into_bytes()),
            (STYLES_PART, styles_with(&[1]).into_bytes()),
            ("word/media/image1.png", PNG_1X1.to_vec()),
        ]
    }

    #[test]
    fn resgata_a_figura_presa_no_grupo_de_formas() {
        let doc = document_with(&grouped_drawing("rId10"));

        let out = run_prepass(&doc, &styles_with(&[1]));

        assert_eq!(out.images_inlined, 1);
        assert!(
            out.document_xml.contains("<wp:inline"),
            "a figura resgatada tem de virar desenho embutido"
        );
        assert!(
            out.document_xml.contains("<mc:AlternateContent"),
            "o grupo original fica onde esta — ele carrega texto que o Pandoc ja le"
        );
        assert_eq!(
            out.document_xml.matches(r#"r:embed="rId10""#).count(),
            2,
            "uma referencia no grupo original, outra no desenho resgatado"
        );
        assert!(
            out.document_xml.contains(r#"<wp:extent cx="914400" cy="914400"/>"#),
            "o tamanho da propria figura tem de sobreviver"
        );
        assert!(
            out.document_xml.contains(r#"descr="Diagrama do processo""#),
            "o texto alternativo tem de sobreviver"
        );
    }

    /// Figura simplesmente ancorada o Pandoc **já lê** — mexer nela só criaria
    /// imagem duplicada. (Esta era a hipótese errada da primeira tentativa: a
    /// perda não vinha da âncora, vinha do grupo de formas.)
    #[test]
    fn nao_mexe_em_imagem_ancorada_que_o_pandoc_ja_le() {
        let doc = document_with(&anchored_drawing("rId10"));

        let out = run_prepass(&doc, &styles_with(&[1]));

        assert_eq!(out.images_inlined, 0);
        assert_eq!(out.document_xml, doc);
    }

    #[test]
    fn nao_mexe_em_imagem_ja_embutida() {
        let doc = document_with(&inline_drawing("rId10"));

        let out = run_prepass(&doc, &styles_with(&[1]));

        assert_eq!(out.images_inlined, 0);
        assert_eq!(out.document_xml, doc);
    }

    #[test]
    fn nao_converte_desenho_ancorado_sem_imagem() {
        // Forma decorativa (retângulo, seta) não é conteúdo: vira nada no
        // Markdown de qualquer jeito, e mexer nela é risco sem retorno.
        let forma = r#"<w:p><w:r><w:drawing><wp:anchor xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"><wp:extent cx="100" cy="100"/><wp:docPr id="2" name="Retangulo"/><a:graphic xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:graphicData uri="http://schemas.microsoft.com/office/word/2010/wordprocessingShape"><wps:wsp xmlns:wps="http://schemas.microsoft.com/office/word/2010/wordprocessingShape"/></a:graphicData></a:graphic></wp:anchor></w:drawing></w:r></w:p>"#;
        let doc = document_with(forma);

        let out = run_prepass(&doc, &styles_with(&[1]));

        assert_eq!(out.images_inlined, 0);
        assert_eq!(out.document_xml, doc);
    }

    #[test]
    fn ponta_a_ponta_a_imagem_ancorada_chega_ao_markdown() {
        let Some(pandoc) = pandoc_sidecar() else {
            eprintln!("sidecar do Pandoc ausente — teste de imagem ignorado");
            return;
        };
        let dir = temp_dir();
        let source = dir.join("com-imagem.docx");
        write_docx_bin(&source, &package_with_image(&grouped_drawing("rId10")));

        let antes_dir = dir.join("antes");
        std::fs::create_dir_all(&antes_dir).unwrap();
        let antes = pandoc_to_markdown(&pandoc, &source, &antes_dir.join("antes.md"));
        assert!(
            !antes.contains("image1"),
            "o fixture precisa reproduzir a perda: imagem ancorada nao chega ao Markdown. Veio:\n{antes}"
        );
        assert!(
            !antes_dir.join("media").exists(),
            "sem a pre-passe nao ha midia a extrair"
        );

        let depois_dir = dir.join("depois");
        std::fs::create_dir_all(&depois_dir).unwrap();
        let prepared = prepare_for_import(&source, &dir).unwrap().expect("esperava preparo");
        let depois = pandoc_to_markdown(&pandoc, &prepared, &depois_dir.join("depois.md"));

        // A asserção é sobre a imagem NÃO SE PERDER, não sobre a sintaxe: quando
        // a figura traz dimensões, o GFM não tem como carregá-las em `![]()` e o
        // Pandoc escreve `<img>`. Preferimos manter o tamanho (fidelidade
        // visual, que é o critério da missão) a ganhar uma sintaxe mais bonita.
        assert!(
            depois.contains("media/image1.png"),
            "apos a pre-passe a imagem tem de chegar ao Markdown. Veio:\n{depois}"
        );
        // O texto alternativo é preservado no `.docx` preparado (`wp:docPr
        // descr`). Se ele chega ou não ao Markdown é decisão do escritor do
        // Pandoc, não nossa — por isso a verificação fica onde temos controle.
        let preparado_xml = read_part_of(&prepared, DOCUMENT_PART).unwrap();
        assert!(
            preparado_xml.contains(r#"descr="Diagrama do processo""#),
            "o texto alternativo tem de sobreviver a pre-passe"
        );
        // Link relativo: o .md tem de continuar portátil se a pasta mudar de lugar.
        assert!(
            !depois.contains(&depois_dir.to_string_lossy().to_string()),
            "o caminho da imagem nao pode sair absoluto. Veio:\n{depois}"
        );
        // E o arquivo tem de existir de verdade, senão o link aponta para o nada.
        let extraida = depois_dir.join("media").join("image1.png");
        assert!(extraida.is_file(), "a midia precisa ser gravada ao lado do .md");
        assert_eq!(
            std::fs::read(&extraida).unwrap(),
            PNG_1X1,
            "os bytes da imagem tem de chegar intactos"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Grupo com mais de uma figura: nenhuma pode ficar para trás, e o texto que
    /// vive dentro do grupo (caixa de texto de capa) tem de sobreviver.
    #[test]
    fn resgata_todas_as_figuras_do_grupo_sem_perder_o_texto() {
        let Some(pandoc) = pandoc_sidecar() else {
            eprintln!("sidecar do Pandoc ausente — teste ignorado");
            return;
        };
        let dir = temp_dir();
        let mut parts = package_with_image(&grouped_drawing_two("rId10", "rId11"));
        // Segunda mídia, com bytes diferentes da primeira.
        let mut outra = PNG_1X1.to_vec();
        outra.extend_from_slice(&[0u8; 4]);
        parts.push(("word/media/image2.png", outra));
        parts = parts
            .into_iter()
            .map(|(name, content)| {
                if name == "word/_rels/document.xml.rels" {
                    let rels = String::from_utf8(content).unwrap().replace(
                        "</Relationships>",
                        r#"<Relationship Id="rId11" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image2.png"/></Relationships>"#,
                    );
                    (name, rels.into_bytes())
                } else {
                    (name, content)
                }
            })
            .collect();
        // Um parágrafo comum ao lado do grupo: é o texto cuja sobrevivência
        // conseguimos afirmar independentemente do que o Pandoc faz com caixas
        // de texto dentro de formas.
        let corpo = format!(
            "{}{}",
            grouped_drawing_two("rId10", "rId11"),
            paragraph(r#"<w:pStyle w:val="Normal"/>"#, "Texto ao lado do grupo")
        );
        parts = parts
            .into_iter()
            .map(|(name, content)| {
                if name == DOCUMENT_PART {
                    (name, document_with(&corpo).into_bytes())
                } else {
                    (name, content)
                }
            })
            .collect();
        let source = dir.join("grupo.docx");
        write_docx_bin(&source, &parts);

        let antes_dir = dir.join("antes");
        let depois_dir = dir.join("depois");
        std::fs::create_dir_all(&antes_dir).unwrap();
        std::fs::create_dir_all(&depois_dir).unwrap();

        let antes = pandoc_to_markdown(&pandoc, &source, &antes_dir.join("a.md"));
        let prepared = prepare_for_import(&source, &dir).unwrap().expect("esperava preparo");
        let depois = pandoc_to_markdown(&pandoc, &prepared, &depois_dir.join("d.md"));

        assert!(depois.contains("image1.png"), "primeira figura sumiu. Veio:\n{depois}");
        assert!(depois.contains("image2.png"), "segunda figura sumiu. Veio:\n{depois}");
        assert!(depois.contains("Texto ao lado do grupo"));

        // A propriedade que vale: o resgate ACRESCENTA. Nada do que o Pandoc já
        // conseguia ler pode desaparecer depois da pré-passe.
        for linha in antes.lines().map(str::trim).filter(|l| !l.is_empty()) {
            assert!(
                depois.contains(linha),
                "a pre-passe engoliu conteudo que existia antes: {linha:?}\nantes:\n{antes}\ndepois:\n{depois}"
            );
        }

        // E o grupo original continua no pacote — com o texto que mora dentro dele.
        let preparado = read_part_of(&prepared, DOCUMENT_PART).unwrap();
        assert!(preparado.contains("Texto dentro da caixa"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn encontra_o_estilo_de_titulo_ignorando_caixa_do_nome() {
        let styles = r#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:type="paragraph" w:styleId="H1"><w:name w:val="Heading 1"/></w:style></w:styles>"#;

        let found = existing_heading_styles(styles);

        assert_eq!(found.get(&1), Some(&"H1".to_string()));
    }
}
