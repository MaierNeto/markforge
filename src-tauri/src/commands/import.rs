use std::fs;
use std::path::{Path, PathBuf};

use tauri_plugin_shell::ShellExt;

use super::export::check_output;
use super::fs_commands::AllowedRoots;

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

async fn import_via_pandoc(app: &tauri::AppHandle, source: &Path, dest: &Path) -> Result<(), String> {
    let cmd = app
        .shell()
        .sidecar("pandoc")
        .map_err(|e| format!("Não foi possível localizar o Pandoc embutido: {e}"))?
        .args([
            source.as_os_str().to_string_lossy().to_string(),
            "--from".into(),
            "docx".into(),
            "--to".into(),
            "markdown+yaml_metadata_block".into(),
            "--wrap".into(),
            "none".into(),
            "-o".into(),
            dest.as_os_str().to_string_lossy().to_string(),
        ]);
    let output = cmd
        .output()
        .await
        .map_err(|e| format!("Falha ao executar o Pandoc: {e}"))?;
    check_output("Pandoc retornou um erro ao importar o .docx", &output)
}

/// Um .txt não tem estrutura para converter — o conteúdo vira o corpo do .md
/// tal como está, sem passar por um parser de Markdown. Isso evita que
/// caracteres literais do texto (ex.: "*", "#") sejam mal-interpretados como
/// sintaxe de formatação.
fn import_plain_text(source: &Path, dest: &Path) -> Result<(), String> {
    let content = fs::read_to_string(source).map_err(|e| format!("Não foi possível ler o .txt: {e}"))?;
    fs::write(dest, content).map_err(|e| format!("Não foi possível gravar o .md: {e}"))
}

/// Importa um documento existente convertendo para Markdown: `.docx` (OOXML)
/// via Pandoc — o mesmo sidecar já usado na exportação, sentido contrário —
/// e `.txt` como cópia literal do conteúdo. Não suporta o formato binário
/// legado `.doc` (Word 97-2003); o Pandoc não lê esse formato.
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
        Some("docx") => import_via_pandoc(&app, &source, &dest).await?,
        Some("txt") => import_plain_text(&source, &dest)?,
        _ => return Err("Formato não suportado — escolha um arquivo .docx ou .txt.".to_string()),
    }

    Ok(dest.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::{derive_markdown_path, import_plain_text};
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

    #[test]
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
}
