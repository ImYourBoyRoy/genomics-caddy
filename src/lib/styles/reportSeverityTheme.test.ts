import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./theme.css', import.meta.url), 'utf8');
const severityBlock = source.match(
  /\/\* ===== Severity-class card styles ===== \*\/([\s\S]*?)\/\* Collapsed cards/
)?.[1] ?? '';
const lightSurfaceBlock = source.match(
  /\/\* Theme-aware report surfaces\.[\s\S]*?:root\[data-theme="light"\] \.app-layout/
)?.[0] ?? '';
const reportSurfaceBlock = source.match(
  /\/\* ===== Report Legend ===== \*\/([\s\S]*?)\/\* ===== Focus Indicators ===== \*/
)?.[1] ?? '';

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

  it('keeps legacy report badges, legend, sources, and score surfaces theme-aware', () => {
    expect(reportSurfaceBlock).not.toMatch(/(?:background|border(?:-color)?|color):\s*(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i);
    for (const token of [
      'var(--status-danger-bg)',
      'var(--status-success-bg)',
      'var(--status-accent-soft-bg)',
      'var(--status-info-bg)',
      'var(--report-severity-high-bg)',
      'var(--report-severity-neutral-bg)',
      'var(--report-claim-bg)',
      'var(--surface-subtle)',
    ]) {
      expect(source).toContain(token);
    }
  });
});
