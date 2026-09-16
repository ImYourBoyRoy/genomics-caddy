import type {
  ClinicalSemantics,
  EvaluatedMarker,
  FindingClinicalState,
  FindingInheritanceModel,
  FindingInterpretationClass,
} from '../types/genomics';

export interface NormalizedFindingSemantics {
  condition_label: string | null;
  interpretation_class: FindingInterpretationClass;
  inheritance_model: FindingInheritanceModel;
  clinical_state: FindingClinicalState;
  evidence_level: string;
  clinical_confirmation_required: boolean;
  applicability_scopes: string[];
}

const INTERPRETATION_LABELS: Record<FindingInterpretationClass, string> = {
  susceptibility_context: 'Susceptibility context',
  carrier_possibility: 'Carrier possibility',
  clinically_actionable_variant: 'Clinically actionable variant',
  research_context: 'Research context',
  protective_context: 'Protective context',
  trait_context: 'Trait context',
  unknown: 'Not classified',
};

const INHERITANCE_LABELS: Record<FindingInheritanceModel, string> = {
  autosomal_dominant: 'Autosomal dominant',
  autosomal_recessive: 'Autosomal recessive',
  x_linked: 'X-linked',
  y_linked: 'Y-linked',
  mitochondrial: 'Mitochondrial',
  unknown: 'Not established',
};

const CLINICAL_STATE_LABELS: Record<FindingClinicalState, string> = {
  clinically_confirmed: 'Clinically confirmed',
  carrier_possibility: 'Carrier possibility',
  unknown: 'Not determined from this DNA result',
  not_applicable: 'Not applicable',
};

function nonEmpty(value: unknown): string | null {
  const text = String(value ?? '').trim();
  return text || null;
}

function evidenceStartsWith(marker: EvaluatedMarker, letter: string): boolean {
  return String(marker.evidence_tier || '').trim().toUpperCase().startsWith(letter);
}

function confirmationRequired(marker: EvaluatedMarker): boolean {
  return marker.clinical_confirmation_required === true
    || marker.severity_class === 'confirmation_required';
}

function hasActionableEvidence(marker: EvaluatedMarker): boolean {
  const tier = String(marker.evidence_tier || '').trim().toUpperCase();
  const text = `${marker.gene} ${marker.variant_name} ${marker.impact} ${marker.interpretation}`.toLowerCase();
  return /PHARMACOGENOMIC|CLINICALLY_ACTIONABLE|DRUG_SAFETY|HIGH_STAKES/.test(tier)
    || (tier.startsWith('A') && /hypersensitivity|severe toxicity|drug-specific|pathogenic|loss.of.function/.test(text));
}

function derivedInterpretationClass(marker: EvaluatedMarker): FindingInterpretationClass {
  if (!marker.interpretation_allowed || marker.severity_class === 'no_data' || marker.severity_class === 'not_evaluated') return 'unknown';
  if (marker.effect_direction === 'protective') return 'protective_context';
  if (marker.effect_direction === 'trait') return 'trait_context';
  if (confirmationRequired(marker) && hasActionableEvidence(marker)) return 'clinically_actionable_variant';
  if (marker.effect_direction === 'risk') {
    return evidenceStartsWith(marker, 'D') || evidenceStartsWith(marker, 'E')
      ? 'research_context'
      : 'susceptibility_context';
  }
  if (marker.effect_direction === 'context_dependent') return 'research_context';
  return 'unknown';
}

/**
 * Normalize optional pack-authored semantics and derive only conservative
 * classifications from existing report fields. In particular, this function
 * never infers active disease, carrier status, inheritance, or diagnosis from
 * a genotype, gene name, or free-text disease label.
 */
export function normalizeFindingSemantics(marker: EvaluatedMarker): NormalizedFindingSemantics {
  const authored: ClinicalSemantics = marker.clinical_semantics || {};
  const interpretationClass = authored.interpretation_class || derivedInterpretationClass(marker);
  const clinicalState = authored.clinical_state
    || (interpretationClass === 'carrier_possibility' ? 'carrier_possibility' : 'unknown');
  const catalogCondition = marker.clingen?.disease_label && confirmationRequired(marker)
    ? nonEmpty(marker.clingen.disease_label)
    : null;

  return {
    condition_label: nonEmpty(authored.condition_label) || catalogCondition,
    interpretation_class: interpretationClass,
    inheritance_model: authored.inheritance_model || 'unknown',
    clinical_state: clinicalState,
    evidence_level: marker.evidence_tier,
    clinical_confirmation_required: confirmationRequired(marker),
    applicability_scopes: marker.sex_scope && marker.sex_scope !== 'all'
      ? [marker.sex_scope]
      : [],
  };
}

export function interpretationClassLabel(value: FindingInterpretationClass): string {
  return INTERPRETATION_LABELS[value];
}

export function inheritanceModelLabel(value: FindingInheritanceModel): string {
  return INHERITANCE_LABELS[value];
}

export function clinicalStateLabel(value: FindingClinicalState): string {
  return CLINICAL_STATE_LABELS[value];
}
