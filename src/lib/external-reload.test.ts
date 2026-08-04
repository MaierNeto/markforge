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

describe("RFC-002: Reload de arquivos (mudança externa no disco)", () => {
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

  it("CA-01: arquivo limpo + mudança externa → oferece 'Recarregar' em 1 passo", async () => {
    api.readFile.mockResolvedValue(`---\ntitle: "Teste"\nauthor: "Walter"\ndate: "2026-08-03"\n---\n\n# Título\n\nConteúdo original.`);

    await act(async () => {
      await useProjectStore.getState().openSingleFile("/projeto/teste.md");
    });

    expect(useProjectStore.getState().openDoc?.mode).toBe("reading");
    expect(useProjectStore.getState().openDoc?.dirty).toBe(false);
    expect(useProjectStore.getState().openDoc?.hashDisco).toBeDefined();

    api.readFile.mockResolvedValue(`---\ntitle: "Teste Modificado"\nauthor: "Walter"\ndate: "2026-08-03"\n---\n\n# Título\n\nConteúdo modificado.`);

    const hasExternalChange = await useProjectStore.getState().checkExternalChange();
    expect(hasExternalChange).toBe(true);

    expect(useProjectStore.getState().openDoc?.externalContent).toBeDefined();
  });

  it("CA-02: arquivo sujo localmente + mudança externa disjunta → mescla automaticamente", async () => {
    const original = `---\ntitle: "Teste"\nauthor: "Walter"\ndate: "2026-08-03"\n---\n\nLinha 1\nLinha 2\nLinha 3`;
    api.readFile.mockResolvedValue(original);

    await act(async () => {
      await useProjectStore.getState().openSingleFile("/projeto/teste.md");
    });

    act(() => {
      useProjectStore.getState().toggleEditMode();
      useProjectStore.getState().updateBody("Linha 1\nLinha 2\nLinha 3\nLinha local");
    });

    expect(useProjectStore.getState().openDoc?.dirty).toBe(true);

    const externalRaw = `---\ntitle: "Teste"\nauthor: "Walter"\ndate: "2026-08-03"\n---\n\nLinha 1\nLinha 2\nLinha 3\nLinha externa`;
    api.readFile.mockResolvedValue(externalRaw);

    const hasExternalChange = await useProjectStore.getState().checkExternalChange();
    expect(hasExternalChange).toBe(true);

    const doc = useProjectStore.getState().openDoc;
    expect(doc?.dirty).toBe(true);
    expect(doc?.body).toContain("Linha externa");
    expect(doc?.body).toContain("Linha local");
    expect(doc?.externalContent).toBeUndefined();
  });
});