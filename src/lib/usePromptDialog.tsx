import { useState } from "react";
import { PromptDialog } from "@/components/PromptDialog";

interface PromptRequest {
  title: string;
  label?: string;
  defaultValue: string;
  confirmLabel?: string;
  resolve: (value: string | null) => void;
}

/**
 * Versão assíncrona de window.prompt() usando PromptDialog (mesma razão:
 * evitar o diálogo nativo do navegador, que expõe "tauri.localhost").
 */
export function usePromptDialog() {
  const [request, setRequest] = useState<PromptRequest | null>(null);

  function promptText(
    title: string,
    defaultValue = "",
    opts?: { label?: string; confirmLabel?: string }
  ): Promise<string | null> {
    return new Promise((resolve) => {
      setRequest({ title, defaultValue, label: opts?.label, confirmLabel: opts?.confirmLabel, resolve });
    });
  }

  const promptDialog = request ? (
    <PromptDialog
      title={request.title}
      label={request.label}
      defaultValue={request.defaultValue}
      confirmLabel={request.confirmLabel}
      onConfirm={(value) => {
        request.resolve(value);
        setRequest(null);
      }}
      onCancel={() => {
        request.resolve(null);
        setRequest(null);
      }}
    />
  ) : null;

  return { promptText, promptDialog };
}
