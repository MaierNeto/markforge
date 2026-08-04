import { describe, it, expect, beforeEach, vi } from "vitest";
import { addRecent, loadRecents, saveRecents, RecentEntry } from "@/lib/recents";

// Ambiente de teste roda em Node puro (sem jsdom) — não há localStorage
// global. Stub mínimo em memória, só para estes testes; em produção o app
// roda no webview, onde localStorage é real.
function fakeLocalStorage() {
  const store = new Map<string, string>();
  return {
    getItem: (k: string) => (store.has(k) ? store.get(k)! : null),
    setItem: (k: string, v: string) => {
      store.set(k, v);
    },
    clear: () => store.clear(),
  };
}

describe("Lista de recentes", () => {
  beforeEach(() => {
    vi.stubGlobal("localStorage", fakeLocalStorage());
  });

  it("adiciona uma entrada nova no topo", () => {
    const list: RecentEntry[] = [];
    const out = addRecent(list, { path: "/a", kind: "folder", label: "a" });
    expect(out).toEqual([{ path: "/a", kind: "folder", label: "a" }]);
  });

  it("reabrir o mesmo caminho move para o topo em vez de duplicar", () => {
    const list: RecentEntry[] = [
      { path: "/a", kind: "folder", label: "a" },
      { path: "/b", kind: "folder", label: "b" },
    ];
    const out = addRecent(list, { path: "/b", kind: "folder", label: "b" });
    expect(out).toEqual([
      { path: "/b", kind: "folder", label: "b" },
      { path: "/a", kind: "folder", label: "a" },
    ]);
  });

  it("limita a 8 entradas, descartando as mais antigas", () => {
    let list: RecentEntry[] = [];
    for (let i = 0; i < 10; i++) {
      list = addRecent(list, { path: `/p${i}`, kind: "folder", label: `p${i}` });
    }
    expect(list).toHaveLength(8);
    expect(list[0].path).toBe("/p9");
    expect(list.find((r) => r.path === "/p0")).toBeUndefined();
    expect(list.find((r) => r.path === "/p1")).toBeUndefined();
  });

  it("persiste e recarrega via localStorage", () => {
    const list = addRecent([], { path: "/proj", kind: "folder", label: "proj" });
    saveRecents(list);
    expect(loadRecents()).toEqual(list);
  });

  it("loadRecents devolve lista vazia quando nao ha nada salvo", () => {
    expect(loadRecents()).toEqual([]);
  });

  it("loadRecents devolve lista vazia se o conteudo salvo estiver corrompido", () => {
    localStorage.setItem("markforge.recents", "{nao e json valido");
    expect(loadRecents()).toEqual([]);
  });
});
