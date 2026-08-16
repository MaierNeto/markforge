/**
 * Chaves de recurso. Recurso novo **nasce desligado**: só liga com o valor
 * exato "1", para que um valor solto no ambiente (`true`, `on`, `sim`) não
 * habilite algo por engano.
 */

export const FEATURE_MINDMAP_EXPORT = "VITE_FEATURE_MINDMAP_EXPORT";

type FeatureEnv = Record<string, string | boolean | undefined>;

export function isFeatureEnabled(flag: string, env?: FeatureEnv): boolean {
  const source = env ?? (import.meta.env as unknown as FeatureEnv);
  return source[flag] === "1";
}
