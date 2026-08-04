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

describe("Trocar de projeto sem fechar o app", () => {
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

  it("openFolder salva edicao pendente do projeto anterior antes de trocar", async () => {
    api.readFile.mockResolvedValue(`---\ntitle: "T"\n---\n\nConteudo original`);
    api.writeFile.mockResolvedValue(undefined);

    await act(async () => {
      await useProjectStore.getState().openSingleFile("/projeto-a/doc.md");
    });
    act(() => {
      useProjectStore.getState().toggleEditMode();
      useProjectStore.getState().updateBody("Conteudo editado, nao salvo ainda");
    });
    expect(useProjectStore.getState().openDoc?.dirty).toBe(true);

    api.listMarkdownTree.mockResolvedValue({
      name: "projeto-b",
      path: "/projeto-b",
      is_dir: true,
      children: [],
    });

    await act(async () => {
      await useProjectStore.getState().openFolder("/projeto-b");
    });

    expect(api.writeFile).toHaveBeenCalledWith(
      "/projeto-a/doc.md",
      expect.stringContaining("Conteudo editado, nao salvo ainda")
    );
    expect(useProjectStore.getState().rootPath).toBe("/projeto-b");
    expect(useProjectStore.getState().openDoc).toBeNull();
  });

  it("openFolder limpa o documento aberto do projeto anterior mesmo sem edicao pendente", async () => {
    api.readFile.mockResolvedValue(`---\ntitle: "T"\n---\n\nConteudo`);

    await act(async () => {
      await useProjectStore.getState().openSingleFile("/projeto-a/doc.md");
    });
    expect(useProjectStore.getState().openDoc).not.toBeNull();

    api.listMarkdownTree.mockResolvedValue({
      name: "projeto-b",
      path: "/projeto-b",
      is_dir: true,
      children: [],
    });

    await act(async () => {
      await useProjectStore.getState().openFolder("/projeto-b");
    });

    expect(useProjectStore.getState().openDoc).toBeNull();
    expect(useProjectStore.getState().tree?.path).toBe("/projeto-b");
  });

  it("abrir pasta e abrir arquivo avulso registram entrada em recentes", async () => {
    api.listMarkdownTree.mockResolvedValue({
      name: "projeto-b",
      path: "/projeto-b",
      is_dir: true,
      children: [],
    });
    await act(async () => {
      await useProjectStore.getState().openFolder("/projeto-b");
    });
    expect(useProjectStore.getState().recents[0]).toEqual({
      path: "/projeto-b",
      kind: "folder",
      label: "projeto-b",
    });

    api.readFile.mockResolvedValue(`---\ntitle: "T"\n---\n\nConteudo`);
    await act(async () => {
      await useProjectStore.getState().openSingleFile("/outra-pasta/doc.md");
    });
    expect(useProjectStore.getState().recents[0]).toEqual({
      path: "/outra-pasta/doc.md",
      kind: "file",
      label: "doc.md",
    });
    // a entrada anterior (pasta) continua na lista, não foi substituída
    expect(useProjectStore.getState().recents.some((r) => r.path === "/projeto-b")).toBe(true);
  });
});
