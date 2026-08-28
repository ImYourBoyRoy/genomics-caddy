import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const source = readFileSync(resolve(process.cwd(), 'src/lib/components/common/ThemeToggle.svelte'), 'utf8');
describe('ThemeToggle desktop toolbar placement', () => {
  it('uses the title-toolbar flow instead of a fixed bottom-corner control', () => {
    const toggle = source.match(/\.theme-toggle \{[\s\S]*?\n  \}/)?.[0] ?? '';
    expect(toggle).toContain('position: relative;');
    expect(toggle).not.toContain('position: fixed;');
    expect(source).not.toContain('bottom: 0.75rem');
  });

  it('opens the theme menu below the toolbar control', () => {
    const options = source.match(/\.theme-options \{[\s\S]*?\n  \}/)?.[0] ?? '';
    expect(options).toContain('top: calc(100% + 0.4rem);');
    expect(options).toContain('right: 0;');
    expect(options).not.toContain('bottom: calc(100% + 0.4rem)');
  });

  it('uses clear mode labels instead of the generic Theme label', () => {
    expect(source).toContain('System default');
    expect(source).toContain('Light mode');
    expect(source).toContain('Dark mode');
    expect(source).not.toContain('<span>Theme</span>');
    expect(source).toContain('aria-label={`Appearance: ${themeModeLabel(mode)}`}');
  });

  it('uses theme-aware floating shadow tokens', () => {
    expect(source).toContain('box-shadow: 0 0.5rem 1.25rem var(--shadow-floating);');
    expect(source).toContain('box-shadow: 0 0.75rem 2rem var(--shadow-floating);');
    expect(source).not.toMatch(/box-shadow:[^;]*(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i);
  });
});
