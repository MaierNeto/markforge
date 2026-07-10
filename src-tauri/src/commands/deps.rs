use serde::Serialize;
use tauri_plugin_shell::ShellExt;

/// Pandoc e Typst agora vêm embutidos no aplicativo (sidecars) — não há mais
/// nada para o usuário instalar. Esta checagem é só uma verificação de
/// sanidade (ex: detectar uma instalação corrompida) e não deveria falhar em
/// condições normais.
#[derive(Serialize)]
pub struct DependencyStatus {
    pandoc: bool,
    typst: bool,
}

async fn sidecar_ok(app: &tauri::AppHandle, name: &str) -> bool {
    let Ok(cmd) = app.shell().sidecar(name) else {
        return false;
    };
    matches!(cmd.arg("--version").output().await, Ok(output) if output.status.success())
}

#[tauri::command]
pub async fn check_dependencies(app: tauri::AppHandle) -> DependencyStatus {
    DependencyStatus {
        pandoc: sidecar_ok(&app, "pandoc").await,
        typst: sidecar_ok(&app, "typst").await,
    }
}
