//! Associação de arquivos e integração ao menu de contexto do Windows, feita
//! inteiramente por usuário (HKEY_CURRENT_USER) — não requer privilégios de
//! administrador. Inspirado no painel Tools → Options do 7-Zip (aba System =
//! associações; aba 7-Zip = menu de contexto).
//!
//! Limitação conhecida do Windows 10/11: a *escolha padrão* de um tipo de
//! arquivo é protegida (UserChoice, com hash verificado pelo Explorer) e não
//! pode ser forçada em silêncio por um app. Aqui registramos o ProgId e a
//! entrada "Abrir com", que já fazem o Markforge aparecer nas opções; para
//! virar o padrão de fato, o usuário confirma em Configurações → Apps padrão
//! (abrimos essa tela por ele).

use serde::Serialize;

/// Extensões que o Markforge gerencia.
pub const MANAGED_EXTS: [&str; 2] = [".md", ".markdown"];
/// ProgId único do Markforge no registro do Windows.
pub const PROGID: &str = "Markforge.Document";

#[derive(Serialize)]
pub struct ExtAssoc {
    pub ext: String,
    /// `true` se o handler efetivo da extensão é o Markforge.
    pub associated: bool,
    /// Handler atual (ProgId), para exibir quando é outro app. `None` = nenhum.
    pub handler: Option<String>,
}

#[derive(Serialize)]
pub struct ContextMenuStatus {
    pub files: bool,
    pub folders: bool,
}

/// Monta a string de comando `"exe" "arg"` usada nas chaves shell\...\command.
fn shell_command(exe: &str, arg: &str) -> String {
    format!("\"{exe}\" \"{arg}\"")
}

/// `true` no Windows (onde a associação é suportada).
#[tauri::command]
pub fn assoc_supported() -> bool {
    cfg!(windows)
}

// ───────────────────────────── Windows ─────────────────────────────
#[cfg(windows)]
mod imp {
    use super::*;
    use winreg::enums::*;
    use winreg::RegKey;

    fn exe_path() -> Result<String, String> {
        std::env::current_exe()
            .map_err(|e| format!("Não foi possível localizar o executável: {e}"))
            .map(|p| p.to_string_lossy().to_string())
    }

    fn icon_ref(exe: &str) -> String {
        format!("{exe},0")
    }

    fn classes() -> Result<RegKey, String> {
        RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags("Software\\Classes", KEY_READ | KEY_WRITE)
            .map_err(|e| format!("Não foi possível abrir o registro (HKCU\\Software\\Classes): {e}"))
    }

    /// ProgId efetivo da extensão: primeiro a UserChoice (o que o Windows
    /// realmente usa), depois o default clássico de HKCU\Software\Classes\.ext.
    fn effective_progid(ext: &str) -> Option<String> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let user_choice = format!(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\FileExts\\{ext}\\UserChoice"
        );
        if let Ok(k) = hkcu.open_subkey(&user_choice) {
            if let Ok(progid) = k.get_value::<String, _>("ProgId") {
                if !progid.is_empty() {
                    return Some(progid);
                }
            }
        }
        if let Ok(k) = hkcu.open_subkey(format!("Software\\Classes\\{ext}")) {
            if let Ok(progid) = k.get_value::<String, _>("") {
                if !progid.is_empty() {
                    return Some(progid);
                }
            }
        }
        None
    }

    pub fn status() -> Result<Vec<ExtAssoc>, String> {
        Ok(MANAGED_EXTS
            .iter()
            .map(|ext| {
                let handler = effective_progid(ext);
                ExtAssoc {
                    ext: (*ext).to_string(),
                    associated: handler.as_deref() == Some(PROGID),
                    handler,
                }
            })
            .collect())
    }

    fn ensure_progid(exe: &str) -> Result<(), String> {
        let classes = classes()?;
        let (progid, _) = classes
            .create_subkey(PROGID)
            .map_err(|e| e.to_string())?;
        progid.set_value("", &"Documento Markdown").map_err(|e| e.to_string())?;
        let (icon, _) = progid.create_subkey("DefaultIcon").map_err(|e| e.to_string())?;
        icon.set_value("", &icon_ref(exe)).map_err(|e| e.to_string())?;
        let (cmd, _) = progid
            .create_subkey("shell\\open\\command")
            .map_err(|e| e.to_string())?;
        cmd.set_value("", &shell_command(exe, "%1")).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn set_association(ext: &str, associate: bool) -> Result<(), String> {
        if !MANAGED_EXTS.contains(&ext) {
            return Err(format!("Extensão não suportada: {ext}"));
        }
        let classes = classes()?;
        if associate {
            let exe = exe_path()?;
            ensure_progid(&exe)?;
            // Aparece em "Abrir com" sem forçar o padrão.
            let (owp, _) = classes
                .create_subkey(format!("{ext}\\OpenWithProgids"))
                .map_err(|e| e.to_string())?;
            owp.set_value(PROGID, &"").map_err(|e| e.to_string())?;
            // Default clássico (respeitado quando não há UserChoice).
            let (extkey, _) = classes.create_subkey(ext).map_err(|e| e.to_string())?;
            extkey.set_value("", &PROGID).map_err(|e| e.to_string())?;
        } else {
            // Remove só as marcas do Markforge; não mexe em UserChoice.
            if let Ok(owp) = classes.open_subkey_with_flags(
                format!("{ext}\\OpenWithProgids"),
                KEY_READ | KEY_WRITE,
            ) {
                let _ = owp.delete_value(PROGID);
            }
            if let Ok(extkey) =
                classes.open_subkey_with_flags(ext, KEY_READ | KEY_WRITE)
            {
                if extkey.get_value::<String, _>("").ok().as_deref() == Some(PROGID) {
                    let _ = extkey.delete_value("");
                }
            }
        }
        Ok(())
    }

    fn write_verb(
        classes: &RegKey,
        base: &str,
        label: &str,
        exe: &str,
        arg: &str,
    ) -> Result<(), String> {
        let (verb, _) = classes
            .create_subkey(format!("{base}\\Markforge"))
            .map_err(|e| e.to_string())?;
        verb.set_value("", &label).map_err(|e| e.to_string())?;
        verb.set_value("Icon", &icon_ref(exe)).map_err(|e| e.to_string())?;
        let (cmd, _) = verb.create_subkey("command").map_err(|e| e.to_string())?;
        cmd.set_value("", &shell_command(exe, arg)).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn context_status() -> Result<ContextMenuStatus, String> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let files = hkcu
            .open_subkey("Software\\Classes\\SystemFileAssociations\\.md\\shell\\Markforge")
            .is_ok();
        let folders = hkcu
            .open_subkey("Software\\Classes\\Directory\\shell\\Markforge")
            .is_ok();
        Ok(ContextMenuStatus { files, folders })
    }

    pub fn set_context_menu(files: bool, folders: bool) -> Result<(), String> {
        let classes = classes()?;
        let exe = exe_path()?;

        for ext in MANAGED_EXTS {
            let base = format!("SystemFileAssociations\\{ext}\\shell");
            if files {
                write_verb(&classes, &base, "Abrir com Markforge", &exe, "%1")?;
            } else {
                let _ = classes.delete_subkey_all(format!("{base}\\Markforge"));
            }
        }

        if folders {
            write_verb(&classes, "Directory\\shell", "Abrir pasta no Markforge", &exe, "%1")?;
            write_verb(
                &classes,
                "Directory\\Background\\shell",
                "Abrir pasta no Markforge",
                &exe,
                "%V",
            )?;
        } else {
            let _ = classes.delete_subkey_all("Directory\\shell\\Markforge");
            let _ = classes.delete_subkey_all("Directory\\Background\\shell\\Markforge");
        }
        Ok(())
    }

    pub fn open_default_apps() -> Result<(), String> {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", "ms-settings:defaultapps"])
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Não foi possível abrir Apps padrão: {e}"))
    }
}

// ─────────────────────────── não-Windows ───────────────────────────
#[cfg(not(windows))]
mod imp {
    use super::*;
    const MSG: &str = "Associação de arquivos só está disponível no Windows.";
    pub fn status() -> Result<Vec<ExtAssoc>, String> {
        Ok(Vec::new())
    }
    pub fn set_association(_ext: &str, _associate: bool) -> Result<(), String> {
        Err(MSG.into())
    }
    pub fn context_status() -> Result<ContextMenuStatus, String> {
        Ok(ContextMenuStatus { files: false, folders: false })
    }
    pub fn set_context_menu(_files: bool, _folders: bool) -> Result<(), String> {
        Err(MSG.into())
    }
    pub fn open_default_apps() -> Result<(), String> {
        Err(MSG.into())
    }
}

#[tauri::command]
pub fn get_association_status() -> Result<Vec<ExtAssoc>, String> {
    imp::status()
}

#[tauri::command]
pub fn set_association(ext: String, associate: bool) -> Result<(), String> {
    imp::set_association(&ext, associate)
}

#[tauri::command]
pub fn get_context_menu_status() -> Result<ContextMenuStatus, String> {
    imp::context_status()
}

#[tauri::command]
pub fn set_context_menu(files: bool, folders: bool) -> Result<(), String> {
    imp::set_context_menu(files, folders)
}

#[tauri::command]
pub fn open_default_apps_settings() -> Result<(), String> {
    imp::open_default_apps()
}

#[cfg(test)]
mod tests {
    use super::{shell_command, MANAGED_EXTS, PROGID};

    #[test]
    fn shell_command_cita_exe_e_arg() {
        assert_eq!(
            shell_command("C:\\Program Files\\Markforge\\markforge.exe", "%1"),
            "\"C:\\Program Files\\Markforge\\markforge.exe\" \"%1\""
        );
    }

    #[test]
    fn constantes_estaveis() {
        // Mudar essas constantes quebra associações já registradas — âncora.
        assert_eq!(PROGID, "Markforge.Document");
        assert_eq!(MANAGED_EXTS, [".md", ".markdown"]);
    }
}
