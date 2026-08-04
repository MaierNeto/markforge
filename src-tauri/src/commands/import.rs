use std::path::{Path, PathBuf};

use tauri_plugin_shell::ShellExt;

use super::export::check_output;
use super::fs_commands::AllowedRoots;

/// Deriva o caminho do .md de destino: nome do .docx de origem (sem
/// extensão), dentro da pasta de destino informada — que pode ser diferente
/// da pasta onde o .docx está (ex.: importar para dentro do projeto aberto,
/// não para a pasta de Downloads). Não sobrescreve — ver import_docx.
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

/// Importa um .docx (OOXML) convertendo para Markdown via Pandoc — o mesmo
/// sidecar já usado na exportação, sentido contrário. Não suporta o formato
/// binário legado .doc (Word 97-2003); o Pandoc não lê esse formato.
#[tauri::command]
pub async fn import_docx(
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
