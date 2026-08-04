import { useState } from "react";
import { useProjectStore } from "@/store/projectStore";
import { api } from "@/lib/tauri";

interface CloseConfirmDialogProps {
  onCancel: () => void;
}

/**
 * RFC-005 — "Salvar alterações antes de fechar?" (G0-2).
 *
 * Aparece quando o usuário pede para fechar a janela com edição não salva:
 * - **Sim** → salva a edição pendente e fecha;
 * - **Não** → descarta a edição pendente e fecha;
 * - **Cancelar** → aborta o fechamento, app permanece aberto.
 *
 * O fechamento real só acontece via comando Rust `quit_app` (que sai sem
 * re-disparar o handler de CloseRequested — evita loop).
 */
export function CloseConfirmDialog({ onCancel }: CloseConfirmDialogProps) {
  const resolveClose = useProjectStore((s) => s.resolveClose);
  const [busy, setBusy] = useState(false);

  async function handleChoice(choice: "save" | "discard" | "cancel") {
    setBusy(true);
    try {
      const verdict = await resolveClose(choice);
      if (verdict === "close") {
        await api.quitApp();
      } else {
        // "stay" (Cancelar) → apenas fecha o diálogo; a janela permanece.
        onCancel();
      }
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="mf-modal-backdrop" onClick={() => handleChoice("cancel")}>
      <div className="mf-modal mf-modal--sm" onClick={(e) => e.stopPropagation()}>
        <div className="mf-modal-header">
          <h2>Salvar alterações antes de fechar?</h2>
        </div>
        <div className="mf-modal-body">
          <p>
            Você tem edições não salvas neste documento. O que deseja fazer antes
            de fechar a janela?
          </p>
        </div>
        <div className="mf-modal-footer">
          <button className="mf-btn-secondary" disabled={busy} onClick={() => handleChoice("cancel")}>
            Cancelar
          </button>
          <button className="mf-btn-secondary" disabled={busy} onClick={() => handleChoice("discard")}>
            Não
          </button>
          <button className="mf-btn-primary" disabled={busy} onClick={() => handleChoice("save")}>
            Sim
          </button>
        </div>
      </div>
    </div>
  );
}
