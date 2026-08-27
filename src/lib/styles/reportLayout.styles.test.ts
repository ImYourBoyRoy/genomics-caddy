import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./theme.css', import.meta.url), 'utf8');

describe('report card wrapping', () => {
  it('allows long marker identities and applicability badges to wrap', () => {
    const markerTop = source.match(/\.marker-top \{[\s\S]*?\n\}/)?.[0] ?? '';
    const markerLabel = source.match(/\.marker-top > \.gene-label \{[\s\S]*?\n\}/)?.[0] ?? '';

    expect(markerTop).toContain('flex-wrap: wrap;');
    expect(markerTop).toContain('min-width: 0;');
    expect(markerLabel).toContain('overflow-wrap: anywhere;');
    expect(markerLabel).toContain('word-break: break-word;');
  });

  it('allows Clinical result labels to wrap without horizontal overflow', () => {
    const markerMiddle = source.match(/\.marker-middle \{[\s\S]*?\n\}/)?.[0] ?? '';
    const genotype = source.match(/\.genotype-val \{[\s\S]*?\n\}/)?.[0] ?? '';
    const severity = source.match(/\.marker-severity-label \{[\s\S]*?\n\}/)?.[0] ?? '';

    expect(markerMiddle).toContain('flex-wrap: wrap;');
    expect(genotype).toContain('overflow-wrap: anywhere;');
    expect(severity).toContain('overflow-wrap: anywhere;');
  });

  it('allows report header health statistics to wrap on narrow screens', () => {
    const summary = source.match(/\.health-summary-row \{[\s\S]*?\n\}/)?.[0] ?? '';
    const stat = source.match(/\.health-stat \{[\s\S]*?\n\}/)?.[0] ?? '';
    const value = source.match(/\.health-stat \.val \{[\s\S]*?\n\}/)?.[0] ?? '';

    expect(summary).toContain('flex-wrap: wrap;');
    expect(stat).toContain('min-width: 0;');
    expect(value).toContain('overflow-wrap: anywhere;');
  });

  it('keeps report filter controls on the active theme border token', () => {
    const filter = source.match(/\.filter-select \{[\s\S]*?\n\}/)?.[0] ?? '';

    expect(filter).toContain('border: 1px solid var(--border-color);');
    expect(filter).not.toMatch(/(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i);
  });
});
