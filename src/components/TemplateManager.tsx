import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, TemplateInfo } from "@/lib/tauri";

interface TemplateManagerProps {
  onClose: () => void;
}

export function TemplateManager({ onClose }: TemplateManagerProps) {
  const [templates, setTemplates] = useState<TemplateInfo[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  function reload() {
    api.listTemplates().then(setTemplates).catch((e) => setError(String(e)));
  }

  useEffect(reload, []);

  async function handleImport() {
    setError(null);
    const selected = await open({
      multiple: false,
      filters: [{ name: "Modelo Word", extensions: ["docx"] }],
    });
    if (typeof selected !== "string") return;
    const name = prompt("Nome para este template:", "Meu template");
    if (!name) return;
    setBusy(true);
    try {
      await api.importTemplate(selected, name);
      reload();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function handleDelete(id: string) {
    if (!confirm("Remover este template?")) return;
    try {
      await api.deleteTemplate(id);
      reload();
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <div className="mf-modal-backdrop" onClick={onClose}>
      <div className="mf-modal" onClick={(e) => e.stopPropagation()}>
        <div className="mf-modal-header">
          <h2>Templates de exportação</h2>
          <button className="mf-icon-btn" onClick={onClose}>✕</button>
        </div>
        <div className="mf-modal-body">
          <p className="mf-metadata-hint">
            Um template é um arquivo .docx de referência: define fontes, cores,
            cabeçalho e rodapé usados na exportação. Importe um .docx próprio
            (ex: com a identidade visual da sua empresa) para usá-lo como
            template.
          </p>
          {error && <div className="mf-error">{error}</div>}
          <ul className="mf-template-list">
            <li className="mf-template-item">
              <span>Markforge Padrão</span>
              <span className="mf-template-tag">embutido</span>
            </li>
            {templates.map((t) => (
              <li key={t.id} className="mf-template-item">
                <span>{t.name}</span>
                <button className="mf-icon-btn danger" onClick={() => handleDelete(t.id)}>
                  Remover
                </button>
              </li>
            ))}
          </ul>
        </div>
        <div className="mf-modal-footer">
          <button className="mf-btn-secondary" onClick={onClose}>Fechar</button>
          <button className="mf-btn-primary" disabled={busy} onClick={handleImport}>
            {busy ? "Importando…" : "Importar .docx…"}
          </button>
        </div>
      </div>
    </div>
  );
}
