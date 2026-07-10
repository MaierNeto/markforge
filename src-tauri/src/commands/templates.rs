use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::Manager;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone)]
pub struct TemplateInfo {
    pub id: String,
    pub name: String,
    pub path: String,
    pub built_in: bool,
}

fn templates_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Não foi possível localizar a pasta de dados do app: {e}"))?
        .join("templates");
    fs::create_dir_all(&dir).map_err(|e| format!("Não foi possível criar a pasta de templates: {e}"))?;
    Ok(dir)
}

fn index_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(templates_dir(app)?.join("index.json"))
}

fn read_index(app: &tauri::AppHandle) -> Result<Vec<TemplateInfo>, String> {
    let path = index_path(app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&raw).map_err(|e| e.to_string())
}

fn write_index(app: &tauri::AppHandle, list: &[TemplateInfo]) -> Result<(), String> {
    let path = index_path(app)?;
    let raw = serde_json::to_string_pretty(list).map_err(|e| e.to_string())?;
    fs::write(&path, raw).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_templates(app: tauri::AppHandle) -> Result<Vec<TemplateInfo>, String> {
    read_index(&app)
}

#[tauri::command]
pub fn import_template(app: tauri::AppHandle, source_path: String, name: String) -> Result<TemplateInfo, String> {
    let id = Uuid::new_v4().to_string();
    let dest = templates_dir(&app)?.join(format!("{id}.docx"));
    fs::copy(&source_path, &dest).map_err(|e| format!("Não foi possível importar o template: {e}"))?;

    let info = TemplateInfo {
        id,
        name,
        path: dest.to_string_lossy().to_string(),
        built_in: false,
    };

    let mut list = read_index(&app)?;
    list.push(info.clone());
    write_index(&app, &list)?;
    Ok(info)
}

#[tauri::command]
pub fn delete_template(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let mut list = read_index(&app)?;
    if let Some(pos) = list.iter().position(|t| t.id == id) {
        let removed = list.remove(pos);
        let _ = fs::remove_file(&removed.path);
        write_index(&app, &list)?;
        Ok(())
    } else {
        Err("Template não encontrado".into())
    }
}

/// Resolve o caminho absoluto do reference-doc para um template_id
/// ("default" = template embutido no app; caso contrário busca no índice).
pub fn resolve_template_path(app: &tauri::AppHandle, template_id: &str) -> Result<PathBuf, String> {
    if template_id == "default" {
        return app
            .path()
            .resolve(
                "templates/default/reference.docx",
                tauri::path::BaseDirectory::Resource,
            )
            .map_err(|e| format!("Template padrão não encontrado: {e}"));
    }
    let list = read_index(app)?;
    list.iter()
        .find(|t| t.id == template_id)
        .map(|t| PathBuf::from(&t.path))
        .ok_or_else(|| "Template selecionado não encontrado".to_string())
}
