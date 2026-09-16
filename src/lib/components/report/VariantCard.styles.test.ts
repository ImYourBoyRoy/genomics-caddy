import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const source = readFileSync(resolve(process.cwd(), 'src/lib/components/report/VariantCard.svelte'), 'utf8');
const theme = readFileSync(resolve(process.cwd(), 'src/lib/styles/theme.css'), 'utf8');
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

  it('bounds enrichment tooltips and chips in the static native report theme', () => {
    expect(theme).toContain('.main-content .sections-container .enrichment-row .tooltip-host {');
    expect(theme).toContain('max-width: 100%;\n  flex: 0 1 auto;');
    expect(theme).toContain('.main-content .sections-container .enrichment-row .tooltip-trigger {');
    expect(theme).toContain('overflow-wrap: anywhere;\n  text-align: left;\n  white-space: normal;');
    expect(theme).toContain('.main-content .sections-container .enrichment-row .pharmgkb-chip');
  });

  it('keeps the complete Simple copy contract visible on individual cards', () => {
    expect(source).toContain("getLaypersonTranslation, getSimpleFindingCopy, getSimpleFindingTitle");
    expect(source).toContain('simpleCopy.signal');
    expect(source).toContain('simpleCopy.why_it_matters');
    expect(source).toContain('simpleCopy.review_action');
    expect(source).toContain('simpleCopy.evidence_label');
    expect(source).toContain('class="simple-next-step"');
  });

  it('keeps technical gene identifiers out of the Simple card heading', () => {
    expect(source).toContain('simpleCopy.plain_title');
    expect(source).toContain('<strong>{simpleFindingTitle}</strong>');
    expect(source).toContain('if (viewMode === \'simple\') return simpleFindingTitle;');
  });

  it('keeps Simple metadata quiet so the meaning and next step stay focal', () => {
    const severityStyles = source.match(/\.simple-severity-label \{[\s\S]*?\n  \}/)?.[0] ?? '';

    expect(source).toContain('class="marker-severity-label simple-severity-label"');
    expect(source).not.toContain('class="simple-status-line"');
    expect(severityStyles).toContain('display: inline-flex;');
    expect(severityStyles).toContain('max-width: 100%;');
    expect(severityStyles).not.toContain('border: 1px solid');
    expect(severityStyles).not.toContain('background: var(--surface-subtle);');
    expect(source).not.toContain('simple-context-summary');
    expect(source).not.toContain('Medical reference context available');
    expect(source).not.toContain('Population context available');
    expect(source).not.toContain('Evidence sources available');
  });

  it('gives the finding title its own row above the compact metadata rail', () => {
    expect(source).toContain('<div class="marker-meta">');
    const markerTopStyles = readFileSync(resolve(process.cwd(), 'src/lib/styles/theme.css'), 'utf8')
      .match(/\.marker-top \{[\s\S]*?\n\}/)?.[0] ?? '';
    const markerMetaStyles = readFileSync(resolve(process.cwd(), 'src/lib/styles/theme.css'), 'utf8')
      .match(/\.marker-meta \{[\s\S]*?\n\}/)?.[0] ?? '';

    expect(markerTopStyles).toContain('flex-direction: column;');
    expect(markerTopStyles).toContain('align-items: stretch;');
    expect(markerMetaStyles).toContain('flex-wrap: wrap;');
    expect(markerMetaStyles).toContain('min-width: 0;');
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

  it('explains non-callable results on the Simple surface without exposing raw DNA', () => {
    expect(source).toContain('class="simple-callability-note"');
    expect(source).toContain('callabilityExplanation(callabilityState)');
    expect(source).toContain('role="note"');
  });

  it('provides a compact plain-language Details disclosure before technical data', () => {
    expect(source).toContain('<details class="simple-details">');
    expect(source).toContain('<summary>Details</summary>');
    expect(source).toContain('<p>{laypersonTranslation.simpleImpact}</p>');
    expect(source).not.toContain('<summary>Why this is shown</summary>');
  });

  it('groups secondary Simple disclosures into one compact rail', () => {
    expect(source).toContain('<div class="simple-card-disclosures">');
    expect(source).toContain('<details class="technical-details">');
    expect(source).toContain('<SourcesList sources={marker.sources} dbSources={marker.db_enriched_sources} />');
    expect(source).toContain('.simple-card-disclosures {');
    expect(source).toContain('gap: 0.25rem 0.85rem;');
    expect(source).toContain('.simple-card-disclosures > .technical-details[open]');
    expect(source).toContain('flex-basis: 100%;');
    expect(source).toContain(':global(.marker-sources-details)');
  });

  it('keeps the highlighted finding state on shared info tokens', () => {
    const highlightStyles = source.match(/:global\(\.marker-card-highlight\) \{[\s\S]*?\n  \}/)?.[0] ?? '';

    expect(highlightStyles).toContain('var(--status-info-text)');
    expect(highlightStyles).toContain('var(--status-info-border)');
    expect(highlightStyles).not.toMatch(/(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i);
  });
});
