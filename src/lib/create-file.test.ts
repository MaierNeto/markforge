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
  },
}));

import { api } from "@/lib/tauri";

describe("Criar o primeiro arquivo de um projeto vazio", () => {
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

  it("createFile abre o arquivo recem-criado, mesmo com a arvore vazia", async () => {
    api.listMarkdownTree.mockResolvedValue({
      name: "projeto-novo",
      path: "/projeto-novo",
      is_dir: true,
      children: [],
    });
    await act(async () => {
      await useProjectStore.getState().openFolder("/projeto-novo");
    });
    expect(useProjectStore.getState().tree?.children).toHaveLength(0);

    api.createMarkdownFile.mockResolvedValue("/projeto-novo/novo-documento.md");
    api.readFile.mockResolvedValue("# Novo documento\n\n");
    api.listMarkdownTree.mockResolvedValue({
      name: "projeto-novo",
      path: "/projeto-novo",
      is_dir: true,
      children: [{ name: "novo-documento.md", path: "/projeto-novo/novo-documento.md", is_dir: false }],
    });

    await act(async () => {
      await useProjectStore.getState().createFile("/projeto-novo", "novo-documento.md");
    });

    expect(useProjectStore.getState().openDoc?.path).toBe("/projeto-novo/novo-documento.md");
    expect(useProjectStore.getState().tree?.children).toHaveLength(1);
  });
});
