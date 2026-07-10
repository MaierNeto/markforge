mod commands;

use commands::{deps, export, fs_commands, templates};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            use tauri::Manager;
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }
        }));
    }

    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            fs_commands::list_markdown_tree,
            fs_commands::read_text_file,
            fs_commands::write_text_file,
            fs_commands::create_markdown_file,
            fs_commands::create_folder,
            fs_commands::rename_path,
            fs_commands::delete_path,
            fs_commands::open_in_file_manager,
            deps::check_dependencies,
            templates::list_templates,
            templates::import_template,
            templates::delete_template,
            export::export_document,
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o Markforge");
}
