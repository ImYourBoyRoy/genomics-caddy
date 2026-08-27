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
    expect(source).toContain("return 'Review the suggested follow-up.'");
    expect(source).toContain("return 'Compare with symptoms, history, and relevant labs.'");
    expect(source).toContain("return 'Consider diet, medications, and lifestyle context.'");
    expect(source).toContain("return 'Compare this with your lived experience.'");
    expect(source).not.toContain('Ask a qualified clinician whether medical-grade confirmation');
    expect(source).toContain('.simple-next-step {\n    display: block;');
  });

  it('keeps technical gene identifiers out of the Simple card heading', () => {
    expect(source).toContain('getSimpleFindingTitle(laypersonTranslation.simpleImpact)');
    expect(source).toContain('<strong>{simpleFindingTitle}</strong>');
    expect(source).toContain('if (viewMode === \'simple\') return simpleFindingTitle;');
  });

  it('keeps Simple metadata quiet so the meaning and next step stay focal', () => {
    const simpleEvidenceStyles = source.slice(source.indexOf('/* Simple mode evidence summary */'));
    const statusStyles = simpleEvidenceStyles.slice(
      simpleEvidenceStyles.indexOf('.simple-status-line {'),
      simpleEvidenceStyles.indexOf('.simple-next-step {'),
    );
    const evidenceRule = simpleEvidenceStyles.match(/\.simple-evidence-summary \{[\s\S]*?\n  \}/)?.[0] ?? '';

    expect(statusStyles).toContain('min-height: 1.25rem;');
    expect(statusStyles).toContain('padding: 0;');
    expect(statusStyles).not.toContain('border: 1px solid');
    expect(statusStyles).not.toContain('background: var(--surface-subtle);');
    expect(simpleEvidenceStyles).toContain('.simple-evidence-summary {');
    expect(evidenceRule).not.toContain('border-top: 1px solid var(--border-color);');
  });

  it('keeps Compare evidence boundaries behind a compact disclosure', () => {
    expect(source).toContain('<summary>Evidence boundary</summary>');
    expect(source).toContain('class="claim-frame" role="note"');
    expect(source).not.toContain('<WarningBlocks');
    expect(source).toContain('class="claim-detail-note"');
    expect(source).toContain('class="claim-limit-list"');
  });

  it('keeps the highlighted finding state on shared info tokens', () => {
    const highlightStyles = source.match(/:global\(\.marker-card-highlight\) \{[\s\S]*?\n  \}/)?.[0] ?? '';

    expect(highlightStyles).toContain('var(--status-info-text)');
    expect(highlightStyles).toContain('var(--status-info-border)');
    expect(highlightStyles).not.toMatch(/(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i);
  });
});
