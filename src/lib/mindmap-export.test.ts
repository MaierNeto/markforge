import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@/lib/tauri", () => ({
  api: {
    exportMindmap: vi.fn(),
  },
}));

import { api } from "@/lib/tauri";
import { mindmapOutputPath, exportProjectMindmap } from "@/lib/mindmap-export";

describe("caminho de saída do mapa mental", () => {
  it("grava ao lado da pasta exportada, com o nome dela", () => {
    expect(mindmapOutputPath("C:\\proj\\meu-projeto")).toBe(
      "C:\\proj\\meu-projeto\\meu-projeto.mm",
    );
  });

  it("funciona com separador POSIX", () => {
    expect(mindmapOutputPath("/home/w/meu-projeto")).toBe(
      "/home/w/meu-projeto/meu-projeto.mm",
    );
  });

  it("ignora barra final", () => {
    expect(mindmapOutputPath("C:\\proj\\meu-projeto\\")).toBe(
      "C:\\proj\\meu-projeto\\meu-projeto.mm",
    );
  });
});

describe("exportação do mapa mental", () => {
  beforeEach(() => {
    vi.mocked(api.exportMindmap).mockReset();
  });

  it("recusa exportar sem pasta aberta, sem chamar o backend", async () => {
    await expect(exportProjectMindmap(null)).rejects.toThrow(/pasta/i);
    expect(api.exportMindmap).not.toHaveBeenCalled();
  });

  it("chama o backend com a raiz e o caminho de saída derivado", async () => {
    vi.mocked(api.exportMindmap).mockResolvedValue("C:\\proj\\p\\p.mm");

    const gravado = await exportProjectMindmap("C:\\proj\\p");

    expect(api.exportMindmap).toHaveBeenCalledWith("C:\\proj\\p", "C:\\proj\\p\\p.mm");
    expect(gravado).toBe("C:\\proj\\p\\p.mm");
  });

  it("propaga a falha do backend em vez de engolir", async () => {
    vi.mocked(api.exportMindmap).mockRejectedValue("disco cheio");
    await expect(exportProjectMindmap("C:\\proj\\p")).rejects.toBeTruthy();
  });
});
