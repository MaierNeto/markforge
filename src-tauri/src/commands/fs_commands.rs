use serde::Serialize;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;

use tauri_plugin_opener::OpenerExt;

/// Raízes que o usuário abriu nesta sessão. Todo comando de arquivo opera
/// somente dentro delas — um caminho fora é recusado, não executado.
#[derive(Default)]
pub struct AllowedRoots(pub Mutex<Vec<PathBuf>>);

/// Decompõe o caminho em componentes já resolvidos (`.` e `..` aplicados
/// lexicamente, sem tocar no disco — o alvo pode ainda não existir). No Windows
/// os componentes viram minúsculas, já que lá o sistema de arquivos não
/// diferencia maiúsculas.
fn path_components(path: &Path) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for comp in path.components() {
        match comp {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => {
                let part = other.as_os_str().to_string_lossy().to_string();
                out.push(if cfg!(windows) {
                    part.to_lowercase()
                } else {
                    part
                });
            }
        }
    }
    out
}

/// `true` se `target` é a própria `root` ou está dentro dela. Compara por
/// componente (não por prefixo de texto), então "/proj" não contém "/projeto",
/// e `..` não escapa. Raiz vazia nunca autoriza nada.
pub fn is_inside(root: &Path, target: &Path) -> bool {
    let root = path_components(root);
    let target = path_components(target);
    !root.is_empty() && target.len() >= root.len() && target[..root.len()] == root[..]
}

fn register_root(roots: &AllowedRoots, path: &Path) {
    if let Ok(mut list) = roots.0.lock() {
        if !list.iter().any(|r| r == path) {
            list.push(path.to_path_buf());
        }
    }
}

/// Porteiro: só deixa passar caminho dentro de alguma raiz aberta.
fn ensure_allowed(roots: &AllowedRoots, path: &Path) -> Result<(), String> {
    let list = roots
        .0
        .lock()
        .map_err(|_| "Não foi possível verificar as permissões de acesso".to_string())?;
    if list.iter().any(|root| is_inside(root, path)) {
        Ok(())
    } else {
        Err(format!(
            "Acesso negado: {} está fora das pastas abertas no Markforge.",
            path.display()
        ))
    }
}

/// Autoriza a pasta que contém um arquivo aberto diretamente (associação de
/// `.md` no sistema ou "Abrir arquivo"). Chamado pelo fluxo de abertura antes
/// da leitura.
#[tauri::command]
pub fn allow_file(roots: tauri::State<AllowedRoots>, path: String) -> Result<(), String> {
    let path = PathBuf::from(&path);
    let dir = path
        .parent()
        .ok_or_else(|| "Caminho inválido".to_string())?
        .to_path_buf();
    register_root(&roots, &dir);
    Ok(())
}

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

/// Abrir uma pasta é o que a autoriza: a partir daqui ela e o que está dentro
/// dela passam a ser acessíveis nesta sessão.
#[tauri::command]
pub fn list_markdown_tree(roots: tauri::State<AllowedRoots>, root: String) -> Result<FileNode, String> {
    let root_path = PathBuf::from(&root);
    if !root_path.is_dir() {
        return Err(format!("Pasta não encontrada: {root}"));
    }
    register_root(&roots, &root_path);
    Ok(scan_dir(&root_path))
}

#[tauri::command]
pub fn read_text_file(roots: tauri::State<AllowedRoots>, path: String) -> Result<String, String> {
    ensure_allowed(&roots, Path::new(&path))?;
    fs::read_to_string(&path).map_err(|e| format!("Não foi possível ler {path}: {e}"))
}

#[tauri::command]
pub fn write_text_file(
    roots: tauri::State<AllowedRoots>,
    path: String,
    content: String,
) -> Result<(), String> {
    ensure_allowed(&roots, Path::new(&path))?;
    fs::write(&path, content).map_err(|e| format!("Não foi possível salvar {path}: {e}"))
}

#[tauri::command]
pub fn create_markdown_file(
    roots: tauri::State<AllowedRoots>,
    dir: String,
    file_name: String,
) -> Result<String, String> {
    ensure_allowed(&roots, Path::new(&dir))?;
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
pub fn create_folder(
    roots: tauri::State<AllowedRoots>,
    dir: String,
    folder_name: String,
) -> Result<String, String> {
    ensure_allowed(&roots, Path::new(&dir))?;
    let path = PathBuf::from(&dir).join(&folder_name);
    if path.exists() {
        return Err(format!("Já existe uma pasta chamada {folder_name}"));
    }
    fs::create_dir_all(&path).map_err(|e| format!("Não foi possível criar a pasta: {e}"))?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn rename_path(
    roots: tauri::State<AllowedRoots>,
    path: String,
    new_name: String,
) -> Result<String, String> {
    let old_path = PathBuf::from(&path);
    ensure_allowed(&roots, &old_path)?;
    let parent = old_path
        .parent()
        .ok_or_else(|| "Caminho inválido".to_string())?;
    let new_path = parent.join(&new_name);
    // O novo nome não pode escapar da raiz (ex.: "../../fora.md").
    ensure_allowed(&roots, &new_path)?;
    fs::rename(&old_path, &new_path).map_err(|e| format!("Não foi possível renomear: {e}"))?;
    Ok(new_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn delete_path(roots: tauri::State<AllowedRoots>, path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    ensure_allowed(&roots, &p)?;
    if p.is_dir() {
        fs::remove_dir_all(&p).map_err(|e| format!("Não foi possível excluir a pasta: {e}"))
    } else {
        fs::remove_file(&p).map_err(|e| format!("Não foi possível excluir o arquivo: {e}"))
    }
}

#[tauri::command]
pub fn open_in_file_manager(
    app: tauri::AppHandle,
    roots: tauri::State<AllowedRoots>,
    path: String,
) -> Result<(), String> {
    ensure_allowed(&roots, Path::new(&path))?;
    app.opener()
        .reveal_item_in_dir(&path)
        .map_err(|e| format!("Não foi possível abrir o local do arquivo: {e}"))
}

#[cfg(test)]
mod tests {
    use super::is_inside;
    use std::path::Path;

    fn inside(root: &str, target: &str) -> bool {
        is_inside(Path::new(root), Path::new(target))
    }

    #[test]
    fn aceita_arquivo_dentro_da_raiz() {
        assert!(inside("C:\\proj", "C:\\proj\\doc.md"));
        assert!(inside("/home/w/proj", "/home/w/proj/sub/doc.md"));
    }

    #[test]
    fn aceita_a_propria_raiz() {
        assert!(inside("C:\\proj", "C:\\proj"));
    }

    #[test]
    fn recusa_caminho_fora_da_raiz() {
        assert!(!inside("C:\\proj", "C:\\Windows\\System32"));
        assert!(!inside("/home/w/proj", "/etc/passwd"));
    }

    #[test]
    fn recusa_prefixo_que_nao_e_fronteira_de_pasta() {
        // "projeto" apenas começa com "proj" — não está dentro dele.
        assert!(!inside("C:\\proj", "C:\\projeto\\doc.md"));
        assert!(!inside("/home/w/proj", "/home/w/projeto/doc.md"));
    }

    #[test]
    fn recusa_escape_por_dot_dot() {
        assert!(!inside("C:\\proj", "C:\\proj\\..\\outro\\doc.md"));
        assert!(!inside("/home/w/proj", "/home/w/proj/../../etc/passwd"));
    }

    #[test]
    fn aceita_dot_dot_que_permanece_dentro() {
        assert!(inside("C:\\proj", "C:\\proj\\sub\\..\\doc.md"));
    }

    #[test]
    fn raiz_vazia_nao_autoriza_nada() {
        assert!(!inside("", "C:\\qualquer\\coisa.md"));
    }

    #[cfg(windows)]
    #[test]
    fn no_windows_ignora_maiusculas() {
        assert!(inside("C:\\Proj", "c:\\proj\\doc.md"));
    }
}
