use std::fs;
use std::path::{Path, PathBuf};

use tauri_plugin_shell::ShellExt;
use uuid::Uuid;

use super::docx_outline;
use super::export::check_output;
use super::fs_commands::AllowedRoots;
use super::pdf_import;

/// Deriva o caminho do .md de destino: nome do .docx de origem (sem
/// extensão), dentro da pasta de destino informada — que pode ser diferente
/// da pasta onde o .docx está (ex.: importar para dentro do projeto aberto,
/// não para a pasta de Downloads). Não sobrescreve — ver import_document.
fn derive_markdown_path(docx_path: &Path, dest_dir: &Path) -> PathBuf {
    let stem = docx_path.file_stem().unwrap_or_default();
    dest_dir.join(stem).with_extension("md")
}

fn register_root(roots: &AllowedRoots, path: &Path) {
    if let Ok(mut list) = roots.0.lock() {
        if !list.iter().any(|r| r == path) {
            list.push(path.to_path_buf());
        }
    }
}

/// Dialeto de Markdown que a importação grava.
///
/// **GFM porque é o que o editor fala.** O Milkdown lê CommonMark + GFM; o
/// dialeto do Pandoc traz construções que só ele entende — span de indicador
/// (`[]{#...}`), atributo de título (`{#id .classe}`), tabela em grade — e todas
/// aparecem como lixo literal na tela.
///
/// **`raw_html` fica LIGADO de propósito.** Desligar (`gfm-raw_html`) deixa a
/// saída mais limpa, mas **apaga conteúdo em silêncio**: tabela com bloco dentro
/// da célula (lista, vários parágrafos) não cabe em tabela de canos e, sem a
/// saída HTML, desaparece inteira. Preferimos `<table>` legível a dado perdido —
/// há teste fixando isso.
///
/// Ponto único: o teste de ponta a ponta lê daqui, para não divergir do que o
/// app realmente faz.
pub(crate) const IMPORT_MARKDOWN_DIALECT: &str = "gfm+yaml_metadata_block";

async fn import_via_pandoc(app: &tauri::AppHandle, source: &Path, dest: &Path) -> Result<(), String> {
    // As imagens do .docx são gravadas ao lado do .md, e o Pandoc escreve no
    // arquivo o mesmo caminho que recebeu aqui. Por isso rodamos com a pasta de
    // destino como diretório de trabalho e pedimos `.`: o link sai relativo
    // (`media/imagem.png`) e o .md continua portátil — se a pasta mudar de
    // lugar, imagem e texto viajam juntos.
    let dest_dir = dest
        .parent()
        .ok_or_else(|| "Pasta de destino inválida.".to_string())?;
    let cmd = app
        .shell()
        .sidecar("pandoc")
        .map_err(|e| format!("Não foi possível localizar o Pandoc embutido: {e}"))?
        .current_dir(dest_dir)
        .args([
            source.as_os_str().to_string_lossy().to_string(),
            "--from".into(),
            "docx".into(),
            "--to".into(),
            IMPORT_MARKDOWN_DIALECT.into(),
            "--wrap".into(),
            "none".into(),
            "--extract-media".into(),
            ".".into(),
            "-o".into(),
            dest.as_os_str().to_string_lossy().to_string(),
        ]);
    let output = cmd
        .output()
        .await
        .map_err(|e| format!("Falha ao executar o Pandoc: {e}"))?;
    check_output("Pandoc retornou um erro ao importar o .docx", &output)
}

/// Converte o `.docx` passando antes pela pré-passe que promove nível de tópico
/// a estilo de título (ver `docx_outline`) — sem ela, documento montado por
/// formatação direta chega ao Markdown sem nenhum `#`.
///
/// A pré-passe é um ganho, não um requisito: se ela falhar por qualquer motivo,
/// a importação segue com o arquivo original em vez de abortar. Um `.docx`
/// realmente inválido falha adiante, no Pandoc, com a mensagem dele.
async fn import_docx(app: &tauri::AppHandle, source: &Path, dest: &Path) -> Result<(), String> {
    let work_dir = std::env::temp_dir().join(format!("markforge-import-{}", Uuid::new_v4()));
    if fs::create_dir_all(&work_dir).is_err() {
        return import_via_pandoc(app, source, dest).await;
    }

    let prepared = docx_outline::prepare_for_import(source, &work_dir);
    let effective_source = match prepared {
        Ok(Some(ref path)) => path.as_path(),
        Ok(None) | Err(_) => source,
    };
    let result = import_via_pandoc(app, effective_source, dest).await;

    fs::remove_dir_all(&work_dir).ok();
    result
}

/// Um .txt não tem estrutura para converter — o conteúdo vira o corpo do .md
/// tal como está, sem passar por um parser de Markdown. Isso evita que
/// caracteres literais do texto (ex.: "*", "#") sejam mal-interpretados como
/// sintaxe de formatação.
fn import_plain_text(source: &Path, dest: &Path) -> Result<(), String> {
    let content = fs::read_to_string(source).map_err(|e| format!("Não foi possível ler o .txt: {e}"))?;
    fs::write(dest, content).map_err(|e| format!("Não foi possível gravar o .md: {e}"))
}

/// Importa um `.pdf` convertendo para Markdown via `lopdf` + heurísticas (ver
/// `pdf_import::import_pdf_to_markdown`). Grava o `.md` no destino e retorna o
/// caminho. Não sobrescreve — colisão é tratada em `import_document`.
fn import_pdf(source: &Path, dest: &Path) -> Result<(), String> {
    let markdown = pdf_import::import_pdf_to_markdown(
        source.as_os_str().to_string_lossy().to_string().as_str()
    )?;
    fs::write(dest, markdown).map_err(|e| format!("Não foi possível gravar o .md: {e}"))
}

/// Importa um documento existente convertendo para Markdown: `.docx` (OOXML)
/// via Pandoc — o mesmo sidecar já usado na exportação, sentido contrário —,
/// `.pdf` via `lopdf` + heurísticas (ver `pdf_import`), e `.txt` como cópia
/// literal do conteúdo. Não suporta o formato binário legado `.doc`
/// (Word 97-2003); o Pandoc não lê esse formato.
#[tauri::command]
pub async fn import_document(
    app: tauri::AppHandle,
    roots: tauri::State<'_, AllowedRoots>,
    source_path: String,
    dest_dir: String,
) -> Result<String, String> {
    let source = PathBuf::from(&source_path);
    if !source.exists() {
        return Err(format!("Arquivo não encontrado: {source_path}"));
    }
    let dest_dir_path = PathBuf::from(&dest_dir);
    if !dest_dir_path.is_dir() {
        return Err(format!("Pasta de destino não encontrada: {dest_dir}"));
    }

    let dest = derive_markdown_path(&source, &dest_dir_path);
    if dest.exists() {
        return Err(format!(
            "Já existe um arquivo \"{}\" — renomeie ou remova antes de importar.",
            dest.file_name().and_then(|n| n.to_str()).unwrap_or("")
        ));
    }

    register_root(&roots, &dest_dir_path);

    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase());
    match ext.as_deref() {
        Some("docx") => import_docx(&app, &source, &dest).await?,
        Some("pdf") => import_pdf(&source, &dest)?,
        Some("txt") => import_plain_text(&source, &dest)?,
        _ => return Err("Formato não suportado — escolha um arquivo .docx, .pdf ou .txt.".to_string()),
    }

    Ok(dest.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::{derive_markdown_path, import_pdf, import_plain_text};
    use std::path::PathBuf;
    use uuid::Uuid;

    #[test]
    fn import_plain_text_copia_o_conteudo_literal_sem_interpretar_como_markdown() {
        let dir = std::env::temp_dir().join(format!("markforge-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let source = dir.join("notas.txt");
        // Conteúdo com caracteres que teriam significado em Markdown (*ênfase*,
        // # título) — precisa sair literal, não interpretado.
        std::fs::write(&source, "Reunião *importante*\n# não é um título\n").unwrap();
        let dest = dir.join("notas.md");

        import_plain_text(&source, &dest).unwrap();

        let written = std::fs::read_to_string(&dest).unwrap();
        assert_eq!(written, "Reunião *importante*\n# não é um título\n");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn troca_extensao_docx_por_md_na_pasta_de_destino() {
        assert_eq!(
            derive_markdown_path(&PathBuf::from("/downloads/Relatorio.docx"), &PathBuf::from("/projeto")),
            PathBuf::from("/projeto/Relatorio.md")
        );
    }

    /// Só faz sentido no Windows: fora dele a barra invertida é um caractere
    /// comum de nome de arquivo, não separador de pasta, e o caminho inteiro
    /// vira um nome só. A intenção — usar a pasta de destino, e não a de origem —
    /// já está coberta de forma portátil pelos dois testes vizinhos.
    #[test]
    #[cfg(windows)]
    fn usa_a_pasta_de_destino_mesmo_quando_diferente_da_origem() {
        assert_eq!(
            derive_markdown_path(
                &PathBuf::from("C:\\Users\\walter\\Downloads\\Ata.docx"),
                &PathBuf::from("C:\\Users\\walter\\Projetos\\doc-projeto")
            ),
            PathBuf::from("C:\\Users\\walter\\Projetos\\doc-projeto\\Ata.md")
        );
    }

    #[test]
    fn preserva_o_nome_quando_pasta_de_destino_e_a_mesma_da_origem() {
        assert_eq!(
            derive_markdown_path(&PathBuf::from("/projeto/Relatorio.docx"), &PathBuf::from("/projeto")),
            PathBuf::from("/projeto/Relatorio.md")
        );
    }

    #[test]
    fn import_pdf_converte_e_grava_md_com_headings() {
        // Fixture gerado por código (`scripts/generate_pdf_fixtures.py`) e mantido
        // fora do repositório — sem ele, o teste se ignora.
        let source = PathBuf::from("tests/fixtures/simple_headings.pdf");
        if !source.exists() {
            eprintln!("fixtures ausentes — teste de importação de PDF ignorado");
            return;
        }
        let dir = std::env::temp_dir().join(format!("markforge-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let dest = dir.join("simple_headings.md");

        import_pdf(&source, &dest).expect("import_pdf deveria gravar o .md");

        let written = std::fs::read_to_string(&dest).unwrap();
        assert!(written.contains("# Relatório de Vendas"), "H1 deveria virar '# '");
        assert!(written.contains("## Primeiro Trimestre"), "H2 deveria virar '## '");

        std::fs::remove_dir_all(&dir).ok();
    }
}
