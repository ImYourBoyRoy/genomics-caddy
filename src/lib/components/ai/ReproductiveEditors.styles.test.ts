import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const sources = [
  readFileSync(new URL('./ReproductiveContextEditor.svelte', import.meta.url), 'utf8'),
  readFileSync(new URL('./CycleDiaryEditor.svelte', import.meta.url), 'utf8'),
];
const styleBlocks = sources.map((source) => source.match(/<style>([\s\S]*?)<\/style>/)?.[1] ?? '');
const rawColor = /(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i;

describe('reproductive dashboard editor theme surfaces', () => {
  it('keeps both editors on shared semantic theme tokens', () => {
    for (const styles of styleBlocks) {
      expect(styles).not.toMatch(rawColor);
      expect(styles).toContain('background: var(--surface-control);');
      expect(styles).toContain('color: var(--text-secondary);');
    }
  });

  it('retains distinct information, success, and error states', () => {
    expect(styleBlocks[0]).toContain('var(--status-info-border)');
    expect(styleBlocks[0]).toContain('var(--status-info-soft-bg)');
    expect(styleBlocks[1]).toContain('var(--status-success-border)');
    expect(styleBlocks[1]).toContain('var(--status-success-bg)');
    expect(styleBlocks[1]).toContain('var(--status-danger-strong-text)');
  });
});
