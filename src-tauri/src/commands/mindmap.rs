//! Exportação de mapa mental (`.mm`, dialeto FreeMind).
//!
//! A **árvore de pastas é o esqueleto** do mapa: existe para todo arquivo, sem
//! depender de link interno. Cada folha carrega o caminho **relativo** como
//! hyperlink — é o que permite mover a pasta inteira sem quebrar o mapa
//! (o Freeplane só é portátil com `hyperlink types: relative`).
//!
//! Dialeto FreeMind (`<map version="1.0.1">`) e não Freeplane: é o que importa
//! em mais ferramentas (XMind, Mindomo, SimpleMind, ConceptDraw). O Freeplane
//! lê o formato FreeMind integralmente.

use std::path::{Path, PathBuf};

use super::fs_commands::{ensure_allowed, scan_markdown_tree, AllowedRoots, FileNode};

/// Escapa os cinco caracteres que não podem entrar cru num atributo XML.
fn escape_xml(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Caminho do alvo relativo à raiz, sempre com `/`.
///
/// Barra invertida do Windows quebra o link quando o mapa é aberto em outra
/// plataforma; `/` funciona nas duas. Se o alvo não estiver sob a raiz, devolve
/// o próprio nome — o mapa não inventa caminho para fora.
fn to_relative_link(root: &Path, target: &Path) -> String {
    let rel = target.strip_prefix(root).unwrap_or(target);
    let text = rel.to_string_lossy().replace('\\', "/");
    if text.is_empty() {
        target
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default()
    } else {
        text
    }
}

/// Profundidade máxima de título que vira nó. H4+ em documento longo transforma
/// o mapa em borrão — o markmap chega à mesma conclusão limitando a expansão.
const MAX_HEADING_LEVEL: u8 = 3;

/// Tamanho do resumo que vai para o `DETAILS`. O nó mostra um resumo, não o
/// documento.
const MAX_SUMMARY: usize = 240;

/// O que se lê de um `.md` para montar sua subárvore.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct DocInfo {
    pub title: String,
    pub summary: Option<String>,
    pub attributes: Vec<(String, String)>,
    /// (nível, texto) dos títulos do corpo, na ordem do documento.
    pub headings: Vec<(u8, String)>,
}

/// Separa o bloco de front-matter YAML do corpo. Devolve os pares na ordem em
/// que aparecem e o corpo restante.
fn parse_frontmatter(markdown: &str) -> (Vec<(String, String)>, &str) {
    let Some(rest) = markdown.strip_prefix("---\n") else {
        return (Vec::new(), markdown);
    };
    let Some(end) = rest.find("\n---\n") else {
        return (Vec::new(), markdown);
    };
    let body = rest[end + 5..].trim_start_matches(['\n', '\r']);

    let mut pares = Vec::new();
    for linha in rest[..end].lines() {
        let linha = linha.trim_end_matches('\r');
        // Só o par simples `chave: valor` do primeiro nível. Bloco aninhado e
        // item de lista são de outro tipo — ignorar é melhor que adivinhar.
        if linha.starts_with([' ', '\t', '-', '#']) || linha.trim().is_empty() {
            continue;
        }
        if let Some((chave, valor)) = linha.split_once(':') {
            let chave = chave.trim();
            let valor = valor.trim().trim_matches(['"', '\'']).trim();
            if !chave.is_empty() {
                pares.push((chave.to_string(), valor.to_string()));
            }
        }
    }
    (pares, body)
}

/// Abre ou fecha cerca de código. Devolve `true` se a linha era uma cerca.
fn toggle_fence(linha: &str, fence: &mut Option<char>) -> bool {
    let t = linha.trim_start();
    if !t.starts_with("```") && !t.starts_with("~~~") {
        return false;
    }
    let marca = t.as_bytes()[0] as char;
    match *fence {
        // fecha só com a mesma marca que abriu
        Some(aberta) if aberta == marca => *fence = None,
        Some(_) => {}
        None => *fence = Some(marca),
    }
    true
}

/// Títulos do corpo até `max_level`, **pulando bloco de código** — senão um
/// `# comentário` dentro de ``` vira nó fantasma.
fn extract_headings(body: &str, max_level: u8) -> Vec<(u8, String)> {
    let mut titulos = Vec::new();
    let mut fence = None;
    for linha in body.lines() {
        if toggle_fence(linha, &mut fence) || fence.is_some() {
            continue;
        }
        let t = linha.trim_start();
        let cerquilhas = t.chars().take_while(|c| *c == '#').count();
        if cerquilhas == 0 || cerquilhas > 6 {
            continue;
        }
        let resto = &t[cerquilhas..];
        // `#etiqueta` não é título — o espaço é o que separa os dois.
        if !resto.starts_with(' ') {
            continue;
        }
        if cerquilhas as u8 > max_level {
            continue;
        }
        let texto = resto.trim().trim_end_matches('#').trim();
        if !texto.is_empty() {
            titulos.push((cerquilhas as u8, texto.to_string()));
        }
    }
    titulos
}

/// Primeiro parágrafo de texto do corpo, ignorando título, lista e cerca de código.
fn first_paragraph(body: &str) -> Option<String> {
    let mut fence = None;
    for linha in body.lines() {
        if toggle_fence(linha, &mut fence) || fence.is_some() {
            continue;
        }
        let t = linha.trim();
        if t.is_empty() {
            continue;
        }
        // Título, lista, citação, tabela e régua não são parágrafo.
        if t.starts_with(['#', '-', '*', '+', '>', '|']) {
            continue;
        }
        if t.chars().all(|c| c == '=' || c == '_') {
            continue;
        }
        return Some(t.to_string());
    }
    None
}

/// Encurta preservando palavra inteira e sinaliza o corte.
fn shorten(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let truncado: String = text.chars().take(max).collect();
    let corte = truncado
        .rfind(char::is_whitespace)
        .unwrap_or(truncado.len());
    format!("{}…", truncado[..corte].trim_end())
}

/// Lê um documento: título em cascata, resumo, atributos e títulos internos.
pub(crate) fn parse_document(markdown: &str, filename: &str) -> DocInfo {
    let (pares, corpo) = parse_frontmatter(markdown);
    let campo = |nome: &str| {
        pares
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(nome))
            .map(|(_, v)| v.clone())
            .filter(|v| !v.is_empty())
    };

    let mut headings = extract_headings(corpo, MAX_HEADING_LEVEL);

    // Cascata do título: front-matter → primeiro H1 → nome do arquivo.
    let (title, veio_do_h1) = match campo("title") {
        Some(t) => (t, false),
        None => match headings.iter().position(|(n, _)| *n == 1) {
            Some(i) => (headings[i].1.clone(), true),
            None => (filename.to_string(), false),
        },
    };
    // O H1 que virou título não se repete como filho do próprio nó.
    if veio_do_h1 {
        if let Some(i) = headings.iter().position(|(n, _)| *n == 1) {
            headings.remove(i);
        }
    }

    let summary = campo("description")
        .or_else(|| first_paragraph(corpo))
        .map(|s| shorten(&s, MAX_SUMMARY));

    // `description` já aparece como resumo; repetir em atributo é ruído.
    let mut attributes: Vec<(String, String)> = pares
        .iter()
        .filter(|(k, v)| !v.is_empty() && !k.eq_ignore_ascii_case("description"))
        .cloned()
        .collect();
    // O nome real em disco sempre fica acessível, mesmo quando o rótulo é outro.
    attributes.push(("arquivo".to_string(), filename.to_string()));

    DocInfo {
        title,
        summary,
        attributes,
        headings,
    }
}

/// Escreve os títulos do documento como nós aninhados, respeitando o nível.
fn write_headings(
    hs: &[(u8, String)],
    idx: &mut usize,
    nivel_pai: u8,
    next_id: &mut u32,
    depth: usize,
    out: &mut String,
) {
    let indent = "  ".repeat(depth + 1);
    while *idx < hs.len() {
        let (nivel, texto) = &hs[*idx];
        if *nivel <= nivel_pai {
            return;
        }
        let nivel = *nivel;
        let id = *next_id;
        *next_id += 1;
        *idx += 1;

        out.push_str(&indent);
        out.push_str(&format!(
            "<node ID=\"ID_{}\" TEXT=\"{}\"",
            id,
            escape_xml(texto)
        ));
        if *idx < hs.len() && hs[*idx].0 > nivel {
            out.push_str(">\n");
            write_headings(hs, idx, nivel, next_id, depth + 1, out);
            out.push_str(&indent);
            out.push_str("</node>\n");
        } else {
            out.push_str("/>\n");
        }
    }
}

/// Serializa um nó e seus filhos. `next_id` mantém o ID sequencial e estável.
fn write_node(
    node: &FileNode,
    root: &Path,
    next_id: &mut u32,
    depth: usize,
    read: &dyn Fn(&Path) -> Option<String>,
    out: &mut String,
) {
    let indent = "  ".repeat(depth + 1);
    let filho = "  ".repeat(depth + 2);
    let id = *next_id;
    *next_id += 1;
    let rel = to_relative_link(root, Path::new(&node.path));

    if node.is_dir {
        out.push_str(&indent);
        out.push_str(&format!("<node ID=\"ID_{}\" TEXT=\"{}\"", id, escape_xml(&node.name)));
        if depth == 0 {
            // A raiz é o próprio mapa: não aponta para si mesma.
            out.push_str(" FOLDED=\"false\"");
        } else {
            // A barra final é o que faz o sistema abrir a pasta, não procurar arquivo.
            out.push_str(&format!(" LINK=\"{}/\"", escape_xml(&rel)));
        }
        out.push_str(">\n");
        out.push_str(&filho);
        out.push_str("<icon BUILTIN=\"folder\"/>\n");
        for child in node.children.as_deref().unwrap_or(&[]) {
            write_node(child, root, next_id, depth + 1, read, out);
        }
        out.push_str(&indent);
        out.push_str("</node>\n");
        return;
    }

    // Documento: o conteúdo é que dá corpo ao nó.
    let Some(texto) = read(Path::new(&node.path)) else {
        // Ilegível não derruba o mapa — cai para o nome do arquivo.
        out.push_str(&indent);
        out.push_str(&format!(
            "<node ID=\"ID_{}\" TEXT=\"{}\" LINK=\"{}\"/>\n",
            id,
            escape_xml(&node.name),
            escape_xml(&rel)
        ));
        return;
    };

    let doc = parse_document(&texto, &node.name);
    out.push_str(&indent);
    out.push_str(&format!(
        "<node ID=\"ID_{}\" TEXT=\"{}\" LINK=\"{}\"",
        id,
        escape_xml(&doc.title),
        escape_xml(&rel)
    ));
    // Documento com seções nasce dobrado: o mapa abre mostrando a estrutura de
    // pastas e documentos, e as seções aparecem quando o leitor pede. Sem isso,
    // uma pasta com dezenas de arquivos abre como parede de texto.
    if !doc.headings.is_empty() {
        out.push_str(" FOLDED=\"true\"");
    }
    out.push_str(">\n");
    out.push_str(&filho);
    out.push_str("<icon BUILTIN=\"bookmark\"/>\n");

    for (chave, valor) in &doc.attributes {
        out.push_str(&filho);
        out.push_str(&format!(
            "<attribute NAME=\"{}\" VALUE=\"{}\"/>\n",
            escape_xml(chave),
            escape_xml(valor)
        ));
    }

    if let Some(resumo) = &doc.summary {
        // DETAILS nasce recolhido: o resumo está a um clique, não no caminho.
        out.push_str(&filho);
        out.push_str("<richcontent TYPE=\"DETAILS\" HIDDEN=\"true\">\n");
        out.push_str(&filho);
        out.push_str(&format!(
            "  <html><body><p>{}</p></body></html>\n",
            escape_xml(resumo)
        ));
        out.push_str(&filho);
        out.push_str("</richcontent>\n");
    }

    let mut idx = 0usize;
    write_headings(&doc.headings, &mut idx, 0, next_id, depth + 1, out);

    out.push_str(&indent);
    out.push_str("</node>\n");
}

/// Monta o `.mm`. `read` entrega o conteúdo de um `.md` — injetado para que o
/// gerador continue puro e testável sem tocar disco.
pub(crate) fn build_mindmap(
    tree: &FileNode,
    root: &Path,
    read: &dyn Fn(&Path) -> Option<String>,
) -> String {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<map version=\"1.0.1\">\n");
    let mut next_id = 1u32;
    write_node(tree, root, &mut next_id, 0, read, &mut out);
    out.push_str("</map>\n");
    out
}

/// Exporta a pasta aberta como mapa mental `.mm`.
///
/// `root` é a pasta já autorizada pela sessão (abrir a pasta é o que autoriza);
/// `out_path` é onde gravar. Devolve o caminho gravado.
#[tauri::command]
pub fn export_mindmap(
    roots: tauri::State<AllowedRoots>,
    root: String,
    out_path: String,
) -> Result<String, String> {
    let root_path = PathBuf::from(&root);
    if !root_path.is_dir() {
        return Err(format!("Pasta não encontrada: {root}"));
    }
    ensure_allowed(&roots, &root_path)?;

    let out = PathBuf::from(&out_path);
    if let Some(parent) = out.parent() {
        ensure_allowed(&roots, parent)?;
    }

    let tree = scan_markdown_tree(&root_path);
    let xml = build_mindmap(&tree, &root_path, &|p| std::fs::read_to_string(p).ok());
    std::fs::write(&out, xml).map_err(|e| format!("Não foi possível gravar o mapa: {e}"))?;
    Ok(out.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str, path: &str, children: Vec<FileNode>) -> FileNode {
        FileNode {
            name: name.into(),
            path: path.into(),
            is_dir: true,
            children: Some(children),
        }
    }

    fn file(name: &str, path: &str) -> FileNode {
        FileNode {
            name: name.into(),
            path: path.into(),
            is_dir: false,
            children: None,
        }
    }

    // ---------- D-13.1b: o documento vira subárvore ----------

    #[test]
    fn front_matter_vira_pares_e_o_corpo_sai_limpo() {
        let md = "---\ntitle: Meu Doc\nauthor: Walter\n---\n\n# Corpo\n\ntexto";
        let (pares, corpo) = parse_frontmatter(md);
        assert_eq!(
            pares,
            vec![
                ("title".to_string(), "Meu Doc".to_string()),
                ("author".to_string(), "Walter".to_string()),
            ]
        );
        assert!(corpo.starts_with("# Corpo"));
    }

    #[test]
    fn sem_front_matter_o_corpo_fica_intacto() {
        let md = "# Só corpo\n\ntexto";
        let (pares, corpo) = parse_frontmatter(md);
        assert!(pares.is_empty());
        assert_eq!(corpo, md);
    }

    #[test]
    fn front_matter_tira_aspas_do_valor() {
        let (pares, _) = parse_frontmatter("---\ntitle: \"Com Aspas\"\n---\n\nx");
        assert_eq!(pares[0].1, "Com Aspas");
    }

    #[test]
    fn titulos_do_corpo_viram_lista_com_nivel() {
        let body = "# Um\n\ntexto\n\n## Dois\n\n### Tres\n";
        assert_eq!(
            extract_headings(body, 3),
            vec![
                (1, "Um".to_string()),
                (2, "Dois".to_string()),
                (3, "Tres".to_string()),
            ]
        );
    }

    #[test]
    fn titulo_dentro_de_bloco_de_codigo_e_ignorado() {
        let body = "# Real\n\n```bash\n# isto e comentario de shell\n## nem isto\n```\n\n## Tambem real\n";
        assert_eq!(
            extract_headings(body, 3),
            vec![(1, "Real".to_string()), (2, "Tambem real".to_string())]
        );
    }

    #[test]
    fn cerca_de_til_tambem_esconde_titulo() {
        let body = "# Real\n\n~~~\n# escondido\n~~~\n";
        assert_eq!(extract_headings(body, 3), vec![(1, "Real".to_string())]);
    }

    #[test]
    fn titulo_abaixo_do_nivel_maximo_nao_entra() {
        let body = "# Um\n\n#### Quatro\n\n##### Cinco\n";
        assert_eq!(extract_headings(body, 3), vec![(1, "Um".to_string())]);
    }

    #[test]
    fn hash_sem_espaco_nao_e_titulo() {
        // "#tag" é etiqueta, não título.
        let body = "#tag no comeco da linha\n\n# Titulo de verdade\n";
        assert_eq!(
            extract_headings(body, 3),
            vec![(1, "Titulo de verdade".to_string())]
        );
    }

    #[test]
    fn primeiro_paragrafo_pula_titulo_e_linha_vazia() {
        let body = "# Titulo\n\n\nEste e o primeiro paragrafo.\n\nO segundo.";
        assert_eq!(
            first_paragraph(body).as_deref(),
            Some("Este e o primeiro paragrafo.")
        );
    }

    #[test]
    fn primeiro_paragrafo_ignora_lista_e_citacao() {
        let body = "# T\n\n- item\n> citacao\n\nParagrafo real.";
        assert_eq!(first_paragraph(body).as_deref(), Some("Paragrafo real."));
    }

    #[test]
    fn documento_so_com_titulo_nao_tem_paragrafo() {
        assert_eq!(first_paragraph("# Só titulo\n"), None);
    }

    #[test]
    fn resumo_longo_e_encurtado_sem_cortar_palavra() {
        let longo = "palavra ".repeat(60);
        let curto = shorten(&longo, 40);
        assert!(curto.chars().count() <= 41, "veio: {curto}");
        assert!(curto.ends_with('…'));
        assert!(!curto.contains("palav…"), "não pode cortar no meio da palavra");
    }

    #[test]
    fn resumo_curto_passa_intacto_sem_reticencia() {
        assert_eq!(shorten("curto", 40), "curto");
    }

    #[test]
    fn titulo_segue_a_cascata_frontmatter_h1_arquivo() {
        // 1º: front-matter vence
        let d = parse_document("---\ntitle: Do Front\n---\n\n# Do H1\n", "arquivo.md");
        assert_eq!(d.title, "Do Front");

        // 2º: sem front-matter, o H1 vence
        let d = parse_document("# Do H1\n\ntexto", "arquivo.md");
        assert_eq!(d.title, "Do H1");

        // 3º: sem os dois, sobra o nome do arquivo
        let d = parse_document("texto solto", "arquivo.md");
        assert_eq!(d.title, "arquivo.md");
    }

    #[test]
    fn resumo_prefere_description_do_frontmatter() {
        let d = parse_document(
            "---\ndescription: O resumo oficial\n---\n\n# T\n\nParagrafo qualquer.",
            "a.md",
        );
        assert_eq!(d.summary.as_deref(), Some("O resumo oficial"));
    }

    #[test]
    fn sem_description_o_resumo_vem_do_primeiro_paragrafo() {
        let d = parse_document("# T\n\nParagrafo qualquer.", "a.md");
        assert_eq!(d.summary.as_deref(), Some("Paragrafo qualquer."));
    }

    #[test]
    fn o_nome_do_arquivo_fica_sempre_disponivel_como_atributo() {
        let d = parse_document("---\ntitle: Bonito\n---\n\ntexto", "FEIO-NO-DISCO.md");
        assert!(d
            .attributes
            .iter()
            .any(|(k, v)| k == "arquivo" && v == "FEIO-NO-DISCO.md"));
    }

    // ---------- D-13.1b: o que isso vira no XML ----------

    fn ler_fixo(conteudo: &'static str) -> impl Fn(&Path) -> Option<String> {
        move |_| Some(conteudo.to_string())
    }

    #[test]
    fn pasta_ganha_link_navegavel_e_icone() {
        let tree = dir(
            "proj",
            "/proj",
            vec![dir("docs", "/proj/docs", vec![file("a.md", "/proj/docs/a.md")])],
        );
        let mm = build_mindmap(&tree, Path::new("/proj"), &ler_fixo("# A"));

        let linha = mm.lines().find(|l| l.contains(r#"TEXT="docs""#)).unwrap();
        assert!(linha.contains(r#"LINK="docs/""#), "veio: {linha}");
        assert!(mm.contains(r#"<icon BUILTIN="folder"/>"#));
    }

    #[test]
    fn documento_mostra_titulo_do_frontmatter_e_nao_o_nome_do_arquivo() {
        let tree = dir("p", "/p", vec![file("FEIO.md", "/p/FEIO.md")]);
        let mm = build_mindmap(
            &tree,
            Path::new("/p"),
            &ler_fixo("---\ntitle: Nome Bonito\n---\n\ntexto"),
        );
        assert!(mm.contains(r#"TEXT="Nome Bonito""#));
        assert!(mm.contains(r#"VALUE="FEIO.md""#), "nome real vira atributo");
    }

    #[test]
    fn documento_carrega_resumo_em_details() {
        let tree = dir("p", "/p", vec![file("a.md", "/p/a.md")]);
        let mm = build_mindmap(
            &tree,
            Path::new("/p"),
            &ler_fixo("# T\n\nEste resumo aparece sob o no."),
        );
        assert!(mm.contains(r#"<richcontent TYPE="DETAILS" HIDDEN="true">"#));
        assert!(mm.contains("Este resumo aparece sob o no."));
    }

    #[test]
    fn titulos_do_documento_viram_nos_aninhados() {
        let tree = dir("p", "/p", vec![file("a.md", "/p/a.md")]);
        let mm = build_mindmap(
            &tree,
            Path::new("/p"),
            &ler_fixo("# Doc\n\n## Secao A\n\n### Sub A1\n\n## Secao B\n"),
        );
        let pa = mm.find(r#"TEXT="Secao A""#).expect("Secao A vira nó");
        let ps = mm.find(r#"TEXT="Sub A1""#).expect("Sub A1 vira nó");
        let pb = mm.find(r#"TEXT="Secao B""#).expect("Secao B vira nó");
        // Sub A1 está entre A e B: é filho de A, não irmão
        assert!(pa < ps && ps < pb, "aninhamento errado:\n{mm}");
    }

    #[test]
    fn documento_com_secoes_nasce_dobrado_para_o_mapa_abrir_legivel() {
        let tree = dir("p", "/p", vec![file("a.md", "/p/a.md")]);
        let mm = build_mindmap(
            &tree,
            Path::new("/p"),
            &ler_fixo("# Doc\n\n## A\n\n## B\n"),
        );
        let linha = mm.lines().find(|l| l.contains(r#"TEXT="Doc""#)).unwrap();
        assert!(linha.contains(r#"FOLDED="true""#), "veio: {linha}");
    }

    #[test]
    fn documento_sem_secoes_nao_nasce_dobrado() {
        // Não há o que revelar — dobrar só esconderia o resumo.
        let tree = dir("p", "/p", vec![file("a.md", "/p/a.md")]);
        let mm = build_mindmap(&tree, Path::new("/p"), &ler_fixo("# Doc\n\ntexto"));
        let linha = mm.lines().find(|l| l.contains(r#"TEXT="Doc""#)).unwrap();
        assert!(!linha.contains("FOLDED="), "veio: {linha}");
    }

    #[test]
    fn documento_ilegivel_nao_derruba_o_mapa() {
        let tree = dir("p", "/p", vec![file("quebrado.md", "/p/quebrado.md")]);
        let mm = build_mindmap(&tree, Path::new("/p"), &|_| None);
        // cai para o nome do arquivo, sem entrar em pânico
        assert!(mm.contains(r#"TEXT="quebrado.md""#));
        assert!(mm.contains(r#"LINK="quebrado.md""#));
    }

    #[test]
    fn resumo_com_caractere_reservado_nao_quebra_o_xml() {
        let tree = dir("p", "/p", vec![file("a.md", "/p/a.md")]);
        let mm = build_mindmap(&tree, Path::new("/p"), &ler_fixo("# T\n\nR&D <alfa> \"beta\"."));
        assert!(mm.contains("R&amp;D &lt;alfa&gt;"));
        assert!(!mm.contains("R&D <alfa>"));
    }

    #[test]
    fn escapa_caracteres_reservados_do_xml() {
        assert_eq!(
            escape_xml(r#"a & b < c > d " e ' f"#),
            "a &amp; b &lt; c &gt; d &quot; e &apos; f"
        );
    }

    #[test]
    fn link_e_relativo_a_raiz() {
        let rel = to_relative_link(Path::new("/proj"), Path::new("/proj/docs/spec.md"));
        assert_eq!(rel, "docs/spec.md");
    }

    #[test]
    fn link_usa_barra_normal_mesmo_em_caminho_windows() {
        let rel = to_relative_link(
            Path::new(r"C:\proj"),
            Path::new(r"C:\proj\docs\sub\spec.md"),
        );
        assert_eq!(rel, "docs/sub/spec.md");
    }

    #[test]
    fn cabecalho_declara_dialeto_freemind() {
        let tree = dir("proj", "/proj", vec![]);
        let mm = build_mindmap(&tree, Path::new("/proj"), &|_| None);
        assert!(mm.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"));
        assert!(mm.contains("<map version=\"1.0.1\">"));
        assert!(mm.trim_end().ends_with("</map>"));
    }

    /// 🟡 **Asserção revista em 15/08/2026 (D-13.1b).** A versão anterior exigia
    /// que pasta **não** tivesse `LINK`. Isso ficou factualmente obsoleto: navegar
    /// até a pasta pelo mapa passou a ser requisito, então pasta ganhou `LINK`
    /// com barra final. A parte de arquivo→link segue valendo e continua aqui.
    #[test]
    fn arquivo_vira_folha_com_link_e_pasta_aponta_para_a_propria_pasta() {
        let tree = dir(
            "proj",
            "/proj",
            vec![dir(
                "docs",
                "/proj/docs",
                vec![file("spec.md", "/proj/docs/spec.md")],
            )],
        );
        let mm = build_mindmap(&tree, Path::new("/proj"), &|_| None);

        assert!(mm.contains(r#"TEXT="spec.md" LINK="docs/spec.md""#));
        let linha_docs = mm
            .lines()
            .find(|l| l.contains(r#"TEXT="docs""#))
            .expect("nó da pasta docs deve existir");
        assert!(linha_docs.contains(r#"LINK="docs/""#), "veio: {linha_docs}");
    }

    #[test]
    fn hierarquia_do_mapa_espelha_a_arvore_de_pastas() {
        let tree = dir(
            "proj",
            "/proj",
            vec![
                dir("a", "/proj/a", vec![file("um.md", "/proj/a/um.md")]),
                file("raiz.md", "/proj/raiz.md"),
            ],
        );
        let mm = build_mindmap(&tree, Path::new("/proj"), &|_| None);

        let pos_a = mm.find(r#"TEXT="a""#).unwrap();
        let pos_um = mm.find(r#"TEXT="um.md""#).unwrap();
        let pos_fecha = mm.find("</node>").unwrap();
        // "um.md" está depois de "a" e antes do primeiro fechamento: é filho dele
        assert!(pos_a < pos_um && pos_um < pos_fecha);
    }

    #[test]
    fn ids_sao_unicos_e_sequenciais() {
        let tree = dir(
            "proj",
            "/proj",
            vec![
                file("a.md", "/proj/a.md"),
                file("b.md", "/proj/b.md"),
                file("c.md", "/proj/c.md"),
            ],
        );
        let mm = build_mindmap(&tree, Path::new("/proj"), &|_| None);
        for esperado in ["ID_1", "ID_2", "ID_3", "ID_4"] {
            assert_eq!(
                mm.matches(&format!("\"{esperado}\"")).count(),
                1,
                "{esperado} deve aparecer exatamente uma vez"
            );
        }
    }

    #[test]
    fn raiz_nasce_desdobrada() {
        let tree = dir("proj", "/proj", vec![file("a.md", "/proj/a.md")]);
        let mm = build_mindmap(&tree, Path::new("/proj"), &|_| None);
        assert!(mm.contains(r#"FOLDED="false""#));
    }

    #[test]
    fn nome_com_caractere_reservado_nao_quebra_o_xml() {
        let tree = dir(
            "proj",
            "/proj",
            vec![file("R&D <rascunho>.md", "/proj/R&D <rascunho>.md")],
        );
        let mm = build_mindmap(&tree, Path::new("/proj"), &|_| None);
        assert!(mm.contains("R&amp;D &lt;rascunho&gt;.md"));
        // nenhum '&' solto sobrou
        assert!(!mm.contains("R&D"));
    }

    /// Smoke do caminho real: varre disco de verdade e gera o mapa.
    /// Os testes acima usam árvore sintética; este prova que a varredura e o
    /// gerador se encaixam — inclusive as exclusões herdadas do `scan_dir`.
    #[test]
    fn ponta_a_ponta_varre_disco_real_e_gera_mapa_valido() {
        use std::fs;

        let base = std::env::temp_dir().join(format!("mf_mindmap_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("docs").join("rfc")).unwrap();
        fs::create_dir_all(base.join("node_modules")).unwrap();
        fs::create_dir_all(base.join(".git")).unwrap();

        fs::write(base.join("README.md"), "# raiz").unwrap();
        fs::write(
            base.join("docs").join("guia.md"),
            "---\ntitle: Guia de Uso\ndescription: Como comecar\nauthor: Fulano\n---\n\n# Guia\n\n## Instalar\n\n### Windows\n\n## Usar\n",
        )
        .unwrap();
        fs::write(
            base.join("docs").join("rfc").join("RFC-001.md"),
            "# Proposta\n\nPrimeiro paragrafo vira resumo.\n\n```bash\n# nao e titulo\n```\n",
        )
        .unwrap();
        fs::write(base.join("docs").join("nao-e-markdown.txt"), "ignorar").unwrap();
        fs::write(base.join("node_modules").join("dep.md"), "# ruido").unwrap();
        fs::write(base.join(".git").join("interno.md"), "# ruido").unwrap();

        let tree = scan_markdown_tree(&base);
        let mm = build_mindmap(&tree, &base, &|p| fs::read_to_string(p).ok());

        // título vem do front-matter, e o nome real fica como atributo
        assert!(mm.contains(r#"TEXT="Guia de Uso" LINK="docs/guia.md""#), "{mm}");
        assert!(mm.contains(r#"<attribute NAME="author" VALUE="Fulano"/>"#));
        assert!(mm.contains(r#"<attribute NAME="arquivo" VALUE="guia.md"/>"#));

        // resumo do front-matter no DETAILS
        assert!(mm.contains(r#"<richcontent TYPE="DETAILS" HIDDEN="true">"#));
        assert!(mm.contains("Como comecar"));

        // títulos do documento viraram nós aninhados
        assert!(mm.contains(r#"TEXT="Instalar""#));
        assert!(mm.contains(r#"TEXT="Windows""#));
        assert!(mm.contains(r#"TEXT="Usar""#));

        // sem front-matter: título vem do H1 e resumo do primeiro parágrafo
        assert!(mm.contains(r#"TEXT="Proposta""#));
        assert!(mm.contains("Primeiro paragrafo vira resumo."));
        // `# nao e titulo` está dentro de bloco de código: não vira nó
        assert!(!mm.contains(r#"TEXT="nao e titulo""#));

        // pasta navegável
        assert!(mm.contains(r#"LINK="docs/""#));
        assert!(mm.contains(r#"<icon BUILTIN="folder"/>"#));

        // o que não é markdown e o que é ruído estrutural ficaram fora
        assert!(!mm.contains("nao-e-markdown"));
        assert!(!mm.contains("node_modules"));
        assert!(!mm.contains("dep.md"));
        assert!(!mm.contains("interno.md"));

        // XML equilibrado. `<icon>`/`<attribute>` também terminam em `/>`, então
        // autofechamento é contado só nas linhas que abrem um <node>.
        let abre = mm.matches("<node ").count();
        let fecha = mm.matches("</node>").count();
        let autofechados = mm
            .lines()
            .filter(|l| l.trim_start().starts_with("<node ") && l.trim_end().ends_with("/>"))
            .count();
        assert_eq!(abre, fecha + autofechados, "todo <node> precisa fechar\n{mm}");

        // nenhum link aponta para fora da raiz
        assert!(!mm.contains("LINK=\"..") && !mm.contains("LINK=\"C:"));

        let _ = fs::remove_dir_all(&base);
    }

    /// Gera uma amostra a partir do próprio `docs/` do repositório, para
    /// conferência visual num programa de mapa mental. Não roda no CI.
    ///
    /// `cargo test --lib gerar_amostra -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn gerar_amostra_para_conferencia_humana() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("docs");
        let tree = scan_markdown_tree(&root);
        let mm = build_mindmap(&tree, &root, &|p| std::fs::read_to_string(p).ok());
        let destino = std::env::temp_dir().join("markforge-amostra.mm");
        std::fs::write(&destino, &mm).unwrap();
        println!("amostra gravada em {}", destino.display());
    }

    /// 🟡 **Asserção revista em 15/08/2026 (D-13.1b).** Antes exigia nó
    /// autofechado (`<node …/>`). Ficou obsoleto porque toda pasta passou a
    /// carregar `<icon>` — logo, tem filho e não autofecha. O que o teste
    /// realmente protege é que pasta vazia não some do mapa; é isso que ficou.
    #[test]
    fn pasta_vazia_continua_no_mapa() {
        let tree = dir("proj", "/proj", vec![dir("vazia", "/proj/vazia", vec![])]);
        let mm = build_mindmap(&tree, Path::new("/proj"), &|_| None);
        assert!(mm.contains(r#"<node ID="ID_2" TEXT="vazia" LINK="vazia/">"#));
        assert!(mm.contains(r#"<icon BUILTIN="folder"/>"#));
    }
}
