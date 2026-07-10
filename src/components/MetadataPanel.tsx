import { useState } from "react";
import { useProjectStore } from "@/store/projectStore";

export function MetadataPanel() {
  const openDoc = useProjectStore((s) => s.openDoc);
  const updateMetadata = useProjectStore((s) => s.updateMetadata);
  const [expanded, setExpanded] = useState(false);

  if (!openDoc) return null;
  const { metadata } = openDoc;

  const field = (key: "title" | "subtitle" | "author" | "date", label: string, placeholder: string) => (
    <label className="mf-field">
      <span>{label}</span>
      <input
        type="text"
        value={metadata[key] ?? ""}
        placeholder={placeholder}
        onChange={(e) => updateMetadata({ ...metadata, [key]: e.target.value })}
      />
    </label>
  );

  return (
    <div className={`mf-metadata-panel ${expanded ? "expanded" : ""}`}>
      <button className="mf-metadata-toggle" onClick={() => setExpanded((v) => !v)}>
        <span>Metadados do documento (capa)</span>
        <span className="chev">{expanded ? "▾" : "▸"}</span>
      </button>
      {expanded && (
        <div className="mf-metadata-fields">
          {field("title", "Título", "Título do documento")}
          {field("subtitle", "Subtítulo", "Subtítulo ou descrição curta")}
          {field("author", "Autor", "Nome do autor")}
          {field("date", "Data", "ex: 10 de julho de 2026")}
          <p className="mf-metadata-hint">
            Esses campos viram o bloco de metadados (front-matter) no início do
            arquivo .md e alimentam a capa ao exportar para DOCX/PDF.
          </p>
        </div>
      )}
    </div>
  );
}
