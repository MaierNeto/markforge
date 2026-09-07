import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { Crepe } from "@milkdown/crepe";
import { getMarkdown } from "@milkdown/utils";
import { Window } from "happy-dom";

const MARP_FIXTURE = `---
marp: true
theme: uncover
paginate: true
backgroundColor: #fff
---

<!-- _class: lead -->

# Título da Apresentação

Subtítulo com **markdown**

---

<!-- _class: section -->

## Slide 2

Conteúdo normal.

---

### Slide 3

\`\`\`mermaid
graph TD
    A --> B
\`\`\`

---

<!-- header: Meu Header -->
<!-- footer: Meu Footer -->

#### Último Slide

Texto final.

`;

describe("RFC-006 CA-00: Gate de verificação round-trip Marp", () => {
  let window: Window;
  let container: HTMLElement;

  beforeAll(async () => {
    window = new Window({
      url: "http://localhost",
      pretendToBeVisual: true,
    });
    global.window = window as any;
    global.document = window.document;
    global.HTMLElement = window.HTMLElement;
    global.HTMLDivElement = window.HTMLDivElement;
    global.Node = window.Node;
    global.Element = window.Element;
    global.Document = window.Document;
    global.getComputedStyle = window.getComputedStyle;
    global.ResizeObserver = window.ResizeObserver;
    global.MutationObserver = window.MutationObserver;
    global.requestAnimationFrame = (cb: FrameRequestCallback) => window.setTimeout(cb, 16);
    global.cancelAnimationFrame = (id: number) => window.clearTimeout(id);
    global.addEventListener = window.addEventListener.bind(window);
    global.removeEventListener = window.removeEventListener.bind(window);
    global.dispatchEvent = window.dispatchEvent.bind(window);
    global.HTMLStyleElement = window.HTMLStyleElement;
    global.CSSStyleDeclaration = window.CSSStyleDeclaration;
    global.HTMLCanvasElement = window.HTMLCanvasElement;

    container = window.document.createElement("div");
    window.document.body.appendChild(container);
  });

  afterAll(() => {
    window.close();
  });

  it("CA-00: round-trip do fixture Marp — preserva 100% (GATE GREEN)", async () => {
    const crepe = new Crepe({
      root: container,
      defaultValue: MARP_FIXTURE,
    });

    const editorPromise = crepe.create();
    const editor = await editorPromise;

    const result = editor.action(getMarkdown());

    console.log("=== ORIGINAL ===");
    console.log(MARP_FIXTURE);
    console.log("=== RESULT ===");
    console.log(result);

    // CA-01: Front-matter sobrevive
    expect(result).toContain("marp: true");
    expect(result).toContain("theme: uncover");
    expect(result).toContain("paginate: true");
    expect(result).toContain("backgroundColor: #fff");

    // CA-02: Directive global sobrevive
    // CA-03: Directive por slide sobrevive
    expect(result).toContain("_class: lead");
    expect(result).toContain("_class: section");
    expect(result).toContain("header: Meu Header");
    expect(result).toContain("footer: Meu Footer");

    // CA-05: Fenced blocks sobrevivem
    expect(result).toContain("```mermaid");

    // CA-04: Slide breaks preservados (--- não vira HR)
    const slideBreaks = (result.match(/^---$/gm) || []).length;
    expect(slideBreaks).toBeGreaterThanOrEqual(4);

    // CA-06: Round-trip completo = diff zero
    expect(result.trim()).toBe(MARP_FIXTURE.trim());
  });
});
