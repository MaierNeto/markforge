import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useProjectStore } from "@/store/projectStore";
import { serializeDocument } from "@/lib/frontmatter";
import {
  api,
  DependencyStatus,
  ExportFormat,
  ExportResult,
  TemplateInfo,
} from "@/lib/tauri";

interface ExportDialogProps {
  onClose: () => void;
}

export function ExportDialog({ onClose }: ExportDialogProps) {
  const openDoc = useProjectStore((s) => s.openDoc);
  const rootPath = useProjectStore((s) => s.rootPath);

  const [deps, setDeps] = useState<DependencyStatus | null>(null);
  const [templates, setTemplates] = useState<TemplateInfo[]>([]);
  const [templateId, setTemplateId] = useState<string>("default");
  const [format, setFormat] = useState<ExportFormat>("both");
  const [includeCover, setIncludeCover] = useState(true);
  const [title, setTitle] = useState(openDoc?.metadata.title ?? "");
  const [subtitle, setSubtitle] = useState(openDoc?.metadata.subtitle ?? "");
  const [author, setAuthor] = useState(openDoc?.metadata.author ?? "");
  const [date, setDate] = useState(openDoc?.metadata.date ?? "");
  const [outputDir, setOutputDir] = useState<string | null>(rootPath);
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<ExportResult | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  useEffect(() => {
    api.checkDependencies().then(setDeps).catch(() => setDeps(null));
    api.listTemplates().then(setTemplates).catch(() => setTemplates([]));
  }, []);

  if (!openDoc) return null;

  const fileStem = openDoc.path.split(/[\\/]/).pop()?.replace(/\.md$/i, "") ?? "documento";
  // Pandoc/Typst vêm embutidos no app — só ficam "faltando" se a instalação
  // estiver corrompida, o que é bem raro.
  const installCorrupted = deps && (!deps.pandoc || !deps.typst);

  async function pickOutputDir() {
    const selected = await open({ directory: true, multiple: false, defaultPath: outputDir ?? undefined });
    if (typeof selected === "string") setOutputDir(selected);
  }

  async function handleExport() {
    if (!openDoc || !outputDir) return;
    setBusy(true);
    setErrorMsg(null);
    setResult(null);
    try {
      const markdown = serializeDocument(
        { title, subtitle, author, date },
        openDoc.body
      );
      const res = await api.exportDocument({
        markdown,
        format,
        template_id: templateId,
        include_cover: includeCover,
        metadata: { title, subtitle, author, date },
        output_dir: outputDir,
        file_stem: fileStem,
        source_dir: openDoc.path.substring(0, Math.max(openDoc.path.lastIndexOf("/"), openDoc.path.lastIndexOf("\\"))),
      });
      setResult(res);
    } catch (e) {
      setErrorMsg(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="mf-modal-backdrop" onClick={onClose}>
      <div className="mf-modal" onClick={(e) => e.stopPropagation()}>
        <div className="mf-modal-header">
          <h2>Exportar documento</h2>
          <button className="mf-icon-btn" onClick={onClose}>✕</button>
        </div>

        {installCorrupted && (
          <div className="mf-warning">
            <p>
              <strong>Algo não está certo na instalação do Markforge.</strong>{" "}
              Os componentes internos de exportação não foram encontrados —
              tente reinstalar o aplicativo.
            </p>
          </div>
        )}

        <div className="mf-modal-body">
          <div className="mf-form-row">
            <label className="mf-field">
              <span>Formato</span>
              <select value={format} onChange={(e) => setFormat(e.target.value as ExportFormat)}>
                <option value="docx">DOCX</option>
                <option value="pdf">PDF</option>
                <option value="both">DOCX + PDF</option>
              </select>
            </label>
            <label className="mf-field">
              <span>Template</span>
              <select value={templateId} onChange={(e) => setTemplateId(e.target.value)}>
                <option value="default">Markforge Padrão</option>
                {templates.map((t) => (
                  <option key={t.id} value={t.id}>{t.name}</option>
                ))}
              </select>
            </label>
          </div>

          <label className="mf-checkbox-row">
            <input type="checkbox" checked={includeCover} onChange={(e) => setIncludeCover(e.target.checked)} />
            <span>Incluir página de capa (título, subtítulo, autor, data)</span>
          </label>

          {includeCover &&
            templateId === "default" &&
            (format === "pdf" || format === "both") && (
              <p className="mf-metadata-hint">
                A capa no PDF só é gerada com um template customizado. No template
                padrão, a capa vale apenas para o DOCX.
              </p>
            )}

          {includeCover && (
            <div className="mf-cover-fields">
              <label className="mf-field">
                <span>Título</span>
                <input value={title} onChange={(e) => setTitle(e.target.value)} placeholder={fileStem} />
              </label>
              <label className="mf-field">
                <span>Subtítulo</span>
                <input value={subtitle} onChange={(e) => setSubtitle(e.target.value)} />
              </label>
              <div className="mf-form-row">
                <label className="mf-field">
                  <span>Autor</span>
                  <input value={author} onChange={(e) => setAuthor(e.target.value)} />
                </label>
                <label className="mf-field">
                  <span>Data</span>
                  <input value={date} onChange={(e) => setDate(e.target.value)} />
                </label>
              </div>
            </div>
          )}

          <label className="mf-field">
            <span>Salvar em</span>
            <div className="mf-path-picker">
              <input readOnly value={outputDir ?? ""} placeholder="Escolha uma pasta de destino" />
              <button onClick={pickOutputDir}>Escolher…</button>
            </div>
          </label>

          {errorMsg && <div className="mf-error">{errorMsg}</div>}
          {result && (
            <div className="mf-success">
              <p>Exportado com sucesso!</p>
              {result.docx_path && <p>📄 {result.docx_path}</p>}
              {result.pdf_path && <p>📄 {result.pdf_path}</p>}
              {result.warnings.map((w, i) => (
                <p key={i} className="mf-warning-line">⚠ {w}</p>
              ))}
            </div>
          )}
        </div>

        <div className="mf-modal-footer">
          <button className="mf-btn-secondary" onClick={onClose}>Cancelar</button>
          <button
            className="mf-btn-primary"
            disabled={busy || !outputDir || !!installCorrupted}
            onClick={handleExport}
          >
            {busy ? "Exportando…" : "Exportar"}
          </button>
        </div>
      </div>
    </div>
  );
}
