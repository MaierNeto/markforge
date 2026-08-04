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

const sampleMarkdown = `---\ntitle: "Teste RFC-005"\nauthor: "Walter"\ndate: "2026-08-03"\n---\n\n# Título\n\nConteúdo original.`;

describe("RFC-005: Salvamento garantido ao fechar a janela", () => {
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

  // CA-01: Fechar com sujo → o handler decide "show-dialog" (Sim/Não/Cancelar)
  it("CA-01: fechar com sujo → decide mostrar diálogo (Sim/Não/Cancelar)", async () => {
    api.readFile.mockResolvedValue(sampleMarkdown);

    await act(async () => {
      await useProjectStore.getState().openSingleFile("/projeto/teste.md");
    });

    // Editar para deixar sujo
    act(() => {
      useProjectStore.getState().toggleEditMode();
      useProjectStore.getState().updateBody("# Título\n\nConteúdo editado.");
    });

    const state = useProjectStore.getState();
    expect(state.openDoc?.dirty).toBe(true);
    expect(state.hasUnsavedChanges()).toBe(true);

    // O pedido de fechamento com edição não salva → mostra diálogo
    const verdict = await state.handleCloseRequested();
    expect(verdict).toBe("show-dialog");
  });

  // CA-02: Sim → salva + fecha
  it("CA-02: Sim → salva + fecha", async () => {
    api.readFile.mockResolvedValue(sampleMarkdown);

    await act(async () => {
      await useProjectStore.getState().openSingleFile("/projeto/teste.md");
    });

    act(() => {
      useProjectStore.getState().toggleEditMode();
      useProjectStore.getState().updateBody("# Título\n\nConteúdo editado.");
    });

    api.writeFile.mockResolvedValue(undefined);

    const verdict = await useProjectStore.getState().resolveClose("save");
    expect(verdict).toBe("close");
    // writeFile foi chamado com o conteúdo atualizado no caminho original
    expect(api.writeFile).toHaveBeenCalled();
    const writtenPath = api.writeFile.mock.calls[0][0];
    expect(writtenPath).toBe("/projeto/teste.md");
  });

  // CA-03: Não → descarta e fecha (sem salvar)
  it("CA-03: Não → descarta e fecha (sem salvar)", async () => {
    api.readFile.mockResolvedValue(sampleMarkdown);

    await act(async () => {
      await useProjectStore.getState().openSingleFile("/projeto/teste.md");
    });

    act(() => {
      useProjectStore.getState().toggleEditMode();
      useProjectStore.getState().updateBody("# Título\n\nConteúdo editado.");
    });

    api.writeFile.mockResolvedValue(undefined);

    const verdict = await useProjectStore.getState().resolveClose("discard");
    expect(verdict).toBe("close");
    // Não deve ter chamado writeFile
    expect(api.writeFile).not.toHaveBeenCalled();
  });

  // CA-04: Cancelar → aborta fechamento, app permanece aberto
  it("CA-04: Cancelar → aborta fechamento, app permanece aberto", async () => {
    api.readFile.mockResolvedValue(sampleMarkdown);

    await act(async () => {
      await useProjectStore.getState().openSingleFile("/projeto/teste.md");
    });

    act(() => {
      useProjectStore.getState().toggleEditMode();
      useProjectStore.getState().updateBody("# Título\n\nConteúdo editado.");
    });

    api.writeFile.mockResolvedValue(undefined);

    const verdict = await useProjectStore.getState().resolveClose("cancel");
    expect(verdict).toBe("stay");
    // Não salvou nem fechou
    expect(api.writeFile).not.toHaveBeenCalled();
    const state = useProjectStore.getState();
    expect(state.openDoc?.dirty).toBe(true); // edição preservada
  });

  // CA-05: Fechar com limpo (ou leitura sem edição) → fecha silencioso, sem diálogo
  it("CA-05: fechar com limpo → fecha silencioso (sem diálogo)", async () => {
    api.readFile.mockResolvedValue(sampleMarkdown);

    await act(async () => {
      await useProjectStore.getState().openSingleFile("/projeto/teste.md");
    });

    const state = useProjectStore.getState();
    expect(state.openDoc?.dirty).toBe(false);
    expect(state.openDoc?.mode).toBe("reading");
    expect(state.hasUnsavedChanges()).toBe(false);

    const verdict = await state.handleCloseRequested();
    expect(verdict).toBe("close");
  });

  // CA-06: Não depende do timer de autosave (autosave nunca disparou, doc sujo)
  it("CA-06: não depende do timer de autosave", async () => {
    api.readFile.mockResolvedValue(sampleMarkdown);

    await act(async () => {
      await useProjectStore.getState().openSingleFile("/projeto/teste.md");
    });

    act(() => {
      useProjectStore.getState().toggleEditMode();
      useProjectStore.getState().updateBody("# Título\n\nConteúdo editado.");
    });

    // Autosave nunca rodou (saveStatus ainda "idle"): doc continua sujo
    const state = useProjectStore.getState();
    expect(state.saveStatus).toBe("idle");
    expect(state.hasUnsavedChanges()).toBe(true);

    // Mesmo assim, fechar exige decisão — e "discard" fecha sem esperar o timer
    const verdict = await state.handleCloseRequested();
    expect(verdict).toBe("show-dialog");
    const closeVerdict = await state.resolveClose("discard");
    expect(closeVerdict).toBe("close");
    expect(api.writeFile).not.toHaveBeenCalled();
  });
});