import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./SectionCard.svelte', import.meta.url), 'utf8');
const styleBlock = source.match(/<style>([\s\S]*?)<\/style>/)?.[1] ?? '';

describe('SectionCard semantic status styling', () => {
  it('keeps active finding and score surfaces on shared theme tokens', () => {
    expect(styleBlock).toContain('var(--status-danger-bg)');
    expect(styleBlock).toContain('var(--status-danger-strong-text)');
    expect(styleBlock).toContain('var(--status-danger-border)');
    expect(styleBlock).toContain('var(--shadow-subtle)');
  });

  it('does not contain raw color literals in the section-card style block', () => {
    expect(styleBlock).not.toMatch(/(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i);
  });

  it('disables the section transition when reduced motion is requested', () => {
    expect(source).toContain("window.matchMedia('(prefers-reduced-motion: reduce)')");
    expect(source).toContain('mediaQuery.addEventListener');
    expect(source).toContain('duration: reduceMotion ? 0 : 200');
  });
});
