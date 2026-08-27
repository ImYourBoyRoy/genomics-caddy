import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./VectorPromotedSection.svelte', import.meta.url), 'utf8');
const styleBlock = source.match(/<style>([\s\S]*?)<\/style>/)?.[1] ?? '';

describe('VectorPromotedSection report hierarchy', () => {
  it('caps secondary research cards at two columns and stacks them on narrow screens', () => {
    expect(styleBlock).toContain('grid-template-columns: repeat(2, minmax(0, 1fr));');
    expect(styleBlock).toContain('@media (max-width: 720px)');
    expect(styleBlock).toContain('grid-template-columns: 1fr;');
  });

  it('uses semantic theme tokens for secondary research surfaces', () => {
    for (const token of [
      '--status-success-border',
      '--status-success-bg',
      '--status-success-text',
      '--surface-card',
      '--border-color',
      '--status-accent-bg',
      '--status-info-bg',
    ]) {
      expect(styleBlock).toContain(`var(${token})`);
    }
    expect(source).toContain('accent="var(--status-success-text)"');
  });

  it('does not contain raw color literals in the component style block', () => {
    expect(styleBlock).not.toMatch(/(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i);
  });

  it('keeps research identifiers and DNA calls behind technical details in Simple mode', () => {
    expect(source).toContain("presentationMode?: 'simple' | 'clinical' | 'compare';");
    expect(source).toContain("{#if presentationMode === 'simple'}");
    expect(source).toContain('<summary>Technical data</summary>');
    expect(source).toContain('<dt>DNA call</dt><dd>{item.user_genotype}</dd>');
  });
});
