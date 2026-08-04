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

describe("Corrida de save: edição durante um save em andamento não pode ser perdida", () => {
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

  it("segunda edição chegando durante o writeFile do primeiro save continua suja até ser persistida", async () => {
    api.readFile.mockResolvedValue(
      `---\ntitle: "Teste"\n---\n\nLinha 1\nLinha 2\nLinha 3`
    );

    await act(async () => {
      await useProjectStore.getState().openSingleFile("/projeto/teste.md");
    });

    act(() => {
      useProjectStore.getState().toggleEditMode();
    });

    const writtenContents: string[] = [];
    let resolveFirstWrite: () => void = () => {};
    let firstWriteStarted = false;

    api.writeFile.mockImplementation((_path: string, content: string) => {
      writtenContents.push(content);
      if (!firstWriteStarted) {
        firstWriteStarted = true;
        return new Promise<void>((resolve) => {
          resolveFirstWrite = resolve;
        });
      }
      return Promise.resolve();
    });

    // Primeira edição: exclui "Linha 3" e dispara o save (simulando o timer).
    act(() => {
      useProjectStore.getState().updateBody("Linha 1\nLinha 2");
    });
    const firstSave = useProjectStore.getState().saveCurrentFile();

    // Enquanto o primeiro writeFile ainda está pendente, chega outra edição
    // (o usuário excluiu mais uma linha antes do save anterior terminar).
    act(() => {
      useProjectStore.getState().updateBody("Linha 1");
    });

    // O primeiro write finalmente resolve.
    resolveFirstWrite();
    await firstSave;

    // A edição mais recente ainda não foi escrita em disco — o documento
    // precisa continuar "sujo", senão o próximo save (já agendado) nunca roda
    // e a exclusão mais recente se perde silenciosamente.
    const doc = useProjectStore.getState().openDoc;
    expect(doc?.body).toBe("Linha 1");
    expect(doc?.dirty).toBe(true);

    // Deixando o save (já agendado pela segunda updateBody) rodar de verdade:
    await act(async () => {
      await useProjectStore.getState().saveCurrentFile();
    });

    expect(useProjectStore.getState().openDoc?.dirty).toBe(false);
    expect(writtenContents[writtenContents.length - 1]).toContain("Linha 1");
    expect(writtenContents[writtenContents.length - 1]).not.toContain("Linha 2");
  });
});
