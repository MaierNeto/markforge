import { useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { useProjectStore } from "@/store/projectStore";

interface TopBarProps {
  onExport: () => void;
  onManageTemplates: () => void;
  onOpenSettings: () => void;
}

const STATUS_LABEL: Record<string, string> = {
  idle: "",
  saving: "Salvando…",
  saved: "Salvo",
  error: "Erro ao salvar",
};

export function TopBar({ onExport, onManageTemplates, onOpenSettings }: TopBarProps) {
  const openDoc = useProjectStore((s) => s.openDoc);
  const rootPath = useProjectStore((s) => s.rootPath);
  const saveStatus = useProjectStore((s) => s.saveStatus);
  const toggleEditMode = useProjectStore((s) => s.toggleEditMode);
  const revertToSnapshot = useProjectStore((s) => s.revertToSnapshot);
  const openFolder = useProjectStore((s) => s.openFolder);
  const openSingleFile = useProjectStore((s) => s.openSingleFile);
  const saveAs = useProjectStore((s) => s.saveAs);
  const importDocumentFile = useProjectStore((s) => s.importDocumentFile);
  const [openMenuVisible, setOpenMenuVisible] = useState(false);

  const relativePath =
    openDoc && rootPath ? openDoc.path.replace(rootPath, "").replace(/^[\\/]/, "") : "";

  const isReading = openDoc?.mode === "reading";
  const isDirty = openDoc?.dirty;

  async function handleOpenAnotherFolder() {
    setOpenMenuVisible(false);
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === "string") await openFolder(selected);
  }

  async function handleOpenAnotherFile() {
    setOpenMenuVisible(false);
    const selected = await open({
      multiple: false,
      filters: [{ name: "Markdown", extensions: ["md", "markdown"] }],
    });
    if (typeof selected === "string") await openSingleFile(selected);
  }

  async function handleImportDocument() {
    setOpenMenuVisible(false);
    if (!rootPath) return;
    const selected = await open({
      multiple: false,
      filters: [{ name: "Documento", extensions: ["docx", "pdf", "txt"] }],
    });
    if (typeof selected === "string") await importDocumentFile(selected, rootPath);
  }

  async function handleSaveAs() {
    if (!openDoc) return;
    const newPath = await save({
      defaultPath: openDoc.path,
      filters: [{ name: "Markdown", extensions: ["md", "markdown"] }],
    });
    if (newPath) await saveAs(newPath);
  }

  return (
    <div className="mf-topbar">
      <div className="mf-topbar-title">
        <span className="mf-brand">Markforge</span>
        {openDoc && <span className="mf-breadcrumb">{relativePath}</span>}
      </div>
      <div className="mf-topbar-actions">
        {openDoc && <span className="mf-save-status">{isDirty ? "● " : ""}{STATUS_LABEL[saveStatus]}</span>}
        {openDoc && isReading && (
          <button className="mf-btn-secondary" onClick={toggleEditMode} title="Habilitar edição">
            Editar
          </button>
        )}
        {openDoc && !isReading && (
          <>
            <button className="mf-btn-secondary" onClick={toggleEditMode} title="Voltar para leitura">
              Leitura
            </button>
            {isDirty && (
              <button className="mf-btn-danger" onClick={revertToSnapshot} title="Reverter ao original">
                Reverter
              </button>
            )}
          </>
        )}
        {openDoc && (
          <button className="mf-btn-secondary" onClick={handleSaveAs} title="Salvar como… (mover para outra pasta)">
            Salvar como…
          </button>
        )}
        <div className="mf-open-menu">
          <button
            className="mf-btn-secondary"
            onClick={() => setOpenMenuVisible((v) => !v)}
            title="Abrir outra pasta ou arquivo"
          >
            Abrir…
          </button>
          {openMenuVisible && (
            <div className="mf-tree-menu mf-open-menu-list">
              <button onClick={handleOpenAnotherFolder}>Abrir pasta…</button>
              <button onClick={handleOpenAnotherFile}>Abrir arquivo .md…</button>
              <button onClick={handleImportDocument} title="Importa para a raiz da pasta do projeto atual">
                Importar documento…
              </button>
            </div>
          )}
        </div>
        <button className="mf-btn-secondary" onClick={onOpenSettings} title="Configurações">
          Configurações
        </button>
        <button className="mf-btn-secondary" onClick={onManageTemplates}>
          Templates
        </button>
        <button className="mf-btn-primary" disabled={!openDoc} onClick={onExport}>
          Exportar…
        </button>
      </div>
    </div>
  );
}
