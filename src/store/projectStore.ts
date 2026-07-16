import { create } from "zustand";
import { api, FileNode } from "@/lib/tauri";
import { parseDocument, serializeDocument, DocMetadata } from "@/lib/frontmatter";
import { dirname, isInside } from "@/lib/paths";

interface OpenDocument {
  path: string;
  metadata: DocMetadata;
  body: string;
  dirty: boolean;
}

interface ProjectState {
  rootPath: string | null;
  tree: FileNode | null;
  openDoc: OpenDocument | null;
  loadingTree: boolean;
  saveStatus: "idle" | "saving" | "saved" | "error";
  error: string | null;

  openFolder: (path: string) => Promise<void>;
  openSingleFile: (path: string) => Promise<void>;
  includeFolder: () => Promise<void>;
  refreshTree: () => Promise<void>;
  openFile: (path: string) => Promise<void>;
  closeFile: () => void;
  updateBody: (body: string) => void;
  updateMetadata: (metadata: DocMetadata) => void;
  saveCurrentFile: () => Promise<void>;
  createFile: (dir: string, name: string) => Promise<void>;
  createFolder: (dir: string, name: string) => Promise<void>;
  renameEntry: (path: string, newName: string) => Promise<void>;
  deleteEntry: (path: string) => Promise<void>;
}

let saveTimer: ReturnType<typeof setTimeout> | null = null;

export const useProjectStore = create<ProjectState>((set, get) => ({
  rootPath: null,
  tree: null,
  openDoc: null,
  loadingTree: false,
  saveStatus: "idle",
  error: null,

  async openFolder(path: string) {
    set({ rootPath: path, loadingTree: true, error: null });
    try {
      const tree = await api.listMarkdownTree(path);
      set({ tree, loadingTree: false });
    } catch (e) {
      set({ error: String(e), loadingTree: false });
    }
  },

  // Abre um arquivo .md avulso (botão "Abrir arquivo" ou associação de .md no
  // SO). Se o arquivo já pertence ao projeto aberto, apenas o seleciona
  // mantendo a árvore; caso contrário entra em "modo arquivo único" — a pasta
  // fica conhecida (rootPath = pasta do arquivo) mas a árvore só é carregada
  // quando o usuário clica em "Incluir pasta".
  async openSingleFile(path: string) {
    const { tree, rootPath, openDoc } = get();
    if (openDoc?.dirty) {
      await get().saveCurrentFile();
    }
    if (tree && rootPath && isInside(rootPath, path)) {
      await get().openFile(path);
      return;
    }
    try {
      // Autoriza a pasta do arquivo antes de ler: o backend só opera dentro
      // das raízes que o usuário abriu.
      await api.allowFile(path);
      const raw = await api.readFile(path);
      const { metadata, body } = parseDocument(raw);
      set({
        rootPath: dirname(path),
        tree: null,
        openDoc: { path, metadata, body, dirty: false },
        saveStatus: "idle",
        error: null,
      });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  // Carrega a árvore da pasta que contém o arquivo aberto, sem fechar o
  // documento atual (transição de "arquivo único" para "projeto").
  async includeFolder() {
    const { rootPath } = get();
    if (!rootPath) return;
    set({ loadingTree: true, error: null });
    try {
      const tree = await api.listMarkdownTree(rootPath);
      set({ tree, loadingTree: false });
    } catch (e) {
      set({ error: String(e), loadingTree: false });
    }
  },

  async refreshTree() {
    const { rootPath, tree } = get();
    if (!rootPath || !tree) return;
    try {
      const tree = await api.listMarkdownTree(rootPath);
      set({ tree });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  async openFile(path: string) {
    // salva o arquivo anterior antes de trocar, se houver alterações pendentes
    const current = get().openDoc;
    if (current?.dirty) {
      await get().saveCurrentFile();
    }
    try {
      const raw = await api.readFile(path);
      const { metadata, body } = parseDocument(raw);
      set({ openDoc: { path, metadata, body, dirty: false }, saveStatus: "idle" });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  closeFile() {
    set({ openDoc: null });
  },

  updateBody(body: string) {
    const doc = get().openDoc;
    if (!doc) return;
    set({ openDoc: { ...doc, body, dirty: true }, saveStatus: "idle" });
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      get().saveCurrentFile();
    }, 800);
  },

  updateMetadata(metadata: DocMetadata) {
    const doc = get().openDoc;
    if (!doc) return;
    set({ openDoc: { ...doc, metadata, dirty: true } });
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      get().saveCurrentFile();
    }, 500);
  },

  async saveCurrentFile() {
    const doc = get().openDoc;
    if (!doc || !doc.dirty) return;
    set({ saveStatus: "saving" });
    try {
      const raw = serializeDocument(doc.metadata, doc.body);
      await api.writeFile(doc.path, raw);
      set((s) =>
        s.openDoc && s.openDoc.path === doc.path
          ? { openDoc: { ...s.openDoc, dirty: false }, saveStatus: "saved" }
          : { saveStatus: "saved" }
      );
    } catch (e) {
      set({ error: String(e), saveStatus: "error" });
    }
  },

  async createFile(dir: string, name: string) {
    await api.createMarkdownFile(dir, name);
    await get().refreshTree();
  },

  async createFolder(dir: string, name: string) {
    await api.createFolder(dir, name);
    await get().refreshTree();
  },

  async renameEntry(path: string, newName: string) {
    await api.renamePath(path, newName);
    await get().refreshTree();
  },

  async deleteEntry(path: string) {
    await api.deletePath(path);
    if (get().openDoc?.path === path) {
      set({ openDoc: null });
    }
    await get().refreshTree();
  },
}));
