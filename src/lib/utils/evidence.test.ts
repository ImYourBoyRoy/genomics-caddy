import { describe, expect, it } from 'vitest';
import { getClaimFrame, getDirectionInfo, getScopeLabel, getSeverityInfo, getTierInfo } from './evidence';

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
      cssClass: 'signal-confirm',
    }));
    expect(getScopeLabel('menstrual_cycle_context')).toBe('Menstrual-cycle context');
    expect(getScopeLabel('new_resource_scope')).toBe('new resource scope');
  });
});
