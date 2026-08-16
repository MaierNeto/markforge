import { api } from "@/lib/tauri";
import { basename } from "@/lib/paths";

/**
 * Onde o mapa é gravado: dentro da própria pasta exportada, com o nome dela.
 *
 * Fica ao lado do conteúdo de propósito — os links dentro do `.mm` são
 * **relativos** à raiz, então mapa e pasta viajam juntos: mover a pasta inteira
 * para outro lugar não quebra nenhum link.
 */
export function mindmapOutputPath(rootPath: string): string {
  const sep = rootPath.includes("\\") ? "\\" : "/";
  const limpo = rootPath.replace(/[/\\]+$/, "");
  return `${limpo}${sep}${basename(limpo)}.mm`;
}

/** Exporta a pasta aberta como mapa mental. Devolve o caminho gravado. */
export async function exportProjectMindmap(rootPath: string | null): Promise<string> {
  if (!rootPath) {
    throw new Error("Abra uma pasta antes de exportar o mapa mental.");
  }
  return api.exportMindmap(rootPath, mindmapOutputPath(rootPath));
}
