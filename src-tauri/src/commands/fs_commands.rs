use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

use tauri_plugin_opener::OpenerExt;

/// Diretórios ignorados ao escanear um projeto em busca de arquivos .md.
const IGNORED_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    ".venv",
    "venv",
    "__pycache__",
    ".next",
    ".cache",
];

#[derive(Serialize, Clone)]
pub struct FileNode {
    name: String,
    path: String,
    is_dir: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    children: Option<Vec<FileNode>>,
}

fn is_markdown(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()),
        Some(ext) if ext == "md" || ext == "markdown"
    )
}

fn scan_dir(dir: &Path) -> FileNode {
    let mut children = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') && path.is_dir() {
                continue;
            }
            if path.is_dir() {
                if IGNORED_DIRS.contains(&name.as_str()) {
                    continue;
                }
                children.push(scan_dir(&path));
            } else if is_markdown(&path) {
                children.push(FileNode {
                    name,
                    path: path.to_string_lossy().to_string(),
                    is_dir: false,
                    children: None,
                });
            }
        }
    }
    // remove subpastas vazias (sem nenhum .md, direta ou indiretamente)
    children.retain(|c| !c.is_dir || c.children.as_ref().map(|ch| !ch.is_empty()).unwrap_or(false));

    FileNode {
        name: dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
        path: dir.to_string_lossy().to_string(),
        is_dir: true,
        children: Some(children),
    }
}

#[tauri::command]
pub fn list_markdown_tree(root: String) -> Result<FileNode, String> {
    let root_path = PathBuf::from(&root);
    if !root_path.is_dir() {
        return Err(format!("Pasta não encontrada: {root}"));
    }
    Ok(scan_dir(&root_path))
}

#[tauri::command]
pub fn read_text_file(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| format!("Não foi possível ler {path}: {e}"))
}

#[tauri::command]
pub fn write_text_file(path: String, content: String) -> Result<(), String> {
    fs::write(&path, content).map_err(|e| format!("Não foi possível salvar {path}: {e}"))
}

#[tauri::command]
pub fn create_markdown_file(dir: String, file_name: String) -> Result<String, String> {
    let mut name = file_name;
    if !name.to_ascii_lowercase().ends_with(".md") {
        name.push_str(".md");
    }
    let path = PathBuf::from(&dir).join(&name);
    if path.exists() {
        return Err(format!("Já existe um arquivo chamado {name}"));
    }
    fs::write(&path, "# Novo documento\n\n")
        .map_err(|e| format!("Não foi possível criar o arquivo: {e}"))?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn create_folder(dir: String, folder_name: String) -> Result<String, String> {
    let path = PathBuf::from(&dir).join(&folder_name);
    if path.exists() {
        return Err(format!("Já existe uma pasta chamada {folder_name}"));
    }
    fs::create_dir_all(&path).map_err(|e| format!("Não foi possível criar a pasta: {e}"))?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn rename_path(path: String, new_name: String) -> Result<String, String> {
    let old_path = PathBuf::from(&path);
    let parent = old_path
        .parent()
        .ok_or_else(|| "Caminho inválido".to_string())?;
    let new_path = parent.join(&new_name);
    fs::rename(&old_path, &new_path).map_err(|e| format!("Não foi possível renomear: {e}"))?;
    Ok(new_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn delete_path(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    if p.is_dir() {
        fs::remove_dir_all(&p).map_err(|e| format!("Não foi possível excluir a pasta: {e}"))
    } else {
        fs::remove_file(&p).map_err(|e| format!("Não foi possível excluir o arquivo: {e}"))
    }
}

#[tauri::command]
pub fn open_in_file_manager(app: tauri::AppHandle, path: String) -> Result<(), String> {
    app.opener()
        .reveal_item_in_dir(&path)
        .map_err(|e| format!("Não foi possível abrir o local do arquivo: {e}"))
}
