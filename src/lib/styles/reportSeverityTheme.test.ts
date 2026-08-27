import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./theme.css', import.meta.url), 'utf8');
const severityBlock = source.match(
  /\/\* ===== Severity-class card styles ===== \*\/([\s\S]*?)\/\* Collapsed cards/
)?.[1] ?? '';
const lightSurfaceBlock = source.match(
  /\/\* Theme-aware report surfaces\.[\s\S]*?:root\[data-theme="light"\] \.app-layout/
)?.[0] ?? '';

describe('report severity theme tokens', () => {
  it('keeps severity card declarations on semantic theme tokens', () => {
    expect(severityBlock).not.toMatch(/(?:border-color|background|color):\s*(?:#[0-9a-f]{3,8}\b|rgba?\()/i);

    for (const token of [
      '--report-severity-high-border',
      '--report-severity-moderate-border',
      '--report-severity-low-border',
      '--report-severity-protective-border',
      '--report-severity-trait-border',
      '--report-severity-context-border',
      '--report-severity-confirm-border',
      '--report-severity-neutral-border',
      '--report-severity-glyph-text',
    ]) {
      expect(source).toContain(token);
    }
  });

  it('does not let the generic light surface rule erase severity surfaces', () => {
    expect(lightSurfaceBlock).not.toContain(':root[data-theme="light"] .marker-card');
  });

  it('keeps shared accent text on the active theme token', () => {
    const accentBlock = source.slice(source.indexOf('.text-accent'), source.indexOf('/* Sections container */'));
    expect(accentBlock).toContain('color: var(--accent);');
    expect(accentBlock).not.toContain('#818cf8');
  });
});
