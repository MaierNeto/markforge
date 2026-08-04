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

describe("Salvar como (mover o documento para outra pasta)", () => {
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

  it("autoriza a pasta de destino antes de gravar, mesmo fora das pastas ja abertas", async () => {
    api.readFile.mockResolvedValue(`---\ntitle: "T"\n---\n\nConteudo`);
    api.writeFile.mockResolvedValue(undefined);

    await act(async () => {
      await useProjectStore.getState().openSingleFile("/projeto-a/doc.md");
    });

    await act(async () => {
      await useProjectStore.getState().saveAs("/outra-pasta-nunca-aberta/copia.md");
    });

    expect(api.allowFile).toHaveBeenCalledWith("/outra-pasta-nunca-aberta/copia.md");
    const allowOrder = api.allowFile.mock.invocationCallOrder[0];
    const writeOrder = api.writeFile.mock.invocationCallOrder[
      api.writeFile.mock.calls.findIndex((c) => c[0] === "/outra-pasta-nunca-aberta/copia.md")
    ];
    expect(allowOrder).toBeLessThan(writeOrder);

    const doc = useProjectStore.getState().openDoc;
    expect(doc?.path).toBe("/outra-pasta-nunca-aberta/copia.md");
    expect(doc?.dirty).toBe(false);
  });
});
