import { describe, expect, it } from 'vitest';
import { getClaimFrame, getDirectionInfo, getScopeLabel, getSeverityInfo, getSimpleSectionSummaryParts, getSimpleTierLabel, getTierInfo } from './evidence';

describe('evidence tier display', () => {
  it('uses the tier prefix for custom evidence labels', () => {
    expect(getTierInfo('B_replicated_common_marker_with_confounding').label).toContain('Tier B');
    expect(getTierInfo('C_biohacker_hypothesis_WITH_STRONG_GUARDRAIL').label).toContain('Tier C');
  });

  it('does not present evidence gaps as research findings', () => {
    const info = getTierInfo('E_negative_evidence_or_gap');
    expect(info.label).toContain('Guardrail');
    expect(info.description).toContain('unsupported claim');
  });

  it('flags unknown evidence labels instead of silently downgrading them to Tier D', () => {
    const info = getTierInfo('not_a_real_tier');
    expect(info.colorClass).toBe('tier-unknown');
    expect(info.confidenceLabel).toContain('Unclassified');
  });

  it('keeps the primary Simple evidence label concise', () => {
    expect(getSimpleTierLabel('A_clinical')).toBe('Higher evidence');
    expect(getSimpleTierLabel('D_research')).toBe('Research only');
    expect(getSimpleTierLabel('not_a_real_tier')).toBe('Evidence context');
  });

  it('frames raw-SNP claims separately from clinical actionability', () => {
    expect(getClaimFrame('B_replicated_common_marker')).toContain('not a personal disease probability');
    expect(getClaimFrame('A_clinical_guideline', true)).toContain('not a diagnosis');
    expect(getClaimFrame('E_negative_evidence_or_gap')).toContain('evidence gap');
  });

  it('loads direction and severity display semantics from the evidence policy resource', () => {
    expect(getDirectionInfo('context_dependent')).toEqual(expect.objectContaining({
      label: 'Context-Dependent',
      plainLabel: 'Depends on context',
      colorClass: 'direction-context_dependent',
    }));
    expect(getSeverityInfo('confirmation_required')).toEqual(expect.objectContaining({
      label: 'Clinical confirmation needed',
      plainLabel: 'Needs clinical lab test to verify',
      legendLabel: 'Needs Confirmation',
      cssClass: 'signal-confirm',
    }));
    expect(getSeverityInfo('not_evaluated')).toEqual(expect.objectContaining({
      label: 'Not evaluated',
      plainLabel: 'Needs an assay-specific rule',
      legendLabel: 'Not Evaluated',
    }));
    expect(getScopeLabel('menstrual_cycle_context')).toBe('Menstrual-cycle context');
    expect(getScopeLabel('new_resource_scope')).toBe('new resource scope');
  });

  it('uses plain-language section summary labels only in Simple mode', () => {
    const summary = {
      risk_effect_count: 2,
      risk_possible: 2,
      protective_effect_count: 1,
      protective_possible: 1,
      trait_count: 1,
      context_dependent_count: 1,
      no_data_count: 0,
      confirmation_required_count: 2,
      total_markers: 9,
      show_percent_score: false,
      all_require_confirmation: false,
      active_marker_count: 7,
      active_risk_marker_count: 2,
      active_protective_marker_count: 1,
      active_trait_marker_count: 1,
      active_context_marker_count: 1,
      blocked_unverified_count: 1,
      benign_modifier_count: 1,
    };

    expect(getSimpleSectionSummaryParts(summary)).toEqual([
      '2 possible associations',
      '1 possible protective association',
      '1 trait finding',
      '1 context finding',
      '1 needs review',
      '2 need confirmation',
    ]);
  });

  it('keeps not-evaluated assertions distinct from absent DNA calls', () => {
    const summary = {
      risk_effect_count: 0,
      risk_possible: 0,
      protective_effect_count: 0,
      protective_possible: 0,
      trait_count: 0,
      context_dependent_count: 0,
      no_data_count: 0,
      not_evaluated_count: 2,
      confirmation_required_count: 0,
      total_markers: 2,
      show_percent_score: false,
      all_require_confirmation: false,
      active_marker_count: 0,
      active_risk_marker_count: 0,
      active_protective_marker_count: 0,
      active_trait_marker_count: 0,
      active_context_marker_count: 0,
      blocked_unverified_count: 0,
      benign_modifier_count: 0,
    };

    expect(getSimpleSectionSummaryParts(summary)).toEqual(['2 not evaluated']);
  });
});
