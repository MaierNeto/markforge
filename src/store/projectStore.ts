import { create } from "zustand";
import { api, FileNode } from "@/lib/tauri";
import { parseDocument, serializeDocument, DocMetadata } from "@/lib/frontmatter";
import { dirname, isInside } from "@/lib/paths";

interface OpenDocument {
  path: string;
  metadata: DocMetadata;
  body: string;
  dirty: boolean;
  mode: "reading" | "editing";
  sessionSnapshot: string; // raw markdown capturado ao abrir (imutável)
  hashDisco: string; // último hash conhecido do arquivo no disco
  // estado derivado (não armazenado diretamente, calculado sob demanda):
  // "limpo" | "sujo" | "externo_modificado" | "conflito"
  // externalContent: conteúdo externo para resolução de conflito
  externalContent?: string;
}

interface ProjectState {
  rootPath: string | null;
  tree: FileNode | null;
  openDoc: OpenDocument | null;
  loadingTree: boolean;
  saveStatus: "idle" | "saving" | "saved" | "error";
  error: string | null;
  saveRawSnapshot: boolean; // flag para salvar o snapshot raw (após revert)

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
  toggleEditMode: () => void;
  revertToSnapshot: () => void;
  computeHash: (content: string) => string;
  checkExternalChange: () => Promise<boolean>;
  isDisjointDiff: (original: string, external: string, local: string) => boolean;
  mergeExternal: (externalContent?: string) => void;
  discardLocalAndReload: () => void;
  saveOverExternal: () => Promise<void>;
  saveAs: (newPath: string) => Promise<void>;
  updateHashDisco: () => void;
  openExportFolder: (exportedFilePath: string) => Promise<void>;
  hasUnsavedChanges: () => boolean;
  handleCloseRequested: () => Promise<"close" | "show-dialog">;
  resolveClose: (choice: "save" | "discard" | "cancel") => Promise<"close" | "stay">;
}

function isFileInTree(node: FileNode | null, targetPath: string): boolean {
  if (!node) return false;
  if (!node.is_dir && node.path === targetPath) return true;
  if (node.children) {
    return node.children.some((child) => isFileInTree(child, targetPath));
  }
  return false;
}

let saveTimer: ReturnType<typeof setTimeout> | null = null;

export const useProjectStore = create<ProjectState>((set, get) => ({
  rootPath: null,
  tree: null,
  openDoc: null,
  loadingTree: false,
  saveStatus: "idle",
  error: null,
  saveRawSnapshot: false,

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
      const hashDisco = get().computeHash(raw);
      set({
        rootPath: dirname(path),
        tree: null,
        openDoc: { path, metadata, body, dirty: false, mode: "reading", sessionSnapshot: raw, hashDisco },
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
    const { rootPath, tree, openDoc } = get();
    if (!rootPath || !tree) return;
    try {
      const newTree = await api.listMarkdownTree(rootPath);
      set({ tree: newTree, error: null });
      // RFC-003 CA-02: se o arquivo aberto foi removido do disco, avisa
      if (openDoc && !isFileInTree(newTree, openDoc.path)) {
        set({ error: `O arquivo aberto foi removido do disco: ${openDoc.path}` });
      }
    } catch (e) {
      set({ error: String(e) });
    }
  },

  // Verifica se deve mostrar aviso de arquivo removido (para RFC-003)
  shouldShowRemovedFileWarning(removedPath: string): boolean {
    const { openDoc } = get();
    return openDoc?.path === removedPath;
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
      const hashDisco = get().computeHash(raw);
      set({ openDoc: { path, metadata, body, dirty: false, mode: "reading", sessionSnapshot: raw, hashDisco }, saveStatus: "idle" });
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
    // Em modo leitura, não permite alterar o buffer
    if (doc.mode === "reading") return;
    set({ openDoc: { ...doc, body, dirty: true }, saveStatus: "idle" });
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      get().saveCurrentFile();
    }, 800);
  },

  toggleEditMode() {
    const doc = get().openDoc;
    if (!doc) return;
    const newMode = doc.mode === "reading" ? "editing" : "reading";
    set({ openDoc: { ...doc, mode: newMode } });
  },

  revertToSnapshot() {
    const doc = get().openDoc;
    if (!doc) return;
    const { metadata, body } = parseDocument(doc.sessionSnapshot);
    set({ openDoc: { ...doc, metadata, body, dirty: false, mode: "reading" }, saveRawSnapshot: true });
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
    const shouldSaveRaw = get().saveRawSnapshot;
    if (!doc || (!doc.dirty && !shouldSaveRaw)) return;
    set({ saveStatus: "saving", saveRawSnapshot: false });
    try {
      const raw = shouldSaveRaw ? doc.sessionSnapshot : serializeDocument(doc.metadata, doc.body);
      await api.writeFile(doc.path, raw);

      // Se outra edição chegou enquanto este writeFile estava em andamento, o
      // conteúdo que acabou de ser gravado já está desatualizado: não marcar
      // como salvo (perderia a edição mais recente em silêncio). O timer da
      // edição nova já está agendado e vai persistir o conteúdo atual.
      const latest = get().openDoc;
      const outdated =
        !shouldSaveRaw &&
        latest &&
        latest.path === doc.path &&
        (latest.body !== doc.body || latest.metadata !== doc.metadata);
      if (outdated) {
        set({ saveStatus: "idle" });
        return;
      }

      // Atualizar hashDisco após save próprio para evitar falso-positivo
      get().updateHashDisco();

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

  // Função de hash simples: length + checksum dos char codes em base36
  computeHash(content: string) {
    const sum = content.split("").reduce((a, b) => a + b.charCodeAt(0), 0);
    return content.length.toString(36) + sum.toString(36);
  },

  // Verifica se o arquivo no disco mudou comparando hash
  async checkExternalChange(): Promise<boolean> {
    const doc = get().openDoc;
    if (!doc) return false;

    try {
      const currentContent = await api.readFile(doc.path);
      const currentHash = get().computeHash(currentContent);

      if (currentHash !== doc.hashDisco) {
        // Hash mudou - arquivo foi modificado externamente
        if (doc.dirty) {
          // Arquivo sujo localmente - verificar se conflito ou merge disjunto
          const sessionHash = get().computeHash(doc.sessionSnapshot);
          if (currentHash === sessionHash) {
            // Conteúdo do disco igual ao snapshot original - nenhuma mudança externa real
            return false;
          }

          // Verificar se é merge disjunto (adições apenas). Comparação precisa ser
          // corpo-a-corpo: sessionSnapshot/currentContent vêm com front-matter, doc.body não.
          const { body: sessionBody } = parseDocument(doc.sessionSnapshot);
          const { body: externalBody } = parseDocument(currentContent);
          const isDisjoint = get().isDisjointDiff(sessionBody, externalBody, doc.body);

          if (isDisjoint) {
            // Merge automático para diff disjunto
            get().mergeExternal(currentContent);
            return true;
          } else {
            // Conflito - armazenar conteúdo externo para resolução
            set({
              openDoc: {
                ...doc,
                externalContent: currentContent,
                // estado derivado seria "conflito"
              },
            });
            return true;
          }
        } else {
          // Arquivo limpo - mudança externa detectada
          set({
            openDoc: {
              ...doc,
              externalContent: currentContent,
              // estado derivado seria "externo_modificado"
            },
          });
          return true;
        }
      }
      return false;
    } catch (e) {
      set({ error: String(e) });
      return false;
    }
  },

  // Verifica se o diff é disjunto (adições em regiões diferentes)
  isDisjointDiff(original: string, external: string, local: string): boolean {
    const origLines = original.split("\n");
    const extLines = external.split("\n");
    const locLines = local.split("\n");

    // Heurística: se external só adicionou linhas ao final do original
    // e local também só adicionou linhas (em posições diferentes), é disjunto
    if (extLines.length > origLines.length && locLines.length > origLines.length) {
      // Verificar se o prefixo original é preservado em ambos
      const origPrefix = origLines.join("\n");
      if (external.startsWith(origPrefix) && local.startsWith(origPrefix)) {
        // Ambos só adicionaram após o conteúdo original
        return true;
      }
    }
    return false;
  },

  // Merge automático para diff disjunto
  mergeExternal(externalContent?: string) {
    const doc = get().openDoc;
    if (!doc || !externalContent) return;

    // Para merge disjunto simples: manter conteúdo externo como base
    // Adicionar linhas locais que não estão no corpo externo
    const { metadata: extMetadata, body: extBody } = parseDocument(externalContent);
    // A body local já está em doc.body
    let mergedBody = extBody;
    const locOnlyLines = doc.body.split("\n").filter((line) => !extBody.includes(line) && line.trim() !== "");
    if (locOnlyLines.length > 0) {
      mergedBody = extBody + "\n\n" + locOnlyLines.join("\n");
    }

    set({
      openDoc: {
        ...doc,
        body: mergedBody,
        metadata: extMetadata, // usar metadados externos (mais recentes)
        dirty: true,
        externalContent: undefined,
        // estado derivado volta para "sujo"
      },
    });
  },

  // Opção (a): Descartar alterações locais e recarregar do disco
  discardLocalAndReload() {
    const doc = get().openDoc;
    if (!doc || !doc.externalContent) return;

    const { metadata, body } = parseDocument(doc.externalContent);
    set({
      openDoc: {
        ...doc,
        metadata,
        body,
        dirty: false,
        mode: "reading",
        sessionSnapshot: doc.externalContent,
        hashDisco: get().computeHash(doc.externalContent),
        externalContent: undefined,
      },
    });
  },

  // Opção (b): Salvar por cima do conteúdo externo (com backup)
  async saveOverExternal() {
    const doc = get().openDoc;
    if (!doc) return;

    // Gerar backup antes de sobrescrever
    const backupPath = doc.path + "~";
    try {
      // Backup do conteúdo externo atual
      if (doc.externalContent) {
        await api.writeFile(backupPath, doc.externalContent);
      }
    } catch (e) {
      set({ error: `Falha ao criar backup: ${e}` });
      // Continuar mesmo se backup falhar
    }

    // Escrever conteúdo local
    const raw = serializeDocument(doc.metadata, doc.body);
    await api.writeFile(doc.path, raw);

    // Atualizar hashDisco para evitar falso-positivo
    get().updateHashDisco();

    set({
      openDoc: {
        ...doc,
        dirty: false,
        sessionSnapshot: raw,
        externalContent: undefined,
      },
      saveStatus: "saved",
    });
  },

  // Opção (c): Salvar como novo arquivo
  async saveAs(newPath: string) {
    const doc = get().openDoc;
    if (!doc) return;

    const raw = serializeDocument(doc.metadata, doc.body);
    await api.writeFile(newPath, raw);

    // Abrir o novo arquivo
    const { metadata, body } = parseDocument(raw);
    set({
      openDoc: {
        path: newPath,
        metadata,
        body,
        dirty: false,
        mode: "reading",
        sessionSnapshot: raw,
        hashDisco: get().computeHash(raw),
        externalContent: undefined,
      },
      saveStatus: "saved",
    });
  },

  // Atualiza hashDisco após save próprio para evitar falso-positivo
  updateHashDisco() {
    const doc = get().openDoc;
    if (!doc) return;

    const raw = serializeDocument(doc.metadata, doc.body);
    const newHash = get().computeHash(raw);

    set({
      openDoc: {
        ...doc,
        hashDisco: newHash,
        sessionSnapshot: raw,
      },
    });
  },

  // RFC-004 (G0-5): "Abrir pasta" é efeito colateral do comando de exportação.
  // Recebe o caminho do arquivo gerado (DOCX/PDF) e abre a pasta que o contém.
  // O destino pode estar fora das raízes abertas — nasceu de escolha humana no
  // diálogo nativo de exportação (trust anchor).
  async openExportFolder(exportedFilePath: string) {
    if (!exportedFilePath) return;
    const folder = dirname(exportedFilePath);
    if (!folder) return;
    await api.openPath(folder);
  },

  // RFC-005: há edição não salva em algum documento aberto?
  hasUnsavedChanges() {
    return get().openDoc?.dirty === true;
  },

  // RFC-005: ao pedir fechamento da janela, decide se precisa de diálogo.
  // Se há edição não salva → "show-dialog"; senão → "close" silencioso (G0-2).
  async handleCloseRequested(): Promise<"close" | "show-dialog"> {
    if (get().hasUnsavedChanges()) {
      return "show-dialog";
    }
    return "close";
  },

  // RFC-005: resolve a escolha do diálogo Sim/Não/Cancelar (G0-2).
  // - "save"    → salva a edição pendente e fecha ("close");
  // - "discard" → descarta a edição pendente e fecha ("close"), sem salvar;
  // - "cancel"  → aborta o fechamento, app permanece aberto ("stay").
  async resolveClose(choice: "save" | "discard" | "cancel"): Promise<"close" | "stay"> {
    if (choice === "cancel") return "stay";
    if (choice === "save") {
      await get().saveCurrentFile();
    }
    return "close";
  },
}));