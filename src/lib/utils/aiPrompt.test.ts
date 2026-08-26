import { describe, expect, it, vi } from 'vitest';

vi.mock('./markerPacksState.svelte', () => ({
  markerPacksStore: {
    manifest: { packs: [] },
    packs: {},
  },
}));

import { buildMarkerPayload } from './aiPrompt';
import type { EvaluatedMarker } from '../types/genomics';

describe('AI marker payload claim boundaries', () => {
  it('keeps confirmation, limitations, callability, and source context beside the interpretation', () => {
    const marker = {
      rsid: 'rs-example',
      gene: 'EXAMPLE',
      variant_name: 'example marker',
      user_genotype: 'AG',
      effect_allele: 'A',
      effect_count: 1,
      effect_direction: 'context_dependent',
      evidence_tier: 'C_candidate_OR_mechanistic',
      severity_class: 'context_dependent',
      assertion_status: 'Verified',
      interpretation_allowed: true,
      impact: 'Example context',
      interpretation: 'This is an association context.',
      do_not_claim: ['diagnosis from raw DNA'],
      confirm_with: ['symptom history'],
      raw_dna_limitation: 'A consumer SNP is incomplete.',
      clinical_confirmation_required: true,
      sources: [{ name: 'Example source' }],
    } as unknown as EvaluatedMarker;

    const payload = buildMarkerPayload(marker, {});

    expect(payload.claim_boundaries).toEqual({
      do_not_claim: ['diagnosis from raw DNA'],
      confirm_with: ['symptom history'],
      raw_dna_limitation: 'A consumer SNP is incomplete.',
      clinical_confirmation_required: true,
    });
    expect(payload.assertion_status).toBe('Verified');
    expect(payload.interpretation_allowed).toBe(true);
    expect(payload.source_names).toEqual(['Example source']);
  });
});
