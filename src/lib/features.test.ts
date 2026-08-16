import { describe, it, expect } from "vitest";
import { isFeatureEnabled, FEATURE_MINDMAP_EXPORT } from "@/lib/features";

describe("feature flags", () => {
  it("recurso novo nasce desligado quando a variável não está definida", () => {
    expect(isFeatureEnabled(FEATURE_MINDMAP_EXPORT, {})).toBe(false);
  });

  it("liga apenas com o valor exato '1'", () => {
    expect(
      isFeatureEnabled(FEATURE_MINDMAP_EXPORT, { VITE_FEATURE_MINDMAP_EXPORT: "1" }),
    ).toBe(true);
  });

  it("qualquer outro valor mantém o recurso desligado", () => {
    for (const valor of ["0", "", "true", "yes", "on", "sim"]) {
      expect(
        isFeatureEnabled(FEATURE_MINDMAP_EXPORT, {
          VITE_FEATURE_MINDMAP_EXPORT: valor,
        }),
      ).toBe(false);
    }
  });
});
