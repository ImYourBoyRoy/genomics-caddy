import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const source = readFileSync(resolve(process.cwd(), 'src/lib/components/report/VariantCard.svelte'), 'utf8');
const enrichmentStyles = source.slice(source.indexOf('/* Reference enrichment chips */'), source.indexOf('/* Simple mode evidence summary */'));

describe('VariantCard enrichment surface', () => {
  it('uses semantic theme tokens for evidence and catalog chips', () => {
    expect(enrichmentStyles).toContain('var(--status-danger-bg)');
    expect(enrichmentStyles).toContain('var(--status-warning-bg)');
    expect(enrichmentStyles).toContain('var(--status-success-bg)');
    expect(enrichmentStyles).toContain('var(--status-info-bg)');
    expect(enrichmentStyles).toContain('var(--status-accent-bg)');
    expect(enrichmentStyles).toContain('var(--report-explainer-bg)');
    expect(enrichmentStyles).toContain('var(--text-secondary)');
  });

  it('does not reintroduce raw color literals in the enrichment block', () => {
    expect(enrichmentStyles).not.toMatch(/(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i);
  });

  it('keeps the Simple next-step copy compact', () => {
    expect(source).toContain("return 'Consider clinical confirmation.'");
    expect(source).not.toContain('Ask a qualified clinician whether medical-grade confirmation');
    expect(source).toContain('.simple-next-step {\n    display: block;');
  });
});
