import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./SourcesList.svelte', import.meta.url), 'utf8');
const styleBlock = source.match(/<style>([\s\S]*?)<\/style>/)?.[1] ?? '';

describe('SourcesList semantic styling', () => {
  it('keeps reference-kind chips on shared theme tokens', () => {
    expect(styleBlock).not.toMatch(/#[0-9a-f]{3,8}\b/i);
    expect(styleBlock).not.toMatch(/\brgba?\(/i);
    expect(styleBlock).toContain('var(--status-info-soft-text)');
    expect(styleBlock).toContain('var(--status-info-soft-bg)');
    expect(styleBlock).toContain('var(--status-info-soft-border)');
  });

  it('wraps long citation and catalog labels inside the reference drawer', () => {
    expect(styleBlock).toContain('.sources-list li,');
    expect(styleBlock).toContain('.source-link,');
    expect(styleBlock).toContain('overflow-wrap: anywhere;');
    expect(styleBlock).toContain('word-break: break-word;');
  });
});
