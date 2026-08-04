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

const sampleMarkdown = `---\ntitle: "Teste RFC-003"\nauthor: "Walter"\ndate: "2026-08-03"\n---\n\n# Título\n\nConteúdo original.`;

describe("RFC-003: Reload de diretório (inclusão/remoção de arquivos)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useProjectStore.setState({
      rootPath: "/projeto",
      tree: {
        name: "projeto",
        path: "/projeto",
        is_dir: true,
        children: [
          {
            name: "teste.md",
            path: "/projeto/teste.md",
            is_dir: false,
            children: undefined,
          },
        ],
      },
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

  it("CA-01: adição externa de .md na raiz atualiza a árvore", async () => {
    api.listMarkdownTree.mockResolvedValueOnce({
      name: "projeto",
      path: "/projeto",
      is_dir: true,
      children: [
        { name: "teste.md", path: "/projeto/teste.md", is_dir: false, children: undefined },
        { name: "novo.md", path: "/projeto/novo.md", is_dir: false, children: undefined },
      ],
    });

    await act(async () => {
      await useProjectStore.getState().refreshTree();
    });

    const state = useProjectStore.getState();
    expect(state.tree?.children?.length).toBe(2);
    expect(state.tree?.children?.find((c) => c.name === "novo.md")).toBeDefined();
  });

  it("CA-02: remoção externa de .md na raiz remove da árvore; se arquivo estava aberto, avisa (RFC-002)", async () => {
    // Abrir o arquivo que será removido
    api.readFile.mockResolvedValue(sampleMarkdown);
    await act(async () => {
      await useProjectStore.getState().openSingleFile("/projeto/teste.md");
    });

    // Remoção externa: nova árvore sem o arquivo aberto
    api.listMarkdownTree.mockResolvedValueOnce({
      name: "projeto",
      path: "/projeto",
      is_dir: true,
      children: [],
    });

    await act(async () => {
      await useProjectStore.getState().refreshTree();
    });

    const state = useProjectStore.getState();
    expect(state.tree?.children?.length).toBe(0);
    // Aviso de arquivo removido (fluxo RFC-002/003)
    expect(state.error).toContain("removido");
  });

  it("CA-03: arquivo em edição permanece aberto e editável durante reload de diretório", async () => {
    api.readFile.mockResolvedValue(sampleMarkdown);

    await act(async () => {
      await useProjectStore.getState().openSingleFile("/projeto/teste.md");
    });

    act(() => {
      useProjectStore.getState().toggleEditMode();
      useProjectStore.getState().updateBody("# Título\n\nConteúdo editado localmente.");
    });

    api.listMarkdownTree.mockResolvedValueOnce({
      name: "projeto",
      path: "/projeto",
      is_dir: true,
      children: [
        { name: "teste.md", path: "/projeto/teste.md", is_dir: false, children: undefined },
        { name: "novo.md", path: "/projeto/novo.md", is_dir: false, children: undefined },
      ],
    });

    await act(async () => {
      await useProjectStore.getState().refreshTree();
    });

    const state = useProjectStore.getState();
    expect(state.tree?.children?.length).toBe(2);
    expect(state.openDoc?.body).toContain("Conteúdo editado localmente");
    expect(state.openDoc?.mode).toBe("editing");
  });

  it("CA-04: botão 'Recarregar pasta' na lateral força atualização imediata", async () => {
    const mockTree = {
      name: "projeto",
      path: "/projeto",
      is_dir: true,
      children: [{ name: "teste.md", path: "/projeto/teste.md", is_dir: false, children: undefined }],
    };

    api.listMarkdownTree.mockResolvedValueOnce(mockTree);

    await act(async () => {
      await useProjectStore.getState().refreshTree();
    });

    // Simular botão de reload na UI
    api.listMarkdownTree.mockResolvedValueOnce({
      name: "projeto",
      path: "/projeto",
      is_dir: true,
      children: [
        { name: "teste.md", path: "/projeto/teste.md", is_dir: false, children: undefined },
        { name: "novo.md", path: "/projeto/novo.md", is_dir: false, children: undefined },
      ],
    });

    await act(async () => {
      await useProjectStore.getState().refreshTree();
    });

    const state = useProjectStore.getState();
    expect(state.tree?.children?.length).toBe(2);
  });
});