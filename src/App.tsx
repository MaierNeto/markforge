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
import "@/styles/app.css";

export default function App() {
  const rootPath = useProjectStore((s) => s.rootPath);
  const openDoc = useProjectStore((s) => s.openDoc);
  const updateBody = useProjectStore((s) => s.updateBody);
  const openSingleFile = useProjectStore((s) => s.openSingleFile);
  const [exportOpen, setExportOpen] = useState(false);
  const [templatesOpen, setTemplatesOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);

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
    </div>
  );
}
