import { describe, it, expect } from "vitest";
import { dirname, basename, fileStem, isInside } from "./paths";

describe("dirname", () => {
  it("lida com separador Windows", () => {
    expect(dirname("C:\\docs\\projeto\\plano.md")).toBe("C:\\docs\\projeto");
  });
  it("lida com separador POSIX", () => {
    expect(dirname("/home/w/notas/a.md")).toBe("/home/w/notas");
  });
  it("retorna vazio sem separador", () => {
    expect(dirname("a.md")).toBe("");
  });
});

describe("basename", () => {
  it("extrai o nome do arquivo (Windows)", () => {
    expect(basename("C:\\docs\\plano.md")).toBe("plano.md");
  });
  it("extrai o nome do arquivo (POSIX)", () => {
    expect(basename("/home/w/a.md")).toBe("a.md");
  });
  it("devolve o próprio valor sem separador", () => {
    expect(basename("a.md")).toBe("a.md");
  });
});

describe("fileStem", () => {
  it("remove a extensão .md", () => {
    expect(fileStem("C:\\docs\\Plano de Ação.md")).toBe("Plano de Ação");
  });
  it("remove a extensão .markdown (case-insensitive)", () => {
    expect(fileStem("/x/Notas.MARKDOWN")).toBe("Notas");
  });
});

describe("isInside", () => {
  it("detecta filho direto (Windows)", () => {
    expect(isInside("C:\\proj", "C:\\proj\\a.md")).toBe(true);
  });
  it("detecta filho indireto (POSIX)", () => {
    expect(isInside("/proj", "/proj/sub/a.md")).toBe(true);
  });
  it("nega caminho fora da raiz", () => {
    expect(isInside("C:\\proj", "C:\\outro\\a.md")).toBe(false);
  });
  it("nega prefixo parcial que não é fronteira de pasta", () => {
    expect(isInside("/proj", "/projeto/a.md")).toBe(false);
  });
});
