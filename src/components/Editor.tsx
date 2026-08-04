import { useEffect, useRef } from "react";
import { Crepe } from "@milkdown/crepe";
import "@milkdown/crepe/theme/common/style.css";
import "@milkdown/crepe/theme/frame.css";
import "@/styles/editor-overrides.css";
import { useProjectStore } from "@/store/projectStore";

interface EditorProps {
  /** chave única do documento (path do arquivo); trocar remonta o editor */
  docKey: string;
  defaultValue: string;
  onChange: (markdown: string) => void;
}

export function Editor({ docKey, defaultValue, onChange }: EditorProps) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const crepeRef = useRef<Crepe | null>(null);
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;

  // Modo de leitura vem do store. Fica numa ref porque o listener do editor
  // (registrado uma vez, na criação) precisa sempre ler o valor mais recente
  // — remontar o editor inteiro a cada troca de modo destruía a instância e
  // recriava outra no mesmo container, perdendo edições em andamento.
  const isReading = useProjectStore((s) => s.openDoc?.mode === "reading");
  const isReadingRef = useRef(isReading);
  isReadingRef.current = isReading;

  useEffect(() => {
    if (!containerRef.current) return;
    let disposed = false;

    const crepe = new Crepe({
      root: containerRef.current,
      defaultValue,
      featureConfigs: {
        placeholder: {
          text: isReadingRef.current
            ? "Modo leitura — clique em 'Editar' para modificar"
            : "Comece a escrever, ou digite \"/\" para ver comandos…",
        },
      },
    });

    crepe.on((listener) => {
      listener.markdownUpdated((_ctx, markdown) => {
        if (!disposed && !isReadingRef.current) onChangeRef.current(markdown);
      });
    });

    crepe.create().then(() => {
      if (disposed) {
        crepe.destroy();
      } else {
        crepe.setReadonly(isReadingRef.current);
        crepeRef.current = crepe;
      }
    });

    return () => {
      disposed = true;
      crepe.destroy();
      crepeRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [docKey]);

  // Alterna somente o modo do editor já existente — sem destruir/recriar.
  useEffect(() => {
    crepeRef.current?.setReadonly(isReading);
  }, [isReading]);

  return <div className="mf-editor" ref={containerRef} />;
}