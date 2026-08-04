import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { act } from "react";
import { useProjectStore } from "@/store/projectStore";

// Mock da API Tauri
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

describe("RFC-001: Modo leitura + edição explícita + reverter ao original", () => {
  const sampleMarkdown = `---\ntitle: "Teste RFC-001"\nauthor: "Walter"\ndate: "2026-08-03"\n---\n\n# Título\n\nConteúdo do teste.`;

  beforeEach(() => {
    vi.clearAllMocks();
    useProjectStore.setState({
      rootPath: null,
      tree: null,
      openDoc: null,
      loadingTree: false,
      saveStatus: "idle",
      error: null,
    });
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  // CA-01: Ao abrir .md, editor está em modo leitura; digitação não altera buffer
  it("CA-01: abre arquivo em modo leitura; digitação não altera buffer", async () => {
    api.readFile.mockResolvedValue(sampleMarkdown);

    await act(async () => {
      await useProjectStore.getState().openSingleFile("/projeto/teste.md");
    });

    const state = useProjectStore.getState();
    expect(state.openDoc).not.toBeNull();
    expect(state.openDoc?.mode).toBe("reading");
    expect(state.openDoc?.dirty).toBe(false);
    expect(state.openDoc?.sessionSnapshot).toBe(sampleMarkdown);

    // Tentar "digitar" (updateBody) em modo leitura não deve alterar
    act(() => {
      useProjectStore.getState().updateBody("# Título\n\nConteúdo alterado.");
    });

    const afterType = useProjectStore.getState();
    // Em modo leitura, o buffer não deve mudar (ou deve rejeitar a alteração)
    expect(afterType.openDoc?.body).toBe("# Título\n\nConteúdo do teste.");
    expect(afterType.openDoc?.dirty).toBe(false);
  });

  // CA-02: Botão "Editar" alterna para edição; estado vira dirty
  it("CA-02: toggleEditMode() alterna para edição e marca dirty", async () => {
    api.readFile.mockResolvedValue(sampleMarkdown);

    await act(async () => {
      await useProjectStore.getState().openSingleFile("/projeto/teste.md");
    });

    // Alternar para edição
    act(() => {
      useProjectStore.getState().toggleEditMode();
    });

    let state = useProjectStore.getState();
    expect(state.openDoc?.mode).toBe("editing");
    expect(state.openDoc?.dirty).toBe(false); // ainda não editou, só mudou modo

    // Agora editar
    act(() => {
      useProjectStore.getState().updateBody("# Título\n\nConteúdo editado.");
    });

    state = useProjectStore.getState();
    expect(state.openDoc?.mode).toBe("editing");
    expect(state.openDoc?.dirty).toBe(true);
    expect(state.openDoc?.body).toContain("Conteúdo editado");
  });

  // CA-03: "Reverter ao original" restaura buffer = snapshotInicial (comparar hash)
  it("CA-03: revertToSnapshot() restaura buffer = snapshotInicial byte a byte", async () => {
    api.readFile.mockResolvedValue(sampleMarkdown);

    await act(async () => {
      await useProjectStore.getState().openSingleFile("/projeto/teste.md");
    });

    // Editar
    act(() => {
      useProjectStore.getState().toggleEditMode();
      useProjectStore.getState().updateBody("# Título\n\nConteúdo totalmente diferente.");
    });

    let state = useProjectStore.getState();
    expect(state.openDoc?.dirty).toBe(true);
    expect(state.openDoc?.body).toContain("totalmente diferente");

    // Reverter
    act(() => {
      useProjectStore.getState().revertToSnapshot();
    });

    state = useProjectStore.getState();
    expect(state.openDoc?.mode).toBe("reading");
    expect(state.openDoc?.dirty).toBe(false);
    // Buffer deve ser igual ao snapshot original (byte a byte)
    const originalBody = "# Título\n\nConteúdo do teste.";
    expect(state.openDoc?.body).toBe(originalBody);
    // Metadados também devem voltar ao original
    expect(state.openDoc?.metadata.title).toBe("Teste RFC-001");
  });

  // CA-04: Snapshot capturado uma vez ao abrir (não muda após editar/reverter)
  it("CA-04: sessionSnapshot é imutável durante a sessão", async () => {
    api.readFile.mockResolvedValue(sampleMarkdown);

    await act(async () => {
      await useProjectStore.getState().openSingleFile("/projeto/teste.md");
    });

    const snapshotInicial = useProjectStore.getState().openDoc?.sessionSnapshot;

    // Editar
    act(() => {
      useProjectStore.getState().toggleEditMode();
      useProjectStore.getState().updateBody("# Título\n\nEditado.");
    });

    // Reverter
    act(() => {
      useProjectStore.getState().revertToSnapshot();
    });

    // Snapshot deve ser o mesmo
    const snapshotFinal = useProjectStore.getState().openDoc?.sessionSnapshot;
    expect(snapshotFinal).toBe(snapshotInicial);
    expect(snapshotFinal).toBe(sampleMarkdown);
  });

  // CA-05: Reverter + salvar grava snapshot em disco (não mistura)
  it("CA-05: reverter + salvar grava snapshot em disco", async () => {
    api.readFile.mockResolvedValue(sampleMarkdown);
    api.writeFile.mockResolvedValue(undefined);

    await act(async () => {
      await useProjectStore.getState().openSingleFile("/projeto/teste.md");
    });

    // Editar
    act(() => {
      useProjectStore.getState().toggleEditMode();
      useProjectStore.getState().updateBody("# Título\n\nEditado para salvar.");
    });

    // Reverter
    act(() => {
      useProjectStore.getState().revertToSnapshot();
    });

    // Salvar
    await act(async () => {
      await useProjectStore.getState().saveCurrentFile();
    });

    // Verificar que writeFile foi chamado com o snapshot original
    expect(api.writeFile).toHaveBeenCalledWith(
      "/projeto/teste.md",
      sampleMarkdown // byte a byte igual ao original
    );
  });

  // CA-06: Front-matter/metadados participam do snapshot (título/subtítulo/autor/data)
  it("CA-06: metadados (front-matter) participam do snapshot e revertem", async () => {
    const markdownComMetadados = `---\ntitle: "Título Original"\nsubtitle: "Subtítulo Original"\nauthor: "Autor Original"\ndate: "2026-01-01"\n---\n\n# Corpo`;
    api.readFile.mockResolvedValue(markdownComMetadados);

    await act(async () => {
      await useProjectStore.getState().openSingleFile("/projeto/com-meta.md");
    });

    let state = useProjectStore.getState();
    expect(state.openDoc?.metadata.title).toBe("Título Original");
    expect(state.openDoc?.metadata.subtitle).toBe("Subtítulo Original");
    expect(state.openDoc?.metadata.author).toBe("Autor Original");

    // Editar metadados via updateMetadata
    act(() => {
      useProjectStore.getState().toggleEditMode();
      useProjectStore.getState().updateMetadata({
        title: "Título Alterado",
        subtitle: "Subtítulo Alterado",
        author: "Autor Alterado",
        date: "2026-12-31",
      });
    });

    state = useProjectStore.getState();
    expect(state.openDoc?.metadata.title).toBe("Título Alterado");
    expect(state.openDoc?.dirty).toBe(true);

    // Reverter
    act(() => {
      useProjectStore.getState().revertToSnapshot();
    });

    state = useProjectStore.getState();
    expect(state.openDoc?.metadata.title).toBe("Título Original");
    expect(state.openDoc?.metadata.subtitle).toBe("Subtítulo Original");
    expect(state.openDoc?.metadata.author).toBe("Autor Original");
    expect(state.openDoc?.metadata.date).toBe("2026-01-01");
    expect(state.openDoc?.mode).toBe("reading");
    expect(state.openDoc?.dirty).toBe(false);
  });
});