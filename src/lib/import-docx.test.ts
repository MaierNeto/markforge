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
    importDocx: vi.fn(),
  },
}));

import { api } from "@/lib/tauri";

describe("Importar .docx para dentro do projeto aberto", () => {
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

  it("importa para a pasta do projeto aberto (nao a pasta do .docx) e atualiza a arvore", async () => {
    api.listMarkdownTree.mockResolvedValue({
      name: "projeto",
      path: "/projeto",
      is_dir: true,
      children: [],
    });
    await act(async () => {
      await useProjectStore.getState().openFolder("/projeto");
    });

    api.importDocx.mockResolvedValue("/projeto/Relatorio.md");
    api.readFile.mockResolvedValue(`---\ntitle: "Relatorio"\n---\n\nConteudo importado`);
    api.listMarkdownTree.mockResolvedValue({
      name: "projeto",
      path: "/projeto",
      is_dir: true,
      children: [{ name: "Relatorio.md", path: "/projeto/Relatorio.md", is_dir: false }],
    });

    let resultPath: string | undefined;
    await act(async () => {
      resultPath = await useProjectStore.getState().importDocxFile("/downloads/Relatorio.docx", "/projeto");
    });

    expect(api.importDocx).toHaveBeenCalledWith("/downloads/Relatorio.docx", "/projeto");
    expect(resultPath).toBe("/projeto/Relatorio.md");
    expect(useProjectStore.getState().openDoc?.path).toBe("/projeto/Relatorio.md");
    expect(useProjectStore.getState().tree?.children?.[0]?.path).toBe("/projeto/Relatorio.md");
  });
});
