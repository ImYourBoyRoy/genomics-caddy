import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./GenomeWideConditionDiscovery.svelte', import.meta.url), 'utf8');

describe('genome-wide disease discovery presentation', () => {
  it('keeps the report surface collapsed and limits each relevance section', () => {
    expect(source).toContain('<details class="genomewide-discovery summary-card card">');
    expect(source).toContain('section.groups.slice(0, INITIAL_VISIBLE)');
    expect(source).toContain('const INITIAL_VISIBLE = 8');
    expect(source).toContain('Matched variants &amp; ClinVar evidence');
  });

  it('groups by relevance without repeating disclaimer paragraphs', () => {
    expect(source).toContain('sectionConditionGroupsByRelevance(conditionGroups)');
    expect(source).toContain('copyCountLabel(assertion.alt_allele_copies)');
    expect(source).not.toContain('It is not a diagnosis');
    expect(source).not.toContain('not DNA-scored');
  });

  it('keys evidence rows by every field that distinguishes a ClinVar assertion', () => {
    expect(source).toContain('`${assertion.rsid}:${assertion.variation_id}:${assertion.scv_accession}:${assertion.condition}`');
  });

  it('prevents long disease names and evidence records from forcing horizontal overflow', () => {
    expect(source).toContain('min-width: 0');
    expect(source).toContain('overflow-wrap: anywhere');
    expect(source).toContain('@media (max-width: 580px)');
  });
});
