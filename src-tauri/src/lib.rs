mod commands;

use std::sync::Mutex;

use std::path::PathBuf;

use commands::fs_commands::AllowedRoots;
use commands::startup::{first_markdown_arg, StartupFile};
use commands::{deps, export, fs_commands, import, startup, templates, win_assoc};

/// RFC-005: encerra o aplicativo de verdade. Chamado pelo frontend **depois** de
/// decidir o destino das edições não salvas (Sim/Não/Cancelar). O `app.exit(0)`
/// dispara o encerramento sem re-entrar no handler de `CloseRequested` — evita
/// loop de "fechou de novo?".
#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Arquivo passado na linha de comando (ex.: duplo-clique num .md associado
    // ao Markforge) — consumido pelo frontend na inicialização.
    let startup_file = first_markdown_arg(&std::env::args().collect::<Vec<_>>());

    // A pasta do arquivo de inicialização já nasce autorizada: o caminho veio da
    // linha de comando, não do webview.
    let initial_roots: Vec<PathBuf> = startup_file
        .as_ref()
        .and_then(|f| PathBuf::from(f).parent().map(|p| p.to_path_buf()))
        .into_iter()
        .collect();

    let mut builder = tauri::Builder::default()
        .manage(StartupFile(Mutex::new(startup_file)))
        .manage(AllowedRoots(Mutex::new(initial_roots)));

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            use tauri::{Emitter, Manager};
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }
            // App já aberto e o usuário abriu outro .md: encaminha para a UI.
            if let Some(path) = first_markdown_arg(&args) {
                let _ = app.emit("open-file", path);
            }
        }));
    }

    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        // RFC-005: fecha a janela só depois que o frontend decidir o destino das
        // edições não salvas. O close é interceptado, prevenido e o webview é
        // avisado; ele responde com quit_app (salvando/descartando) ou com nada
        // (Cancelar → a janela permanece aberta).
        .on_window_event(|window, event| {
            use tauri::Emitter;
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.emit("close-requested", ());
            }
        })
        .invoke_handler(tauri::generate_handler![
            fs_commands::list_markdown_tree,
            fs_commands::read_text_file,
            fs_commands::write_text_file,
            fs_commands::create_markdown_file,
            fs_commands::create_folder,
            fs_commands::rename_path,
            fs_commands::delete_path,
            fs_commands::open_in_file_manager,
            fs_commands::allow_file,
            deps::check_dependencies,
            templates::list_templates,
            templates::import_template,
            templates::delete_template,
            export::export_document,
            import::import_docx,
            startup::take_startup_file,
            win_assoc::assoc_supported,
            win_assoc::get_association_status,
            win_assoc::set_association,
            win_assoc::get_context_menu_status,
            win_assoc::set_context_menu,
            win_assoc::open_default_apps_settings,
            quit_app,
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o Markforge");
}
