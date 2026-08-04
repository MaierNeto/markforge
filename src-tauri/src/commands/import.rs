use std::path::{Path, PathBuf};

use tauri_plugin_shell::ShellExt;

use super::export::check_output;
use super::fs_commands::AllowedRoots;

/// Deriva o caminho do .md de destino a partir do .docx de origem: mesma
/// pasta, mesmo nome, extensão trocada. Não sobrescreve — ver import_docx.
fn derive_markdown_path(docx_path: &Path) -> PathBuf {
    docx_path.with_extension("md")
}

fn register_root(roots: &AllowedRoots, path: &Path) {
    if let Ok(mut list) = roots.0.lock() {
        if !list.iter().any(|r| r == path) {
            list.push(path.to_path_buf());
        }
    }
}

/// Importa um .docx (OOXML) convertendo para Markdown via Pandoc — o mesmo
/// sidecar já usado na exportação, sentido contrário. Não suporta o formato
/// binário legado .doc (Word 97-2003); o Pandoc não lê esse formato.
#[tauri::command]
pub async fn import_docx(
    app: tauri::AppHandle,
    roots: tauri::State<'_, AllowedRoots>,
    source_path: String,
) -> Result<String, String> {
    let source = PathBuf::from(&source_path);
    if !source.exists() {
        return Err(format!("Arquivo não encontrado: {source_path}"));
    }

    let dest = derive_markdown_path(&source);
    if dest.exists() {
        return Err(format!(
            "Já existe um arquivo \"{}\" — renomeie ou remova antes de importar.",
            dest.file_name().and_then(|n| n.to_str()).unwrap_or("")
        ));
    }

    let dest_dir = dest
        .parent()
        .ok_or_else(|| "Caminho de destino inválido".to_string())?;
    register_root(&roots, dest_dir);

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
    check_output("Pandoc retornou um erro ao importar o .docx", &output)?;

    Ok(dest.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::derive_markdown_path;
    use std::path::PathBuf;

    #[test]
    fn troca_extensao_docx_por_md() {
        assert_eq!(
            derive_markdown_path(&PathBuf::from("/projeto/Relatorio.docx")),
            PathBuf::from("/projeto/Relatorio.md")
        );
    }

    #[test]
    fn preserva_a_pasta_de_origem() {
        assert_eq!(
            derive_markdown_path(&PathBuf::from("C:\\Users\\walter\\Documentos\\Ata.docx")),
            PathBuf::from("C:\\Users\\walter\\Documentos\\Ata.md")
        );
    }
}
