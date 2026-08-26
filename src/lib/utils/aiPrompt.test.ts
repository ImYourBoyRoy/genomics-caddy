import { describe, expect, it, vi } from 'vitest';

vi.mock('./markerPacksState.svelte', () => ({
  markerPacksStore: {
    manifest: { packs: [{ id: 'hormones_reproductive', label: 'Hormone & Reproductive Context' }] },
    packs: {},
  },
}));

import { markerPacksStore } from './markerPacksState.svelte';
import { buildMarkerPayload, buildSystemPrompt, CONSULTATION_MODES, DEFAULT_INSTRUCTIONS, getActiveCategories, getDynamicQuestions } from './aiPrompt';
import type { EvaluatedMarker, GeneratedReport, GenomeSample } from '../types/genomics';
import type { PersonalSafetyContext } from './personalSafetyContext';

describe('AI marker payload claim boundaries', () => {
  it('uses the resource-authored default policy for probability and safety language', () => {
    expect(DEFAULT_INSTRUCTIONS).toContain('More markers increase coverage, not certainty.');
    expect(DEFAULT_INSTRUCTIONS).toContain('Do not describe the late-luteal or menstrual transition as an estrogen spike.');
    expect(DEFAULT_INSTRUCTIONS).toContain('never recommend starting/stopping a medication or supplement');
  });

  it('keeps hormone consultation inclusive while preserving phenotype and clinical boundaries', () => {
    expect(CONSULTATION_MODES.hormones_reproductive.label).toBe('Hormone & Reproductive Context');
    expect(CONSULTATION_MODES.hormones_reproductive.instructions).toContain('different bodies');
    expect(CONSULTATION_MODES.hormones_reproductive.instructions).toContain('not a diagnosis');
    expect(CONSULTATION_MODES.hormones_reproductive.instructions).toContain('exact contraceptive or hormone product');
  });

  it('derives quick helper prompts from resource-authored relevance IDs', () => {
    const questions = getDynamicQuestions({
      hormones_reproductive: true,
      activity_recovery: true,
      food_supplement_safety: true,
    });

    expect(questions.map((question) => question.label)).toEqual([
      '🥗 Food & Supplement Safety',
      '🏃 Activity & Recovery Context',
      '🌸 Hormone & Reproductive Context',
    ]);
    expect(getDynamicQuestions({})).toEqual([{
      label: '🧬 Genomic Overview',
      text: 'Give me a high-level summary of the active marker findings in my profile and what they mean.',
    }]);
  });

  it('activates the hormone helper from a selected pack and active callable finding', () => {
    const categories = getActiveCategories({
      sections: [{
        name: 'Hormone & Reproductive Context',
        markers: [{
          rsid: 'rs-example',
          gene: 'ESR1',
          user_genotype: 'AG',
          effect_count: 1,
        }],
      }],
    } as unknown as GeneratedReport, { hormones_reproductive: true });

    expect(categories.hormones_reproductive).toBe(true);
    expect(categories.food_supplement_safety).toBe(false);
    expect(markerPacksStore.manifest.packs).toHaveLength(1);
  });

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
    expect(payload.layperson_summary?.simple_impact).toBe('Genetic context marker');
    expect(payload.layperson_summary?.simple_meaning).toContain('probabilistic association');
  });

  it('injects explicitly supplied per-profile safety context without treating it as genotype evidence', () => {
    const personalSafetyContext: PersonalSafetyContext = {
      medications: ['norethindrone'],
      supplements: ['magnesium'],
      allergies: ['fish'],
      symptoms: ['cycle-linked mood changes'],
      labObservations: ['ferritin 18 ng/mL'],
      reproductiveIntake: {
        symptom_onset_timing: 'about 5 days before bleeding',
        mood_behavior_symptoms: 'needs quiet and becomes avoidant',
        active_ingredients: 'norethindrone 0.35 mg',
        question_or_belief_to_verify: 'I think this is progesterone-only; verify the label',
      },
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
    expect(prompt).toContain('active_ingredients');
    expect(prompt).toContain('I think this is progesterone-only');
    expect(prompt).toContain('reproductive_context_intake');
    expect(prompt).toContain('self-reported context for this DNA profile, not genotype evidence');
  });
});
