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
      link_id: 'test:example',
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
    expect(payload.link_id).toBe('test:example');
    expect(JSON.parse(payload.assertion_key).version).toBe(1);
    expect(payload.reference_ids).toEqual([]);
    expect(payload.callability_state).toBe('callable');
    expect(payload.orientation_state).toBe('not_required');
    expect(payload.callability_policy.scoring_policy).toBe('snp_allele_count');
    expect(payload.clinical_semantics).toEqual({
      condition_label: null,
      interpretation_class: 'research_context',
      inheritance_model: 'unknown',
      clinical_state: 'unknown',
      evidence_level: 'C_candidate_OR_mechanistic',
      clinical_confirmation_required: true,
      applicability_scopes: [],
    });
    expect(payload.source_names).toEqual(['Example source']);
    expect(payload.layperson_summary?.simple_impact).toBe('Genetic research signal');
    expect(payload.layperson_summary?.simple_meaning).toContain('biological pathway signal');
    expect(payload.layperson_summary?.direction).toBe('Context-dependent');
    expect(payload.layperson_summary?.review_action).toBe('Review the related clinical test route.');
    expect(payload.layperson_summary?.simple_meaning).not.toContain('does not predict whether you have a condition');
    expect(payload.layperson_summary?.simple_meaning).not.toContain('This is an association context.');
    expect(payload.layperson_summary?.simple_meaning).not.toContain('A consumer SNP is incomplete.');

    const hlaPayload = buildMarkerPayload({
      ...marker,
      variant_type: 'hla_tag',
      effect_allele: 'A',
    } as unknown as EvaluatedMarker, {});
    expect(hlaPayload.callability_policy.scoring_policy).toBe('not_evaluated');
    expect(hlaPayload.callability_policy.display_policy).toBe('context_only');
    expect(hlaPayload.callability_state).toBe('not_callable');
  });

  it('always exposes exact raw calls in the AI payload for included findings', () => {
    const marker = {
      rsid: 'rs-ai-raw',
      gene: 'AI_RAW',
      variant_name: 'AI raw contract marker',
      user_genotype: 'AG',
      normalized_genotype: 'AG',
      effect_allele: 'A',
      effect_count: 1,
      effect_direction: 'risk',
      evidence_tier: 'B_replicated_common_marker',
      severity_class: 'moderate_risk',
      assertion_status: 'Verified',
      interpretation_allowed: true,
      sources: [],
      do_not_claim: [],
      confirm_with: [],
    } as unknown as EvaluatedMarker;

    expect(buildMarkerPayload(marker, {})).toMatchObject({
      user_genotype: 'AG',
      normalized_genotype: 'AG',
      genotype: 'AG',
    });
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
      cycleDiary: [{
        id: 'day-1',
        values: {
          entry_date: '2026-08-01',
          mood_behavior_score: '3',
          mood_behavior_notes: 'needs quiet',
        },
      }],
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
    expect(prompt).toContain('cycle_symptom_diary');
    expect(prompt).toContain('needs quiet');
    expect(prompt).toContain('self-reported context for this DNA profile, not genotype evidence');
  });

  it('injects named condition evidence with coverage counts into the AI context', () => {
    const prompt = buildSystemPrompt({
      selectedSample: { id: 7, name: 'Example', genetic_sex: 'unknown' } as unknown as GenomeSample,
      generatedReport: {
        sections: [{
          name: 'Hormones',
          markers: [{
            rsid: 'rs2234693',
            gene: 'ESR1',
            link_id: 'test:pmdd',
            user_genotype: 'SYNTHETIC_CALL',
            normalized_genotype: 'SYNTHETIC_CALL',
            effect_allele: 'A',
            effect_count: 1,
            evidence_tier: 'B_replicated_common_marker',
            effect_direction: 'risk',
            severity_class: 'moderate_risk',
            assertion_status: 'Verified',
            interpretation_allowed: true,
            sources: [],
            db_enriched_sources: [],
            confirm_with: [],
            do_not_claim: [],
          } as unknown as EvaluatedMarker],
        }],
      } as unknown as GeneratedReport,
      selectedPacks: {},
      onlyActiveFindings: true,
      contextMode: 'active_findings',
      consultationMode: 'general',
      userProfile: {
        goals: '', challenges: '', relevantBodySystems: '', reproductiveHormoneContext: '', diet: '',
        supplements: '', medications: '', bloodwork: '', diagnoses: '', supportiveTests: '', injectProfile: false,
      },
      systemInstructions: 'Test instructions',
      laypersonMap: {},
    });

    expect(prompt).toContain('condition_evidence');
    expect(prompt).toContain('condition_coverage');
    expect(prompt).toContain('PMDD-related steroid sensitivity');
    expect(prompt).toContain('coded_indicator_count');
    expect(prompt).toContain('matched_indicator_count');
    expect(prompt).toContain('not probabilities');
    const payload = JSON.parse(prompt.slice(prompt.indexOf('[JSON CONTEXT]') + '[JSON CONTEXT]'.length));
    expect(payload.sample_context.raw_genotypes_included).toBe(true);
    expect(payload.sample_context.sections[0].findings[0]).toHaveProperty('user_genotype');
  });

  it('does not route or include canonical profile context when Chat context is disabled', () => {
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
        injectProfile: false,
      },
      reproductiveContext: 'cyclic_mood_symptoms',
      personalSafetyContext: {
        medications: ['private medication'],
        supplements: ['private supplement'],
        allergies: ['private allergy'],
        symptoms: ['private symptom'],
        labObservations: ['private lab'],
        cycleDiary: [{ id: 'private-entry', values: { entry_date: '2026-08-01' } }],
      },
      systemInstructions: 'Test instructions',
      laypersonMap: {},
    });

    const payload = JSON.parse(prompt.slice(prompt.indexOf('[JSON CONTEXT]') + '[JSON CONTEXT]'.length));
    expect(payload.sample_context.profile).toBeUndefined();
    expect(payload.sample_context.personal_safety_context).toBeUndefined();
    expect(payload.support_resources.context_routing.profile_context_ids).toEqual([]);
    expect(payload.support_resources.cycle_support.active_context_ids).toEqual([]);
    expect(payload.support_resources.cycle_support.selected_context_id).toBeNull();
    expect(payload.support_resources.cycle_support.diary_review).toBeNull();
  });
});
