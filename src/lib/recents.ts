export interface RecentEntry {
  path: string;
  kind: "folder" | "file";
  label: string;
}

const STORAGE_KEY = "markforge.recents";
const MAX_RECENTS = 8;

/** Move o caminho para o topo (sem duplicar) e limita a MAX_RECENTS. */
export function addRecent(list: RecentEntry[], entry: RecentEntry): RecentEntry[] {
  const withoutDupe = list.filter((r) => r.path !== entry.path);
  return [entry, ...withoutDupe].slice(0, MAX_RECENTS);
}

export function loadRecents(): RecentEntry[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

export function saveRecents(list: RecentEntry[]): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(list));
  } catch {
    // localStorage indisponível (ex.: modo privado) — ignora silenciosamente,
    // a lista de recentes é conveniência, não algo crítico.
  }
}
