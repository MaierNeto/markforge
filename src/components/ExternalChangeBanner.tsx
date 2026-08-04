import { useState } from "react";
import { confirm } from "@tauri-apps/plugin-dialog";
import { useProjectStore } from "@/store/projectStore";
import { usePromptDialog } from "@/lib/usePromptDialog";

/**
 * RFC-002 — Aviso de mudança externa no disco.
 *
 * Aparece quando o arquivo aberto foi alterado por outro programa:
 * - arquivo limpo → "Recarregar" em 1 passo (recarrega o conteúdo externo);
 * - arquivo sujo com conflito → 3 opções: Descartar local / Salvar por cima /
 *   Salvar como (o merge automático de diffs disjuntos acontece na store, sem banner).
 */
export function ExternalChangeBanner() {
  const openDoc = useProjectStore((s) => s.openDoc);
  const checkExternalChange = useProjectStore((s) => s.checkExternalChange);
  const discardLocalAndReload = useProjectStore((s) => s.discardLocalAndReload);
  const saveOverExternal = useProjectStore((s) => s.saveOverExternal);
  const [busy, setBusy] = useState(false);
  const { promptText, promptDialog } = usePromptDialog();

  if (!openDoc) return null;

  const external = openDoc.externalContent;
  const isDirty = openDoc.dirty;
  // Sem mudança externa detectada ainda — oferece o scan manual (foco/retorno).
  if (!external) return null;

  // Arquivo limpo + mudança externa → "Recarregar" em 1 passo.
  if (!isDirty) {
    return (
      <div className="mf-external-banner">
        <span className="mf-external-msg">
          Este arquivo foi alterado por outro programa no disco.
        </span>
        <div className="mf-external-actions">
          <button
            className="mf-btn-primary"
            disabled={busy}
            onClick={async () => {
              setBusy(true);
              await discardLocalAndReload();
              setBusy(false);
            }}
          >
            Recarregar
          </button>
          <button
            className="mf-btn-secondary"
            disabled={busy}
            onClick={() => checkExternalChange()}
          >
            Re-checar
          </button>
        </div>
      </div>
    );
  }

  // Arquivo sujo + conflito → 3 opções de resolução.
  return (
    <div className="mf-external-banner mf-external-banner--conflict">
      <span className="mf-external-msg">
        Você editou este arquivo e ele também foi alterado por outro programa.
        Escolha como resolver:
      </span>
      <div className="mf-external-actions">
        <button
          className="mf-btn-secondary"
          disabled={busy}
          onClick={async () => {
            if (await confirm("Descartar suas alterações locais e recarregar a versão do disco?", { kind: "warning" })) {
              setBusy(true);
              discardLocalAndReload();
              setBusy(false);
            }
          }}
        >
          Descartar local e recarregar
        </button>
        <button
          className="mf-btn-primary"
          disabled={busy}
          onClick={async () => {
            if (await confirm("Salvar suas alterações por cima da versão externa? Um backup .md~ será criado.", { kind: "warning" })) {
              setBusy(true);
              await saveOverExternal();
              setBusy(false);
            }
          }}
        >
          Salvar por cima
        </button>
        <button
          className="mf-btn-secondary"
          disabled={busy}
          onClick={async () => {
            const newPath = await promptText(
              "Salvar como…",
              openDoc.path.replace(/\.md$/i, "-conflito.md"),
              { label: "Salvar como um novo arquivo (mantém a versão externa intacta):" }
            );
            if (newPath) {
              setBusy(true);
              const saveAs = useProjectStore.getState().saveAs;
              await saveAs(newPath);
              setBusy(false);
            }
          }}
        >
          Salvar como…
        </button>
      </div>
      {promptDialog}
    </div>
  );
}
