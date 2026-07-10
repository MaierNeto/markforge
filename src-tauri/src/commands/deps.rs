use serde::Serialize;
use std::process::Command;

#[derive(Serialize)]
pub struct DependencyStatus {
    pandoc: bool,
    pandoc_version: Option<String>,
    soffice: bool,
    soffice_version: Option<String>,
}

fn first_line(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string()
}

/// Tenta rodar `<candidato> --version` para cada nome/caminho candidato,
/// retornando a primeira linha da saída no primeiro que funcionar.
fn probe(candidates: &[&str]) -> Option<String> {
    for candidate in candidates {
        if let Ok(output) = Command::new(candidate).arg("--version").output() {
            if output.status.success() {
                let line = first_line(&output.stdout);
                if !line.is_empty() {
                    return Some(line);
                }
            }
        }
    }
    None
}

fn pandoc_candidates() -> Vec<&'static str> {
    let mut v = vec!["pandoc"];
    if cfg!(target_os = "windows") {
        v.push(r"C:\Program Files\Pandoc\pandoc.exe");
    }
    v
}

fn soffice_candidates() -> Vec<&'static str> {
    let mut v = vec!["soffice", "libreoffice"];
    if cfg!(target_os = "windows") {
        v.push(r"C:\Program Files\LibreOffice\program\soffice.exe");
        v.push(r"C:\Program Files (x86)\LibreOffice\program\soffice.exe");
    } else if cfg!(target_os = "macos") {
        v.push("/Applications/LibreOffice.app/Contents/MacOS/soffice");
    } else {
        v.push("/usr/bin/soffice");
        v.push("/snap/bin/libreoffice");
    }
    v
}

#[tauri::command]
pub fn check_dependencies() -> DependencyStatus {
    let pandoc_version = probe(&pandoc_candidates());
    let soffice_version = probe(&soffice_candidates());
    DependencyStatus {
        pandoc: pandoc_version.is_some(),
        pandoc_version,
        soffice: soffice_version.is_some(),
        soffice_version,
    }
}

/// Resolve o executável utilizável (nome no PATH ou caminho absoluto) para uso
/// em outros comandos (export). Retorna erro amigável se não encontrado.
pub fn resolve_executable(candidates: &[&str], friendly_name: &str, install_hint: &str) -> Result<String, String> {
    for candidate in candidates {
        if let Ok(output) = Command::new(candidate).arg("--version").output() {
            if output.status.success() {
                return Ok(candidate.to_string());
            }
        }
    }
    Err(format!(
        "{friendly_name} não encontrado no sistema. {install_hint}"
    ))
}

pub fn pandoc_executable() -> Result<String, String> {
    resolve_executable(
        &pandoc_candidates(),
        "Pandoc",
        "Instale em https://pandoc.org/installing.html",
    )
}

pub fn soffice_executable() -> Result<String, String> {
    resolve_executable(
        &soffice_candidates(),
        "LibreOffice",
        "Instale em https://www.libreoffice.org/download/download/",
    )
}
