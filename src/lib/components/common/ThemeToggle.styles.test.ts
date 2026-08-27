import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const source = readFileSync(resolve(process.cwd(), 'src/lib/components/common/ThemeToggle.svelte'), 'utf8');
const narrowStyles = source.slice(source.indexOf('@media (max-width: 900px)'), source.indexOf(':global(.app-layout.focus-mode)'));

describe('ThemeToggle responsive placement', () => {
  it('moves the fixed control out of the scrollable content on narrow layouts', () => {
    expect(narrowStyles).toContain('bottom: 0.75rem');
    expect(narrowStyles).not.toContain('top: 0.75rem');
  });

  it('opens the theme menu above the bottom-anchored control', () => {
    expect(narrowStyles).toContain('bottom: calc(100% + 0.4rem)');
    expect(narrowStyles).not.toContain('top: calc(100% + 0.4rem)');
    expect(narrowStyles).toContain('.theme-options');
  });

  it('keeps the control on the report side while the mobile data drawer is open', () => {
    expect(narrowStyles).toContain(':global(.app-layout.mobile-sidebar-open) .theme-toggle');
    expect(narrowStyles).toContain('left: calc(var(--sidebar-width) + 0.75rem);');
  });

  it('uses theme-aware floating shadow tokens', () => {
    expect(source).toContain('box-shadow: 0 0.5rem 1.25rem var(--shadow-floating);');
    expect(source).toContain('box-shadow: 0 0.75rem 2rem var(--shadow-floating);');
    expect(source).not.toMatch(/box-shadow:[^;]*(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i);
  });
});
