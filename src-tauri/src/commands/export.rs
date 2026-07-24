use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::Manager;
use tauri_plugin_shell::process::Output;
use tauri_plugin_shell::ShellExt;
use uuid::Uuid;

use super::templates::resolve_template_path;

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Docx,
    Pdf,
    Both,
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct ExportMetadata {
    title: Option<String>,
    subtitle: Option<String>,
    author: Option<String>,
    date: Option<String>,
}

#[derive(Deserialize)]
pub struct ExportOptions {
    markdown: String,
    format: ExportFormat,
    template_id: String,
    include_cover: bool,
    #[allow(dead_code)]
    metadata: ExportMetadata,
    output_dir: String,
    file_stem: String,
    source_dir: String,
}

#[derive(Serialize, Default)]
pub struct ExportResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    docx_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pdf_path: Option<String>,
    warnings: Vec<String>,
}

/// Remove o bloco de front-matter YAML (usado quando "Incluir capa" está
/// desmarcado, para que nenhum título/autor/data vaze no corpo do documento).
fn strip_frontmatter(markdown: &str) -> String {
    if let Some(rest) = markdown.strip_prefix("---\n") {
        if let Some(end_idx) = rest.find("\n---\n") {
            return rest[end_idx + 5..]
                .trim_start_matches(['\n', '\r'])
                .to_string();
        }
    }
    markdown.to_string()
}

/// Insere uma quebra de página OOXML logo após o front-matter, separando a
/// "capa" (título/subtítulo/autor/data) do corpo — usado apenas no pipeline
/// DOCX (o pipeline PDF/Typst cuida da quebra de página dentro do próprio
/// template, via `has-cover`).
fn with_cover_pagebreak_docx(markdown: &str) -> String {
    if let Some(rest) = markdown.strip_prefix("---\n") {
        if let Some(end_idx) = rest.find("\n---\n") {
            let front = &rest[..end_idx];
            let body = rest[end_idx + 5..].trim_start_matches(['\n', '\r']);
            return format!(
                "---\n{front}\n---\n\n```{{=openxml}}\n<w:p><w:r><w:br w:type=\"page\"/></w:r></w:p>\n```\n\n{body}"
            );
        }
    }
    markdown.to_string()
}

/// Decide se o PDF deve incluir a página de capa. Regra de produto: a capa só
/// entra quando há um template customizado selecionado — o template padrão
/// embutido não carrega identidade visual de capa, então gerar uma capa
/// genérica no PDF fica pior do que não ter. (No DOCX a capa segue valendo
/// para o template padrão; essa regra é só do PDF.)
fn pdf_should_include_cover(include_cover: bool, template_id: &str) -> bool {
    include_cover && template_id != "default"
}

fn write_temp_markdown(dir: &Path, content: &str) -> Result<PathBuf, String> {
    let path = dir.join("source.md");
    fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(path)
}

fn check_output(context: &str, output: &Output) -> Result<(), String> {
    if !output.status.success() {
        return Err(format!(
            "{context}:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

async fn run_pandoc_to_docx(
    app: &tauri::AppHandle,
    md_path: &Path,
    resource_path: &str,
    reference_doc: &Path,
    out_docx: &Path,
) -> Result<(), String> {
    let cmd = app
        .shell()
        .sidecar("pandoc")
        .map_err(|e| format!("Não foi possível localizar o Pandoc embutido: {e}"))?
        .args([
            md_path.as_os_str().to_string_lossy().to_string(),
            "--from".into(),
            // -citations: "@algo" (pacote npm com escopo, menção, e-mail) é
            // texto literal, não citação bibliográfica — ver run_pandoc_to_pdf.
            "markdown+yaml_metadata_block+raw_attribute-citations".into(),
            "--reference-doc".into(),
            reference_doc.as_os_str().to_string_lossy().to_string(),
            "--resource-path".into(),
            resource_path.to_string(),
            "--standalone".into(),
            "-o".into(),
            out_docx.as_os_str().to_string_lossy().to_string(),
        ]);
    let output = cmd
        .output()
        .await
        .map_err(|e| format!("Falha ao executar o Pandoc: {e}"))?;
    check_output("Pandoc retornou um erro ao gerar o DOCX", &output)
}

/// Resolve o caminho absoluto de um binário "sidecar" empacotado pelo Tauri.
/// Sidecars (`externalBin`) são instalados sempre ao lado do executável
/// principal do app, com o sufixo de target-triple removido — vale para
/// .deb, AppImage, MSI e NSIS.
fn sidecar_path(name: &str) -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| format!("Não foi possível localizar o executável do app: {e}"))?;
    let dir = exe
        .parent()
        .ok_or_else(|| "Não foi possível localizar o diretório do executável".to_string())?;
    let filename = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    };
    let path = dir.join(filename);
    if !path.exists() {
        return Err(format!(
            "Binário embutido '{name}' não encontrado em {} — a instalação pode estar corrompida.",
            path.display()
        ));
    }
    Ok(path)
}

/// Resolve um caminho para o binário do Typst cujo *nome* seja exatamente
/// "typst[.exe]" — exigência do Pandoc para aceitar `--pdf-engine`. Em produção
/// o sidecar já é instalado com esse nome ao lado do executável; em `tauri dev`
/// o binário cru mora em `src-tauri/binaries/` com o sufixo de target-triple
/// (que o Pandoc rejeita), então o copiamos para o diretório temporário com o
/// nome puro. Assim dev e produção passam pelo mesmo caminho de PDF.
// Em release (sem debug_assertions) o bloco de dev some e `tmp_dir` não é
// usado — esperado.
#[cfg_attr(not(debug_assertions), allow(unused_variables))]
fn resolve_typst_for_pandoc(tmp_dir: &Path) -> Result<PathBuf, String> {
    // Produção: ao lado do executável, já com o nome "puro".
    if let Ok(path) = sidecar_path("typst") {
        return Ok(path);
    }

    // Dev (debug): binaries/typst-<triple>[.exe] — precisa do nome puro.
    #[cfg(debug_assertions)]
    {
        let triple = tauri::utils::platform::target_triple()
            .map_err(|e| format!("Não foi possível detectar o target triple: {e}"))?;
        let raw_name = if cfg!(windows) {
            format!("typst-{triple}.exe")
        } else {
            format!("typst-{triple}")
        };
        let src = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("binaries")
            .join(&raw_name);
        if src.exists() {
            let dest = tmp_dir.join(if cfg!(windows) { "typst.exe" } else { "typst" });
            fs::copy(&src, &dest)
                .map_err(|e| format!("Não foi possível preparar o Typst para o Pandoc: {e}"))?;
            return Ok(dest);
        }
    }

    Err(
        "Binário embutido do Typst não encontrado — a instalação pode estar corrompida."
            .to_string(),
    )
}

async fn run_pandoc_to_pdf(
    app: &tauri::AppHandle,
    md_path: &Path,
    resource_path: &str,
    pdf_template: &Path,
    include_cover: bool,
    typst_path: &Path,
    out_pdf: &Path,
) -> Result<(), String> {
    let mut args: Vec<String> = vec![
        md_path.as_os_str().to_string_lossy().to_string(),
        "--from".into(),
        // -citations: sem isto, o Pandoc trata "@algo" como citação e o Typst
        // aborta por não haver bibliografia (ex.: @escopo/pacote em docs).
        "markdown+yaml_metadata_block-citations".into(),
        "--template".into(),
        pdf_template.as_os_str().to_string_lossy().to_string(),
        "--resource-path".into(),
        resource_path.to_string(),
        "--pdf-engine".into(),
        typst_path.as_os_str().to_string_lossy().to_string(),
        "-o".into(),
        out_pdf.as_os_str().to_string_lossy().to_string(),
    ];
    if include_cover {
        args.push("--metadata".into());
        args.push("has-cover:true".into());
    }

    let cmd = app
        .shell()
        .sidecar("pandoc")
        .map_err(|e| format!("Não foi possível localizar o Pandoc embutido: {e}"))?
        .args(args);
    let output = cmd
        .output()
        .await
        .map_err(|e| format!("Falha ao executar o Pandoc/Typst: {e}"))?;
    check_output("Falha ao gerar o PDF (Pandoc + Typst)", &output)
}

fn resolve_pdf_template(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .resolve(
            "templates/default/pdf-template.typ",
            tauri::path::BaseDirectory::Resource,
        )
        .map_err(|e| format!("Template de PDF não encontrado: {e}"))
}

#[tauri::command]
pub async fn export_document(
    app: tauri::AppHandle,
    options: ExportOptions,
) -> Result<ExportResult, String> {
    let reference_doc = resolve_template_path(&app, &options.template_id)?;
    if !reference_doc.exists() {
        return Err(format!(
            "Template DOCX não encontrado em {}",
            reference_doc.display()
        ));
    }
    let pdf_template = resolve_pdf_template(&app)?;

    let normalized = options.markdown.replace("\r\n", "\n");

    let tmp_dir = std::env::temp_dir().join(format!("markforge-{}", Uuid::new_v4()));
    fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;

    let output_dir = PathBuf::from(&options.output_dir);
    fs::create_dir_all(&output_dir)
        .map_err(|e| format!("Não foi possível usar a pasta de destino: {e}"))?;

    let mut result = ExportResult::default();

    if matches!(options.format, ExportFormat::Docx | ExportFormat::Both) {
        let content = if options.include_cover {
            with_cover_pagebreak_docx(&normalized)
        } else {
            strip_frontmatter(&normalized)
        };
        let md_path = write_temp_markdown(&tmp_dir, &content)?;
        let docx_target = output_dir.join(format!("{}.docx", options.file_stem));
        run_pandoc_to_docx(&app, &md_path, &options.source_dir, &reference_doc, &docx_target).await?;
        result.docx_path = Some(docx_target.to_string_lossy().to_string());
    }

    if matches!(options.format, ExportFormat::Pdf | ExportFormat::Both) {
        let pdf_cover = pdf_should_include_cover(options.include_cover, &options.template_id);
        let content = if pdf_cover {
            normalized.clone()
        } else {
            strip_frontmatter(&normalized)
        };
        let md_path = write_temp_markdown(&tmp_dir, &content)?;
        let typst_path = resolve_typst_for_pandoc(&tmp_dir)?;
        let pdf_target = output_dir.join(format!("{}.pdf", options.file_stem));
        run_pandoc_to_pdf(
            &app,
            &md_path,
            &options.source_dir,
            &pdf_template,
            pdf_cover,
            &typst_path,
            &pdf_target,
        )
        .await?;
        result.pdf_path = Some(pdf_target.to_string_lossy().to_string());
    }

    let _ = fs::remove_dir_all(&tmp_dir);

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::{strip_frontmatter, with_cover_pagebreak_docx};

    #[test]
    fn strip_frontmatter_remove_bloco_e_apara_corpo() {
        let md = "---\ntitle: T\nauthor: W\n---\n\n# Corpo\n";
        assert_eq!(strip_frontmatter(md), "# Corpo\n");
    }

    #[test]
    fn strip_frontmatter_sem_bloco_retorna_intacto() {
        let md = "# Só o corpo\n\ntexto";
        assert_eq!(strip_frontmatter(md), md);
    }

    #[test]
    fn strip_frontmatter_ignora_hifens_no_meio_do_texto() {
        // "--- com hífens" no corpo não pode ser confundido com o fim do bloco.
        let md = "---\ntitle: T\n---\n\ntexto --- com hifens\n";
        assert_eq!(strip_frontmatter(md), "texto --- com hifens\n");
    }

    #[test]
    fn with_cover_pagebreak_insere_quebra_apos_frontmatter() {
        let md = "---\ntitle: T\n---\n\n# Corpo\n";
        let out = with_cover_pagebreak_docx(md);
        assert!(out.starts_with("---\ntitle: T\n---\n\n```{=openxml}"));
        assert!(out.contains("<w:br w:type=\"page\"/>"));
        assert!(out.trim_end().ends_with("# Corpo"));
    }

    #[test]
    fn with_cover_pagebreak_sem_frontmatter_inalterado() {
        let md = "# Corpo sem capa\n";
        assert_eq!(with_cover_pagebreak_docx(md), md);
    }

    use super::pdf_should_include_cover;

    #[test]
    fn pdf_capa_suprimida_no_template_padrao() {
        assert!(!pdf_should_include_cover(true, "default"));
    }

    #[test]
    fn pdf_capa_incluida_com_template_customizado() {
        assert!(pdf_should_include_cover(true, "abc-123-uuid"));
    }

    #[test]
    fn pdf_sem_capa_quando_opcao_desmarcada() {
        assert!(!pdf_should_include_cover(false, "abc-123-uuid"));
        assert!(!pdf_should_include_cover(false, "default"));
    }
}
