import { open } from "@tauri-apps/plugin-dialog";
import { useProjectStore } from "@/store/projectStore";

export function WelcomeScreen() {
  const openFolder = useProjectStore((s) => s.openFolder);
  const openSingleFile = useProjectStore((s) => s.openSingleFile);

  async function handleOpenFolder() {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === "string") {
      await openFolder(selected);
    }
  }

  async function handleOpenFile() {
    const selected = await open({
      multiple: false,
      filters: [{ name: "Markdown", extensions: ["md", "markdown"] }],
    });
    if (typeof selected === "string") {
      await openSingleFile(selected);
    }
  }

  return (
    <div className="mf-welcome">
      <div className="mf-welcome-card">
        <div className="mf-welcome-mark">M</div>
        <h1>Markforge</h1>
        <p>
          Edite visualmente os arquivos <code>.md</code> do seu projeto e exporte
          documentos DOCX/PDF prontos, com capa, cabeçalho e rodapé.
        </p>
        <div className="mf-welcome-actions">
          <button className="mf-btn-primary" onClick={handleOpenFolder}>
            Abrir pasta do projeto
          </button>
          <button className="mf-btn-secondary" onClick={handleOpenFile}>
            Abrir arquivo .md
          </button>
        </div>
        <p className="mf-welcome-hint">
          Ideal para pastas de controle de projetos com agentes de IA — abra a raiz
          do repositório e edite os arquivos <code>.md</code> com uma interface
          visual, sem perder a compatibilidade com o texto puro.
        </p>
      </div>
    </div>
  );
}
