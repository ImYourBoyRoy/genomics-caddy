import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const source = readFileSync(resolve(process.cwd(), 'src/lib/components/common/ThemeToggle.svelte'), 'utf8');
const styles = readFileSync(
  resolve(process.cwd(), 'src/lib/styles/components/theme-toggle.css'),
  'utf8',
);

describe('ThemeToggle sidebar footer control', () => {
  it('uses a compact inline control with no floating menu', () => {
    const options = styles.match(/\.theme-toggle-options \{[\s\S]*?\n\}/)?.[0] ?? '';

    expect(styles).toContain('width: 100%;');
    expect(styles).not.toContain('position: fixed;');
    expect(options).toContain('grid-template-columns: repeat(3, minmax(0, 1fr));');
    expect(styles).toContain('min-height: 44px;');
    expect(source).not.toContain('theme-options');
    expect(source).not.toContain('aria-haspopup="menu"');
  });

  it('exposes direct Auto, Light, and Dark choices', () => {
    expect(source).toContain("shortLabel: 'Auto'");
    expect(source).toContain("shortLabel: 'Light'");
    expect(source).toContain("shortLabel: 'Dark'");
    expect(source).toContain('data-theme-mode={option.id}');
    expect(source).toContain('aria-pressed={mode === option.id}');
    expect(source).toContain('aria-label={option.label}');
    expect(source).not.toContain('theme-toggle-label');
    expect(source).not.toContain('>Theme<');
    expect(source).not.toContain('☀️');
    expect(source).not.toContain('🌙');
    expect(source).toContain('System default');
    expect(source).toContain('Light mode');
    expect(source).toContain('Dark mode');
  });

  it('persists the selected mode without document-level menu listeners', () => {
    expect(source).toContain("localStorage.setItem(STORAGE_KEY, nextMode);");
    expect(source).toContain('root.dataset.theme = nextMode;');
    expect(source).toContain('document.documentElement.dataset.theme = mode;');
    expect(source).not.toContain('document.addEventListener');
    expect(source).not.toContain('handleDocumentScroll');
    expect(source).not.toContain('isOpen');
  });

  it('uses a short, temporary transition state when changing themes', () => {
    expect(source).toContain('const THEME_TRANSITION_MS = 2500;');
    expect(source).toContain("root.dataset.themeTransition = 'cover';");
    expect(source).toContain("root.dataset.themeTransition = 'reveal';");
    expect(source).toContain('window.requestAnimationFrame');
    expect(source).toContain('window.cancelAnimationFrame');
    expect(source).toContain('void root.offsetWidth;');
    expect(source).toContain('delete root.dataset.themeTransition;');
    expect(source).toContain('applyTheme(nextMode, true);');
  });

  it('keeps Light and Dark visible with a themed track instead of sidebar-matching fills', () => {
    expect(styles).toContain('background: var(--surface-control);');
    expect(styles).toContain('border: 1px solid var(--border-strong);');
    expect(styles).toContain('background: transparent;');
    expect(styles).toContain('border-color: var(--accent);');
    expect(styles).toContain('background: var(--accent-soft);');
    expect(styles).toContain('appearance: none;');
    expect(styles).toContain('color-scheme: inherit;');
    expect(styles).not.toMatch(/box-shadow:[^;]*(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i);
  });
});
