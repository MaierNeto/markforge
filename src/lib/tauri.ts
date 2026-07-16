import { invoke } from "@tauri-apps/api/core";

export interface FileNode {
  name: string;
  path: string;
  is_dir: boolean;
  children?: FileNode[];
}

export interface DependencyStatus {
  pandoc: boolean;
  typst: boolean;
}

export interface TemplateInfo {
  id: string;
  name: string;
  path: string;
  built_in: boolean;
}

export type ExportFormat = "docx" | "pdf" | "both";

export interface ExportMetadata {
  title?: string;
  subtitle?: string;
  author?: string;
  date?: string;
}

export interface ExportOptions {
  markdown: string;
  format: ExportFormat;
  template_id: string;
  include_cover: boolean;
  metadata: ExportMetadata;
  output_dir: string;
  file_stem: string;
  source_dir: string;
}

export interface ExportResult {
  docx_path?: string;
  pdf_path?: string;
  warnings: string[];
}

export interface ExtAssoc {
  ext: string;
  associated: boolean;
  handler: string | null;
}

export interface ContextMenuStatus {
  files: boolean;
  folders: boolean;
}

export const api = {
  listMarkdownTree(root: string): Promise<FileNode> {
    return invoke("list_markdown_tree", { root });
  },
  readFile(path: string): Promise<string> {
    return invoke("read_text_file", { path });
  },
  writeFile(path: string, content: string): Promise<void> {
    return invoke("write_text_file", { path, content });
  },
  createMarkdownFile(dir: string, fileName: string): Promise<string> {
    return invoke("create_markdown_file", { dir, fileName });
  },
  createFolder(dir: string, folderName: string): Promise<string> {
    return invoke("create_folder", { dir, folderName });
  },
  renamePath(path: string, newName: string): Promise<string> {
    return invoke("rename_path", { path, newName });
  },
  deletePath(path: string): Promise<void> {
    return invoke("delete_path", { path });
  },
  checkDependencies(): Promise<DependencyStatus> {
    return invoke("check_dependencies");
  },
  listTemplates(): Promise<TemplateInfo[]> {
    return invoke("list_templates");
  },
  importTemplate(sourcePath: string, name: string): Promise<TemplateInfo> {
    return invoke("import_template", { sourcePath, name });
  },
  deleteTemplate(id: string): Promise<void> {
    return invoke("delete_template", { id });
  },
  exportDocument(options: ExportOptions): Promise<ExportResult> {
    return invoke("export_document", { options });
  },
  openPath(path: string): Promise<void> {
    return invoke("open_in_file_manager", { path });
  },
  takeStartupFile(): Promise<string | null> {
    return invoke("take_startup_file");
  },
  allowFile(path: string): Promise<void> {
    return invoke("allow_file", { path });
  },
  assocSupported(): Promise<boolean> {
    return invoke("assoc_supported");
  },
  getAssociationStatus(): Promise<ExtAssoc[]> {
    return invoke("get_association_status");
  },
  setAssociation(ext: string, associate: boolean): Promise<void> {
    return invoke("set_association", { ext, associate });
  },
  getContextMenuStatus(): Promise<ContextMenuStatus> {
    return invoke("get_context_menu_status");
  },
  setContextMenu(files: boolean, folders: boolean): Promise<void> {
    return invoke("set_context_menu", { files, folders });
  },
  openDefaultAppsSettings(): Promise<void> {
    return invoke("open_default_apps_settings");
  },
};
