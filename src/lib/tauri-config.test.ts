import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";

// Contrato da config do Tauri (D-10 — instalador em PT e EN).
// O teste lê o tauri.conf.json real: a config é especificação executável, não
// comentário. Se a config divergir do que o produto prometeu, o teste falha.
const config = JSON.parse(
  readFileSync(new URL("../../src-tauri/tauri.conf.json", import.meta.url), "utf8"),
);

describe("tauri.conf.json — instalador NSIS em português e inglês (D-10)", () => {
  const nsis = config.bundle?.windows?.nsis;

  it("define a config NSIS no bundle do Windows", () => {
    expect(nsis).toBeDefined();
  });

  it("embute os idiomas English e PortugueseBR", () => {
    expect(nsis.languages).toEqual(expect.arrayContaining(["English", "PortugueseBR"]));
  });

  it("usa English como fallback (primeiro da lista) para SO fora de pt-BR", () => {
    // O Tauri usa o idioma do SO; fora da lista, cai no primeiro item.
    expect(nsis.languages[0]).toBe("English");
  });

  it("deixa o usuário escolher o idioma no seletor antes de instalar", () => {
    expect(nsis.displayLanguageSelector).toBe(true);
  });

  it("mantém instalação por usuário corrente (sem exigir admin por acidente)", () => {
    expect(nsis.installMode).toBe("currentUser");
  });

  it("continua gerando as demais plataformas (deb/appimage/nsis)", () => {
    expect(config.bundle.targets).toEqual(expect.arrayContaining(["deb", "appimage", "nsis"]));
  });
});
