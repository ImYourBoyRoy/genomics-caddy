import { describe, expect, it } from 'vitest';
import { buildSupportResourceContext } from './supportResourceContext';

describe('support resource context', () => {
  it('selects reproductive phenotype and lab resources without inventing a diagnosis', () => {
    const context = buildSupportResourceContext({
      packIds: ['hormones_reproductive'],
      consultationMode: 'hormones_reproductive',
    });

    expect(context.phenotype_prompts.some((domain) => domain.id === 'hormones_reproductive')).toBe(true);
    expect(context.lab_overlays.some((overlay) => overlay.domain === 'adenomyosis_heavy_bleeding_pelvic_pain')).toBe(true);
    expect(context.safety_guardrails.some((rule) => rule.id === 'ADENOMYOSIS_REQUIRES_GYNECOLOGIC_WORKUP')).toBe(true);
    expect(context.medication_context.ask_for.some((item) => item.includes('active ingredient'))).toBe(true);
    expect(context.supplement_safety.rules.some((rule) => rule.id === 'selenium')).toBe(true);
    expect(context.food_safety.profile_minimum_required.length).toBeGreaterThan(0);
    expect(context.food_safety.source_registry.nih_ods_selenium).toBeDefined();
    expect(context.food_safety.conditional_questions.pmdd_cycle).toBeDefined();
  });

  it('selects PGx confirmation resources and retains medication safety priorities', () => {
    const context = buildSupportResourceContext({ packIds: ['pgx'], consultationMode: 'pgx' });

    expect(context.lab_overlays.some((overlay) => overlay.domain === 'pgx')).toBe(true);
    expect(context.callability_rules.some((rule) => rule.id === 'STAR_ALLELE_DIPLOTYPE_REQUIRED')).toBe(true);
    expect(context.safety_guardrails.some((rule) => rule.id === 'PGX_NO_MED_CHANGE')).toBe(true);
    expect(context.food_safety.priority_order[1]).toContain('medication');
    expect(context.supplement_safety.do_not_do.some((item) => item.includes('common SNP'))).toBe(true);
    expect(context.food_safety.nutrient_matrix.length).toBeGreaterThan(0);
  });

  it('keeps PRS and common-marker evidence probabilistic', () => {
    const context = buildSupportResourceContext({ packIds: ['cardiovascular'] });

    expect(context.evidence_policy.claim_policy.never_claim).toContain('diagnosis from raw DNA');
    expect(context.prs_policy.principle).toContain('Do not compute');
    expect(context.actionability_policy.safety_notes?.[0]).toContain('More markers increase coverage');
    expect(context.activity_safety.relevant_domains.some((domain) => domain.id === 'cardiovascular')).toBe(true);
    expect(context.activity_safety.stop_and_escalate.some((item) => item.includes('Chest pain'))).toBe(true);
  });
});
