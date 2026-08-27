import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./theme.css', import.meta.url), 'utf8');

describe('report card wrapping', () => {
  it('defines and applies the shared report spacing scale', () => {
    expect(source).toContain('--space-1: 0.5rem;');
    expect(source).toContain('--space-2: 1rem;');
    expect(source).toContain('--space-3: 1.5rem;');
    expect(source).toContain('--space-4: 2rem;');
    expect(source).toContain('gap: var(--space-3);');
    expect(source).toContain('gap: var(--space-2);');
    expect(source).toContain('padding: var(--space-2);');
  });

  it('uses a theme-aware hover shadow for marker cards', () => {
    const markerHover = source.match(/\.marker-card:hover \{[\s\S]*?\n\}/)?.[0] ?? '';

    expect(markerHover).toContain('var(--shadow-card-hover)');
    expect(markerHover).not.toMatch(/(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i);
  });

  it('uses a theme-aware floating shadow for the focus control', () => {
    const focusToggle = source.match(/\.focus-toggle \{[\s\S]*?\n\}/)?.[0] ?? '';

    expect(focusToggle).toContain('var(--shadow-floating)');
    expect(focusToggle).not.toMatch(/(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i);
  });

  it('allows long marker identities and applicability badges to wrap', () => {
    const markerTop = source.match(/\.marker-top \{[\s\S]*?\n\}/)?.[0] ?? '';
    const markerLabel = source.match(/\.marker-top > \.gene-label \{[\s\S]*?\n\}/)?.[0] ?? '';

    expect(markerTop).toContain('flex-wrap: wrap;');
    expect(markerTop).toContain('min-width: 0;');
    expect(markerLabel).toContain('overflow-wrap: anywhere;');
    expect(markerLabel).toContain('word-break: break-word;');
  });

  it('keeps marker cards inside their one-column track on narrow screens', () => {
    const grid = source.match(/\.markers-grid \{[\s\S]*?\n\}/)?.[0] ?? '';
    const card = source.match(/\.marker-card \{[\s\S]*?\n\}/)?.[0] ?? '';

    expect(grid).toContain('min-width: 0;');
    expect(grid).toContain('max-width: 100%;');
    expect(grid).toContain('width: min(100%, 72rem);');
    expect(grid).toContain('margin-inline: auto;');
    expect(grid).toContain('box-sizing: border-box;');
    expect(grid).toContain('align-items: start;');
    expect(card).toContain('min-width: 0;');
    expect(card).toContain('max-width: 100%;');
    expect(card).toContain('box-sizing: border-box;');
    expect(card).toContain('gap: var(--space-1);');
  });

  it('keeps marker header spacing on the shared compact token', () => {
    const markerTop = source.match(/\.marker-top \{[\s\S]*?\n\}/)?.[0] ?? '';

    expect(markerTop).toContain('gap: var(--space-1);');
  });

  it('allows Clinical result labels to wrap without horizontal overflow', () => {
    const markerMiddle = source.match(/\.marker-middle \{[\s\S]*?\n\}/)?.[0] ?? '';
    const genotype = source.match(/\.genotype-val \{[\s\S]*?\n\}/)?.[0] ?? '';
    const severity = source.match(/\.marker-severity-label \{[\s\S]*?\n\}/)?.[0] ?? '';

    expect(markerMiddle).toContain('flex-wrap: wrap;');
    expect(genotype).toContain('overflow-wrap: anywhere;');
    expect(severity).toContain('overflow-wrap: anywhere;');
  });

  it('keeps the desktop Focus report control in normal layout flow', () => {
    const toolbar = source.match(/\.focus-toolbar \{[\s\S]*?\n\}/)?.[0] ?? '';
    const focusToggle = source.match(/\.focus-toggle \{[\s\S]*?\n\}/)?.[0] ?? '';

    expect(toolbar).toContain('flex: 0 0 auto;');
    expect(toolbar).toContain('padding: clamp(20px, 3vw, 36px) clamp(20px, 3vw, 36px) 0;');
    expect(focusToggle).toContain('position: static;');
    expect(focusToggle).not.toContain('position: fixed;');
  });

  it('keeps report filter controls on the active theme border token', () => {
    const filter = source.match(/\.filter-select \{[\s\S]*?\n\}/)?.[0] ?? '';

    expect(filter).toContain('border: 1px solid var(--border-color);');
    expect(filter).not.toMatch(/(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i);
  });

  it('allows the narrow reading-mode control to wrap inside its width', () => {
    const narrow = source.match(/@media \(max-width: 720px\) \{[\s\S]*?\n\}/)?.[0] ?? '';

    expect(narrow).toContain('.report-controls .view-mode-buttons');
    expect(narrow).toContain('max-width: 100%;');
    expect(narrow).toContain('box-sizing: border-box;');
    expect(narrow).toContain('flex-wrap: wrap;');
  });

  it('keeps shared error surfaces on semantic danger tokens', () => {
    const errorCard = source.match(/\.error-card \{[\s\S]*?\n\}/)?.[0] ?? '';
    const errorHeading = source.match(/\.error-header h4 \{[\s\S]*?\n\}/)?.[0] ?? '';
    const errorDetails = source.match(/\.error-details \{[\s\S]*?\n\}/)?.[0] ?? '';

    expect(errorCard).toContain('border: 1px solid var(--status-danger-border);');
    expect(errorCard).toContain('background: var(--status-danger-bg);');
    expect(errorHeading).toContain('color: var(--status-danger-strong-text);');
    expect(errorDetails).toContain('color: var(--status-danger-text);');
    expect(`${errorCard}${errorHeading}${errorDetails}`).not.toMatch(/(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i);
  });
});
