import { describe, expect, it, vi } from 'vitest';

vi.mock('./markerPacksState.svelte', () => ({
  markerPacksStore: {
    manifest: { packs: [] },
    packs: {},
  },
}));

import { buildMarkerPayload, buildSystemPrompt } from './aiPrompt';
import type { EvaluatedMarker, GeneratedReport, GenomeSample } from '../types/genomics';
import type { PersonalSafetyContext } from './personalSafetyContext';

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

  it('injects explicitly supplied per-profile safety context without treating it as genotype evidence', () => {
    const personalSafetyContext: PersonalSafetyContext = {
      medications: ['norethindrone'],
      supplements: ['magnesium'],
      allergies: ['fish'],
      symptoms: ['cycle-linked mood changes'],
      labObservations: ['ferritin 18 ng/mL'],
    };
    const prompt = buildSystemPrompt({
      selectedSample: { id: 7, name: 'Example', genetic_sex: 'unknown' } as unknown as GenomeSample,
      generatedReport: { sections: [] } as unknown as GeneratedReport,
      selectedPacks: {},
      onlyActiveFindings: true,
      contextMode: 'active_findings',
      consultationMode: 'general',
      userProfile: {
        goals: '',
        challenges: '',
        relevantBodySystems: '',
        reproductiveHormoneContext: '',
        diet: '',
        supplements: '',
        medications: '',
        bloodwork: '',
        diagnoses: '',
        supportiveTests: '',
        injectProfile: true,
      },
      personalSafetyContext,
      systemInstructions: 'Test instructions',
      laypersonMap: {},
    });

    expect(prompt).toContain('personal_safety_context');
    expect(prompt).toContain('norethindrone');
    expect(prompt).toContain('self-reported context for this DNA profile, not genotype evidence');
  });
});
