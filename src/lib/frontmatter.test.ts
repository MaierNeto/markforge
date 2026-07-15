import { describe, it, expect } from "vitest";
import { parseDocument, serializeDocument } from "./frontmatter";

describe("parseDocument", () => {
  it("retorna corpo intacto quando não há front-matter", () => {
    const raw = "# Só o corpo\n\nSem metadados aqui.";
    const { metadata, body, hasFrontmatter } = parseDocument(raw);
    expect(hasFrontmatter).toBe(false);
    expect(metadata).toEqual({});
    expect(body).toBe(raw);
  });

  it("extrai pares chave/valor simples e separa o corpo", () => {
    const raw = "---\ntitle: Meu Doc\nauthor: Walter\n---\n\n# Corpo\n";
    const { metadata, body, hasFrontmatter } = parseDocument(raw);
    expect(hasFrontmatter).toBe(true);
    expect(metadata.title).toBe("Meu Doc");
    expect(metadata.author).toBe("Walter");
    expect(body).toBe("# Corpo\n");
  });

  it("remove aspas simples e duplas dos valores", () => {
    const raw = `---\ntitle: "Com dois pontos: sim"\nsubtitle: 'aspas simples'\n---\n\nx`;
    const { metadata } = parseDocument(raw);
    expect(metadata.title).toBe("Com dois pontos: sim");
    expect(metadata.subtitle).toBe("aspas simples");
  });

  it("aceita front-matter com CRLF", () => {
    const raw = "---\r\ntitle: CRLF\r\n---\r\n\r\ncorpo";
    const { metadata, body } = parseDocument(raw);
    expect(metadata.title).toBe("CRLF");
    expect(body).toBe("corpo");
  });
});

describe("serializeDocument", () => {
  it("retorna só o corpo quando não há metadados preenchidos", () => {
    expect(serializeDocument({}, "corpo puro")).toBe("corpo puro");
    expect(serializeDocument({ title: "" }, "corpo puro")).toBe("corpo puro");
  });

  it("emite as chaves conhecidas na ordem canônica", () => {
    const out = serializeDocument(
      { date: "2026", author: "W", title: "T", subtitle: "S" },
      "corpo"
    );
    const yaml = out.slice(0, out.indexOf("\n---"));
    expect(yaml).toBe("---\ntitle: T\nsubtitle: S\nauthor: W\ndate: 2026");
  });

  it("cita valores com caracteres especiais de YAML", () => {
    const out = serializeDocument({ title: "a: b" }, "corpo");
    expect(out).toContain(`title: "a: b"`);
  });
});

describe("round-trip parse ∘ serialize", () => {
  it("preserva valores simples", () => {
    const meta = { title: "Plano", author: "Walter", date: "10/07/2026" };
    const { metadata } = parseDocument(serializeDocument(meta, "corpo"));
    expect(metadata.title).toBe("Plano");
    expect(metadata.author).toBe("Walter");
    expect(metadata.date).toBe("10/07/2026");
  });

  it("preserva um valor com dois-pontos", () => {
    const meta = { title: "Aegis: SDLC" };
    const { metadata } = parseDocument(serializeDocument(meta, "corpo"));
    expect(metadata.title).toBe("Aegis: SDLC");
  });

  it("preserva um valor que contém aspas duplas (não acumula escapes)", () => {
    const meta = { title: `Ele disse "olá"` };
    const once = parseDocument(serializeDocument(meta, "corpo")).metadata.title;
    expect(once).toBe(`Ele disse "olá"`);

    // idempotência: um segundo ciclo não pode introduzir barras extras
    const twice = parseDocument(
      serializeDocument({ title: once }, "corpo")
    ).metadata.title;
    expect(twice).toBe(`Ele disse "olá"`);
  });
});
