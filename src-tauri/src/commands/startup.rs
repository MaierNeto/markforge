use std::sync::Mutex;

/// Guarda o caminho do arquivo passado na linha de comando ao abrir o app
/// (ex.: duplo-clique num .md associado). É consumido uma única vez pelo
/// frontend via `take_startup_file`.
pub struct StartupFile(pub Mutex<Option<String>>);

/// Extrai o primeiro argumento que aparenta ser um arquivo Markdown de uma
/// lista de argumentos de linha de comando. O argumento 0 (caminho do
/// executável) é naturalmente ignorado por não terminar em .md/.markdown.
pub fn first_markdown_arg(args: &[String]) -> Option<String> {
    args.iter()
        .find(|a| {
            let lower = a.to_ascii_lowercase();
            lower.ends_with(".md") || lower.ends_with(".markdown")
        })
        .cloned()
}

/// Devolve (e consome) o arquivo de inicialização, se houver. Retornar `None`
/// nas chamadas seguintes evita reabrir o mesmo arquivo em remontagens da UI.
#[tauri::command]
pub fn take_startup_file(state: tauri::State<StartupFile>) -> Option<String> {
    state.0.lock().ok().and_then(|mut guard| guard.take())
}

#[cfg(test)]
mod tests {
    use super::first_markdown_arg;

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn ignora_o_executavel_e_acha_o_md() {
        let args = v(&["C:\\app\\markforge.exe", "C:\\docs\\plano.md"]);
        assert_eq!(first_markdown_arg(&args).as_deref(), Some("C:\\docs\\plano.md"));
    }

    #[test]
    fn aceita_extensao_markdown_e_maiuscula() {
        let args = v(&["markforge", "/home/w/Notas.MARKDOWN"]);
        assert_eq!(first_markdown_arg(&args).as_deref(), Some("/home/w/Notas.MARKDOWN"));
    }

    #[test]
    fn retorna_none_sem_arquivo_md() {
        let args = v(&["markforge", "--flag", "outra-coisa.txt"]);
        assert_eq!(first_markdown_arg(&args), None);
    }

    #[test]
    fn pega_o_primeiro_quando_ha_varios() {
        let args = v(&["markforge", "a.md", "b.md"]);
        assert_eq!(first_markdown_arg(&args).as_deref(), Some("a.md"));
    }
}
