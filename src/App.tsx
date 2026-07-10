import { useState } from "react";
import { useProjectStore } from "@/store/projectStore";
import { WelcomeScreen } from "@/components/WelcomeScreen";
import { FileTree } from "@/components/FileTree";
import { Editor } from "@/components/Editor";
import { MetadataPanel } from "@/components/MetadataPanel";
import { TopBar } from "@/components/TopBar";
import { ExportDialog } from "@/components/ExportDialog";
import { TemplateManager } from "@/components/TemplateManager";
import "@/styles/app.css";

export default function App() {
  const rootPath = useProjectStore((s) => s.rootPath);
  const openDoc = useProjectStore((s) => s.openDoc);
  const updateBody = useProjectStore((s) => s.updateBody);
  const [exportOpen, setExportOpen] = useState(false);
  const [templatesOpen, setTemplatesOpen] = useState(false);

  if (!rootPath) {
    return <WelcomeScreen />;
  }

  return (
    <div className="mf-shell">
      <TopBar onExport={() => setExportOpen(true)} onManageTemplates={() => setTemplatesOpen(true)} />
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
    </div>
  );
}
