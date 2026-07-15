import { useState } from "react";
import { FileNode } from "@/lib/tauri";
import { basename } from "@/lib/paths";
import { useProjectStore } from "@/store/projectStore";

interface NodeProps {
  node: FileNode;
  depth: number;
  activePath: string | null;
  onOpenFile: (path: string) => void;
}

function TreeNode({ node, depth, activePath, onOpenFile }: NodeProps) {
  const [expanded, setExpanded] = useState(true);
  const { createFile, createFolder, renameEntry, deleteEntry } = useProjectStore();
  const [menuOpen, setMenuOpen] = useState(false);

  if (!node.is_dir) {
    const isActive = node.path === activePath;
    return (
      <div
        className={`mf-tree-file ${isActive ? "active" : ""}`}
        style={{ paddingLeft: depth * 14 + 10 }}
        onClick={() => onOpenFile(node.path)}
        title={node.path}
      >
        <span className="mf-tree-icon">📄</span>
        <span className="mf-tree-label">{node.name}</span>
      </div>
    );
  }

  const children = node.children ?? [];

  return (
    <div>
      <div
        className="mf-tree-folder"
        style={{ paddingLeft: depth * 14 + 10 }}
        onClick={() => setExpanded((v) => !v)}
      >
        <span className="mf-tree-icon">{expanded ? "📂" : "📁"}</span>
        <span className="mf-tree-label">{node.name}</span>
        <button
          className="mf-tree-menu-btn"
          onClick={(e) => {
            e.stopPropagation();
            setMenuOpen((v) => !v);
          }}
        >
          ⋯
        </button>
      </div>
      {menuOpen && (
        <div className="mf-tree-menu" style={{ marginLeft: depth * 14 + 24 }}>
          <button
            onClick={async () => {
              const name = prompt("Nome do novo arquivo (.md):", "novo-documento.md");
              if (name) await createFile(node.path, name.endsWith(".md") ? name : `${name}.md`);
              setMenuOpen(false);
            }}
          >
            + Arquivo
          </button>
          <button
            onClick={async () => {
              const name = prompt("Nome da nova pasta:", "nova-pasta");
              if (name) await createFolder(node.path, name);
              setMenuOpen(false);
            }}
          >
            + Pasta
          </button>
          {depth > 0 && (
            <>
              <button
                onClick={async () => {
                  const name = prompt("Renomear para:", node.name);
                  if (name) await renameEntry(node.path, name);
                  setMenuOpen(false);
                }}
              >
                Renomear
              </button>
              <button
                className="danger"
                onClick={async () => {
                  if (confirm(`Excluir "${node.name}" e todo o conteúdo?`)) {
                    await deleteEntry(node.path);
                  }
                  setMenuOpen(false);
                }}
              >
                Excluir
              </button>
            </>
          )}
        </div>
      )}
      {expanded &&
        children
          .slice()
          .sort((a, b) => {
            if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1;
            return a.name.localeCompare(b.name, "pt-BR");
          })
          .map((child) => (
            <TreeNode
              key={child.path}
              node={child}
              depth={depth + 1}
              activePath={activePath}
              onOpenFile={onOpenFile}
            />
          ))}
    </div>
  );
}

export function FileTree() {
  const tree = useProjectStore((s) => s.tree);
  const openDoc = useProjectStore((s) => s.openDoc);
  const openFile = useProjectStore((s) => s.openFile);
  const includeFolder = useProjectStore((s) => s.includeFolder);
  const loading = useProjectStore((s) => s.loadingTree);

  if (loading) {
    return <div className="mf-tree-empty">Carregando arquivos…</div>;
  }
  // Modo arquivo único: sem árvore carregada, mas com um documento aberto.
  if (!tree) {
    if (openDoc) {
      return (
        <div className="mf-tree">
          <div className="mf-tree-file active" title={openDoc.path}>
            <span className="mf-tree-icon">📄</span>
            <span className="mf-tree-label">{basename(openDoc.path)}</span>
          </div>
          <button className="mf-include-folder-btn" onClick={() => includeFolder()}>
            + Incluir a pasta deste arquivo
          </button>
        </div>
      );
    }
    return <div className="mf-tree-empty">Nenhuma pasta aberta.</div>;
  }
  const children = tree.children ?? [];
  if (children.length === 0) {
    return <div className="mf-tree-empty">Nenhum arquivo .md encontrado nesta pasta.</div>;
  }

  return (
    <div className="mf-tree">
      {children
        .slice()
        .sort((a, b) => {
          if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1;
          return a.name.localeCompare(b.name, "pt-BR");
        })
        .map((child) => (
          <TreeNode
            key={child.path}
            node={child}
            depth={0}
            activePath={openDoc?.path ?? null}
            onOpenFile={openFile}
          />
        ))}
    </div>
  );
}
