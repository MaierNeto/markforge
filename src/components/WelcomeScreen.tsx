import { open } from "@tauri-apps/plugin-dialog";
import { useProjectStore } from "@/store/projectStore";

export function WelcomeScreen() {
  const openFolder = useProjectStore((s) => s.openFolder);

  async function handleOpen() {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === "string") {
      await openFolder(selected);
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
        <button className="mf-btn-primary" onClick={handleOpen}>
          Abrir pasta do projeto
        </button>
        <p className="mf-welcome-hint">
          Ideal para pastas de controle de projetos com agentes de IA — abra a raiz
          do repositório e edite os arquivos <code>.md</code> com uma interface
          visual, sem perder a compatibilidade com o texto puro.
        </p>
      </div>
    </div>
  );
}
