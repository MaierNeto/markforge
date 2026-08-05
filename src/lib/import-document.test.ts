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
    importDocument: vi.fn(),
  },
}));

import { api } from "@/lib/tauri";

describe("Importar documento (.docx/.txt) para dentro do projeto aberto", () => {
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
      recents: [],
    });
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("importa .docx para a pasta do projeto aberto (nao a pasta do documento) e atualiza a arvore", async () => {
    api.listMarkdownTree.mockResolvedValue({
      name: "projeto",
      path: "/projeto",
      is_dir: true,
      children: [],
    });
    await act(async () => {
      await useProjectStore.getState().openFolder("/projeto");
    });

    api.importDocument.mockResolvedValue("/projeto/Relatorio.md");
    api.readFile.mockResolvedValue(`---\ntitle: "Relatorio"\n---\n\nConteudo importado`);
    api.listMarkdownTree.mockResolvedValue({
      name: "projeto",
      path: "/projeto",
      is_dir: true,
      children: [{ name: "Relatorio.md", path: "/projeto/Relatorio.md", is_dir: false }],
    });

    let resultPath: string | undefined;
    await act(async () => {
      resultPath = await useProjectStore.getState().importDocumentFile("/downloads/Relatorio.docx", "/projeto");
    });

    expect(api.importDocument).toHaveBeenCalledWith("/downloads/Relatorio.docx", "/projeto");
    expect(resultPath).toBe("/projeto/Relatorio.md");
    expect(useProjectStore.getState().openDoc?.path).toBe("/projeto/Relatorio.md");
    expect(useProjectStore.getState().tree?.children?.[0]?.path).toBe("/projeto/Relatorio.md");
  });

  it("importa .txt do mesmo jeito (a store nao diferencia formato, so repassa o caminho)", async () => {
    api.listMarkdownTree.mockResolvedValue({
      name: "projeto",
      path: "/projeto",
      is_dir: true,
      children: [],
    });
    await act(async () => {
      await useProjectStore.getState().openFolder("/projeto");
    });

    api.importDocument.mockResolvedValue("/projeto/notas.md");
    api.readFile.mockResolvedValue("notas soltas em texto puro");
    api.listMarkdownTree.mockResolvedValue({
      name: "projeto",
      path: "/projeto",
      is_dir: true,
      children: [{ name: "notas.md", path: "/projeto/notas.md", is_dir: false }],
    });

    await act(async () => {
      await useProjectStore.getState().importDocumentFile("/downloads/notas.txt", "/projeto");
    });

    expect(api.importDocument).toHaveBeenCalledWith("/downloads/notas.txt", "/projeto");
    expect(useProjectStore.getState().openDoc?.path).toBe("/projeto/notas.md");
  });
});
