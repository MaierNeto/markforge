import { useEffect, useRef, useState } from "react";
import { Crepe } from "@milkdown/crepe";
import "@milkdown/crepe/theme/common/style.css";
import "@milkdown/crepe/theme/frame.css";
import "@/styles/editor-overrides.css";

/**
 * Página DEV isolada para testes Playwright do round-trip Marp.
 * Só disponível em `import.meta.env.DEV` — não entra no build de produção.
 */
export function DevTestPage() {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const crepeRef = useRef<Crepe | null>(null);
  const [ready, setReady] = useState(false);

  useEffect(() => {
    if (!containerRef.current) return;
    let disposed = false;

    const crepe = new Crepe({
      root: containerRef.current,
      defaultValue: "",
    });

    crepe.create().then(() => {
      if (!disposed) {
        crepeRef.current = crepe;
        setReady(true);
      }
    });

    return () => {
      disposed = true;
      crepe.destroy();
      crepeRef.current = null;
      setReady(false);
    };
  }, []);

  // Expõe no window só para o Playwright (DEV)
  useEffect(() => {
    if (import.meta.env.DEV && crepeRef.current) {
      (window as any).__MILKDOWN_CREPE__ = crepeRef.current;
    }
    return () => {
      if (import.meta.env.DEV) {
        delete (window as any).__MILKDOWN_CREPE__;
      }
    };
  }, [crepeRef.current]);

  return (
    <div style={{ padding: "20px", fontFamily: "system-ui, sans-serif" }}>
      <h1 style={{ marginBottom: "16px" }}>MarkForge — Dev Test Harness (Marp Round-trip)</h1>
      <p style={{ color: "#666", marginBottom: "24px" }}>
        Página exclusiva para testes E2E. Não disponível em produção.
      </p>
      <div
        className="mf-editor"
        ref={containerRef}
        style={{
          minHeight: "400px",
          border: "1px solid #ddd",
          borderRadius: "4px",
          background: "#fff",
        }}
      />
      <div style={{ marginTop: "16px", padding: "12px", background: "#f5f5f5", borderRadius: "4px", fontSize: "14px" }}>
        <strong>Status:</strong> {ready ? "✅ Editor pronto" : "⏳ Carregando..."}
      </div>
    </div>
  );
}