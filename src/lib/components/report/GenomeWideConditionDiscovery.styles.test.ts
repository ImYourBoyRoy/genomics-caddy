import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./GenomeWideConditionDiscovery.svelte', import.meta.url), 'utf8');

describe('genome-wide disease discovery presentation', () => {
  it('keeps the report surface collapsed and limits the initial condition list', () => {
    expect(source).toContain('<details class="genomewide-discovery summary-card card">');
    expect(source).toContain('conditionGroups.slice(0, visibleLimit)');
    expect(source).toContain('let visibleLimit = $state(8)');
    expect(source).toContain('Matched variants &amp; ClinVar evidence');
  });

  it('keeps POTS as clinical context instead of inventing a DNA association', () => {
    expect(source).toContain("gap.id === 'pots' && gap.clinical_next_step");
    expect(source).toContain('Clinical context · not DNA-scored');
    expect(source).toContain('{assertion.association_is} · {assertion.association_scope}-level');
  });

  it('prevents long disease names and evidence records from forcing horizontal overflow', () => {
    expect(source).toContain('min-width: 0');
    expect(source).toContain('overflow-wrap: anywhere');
    expect(source).toContain('@media (max-width: 580px)');
  });
});
