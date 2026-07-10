/**
 * Parser/serializador minimalista de front-matter YAML (bloco --- ... ---)
 * usado nos arquivos .md. Evita depender de uma lib externa de YAML: só
 * lidamos com pares "chave: valor" simples, que é o suficiente para os
 * metadados de capa (title, subtitle, author, date, logo).
 */

export interface DocMetadata {
  title?: string;
  subtitle?: string;
  author?: string;
  date?: string;
  [key: string]: string | undefined;
}

export interface ParsedDocument {
  metadata: DocMetadata;
  body: string;
  hasFrontmatter: boolean;
}

const FRONTMATTER_RE = /^---\r?\n([\s\S]*?)\r?\n---\r?\n?/;

function unquote(value: string): string {
  const v = value.trim();
  if (
    (v.startsWith('"') && v.endsWith('"')) ||
    (v.startsWith("'") && v.endsWith("'"))
  ) {
    return v.slice(1, -1);
  }
  return v;
}

function quoteIfNeeded(value: string): string {
  if (value === "") return '""';
  const needsQuote = /[:#\-{}\[\],&*!|>'"%@`]/.test(value) || /^\s|\s$/.test(value);
  if (needsQuote) {
    return `"${value.replace(/"/g, '\\"')}"`;
  }
  return value;
}

export function parseDocument(raw: string): ParsedDocument {
  const match = raw.match(FRONTMATTER_RE);
  if (!match) {
    return { metadata: {}, body: raw, hasFrontmatter: false };
  }
  const yamlBlock = match[1];
  const metadata: DocMetadata = {};
  for (const line of yamlBlock.split(/\r?\n/)) {
    const m = line.match(/^([A-Za-z0-9_-]+):\s*(.*)$/);
    if (m) {
      metadata[m[1]] = unquote(m[2]);
    }
  }
  const body = raw.slice(match[0].length);
  return { metadata, body, hasFrontmatter: true };
}

const KNOWN_ORDER = ["title", "subtitle", "author", "date"];

export function serializeDocument(metadata: DocMetadata, body: string): string {
  const keys = Object.keys(metadata).filter(
    (k) => metadata[k] !== undefined && metadata[k] !== null && metadata[k] !== ""
  );
  if (keys.length === 0) {
    return body;
  }
  const ordered = [
    ...KNOWN_ORDER.filter((k) => keys.includes(k)),
    ...keys.filter((k) => !KNOWN_ORDER.includes(k)),
  ];
  const yamlLines = ordered.map((k) => `${k}: ${quoteIfNeeded(metadata[k] as string)}`);
  const bodyTrimmed = body.replace(/^\r?\n+/, "");
  return `---\n${yamlLines.join("\n")}\n---\n\n${bodyTrimmed}`;
}
