import { useState } from "react";

interface PromptDialogProps {
  title: string;
  label?: string;
  defaultValue?: string;
  confirmLabel?: string;
  onConfirm: (value: string) => void;
  onCancel: () => void;
}

/**
 * Substitui window.prompt(): o nativo do navegador mostra a origem interna
 * do webview ("tauri.localhost diz") em vez do nome do Markforge.
 */
export function PromptDialog({
  title,
  label,
  defaultValue = "",
  confirmLabel = "OK",
  onConfirm,
  onCancel,
}: PromptDialogProps) {
  const [value, setValue] = useState(defaultValue);

  function handleConfirm() {
    const trimmed = value.trim();
    if (trimmed) onConfirm(trimmed);
  }

  return (
    <div className="mf-modal-backdrop" onClick={onCancel}>
      <div className="mf-modal mf-modal--sm" onClick={(e) => e.stopPropagation()}>
        <div className="mf-modal-header">
          <h2>{title}</h2>
        </div>
        <div className="mf-modal-body">
          <label className="mf-field">
            {label}
            <input
              autoFocus
              value={value}
              onChange={(e) => setValue(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") handleConfirm();
                if (e.key === "Escape") onCancel();
              }}
            />
          </label>
        </div>
        <div className="mf-modal-footer">
          <button className="mf-btn-secondary" onClick={onCancel}>
            Cancelar
          </button>
          <button className="mf-btn-primary" onClick={handleConfirm}>
            {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
