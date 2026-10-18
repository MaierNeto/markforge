/**
 * Link Handler — RFC-007: Links externos abrem no navegador padrão
 * Intercepta cliques em links <a> e abre URLs externas (http/https) no navegador padrão do SO
 *
 * TDD Red Phase: funções com stubs, testes devem FALHAR até implementação.
 * Mapeia para Critérios de Aceite:
 * - CA-01: HTTPS/HTTP links abrem em navegador padrão
 * - CA-02: MarkForge mantém foco (operação não-blocking)
 * - CA-03: Sem travamento em modo leitura
 * - CA-04: Sem travamento em modo edição
 * - CA-05: Links internos (#anchor, ., /) funcionam normalmente
 * - CA-06: `npm test` passa, `npm run build` OK, zero console errors
 */

import { open } from '@tauri-apps/api/shell';

/**
 * Verifica se uma URL é externa (http:// ou https://)
 *
 * Retorna true APENAS para URLs absolutas que começam com http:// ou https://
 * Retorna false para:
 * - Anchors (#seção-2)
 * - Paths relativos (./file.md, ../folder/file.md)
 * - Paths absolutos (/docs/guide.md)
 * - URLs vazias ou null/undefined
 *
 * @param href — valor do atributo href de um link <a>
 * @returns true se é URL externa (http/https), false caso contrário
 */
export function isExternalLink(href: string | null | undefined): boolean {
  if (!href) return false;
  return href.startsWith('http://') || href.startsWith('https://');
}

/**
 * Setup global click listener no container — intercepta cliques em links externos
 *
 * Quando um usuário clica em um <a> tag:
 * 1. Se href é externo (http/https):
 *    - Prevenir navegação in-page (preventDefault)
 *    - Abrir URL no navegador padrão via tauri.open()
 *    - Capturar e logar erros (sem re-throw)
 * 2. Se href é interno (anchor, relativo, absoluto):
 *    - Deixar comportamento padrão (navegação normal do editor)
 *
 * CA-02: Operação deve ser não-blocking (< 5ms) para manter UI responsiva
 * CA-03/CA-04: Sem travamento em qualquer contexto (leitura/edição)
 *
 * @param container — elemento pai que contém os links (ex: div.mf-editor)
 */
export function setupLinkHandler(container: HTMLElement): void {
  container.addEventListener('click', (event: MouseEvent) => {
    const link = (event.target as Element).closest('a');
    if (!link) return;

    const href = link.getAttribute('href');
    if (!href) return;

    if (isExternalLink(href)) {
      event.preventDefault();
      open(href).catch((err) => console.error('Failed to open link:', err));
    }
  });
}
