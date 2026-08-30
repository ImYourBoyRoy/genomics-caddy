import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const source = readFileSync(resolve(process.cwd(), 'src/lib/components/common/ThemeToggle.svelte'), 'utf8');

describe('ThemeToggle sidebar footer control', () => {
  it('uses a compact inline control with no floating menu', () => {
    const toggle = source.match(/\.theme-toggle \{[\s\S]*?\n  \}/)?.[0] ?? '';
    const options = source.match(/\.theme-toggle-options \{[\s\S]*?\n  \}/)?.[0] ?? '';

    expect(toggle).toContain('width: 100%;');
    expect(toggle).not.toContain('position: fixed;');
    expect(options).toContain('grid-template-columns: repeat(3, minmax(0, 1fr));');
    expect(source).toContain('min-height: 44px;');
    expect(source).not.toContain('theme-options');
    expect(source).not.toContain('aria-haspopup="menu"');
  });

  it('exposes direct Auto, Light, and Dark choices', () => {
    expect(source).toContain('shortLabel: \'Auto\'');
    expect(source).toContain('shortLabel: \'Light\'');
    expect(source).toContain('shortLabel: \'Dark\'');
    expect(source).toContain('data-theme-mode={option.id}');
    expect(source).toContain('aria-pressed={mode === option.id}');
    expect(source).toContain('aria-label={option.label}');
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

  it('uses theme-aware controls without hard-coded shadow colors', () => {
    expect(source).toContain('border-color: var(--accent);');
    expect(source).toContain('background: var(--accent-soft);');
    expect(source).not.toMatch(/box-shadow:[^;]*(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i);
  });
});
