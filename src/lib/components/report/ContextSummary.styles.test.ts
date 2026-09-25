import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./ContextSummary.svelte', import.meta.url), 'utf8');
const styleBlock = source.match(/<style>([\s\S]*?)<\/style>/)?.[1] ?? '';

describe('ContextSummary layout', () => {
  it('uses a compact, wrapping presentation with progressive disclosure', () => {
    expect(source).toContain('maxVisible?: number;');
    expect(source).toContain('+{overflowIndicators.length} more');
    expect(source).toContain('Also relevant');
    expect(styleBlock).toContain('flex-wrap: wrap;');
    expect(styleBlock).toContain('max-width: 100%;');
    expect(styleBlock).toContain('.context-overflow[open]');
    expect(styleBlock).toContain('font-weight: 650;');
    expect(styleBlock).not.toContain('border-radius: 999px;');
    expect(styleBlock).not.toContain('background: var(--status-info-soft-bg);');
    expect(source).toContain('title={indicator.description}');
    expect(source).toContain('role="note"');
    expect(source).toContain('aria-label={`${indicator.label}. ${indicator.description}`}');
    expect(source).not.toContain('tabindex="0"');
  });

  it('keeps expanded context in normal document flow', () => {
    expect(styleBlock).not.toContain('position: absolute;');
    expect(styleBlock).not.toContain('position: fixed;');
    expect(styleBlock).toContain('flex-basis: 100%;');
    expect(source).toContain('aria-label={`Show ${overflowIndicators.length} additional biological contexts`}');
  });

  it('uses theme tokens rather than hard-coded colors', () => {
    expect(styleBlock).toContain('var(--report-scope-text)');
    expect(styleBlock).toContain('var(--report-scope-border)');
    expect(styleBlock).not.toMatch(/(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i);
  });
});
