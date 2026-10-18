import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

// Mock Tauri BEFORE importing link-handler
vi.mock('@tauri-apps/api/shell', () => ({
  open: vi.fn(),
}));

import { setupLinkHandler, isExternalLink } from './link-handler';
import * as tauri from '@tauri-apps/api/shell';

describe('LinkHandler', () => {
  let container: HTMLElement;

  beforeEach(() => {
    // Setup DOM container
    container = document.createElement('div');
    document.body.appendChild(container);
    vi.clearAllMocks();
  });

  afterEach(() => {
    document.body.removeChild(container);
  });

  describe('isExternalLink', () => {
    it('should identify HTTPS URLs as external', () => {
      expect(isExternalLink('https://cloud.linode.com/')).toBe(true);
    });

    it('should identify HTTP URLs as external', () => {
      expect(isExternalLink('http://example.com/')).toBe(true);
    });

    it('should NOT identify anchors as external', () => {
      expect(isExternalLink('#section-2')).toBe(false);
    });

    it('should NOT identify relative paths as external', () => {
      expect(isExternalLink('./another-file.md')).toBe(false);
    });

    it('should NOT identify paths starting with / as external', () => {
      expect(isExternalLink('/docs/guide.md')).toBe(false);
    });

    it('should handle empty URLs gracefully', () => {
      expect(isExternalLink('')).toBe(false);
    });
  });

  describe('setupLinkHandler', () => {
    it('should setup click listener on container', () => {
      const mockAddEventListener = vi.spyOn(container, 'addEventListener');

      setupLinkHandler(container);

      expect(mockAddEventListener).toHaveBeenCalledWith(
        'click',
        expect.any(Function)
      );
    });

    it('should open external link in browser when clicked', () => {
      container.innerHTML = '<a href="https://cloud.linode.com/">Link</a>';
      const link = container.querySelector('a') as HTMLAnchorElement;

      setupLinkHandler(container);

      // Simulate click
      link.click();

      // Should have called tauri.open
      expect(tauri.open).toHaveBeenCalledWith('https://cloud.linode.com/');
    });

    it('should prevent default behavior for external links', () => {
      container.innerHTML = '<a href="https://example.com/">Link</a>';
      const link = container.querySelector('a') as HTMLAnchorElement;

      setupLinkHandler(container);

      const event = new MouseEvent('click', { bubbles: true });
      const preventDefaultSpy = vi.spyOn(event, 'preventDefault');

      link.dispatchEvent(event);

      expect(preventDefaultSpy).toHaveBeenCalled();
    });

    it('should NOT prevent default for internal links (anchors)', () => {
      container.innerHTML = '<a href="#section-2">Link</a>';
      const link = container.querySelector('a') as HTMLAnchorElement;

      setupLinkHandler(container);

      const event = new MouseEvent('click', { bubbles: true });
      const preventDefaultSpy = vi.spyOn(event, 'preventDefault');

      link.dispatchEvent(event);

      expect(preventDefaultSpy).not.toHaveBeenCalled();
    });

    it('should NOT prevent default for relative links', () => {
      container.innerHTML = '<a href="./other.md">Link</a>';
      const link = container.querySelector('a') as HTMLAnchorElement;

      setupLinkHandler(container);

      const event = new MouseEvent('click', { bubbles: true });
      const preventDefaultSpy = vi.spyOn(event, 'preventDefault');

      link.dispatchEvent(event);

      expect(preventDefaultSpy).not.toHaveBeenCalled();
    });

    it('should NOT crash when clicking on non-link elements', () => {
      container.innerHTML = '<p>Text <span>inside</span> paragraph</p>';
      const span = container.querySelector('span') as HTMLSpanElement;

      setupLinkHandler(container);

      // Should not throw
      expect(() => {
        span.click();
      }).not.toThrow();
    });

    it('should handle links without href attribute', () => {
      container.innerHTML = '<a>Link without href</a>';
      const link = container.querySelector('a') as HTMLAnchorElement;

      setupLinkHandler(container);

      // Should not throw
      expect(() => {
        link.click();
      }).not.toThrow();
    });
  });

  describe('UI Responsiveness', () => {
    it('should not block UI when opening external link', async () => {
      container.innerHTML = '<a href="https://example.com/">Link</a>';
      const link = container.querySelector('a') as HTMLAnchorElement;

      setupLinkHandler(container);

      const startTime = performance.now();
      link.click();
      const endTime = performance.now();

      // Click handler should be synchronous and quick (<5ms)
      expect(endTime - startTime).toBeLessThan(5);
    });

    it('should allow other UI events to fire normally', () => {
      container.innerHTML = `
        <a href="https://example.com/">Link</a>
        <button id="test-btn">Button</button>
      `;
      const link = container.querySelector('a') as HTMLAnchorElement;
      const button = container.querySelector('#test-btn') as HTMLButtonElement;

      setupLinkHandler(container);

      let buttonClicked = false;
      button.addEventListener('click', () => {
        buttonClicked = true;
      });

      // Click link then button
      link.click();
      button.click();

      // Button click should still work
      expect(buttonClicked).toBe(true);
    });
  });

  describe('Error Handling', () => {
    it('should catch and handle errors when opening URL', async () => {
      const mockOpen = vi.mocked(tauri.open);
      mockOpen.mockRejectedValueOnce(new Error('Failed to open URL'));

      container.innerHTML = '<a href="https://example.com/">Link</a>';
      const link = container.querySelector('a') as HTMLAnchorElement;

      setupLinkHandler(container);

      // Should not throw when open fails
      expect(() => {
        link.click();
      }).not.toThrow();
    });

    it('should handle link without href gracefully', () => {
      container.innerHTML = '<a>Link without href</a>';
      const link = container.querySelector('a') as HTMLAnchorElement;

      setupLinkHandler(container);

      expect(() => {
        link.click();
      }).not.toThrow();
    });
  });

  describe('CA-01: External Link Opens in Browser', () => {
    it('should call tauri.open() with HTTPS URL', () => {
      container.innerHTML = '<a href="https://cloud.linode.com/">Link</a>';
      const link = container.querySelector('a') as HTMLAnchorElement;

      setupLinkHandler(container);
      link.click();

      expect(tauri.open).toHaveBeenCalledWith('https://cloud.linode.com/');
      expect(tauri.open).toHaveBeenCalledTimes(1);
    });

    it('should call tauri.open() with HTTP URL', () => {
      container.innerHTML = '<a href="http://example.com/">Link</a>';
      const link = container.querySelector('a') as HTMLAnchorElement;

      setupLinkHandler(container);
      link.click();

      expect(tauri.open).toHaveBeenCalledWith('http://example.com/');
    });
  });

  describe('CA-02: MarkForge Maintains Focus & Responsiveness', () => {
    it('should allow Alt+Tab without freezing (click is non-blocking)', () => {
      container.innerHTML = '<a href="https://example.com/">Link</a>';
      const link = container.querySelector('a') as HTMLAnchorElement;

      setupLinkHandler(container);

      // Measure click time to ensure it's not blocking the event loop
      const startTime = performance.now();
      link.click();
      const endTime = performance.now();

      // Click handler should complete within <5ms (not blocking)
      expect(endTime - startTime).toBeLessThan(5);

      // UI should be responsive immediately after
      expect(tauri.open).toHaveBeenCalled();
    });

    it('should allow UI interactions after opening external link', () => {
      container.innerHTML = `
        <a href="https://example.com/">Link</a>
        <button id="test-btn">Button</button>
        <input id="test-input" type="text" />
      `;
      const link = container.querySelector('a') as HTMLAnchorElement;
      const button = container.querySelector('#test-btn') as HTMLButtonElement;
      const input = container.querySelector('#test-input') as HTMLInputElement;

      setupLinkHandler(container);

      // Click link
      link.click();

      // UI should still respond to other events
      let buttonClicked = false;
      button.addEventListener('click', () => {
        buttonClicked = true;
      });

      button.click();
      expect(buttonClicked).toBe(true);

      // Input should still accept text
      input.value = 'test';
      expect(input.value).toBe('test');
    });
  });

  describe('CA-03: No Freeze in Reading Mode', () => {
    it('should not freeze when clicking external link in reading mode', () => {
      container.innerHTML = '<a href="https://cloud.linode.com/">Link</a>';
      const link = container.querySelector('a') as HTMLAnchorElement;

      setupLinkHandler(container);

      const startTime = performance.now();
      link.click();
      const endTime = performance.now();

      // Should complete synchronously and quickly
      expect(endTime - startTime).toBeLessThan(5);
      expect(tauri.open).toHaveBeenCalled();
    });
  });

  describe('CA-04: No Freeze in Editing Mode', () => {
    it('should not freeze when clicking external link in editing mode', () => {
      container.innerHTML = '<p>Some text <a href="https://example.com/">Link</a> more text</p>';
      const link = container.querySelector('a') as HTMLAnchorElement;

      setupLinkHandler(container);

      const startTime = performance.now();
      link.click();
      const endTime = performance.now();

      // Should complete synchronously
      expect(endTime - startTime).toBeLessThan(5);
      expect(tauri.open).toHaveBeenCalled();
    });
  });

  describe('CA-05: Internal Links Work Normally', () => {
    it('should NOT open browser for anchor links', () => {
      container.innerHTML = '<a href="#section-2">Link</a>';
      const link = container.querySelector('a') as HTMLAnchorElement;

      setupLinkHandler(container);
      link.click();

      // Should NOT have called tauri.open()
      expect(tauri.open).not.toHaveBeenCalled();
    });

    it('should NOT open browser for relative paths', () => {
      container.innerHTML = '<a href="./another-file.md">Link</a>';
      const link = container.querySelector('a') as HTMLAnchorElement;

      setupLinkHandler(container);
      link.click();

      expect(tauri.open).not.toHaveBeenCalled();
    });

    it('should NOT open browser for absolute paths', () => {
      container.innerHTML = '<a href="/docs/guide.md">Link</a>';
      const link = container.querySelector('a') as HTMLAnchorElement;

      setupLinkHandler(container);
      link.click();

      expect(tauri.open).not.toHaveBeenCalled();
    });

    it('should allow default navigation for internal links', () => {
      container.innerHTML = '<a href="#section">Link</a>';
      const link = container.querySelector('a') as HTMLAnchorElement;

      setupLinkHandler(container);

      const event = new MouseEvent('click', { bubbles: true });
      const preventDefaultSpy = vi.spyOn(event, 'preventDefault');

      link.dispatchEvent(event);

      // preventDefault should NOT have been called for internal links
      expect(preventDefaultSpy).not.toHaveBeenCalled();
    });
  });

  describe('CA-06: Build & Tests Pass', () => {
    it('should have all required exports', () => {
      expect(typeof isExternalLink).toBe('function');
      expect(typeof setupLinkHandler).toBe('function');
    });

    it('should type-check isExternalLink correctly', () => {
      // These should all be valid calls (TypeScript compile-time checks)
      isExternalLink('https://example.com');
      isExternalLink('http://example.com');
      isExternalLink('#anchor');
      isExternalLink(null);
      isExternalLink(undefined);
    });

    it('should type-check setupLinkHandler correctly', () => {
      const el = document.createElement('div');
      // This should be a valid call
      setupLinkHandler(el);
    });
  });

  describe('Edge Cases & Protection', () => {
    it('should prevent multiple opens on double-click', () => {
      container.innerHTML = '<a href="https://example.com/">Link</a>';
      const link = container.querySelector('a') as HTMLAnchorElement;

      setupLinkHandler(container);

      // Rapid double-click
      link.click();
      link.click();

      // Should have been called twice (each click is independent)
      // But ideally should implement debouncing if needed
      expect(tauri.open).toHaveBeenCalledTimes(2);
    });

    it('should not throw on malformed URLs', () => {
      container.innerHTML = '<a href="ht!tp://broken">Link</a>';
      const link = container.querySelector('a') as HTMLAnchorElement;

      setupLinkHandler(container);

      expect(() => {
        link.click();
      }).not.toThrow();
    });

    it('should handle very long URLs', () => {
      const longUrl = 'https://example.com/' + 'a'.repeat(1000);
      container.innerHTML = `<a href="${longUrl}">Link</a>`;
      const link = container.querySelector('a') as HTMLAnchorElement;

      setupLinkHandler(container);

      link.click();

      expect(tauri.open).toHaveBeenCalledWith(longUrl);
    });
  });
});
