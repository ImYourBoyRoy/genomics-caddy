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
    expect(source).toContain("import { getCompactSimpleMeaning, getLaypersonTranslation, getSimpleFindingTitle, getSimpleNextStep }");
    expect(source).toContain('return getSimpleNextStep(marker);');
    expect(source).toContain('.simple-next-step {\n    display: block;');
  });

  it('keeps technical gene identifiers out of the Simple card heading', () => {
    expect(source).toContain('getSimpleFindingTitle(laypersonTranslation.simpleImpact)');
    expect(source).toContain('<strong>{simpleFindingTitle}</strong>');
    expect(source).toContain('if (viewMode === \'simple\') return simpleFindingTitle;');
  });

  it('keeps Simple metadata quiet so the meaning and next step stay focal', () => {
    const simpleContextStyles = source.slice(source.indexOf('/* Simple mode reference context summary */'));
    const severityStyles = simpleContextStyles.match(/\.simple-severity-label \{[\s\S]*?\n  \}/)?.[0] ?? '';
    const contextRule = simpleContextStyles.match(/\.simple-context-summary \{[\s\S]*?\n  \}/)?.[0] ?? '';

    expect(source).toContain('class="marker-severity-label simple-severity-label"');
    expect(source).not.toContain('class="simple-status-line"');
    expect(severityStyles).toContain('display: inline-flex;');
    expect(severityStyles).toContain('max-width: 100%;');
    expect(severityStyles).not.toContain('border: 1px solid');
    expect(severityStyles).not.toContain('background: var(--surface-subtle);');
    expect(source).toContain('class="simple-context-summary" aria-label="Additional reference context"');
    expect(source).not.toContain('Evidence sources available');
    expect(contextRule).not.toContain('border-top: 1px solid var(--border-color);');
  });

  it('keeps Compare evidence boundaries behind a compact disclosure', () => {
    expect(source).toContain('<summary>Evidence boundary</summary>');
    expect(source).toContain('class="claim-frame" role="note"');
    expect(source).not.toContain('<WarningBlocks');
    expect(source).toContain('class="claim-detail-note"');
    expect(source).toContain('class="claim-limit-list"');
  });

  it('keeps repeated claim-boundary copy out of the default Simple card', () => {
    expect(source).not.toContain('<summary>Why this is shown</summary>');
    expect(source).toContain('<summary>Technical data</summary>');
    expect(source).toContain('<dt>Claim boundary</dt>');
  });

  it('keeps the Simple card from repeating its direction label', () => {
    expect(source).toContain('{#if marker.effect_direction && viewMode !== \'simple\'}');
    expect(source).toContain('<EffectDirectionBadge direction={marker.effect_direction} />');
  });

  it('keeps the highlighted finding state on shared info tokens', () => {
    const highlightStyles = source.match(/:global\(\.marker-card-highlight\) \{[\s\S]*?\n  \}/)?.[0] ?? '';

    expect(highlightStyles).toContain('var(--status-info-text)');
    expect(highlightStyles).toContain('var(--status-info-border)');
    expect(highlightStyles).not.toMatch(/(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i);
  });
});
