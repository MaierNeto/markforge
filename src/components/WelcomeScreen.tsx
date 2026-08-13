import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useProjectStore } from "@/store/projectStore";
import { dirname } from "@/lib/paths";

interface WelcomeScreenProps {
  onOpenSettings: () => void;
}

export function WelcomeScreen({ onOpenSettings }: WelcomeScreenProps) {
  const openFolder = useProjectStore((s) => s.openFolder);
  const openSingleFile = useProjectStore((s) => s.openSingleFile);
  const importDocumentFile = useProjectStore((s) => s.importDocumentFile);
  const recents = useProjectStore((s) => s.recents);
  const [importing, setImporting] = useState(false);
  const [importError, setImportError] = useState<string | null>(null);

  function handleOpenRecent(entry: { path: string; kind: "folder" | "file" }) {
    if (entry.kind === "folder") {
      openFolder(entry.path);
    } else {
      openSingleFile(entry.path);
    }
  }

  async function handleOpenFolder() {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === "string") {
      await openFolder(selected);
    }
  }

  async function handleOpenFile() {
    const selected = await open({
      multiple: false,
      filters: [{ name: "Markdown", extensions: ["md", "markdown"] }],
    });
    if (typeof selected === "string") {
      await openSingleFile(selected);
    }
  }

  async function handleImportDocument() {
    setImportError(null);
    const selected = await open({
      multiple: false,
      filters: [{ name: "Documento", extensions: ["docx", "pdf", "txt"] }],
    });
    if (typeof selected !== "string") return;
    setImporting(true);
    try {
      // Sem projeto aberto ainda: importa para a mesma pasta do documento.
      await importDocumentFile(selected, dirname(selected));
    } catch (e) {
      setImportError(String(e));
    } finally {
      setImporting(false);
    }
  }

  return (
    <div className="mf-welcome">
      <div className="mf-welcome-card">
        <div className="mf-welcome-mark">M</div>
        <h1>Markforge</h1>
        <p>
          Edite visualmente os arquivos <code>.md</code> do seu projeto e exporte
          documentos DOCX/PDF prontos, com capa, cabeçalho e rodapé.
        </p>
        <div className="mf-welcome-actions">
          <button className="mf-btn-primary" onClick={handleOpenFolder}>
            Abrir pasta do projeto
          </button>
          <button className="mf-btn-secondary" onClick={handleOpenFile}>
            Abrir arquivo .md
          </button>
          <button className="mf-btn-secondary" disabled={importing} onClick={handleImportDocument}>
            {importing ? "Importando…" : "Importar documento…"}
          </button>
        </div>
        {importError && <div className="mf-error">{importError}</div>}
        {recents.length > 0 && (
          <div className="mf-welcome-recents">
            <span className="mf-welcome-recents-title">Recentes</span>
            {recents.map((entry) => (
              <button
                key={entry.path}
                className="mf-welcome-recent-item"
                title={entry.path}
                onClick={() => handleOpenRecent(entry)}
              >
                <span>{entry.kind === "folder" ? "📂" : "📄"}</span>
                {entry.label}
              </button>
            ))}
          </div>
        )}
        <p className="mf-welcome-hint">
          Ideal para pastas de controle de projetos com agentes de IA — abra a raiz
          do repositório e edite os arquivos <code>.md</code> com uma interface
          visual, sem perder a compatibilidade com o texto puro.
        </p>
        <button className="mf-welcome-settings" onClick={onOpenSettings}>
          Configurações e associação de arquivos
        </button>
      </div>
    </div>
  );
}
