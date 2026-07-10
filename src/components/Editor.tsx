import { useEffect, useRef } from "react";
import { Crepe } from "@milkdown/crepe";
import "@milkdown/crepe/theme/common/style.css";
import "@milkdown/crepe/theme/frame.css";
import "@/styles/editor-overrides.css";

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

  useEffect(() => {
    if (!containerRef.current) return;
    let disposed = false;

    const crepe = new Crepe({
      root: containerRef.current,
      defaultValue,
      featureConfigs: {
        placeholder: {
          text: "Comece a escrever, ou digite \"/\" para ver comandos…",
        },
      },
    });

    crepe.on((listener) => {
      listener.markdownUpdated((_ctx, markdown) => {
        if (!disposed) onChangeRef.current(markdown);
      });
    });

    crepe.create().then(() => {
      if (disposed) {
        crepe.destroy();
      } else {
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

  return <div className="mf-editor" ref={containerRef} />;
}
