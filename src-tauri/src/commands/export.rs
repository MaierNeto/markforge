use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use uuid::Uuid;

use super::deps::{pandoc_executable, soffice_executable};
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

/// Insere uma quebra de página OOXML logo após o bloco de front-matter,
/// separando a "capa" (título/subtítulo/autor/data) do corpo do documento.
fn with_cover_pagebreak(markdown: &str, include_cover: bool) -> String {
    if !include_cover {
        return markdown.to_string();
    }
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

fn write_temp_markdown(content: &str) -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join(format!("markforge-{}", Uuid::new_v4()));
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("source.md");
    fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(path)
}

fn run_pandoc_to_docx(
    pandoc_bin: &str,
    md_path: &Path,
    resource_path: &str,
    reference_doc: &Path,
    out_docx: &Path,
) -> Result<(), String> {
    let output = Command::new(pandoc_bin)
        .arg(md_path)
        .arg("--from")
        .arg("markdown+yaml_metadata_block+raw_attribute")
        .arg("--reference-doc")
        .arg(reference_doc)
        .arg("--resource-path")
        .arg(resource_path)
        .arg("--standalone")
        .arg("-o")
        .arg(out_docx)
        .output()
        .map_err(|e| format!("Falha ao executar o Pandoc: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "Pandoc retornou um erro:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

fn file_url(path: &Path) -> String {
    let s = path.to_string_lossy().replace('\\', "/");
    if let Some(stripped) = s.strip_prefix('/') {
        format!("file:///{stripped}")
    } else {
        format!("file:///{s}")
    }
}

fn run_soffice_to_pdf(soffice_bin: &str, docx_path: &Path, out_dir: &Path) -> Result<PathBuf, String> {
    let profile_dir = std::env::temp_dir().join(format!("markforge-soffice-{}", Uuid::new_v4()));
    let profile_arg = format!("-env:UserInstallation={}", file_url(&profile_dir));

    let output = Command::new(soffice_bin)
        .arg("--headless")
        .arg("--norestore")
        .arg(&profile_arg)
        .arg("--convert-to")
        .arg("pdf")
        .arg("--outdir")
        .arg(out_dir)
        .arg(docx_path)
        .output()
        .map_err(|e| format!("Falha ao executar o LibreOffice: {e}"))?;

    let _ = fs::remove_dir_all(&profile_dir);

    if !output.status.success() {
        return Err(format!(
            "LibreOffice retornou um erro:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stem = docx_path
        .file_stem()
        .ok_or_else(|| "Nome de arquivo DOCX inválido".to_string())?
        .to_string_lossy()
        .to_string();
    let pdf_path = out_dir.join(format!("{stem}.pdf"));
    if !pdf_path.exists() {
        return Err("A conversão para PDF não gerou o arquivo esperado.".into());
    }
    Ok(pdf_path)
}

#[tauri::command]
pub fn export_document(app: tauri::AppHandle, options: ExportOptions) -> Result<ExportResult, String> {
    let pandoc_bin = pandoc_executable()?;
    let reference_doc = resolve_template_path(&app, &options.template_id)?;
    if !reference_doc.exists() {
        return Err(format!(
            "Template não encontrado em {}",
            reference_doc.display()
        ));
    }

    let normalized = options.markdown.replace("\r\n", "\n");
    let content = with_cover_pagebreak(&normalized, options.include_cover);
    let tmp_md = write_temp_markdown(&content)?;
    let tmp_dir = tmp_md
        .parent()
        .ok_or_else(|| "Falha ao preparar arquivo temporário".to_string())?
        .to_path_buf();

    let output_dir = PathBuf::from(&options.output_dir);
    fs::create_dir_all(&output_dir).map_err(|e| format!("Não foi possível usar a pasta de destino: {e}"))?;

    let mut result = ExportResult::default();
    let needs_docx_in_output = matches!(options.format, ExportFormat::Docx | ExportFormat::Both);

    let docx_target = if needs_docx_in_output {
        output_dir.join(format!("{}.docx", options.file_stem))
    } else {
        tmp_dir.join(format!("{}.docx", options.file_stem))
    };

    run_pandoc_to_docx(
        &pandoc_bin,
        &tmp_md,
        &options.source_dir,
        &reference_doc,
        &docx_target,
    )?;

    if needs_docx_in_output {
        result.docx_path = Some(docx_target.to_string_lossy().to_string());
    }

    if matches!(options.format, ExportFormat::Pdf | ExportFormat::Both) {
        let soffice_bin = soffice_executable()?;
        let pdf_out_dir = if needs_docx_in_output {
            output_dir.clone()
        } else {
            tmp_dir.clone()
        };
        let generated_pdf = run_soffice_to_pdf(&soffice_bin, &docx_target, &pdf_out_dir)?;
        let final_pdf = output_dir.join(format!("{}.pdf", options.file_stem));
        if generated_pdf != final_pdf {
            if fs::rename(&generated_pdf, &final_pdf).is_err() {
                fs::copy(&generated_pdf, &final_pdf)
                    .map_err(|e| format!("Não foi possível mover o PDF gerado: {e}"))?;
            }
        }
        result.pdf_path = Some(final_pdf.to_string_lossy().to_string());
    }

    let _ = fs::remove_dir_all(&tmp_dir);

    Ok(result)
}
