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

  const relativePath =
    openDoc && rootPath ? openDoc.path.replace(rootPath, "").replace(/^[\\/]/, "") : "";

  const isReading = openDoc?.mode === "reading";
  const isDirty = openDoc?.dirty;

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
