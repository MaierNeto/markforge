import { useEffect, useState } from "react";
import { api, ExtAssoc, ContextMenuStatus } from "@/lib/tauri";

interface SettingsDialogProps {
  onClose: () => void;
}

type Tab = "assoc" | "context";

export function SettingsDialog({ onClose }: SettingsDialogProps) {
  const [supported, setSupported] = useState<boolean | null>(null);
  const [tab, setTab] = useState<Tab>("assoc");
  const [assocs, setAssocs] = useState<ExtAssoc[]>([]);
  const [ctx, setCtx] = useState<ContextMenuStatus>({ files: false, folders: false });
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function reload() {
    try {
      const [a, c] = await Promise.all([
        api.getAssociationStatus(),
        api.getContextMenuStatus(),
      ]);
      setAssocs(a);
      setCtx(c);
    } catch (e) {
      setError(String(e));
    }
  }

  useEffect(() => {
    api.assocSupported().then((s) => {
      setSupported(s);
      if (s) reload();
    });
  }, []);

  async function toggleAssoc(ext: string, associate: boolean) {
    setBusy(true);
    setError(null);
    try {
      await api.setAssociation(ext, associate);
      await reload();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function updateContext(next: ContextMenuStatus) {
    setBusy(true);
    setError(null);
    try {
      await api.setContextMenu(next.files, next.folders);
      await reload();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  function stateLabel(a: ExtAssoc): string {
    if (a.associated) return "Markforge";
    if (a.handler) return `Outro app (${a.handler})`;
    return "Não associado";
  }

  return (
    <div className="mf-modal-backdrop" onClick={onClose}>
      <div className="mf-modal" onClick={(e) => e.stopPropagation()}>
        <div className="mf-modal-header">
          <h2>Configurações</h2>
          <button className="mf-icon-btn" onClick={onClose}>✕</button>
        </div>

        {supported === false && (
          <div className="mf-modal-body">
            <p className="mf-metadata-hint">
              A associação de arquivos e o menu de contexto estão disponíveis
              apenas no Windows.
            </p>
          </div>
        )}

        {supported && (
          <>
            <div className="mf-tabs">
              <button
                className={`mf-tab ${tab === "assoc" ? "active" : ""}`}
                onClick={() => setTab("assoc")}
              >
                Associações de arquivo
              </button>
              <button
                className={`mf-tab ${tab === "context" ? "active" : ""}`}
                onClick={() => setTab("context")}
              >
                Menu de contexto
              </button>
            </div>

            <div className="mf-modal-body">
              {error && <div className="mf-error">{error}</div>}

              {tab === "assoc" && (
                <>
                  <p className="mf-metadata-hint">
                    Associe os tipos <code>.md</code>/<code>.markdown</code> ao
                    Markforge (só para o seu usuário — não exige administrador).
                  </p>
                  <ul className="mf-assoc-list">
                    {assocs.map((a) => (
                      <li key={a.ext} className="mf-assoc-item">
                        <span className="mf-assoc-ext">{a.ext}</span>
                        <span
                          className={`mf-assoc-state ${a.associated ? "on" : ""}`}
                        >
                          {stateLabel(a)}
                        </span>
                        <button
                          className={a.associated ? "mf-btn-secondary" : "mf-btn-primary"}
                          disabled={busy}
                          onClick={() => toggleAssoc(a.ext, !a.associated)}
                        >
                          {a.associated ? "Remover" : "Associar"}
                        </button>
                      </li>
                    ))}
                  </ul>
                  <div className="mf-assoc-default">
                    <p className="mf-metadata-hint">
                      No Windows 10/11, definir o Markforge como aplicativo{" "}
                      <strong>padrão</strong> exige uma confirmação sua na tela de
                      Apps padrão do sistema.
                    </p>
                    <button
                      className="mf-btn-secondary"
                      onClick={() => api.openDefaultAppsSettings().catch((e) => setError(String(e)))}
                    >
                      Abrir Apps padrão do Windows…
                    </button>
                  </div>
                </>
              )}

              {tab === "context" && (
                <>
                  <p className="mf-metadata-hint">
                    Escolha quais atalhos aparecem ao clicar com o botão direito.
                  </p>
                  <label className="mf-checkbox-row">
                    <input
                      type="checkbox"
                      checked={ctx.files}
                      disabled={busy}
                      onChange={(e) => updateContext({ ...ctx, files: e.target.checked })}
                    />
                    <span>“Abrir com Markforge” em arquivos .md / .markdown</span>
                  </label>
                  <label className="mf-checkbox-row">
                    <input
                      type="checkbox"
                      checked={ctx.folders}
                      disabled={busy}
                      onChange={(e) => updateContext({ ...ctx, folders: e.target.checked })}
                    />
                    <span>“Abrir pasta no Markforge” em pastas</span>
                  </label>
                  <p className="mf-metadata-hint">
                    As mudanças no menu de contexto podem levar alguns segundos
                    para o Windows Explorer refletir.
                  </p>
                </>
              )}
            </div>
          </>
        )}

        <div className="mf-modal-footer">
          <button className="mf-btn-secondary" onClick={onClose}>Fechar</button>
        </div>
      </div>
    </div>
  );
}
