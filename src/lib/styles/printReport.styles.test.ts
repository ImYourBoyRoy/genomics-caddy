import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const theme = readFileSync(new URL('./theme.css', import.meta.url), 'utf8');

describe('desktop report print surface', () => {
  it('switches the application shell to a readable paper layout', () => {
    const printBlock = theme.match(/@media print \{[\s\S]*$/)?.[0] ?? '';

    expect(printBlock).toContain('@page');
    expect(printBlock).toContain('overflow: visible;');
    expect(printBlock).toContain('.app-layout {');
    expect(printBlock).toContain('display: block;');
    expect(printBlock).toContain('.sidebar-slot,');
    expect(printBlock).toContain('.no-print,');
    expect(printBlock).toContain('display: none !important;');
  });

  it('keeps printed findings readable and prevents card splitting', () => {
    const printBlock = theme.match(/@media print \{[\s\S]*$/)?.[0] ?? '';

    expect(printBlock).toContain('.markers-grid {');
    expect(printBlock).toContain('grid-template-columns: minmax(0, 1fr) !important;');
    expect(printBlock).toContain('.marker-card {');
    expect(printBlock).toContain('break-inside: avoid;');
    expect(printBlock).toContain('box-shadow: none !important;');
    expect(printBlock).toContain('print-color-adjust: exact;');
  });

  it('provides print palette tokens for every explicit theme mode', () => {
    const printBlock = theme.match(/@media print \{[\s\S]*$/)?.[0] ?? '';

    expect(printBlock).toContain(':root[data-theme="dark"]');
    expect(printBlock).toContain(':root[data-theme="light"]');
    expect(printBlock).toContain(':root[data-theme="system"]');
    expect(printBlock).toContain('--text-primary: #111827;');
    expect(printBlock).toContain('--surface-raised: #ffffff;');
    expect(printBlock).toContain('--border-color: #cbd5e1;');
  });
});
