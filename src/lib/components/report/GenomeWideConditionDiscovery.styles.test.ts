import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./GenomeWideConditionDiscovery.svelte', import.meta.url), 'utf8');

describe('genome-wide disease discovery presentation', () => {
  it('keeps the report surface collapsed and limits large result lists', () => {
    expect(source).toContain('<details class="genomewide-discovery summary-card card">');
    expect(source).toContain('section.groups.slice(0, INITIAL_VISIBLE)');
    expect(source).toContain('const INITIAL_VISIBLE = 8');
    expect(source).toContain('const INITIAL_VISIBLE_LAB_REPORTS = 5');
    expect(source).toContain('View lab reports ({group.assertions.length})');
    expect(source).toContain('Show all ${group.assertions.length} lab reports');
    expect(source).toContain('plainConditionSummary(group)');
  });

  it('explains what a condition match represents in plain language', () => {
    expect(source).toContain('sectionConditionGroupsByRelevance(conditionGroups)');
    expect(source).toContain('copyCountLabel(assertion.alt_allele_copies)');
    expect(source).toContain('this is not a measure of how much of your whole genome was read');
    expect(source).toContain('Each item links one condition to one marker.');
    expect(source).toContain('ClinVar condition links ({summaryCounts}). These links are not diagnoses.');
    expect(source).toContain('DNA marker {group.rsid}');
    expect(source).toContain('yourCopiesLabel(group.copyCount)');
    expect(source).toContain('DNA variant copy count');
    expect(source).toContain('A copy count by itself does not mean you have the condition.');
    expect(source).toContain('button.genomewide-condition-count:focus-visible');
    expect(source).toContain('Variant-wide summary (not specific to this condition)');
  });

  it('keys evidence rows by every field that distinguishes a ClinVar assertion', () => {
    expect(source).toContain('`${assertion.rsid}:${assertion.variation_id}:${assertion.scv_accession}:${assertion.condition}`');
  });

  it('prevents long disease names and evidence records from forcing horizontal overflow', () => {
    expect(source).toContain('min-width: 0');
    expect(source).toContain('overflow-wrap: anywhere');
    expect(source).toContain('grid-template-columns: minmax(0, 1fr) auto auto;');
    expect(source).toContain('@media (max-width: 580px)');
  });

  it('keeps disclosure arrows and focus states visible after replacing browser markers', () => {
    expect(source).toContain('.genomewide-discovery-summary::marker');
    expect(source).toContain('.genomewide-discovery-summary::after');
    expect(source).toContain(".genomewide-discovery[open] > .genomewide-discovery-summary::after { content: '▾'; }");
    expect(source).toContain('.genomewide-discovery-summary:focus-visible');
  });
});
