import { useProjectStore } from "@/store/projectStore";

interface TopBarProps {
  onExport: () => void;
  onManageTemplates: () => void;
}

const STATUS_LABEL: Record<string, string> = {
  idle: "",
  saving: "Salvando…",
  saved: "Salvo",
  error: "Erro ao salvar",
};

export function TopBar({ onExport, onManageTemplates }: TopBarProps) {
  const openDoc = useProjectStore((s) => s.openDoc);
  const rootPath = useProjectStore((s) => s.rootPath);
  const saveStatus = useProjectStore((s) => s.saveStatus);

  const relativePath =
    openDoc && rootPath ? openDoc.path.replace(rootPath, "").replace(/^[\\/]/, "") : "";

  return (
    <div className="mf-topbar">
      <div className="mf-topbar-title">
        <span className="mf-brand">Markforge</span>
        {openDoc && <span className="mf-breadcrumb">{relativePath}</span>}
      </div>
      <div className="mf-topbar-actions">
        {openDoc && <span className="mf-save-status">{STATUS_LABEL[saveStatus]}</span>}
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
