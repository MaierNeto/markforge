import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "@/lib/tauri";
import { useProjectStore } from "@/store/projectStore";
import { WelcomeScreen } from "@/components/WelcomeScreen";
import { FileTree } from "@/components/FileTree";
import { Editor } from "@/components/Editor";
import { MetadataPanel } from "@/components/MetadataPanel";
import { TopBar } from "@/components/TopBar";
import { ExportDialog } from "@/components/ExportDialog";
import { TemplateManager } from "@/components/TemplateManager";
import { SettingsDialog } from "@/components/SettingsDialog";
import { ExternalChangeBanner } from "@/components/ExternalChangeBanner";
import { CloseConfirmDialog } from "@/components/CloseConfirmDialog";
import "@/styles/app.css";

export default function App() {
  const rootPath = useProjectStore((s) => s.rootPath);
  const openDoc = useProjectStore((s) => s.openDoc);
  const updateBody = useProjectStore((s) => s.updateBody);
  const openSingleFile = useProjectStore((s) => s.openSingleFile);
  const checkExternalChange = useProjectStore((s) => s.checkExternalChange);
  const handleCloseRequested = useProjectStore((s) => s.handleCloseRequested);
  const [exportOpen, setExportOpen] = useState(false);
  const [templatesOpen, setTemplatesOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [closeConfirmOpen, setCloseConfirmOpen] = useState(false);

  // Abre o arquivo passado ao iniciar (associação de .md) e escuta pedidos de
  // abertura vindos de uma segunda instância (duplo-clique com o app já aberto).
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    api
      .takeStartupFile()
      .then((path) => {
        if (path) openSingleFile(path);
      })
      .catch(() => {});
    listen<string>("open-file", (event) => {
      if (event.payload) openSingleFile(event.payload);
    }).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, [openSingleFile]);

  // RFC-002 (G0-3): detecta mudança externa ao retomar o foco da janela.
  useEffect(() => {
    const onFocus = () => {
      checkExternalChange().catch(() => {});
    };
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, [checkExternalChange]);

  // RFC-005: pedido de fechamento vindo do backend (CloseRequested interceptado).
  // Decide: se há edição não salva → mostra o diálogo; senão → sai direto.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    const handleClose = async () => {
      const verdict = await handleCloseRequested();
      if (verdict === "close") {
        await api.quitApp();
      } else {
        setCloseConfirmOpen(true);
      }
    };
    listen("close-requested", handleClose).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, [handleCloseRequested]);

  if (!rootPath && !openDoc) {
    return (
      <>
        <WelcomeScreen onOpenSettings={() => setSettingsOpen(true)} />
        {settingsOpen && <SettingsDialog onClose={() => setSettingsOpen(false)} />}
      </>
    );
  }

  return (
    <div className="mf-shell">
      <TopBar
        onExport={() => setExportOpen(true)}
        onManageTemplates={() => setTemplatesOpen(true)}
        onOpenSettings={() => setSettingsOpen(true)}
      />
      <div className="mf-body">
        <aside className="mf-sidebar">
          <FileTree />
        </aside>
        <main className="mf-main">
          {openDoc ? (
            <>
              <ExternalChangeBanner />
              <MetadataPanel />
              <Editor
                docKey={openDoc.path}
                defaultValue={openDoc.body}
                onChange={updateBody}
              />
            </>
          ) : (
            <div className="mf-no-doc">Selecione um arquivo .md na barra lateral para editar.</div>
          )}
        </main>
      </div>
      {exportOpen && <ExportDialog onClose={() => setExportOpen(false)} />}
      {templatesOpen && <TemplateManager onClose={() => setTemplatesOpen(false)} />}
      {settingsOpen && <SettingsDialog onClose={() => setSettingsOpen(false)} />}
      {closeConfirmOpen && <CloseConfirmDialog onCancel={() => setCloseConfirmOpen(false)} />}
    </div>
  );
}
