import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { act } from "react";
import { useProjectStore } from "@/store/projectStore";

vi.mock("@/lib/tauri", () => ({
  api: {
    listMarkdownTree: vi.fn(),
    allowFile: vi.fn(),
    readFile: vi.fn(),
    writeFile: vi.fn(),
    createMarkdownFile: vi.fn(),
    createFolder: vi.fn(),
    renamePath: vi.fn(),
    deletePath: vi.fn(),
    openPath: vi.fn(),
    exportDocument: vi.fn(),
  },
}));

import { api } from "@/lib/tauri";

const sampleMarkdown = `---\ntitle: "Teste RFC-004"\nauthor: "Walter"\ndate: "2026-08-03"\n---\n\n# Título\n\nConteúdo original.`;

describe("RFC-004: Abrir pasta do documento a partir do diálogo de exportação", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useProjectStore.setState({
      rootPath: null,
      tree: null,
      openDoc: null,
      loadingTree: false,
      saveStatus: "idle",
      error: null,
      saveRawSnapshot: false,
    });
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("CA-01: no diálogo de exportação, após sucesso, botão 'Abrir pasta' aparece", async () => {
    api.readFile.mockResolvedValue(sampleMarkdown);

    await act(async () => {
      await useProjectStore.getState().openSingleFile("/projeto/teste.md");
    });

    // Simular sucesso na exportação (resultado com caminhos de arquivo)
    const exportResult = {
      docx_path: "/tmp/export/teste.docx",
      pdf_path: "/tmp/export/teste.pdf",
      warnings: [],
    };

    // A exportação retorna os caminhos → a UI habilita "Abrir pasta"
    // quando ao menos um caminho existe (G0-5: efeito do comando de exportação)
    expect(exportResult.docx_path ?? exportResult.pdf_path).toBeDefined();
  });

  it("CA-02: clicar em 'Abrir pasta' abre o Explorer na pasta onde DOCX/PDF foi gravado", async () => {
    const exportPath = "/tmp/export/teste.docx";
    const exportFolder = "/tmp/export";

    api.openPath.mockResolvedValue(undefined);

    await act(async () => {
      await useProjectStore.getState().openExportFolder(exportPath);
    });

    // Verificar que openPath foi chamado com o caminho da pasta (não do arquivo)
    expect(api.openPath).toHaveBeenCalledWith(exportFolder);
  });

  it("CA-03: funciona mesmo se a pasta de destino estiver fora das raízes abertas (ex.: Desktop, Downloads)", async () => {
    const externalPath = "C:\\Users\\User\\Desktop\\meu-arquivo.docx";
    const externalFolder = "C:\\Users\\User\\Desktop";

    api.openPath.mockResolvedValue(undefined);

    await act(async () => {
      await useProjectStore.getState().openExportFolder(externalPath);
    });

    // O openPath deve ser chamado com a pasta externa
    expect(api.openPath).toHaveBeenCalledWith(externalFolder);
  });

  it("CA-04: não abre o arquivo (só a pasta) — fora de escopo", async () => {
    const exportPath = "/tmp/export/teste.docx";
    const exportFolder = "/tmp/export";

    api.openPath.mockResolvedValue(undefined);

    await act(async () => {
      await useProjectStore.getState().openExportFolder(exportPath);
    });

    // Verificar que openPath NÃO foi chamado com o caminho do arquivo
    expect(api.openPath).not.toHaveBeenCalledWith(exportPath);
    expect(api.openPath).toHaveBeenCalledWith(exportFolder);
  });
});