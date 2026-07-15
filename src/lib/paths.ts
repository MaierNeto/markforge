/**
 * Utilitários mínimos de caminho que funcionam tanto com separador POSIX (/)
 * quanto Windows (\), já que os caminhos vêm do backend nativo em ambos.
 */

/** Diretório que contém `path` (sem a barra final). "" se não houver separador. */
export function dirname(path: string): string {
  const idx = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return idx >= 0 ? path.slice(0, idx) : "";
}

/** Último componente do caminho (nome do arquivo/pasta). */
export function basename(path: string): string {
  const idx = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return idx >= 0 ? path.slice(idx + 1) : path;
}

/** Nome do arquivo sem a extensão .md/.markdown. */
export function fileStem(path: string): string {
  return basename(path).replace(/\.(md|markdown)$/i, "");
}

/** `true` se `child` está dentro (direta ou indiretamente) de `parent`. */
export function isInside(parent: string, child: string): boolean {
  return child.startsWith(parent + "/") || child.startsWith(parent + "\\");
}
