import type {
  EvaluatedMarker,
  GeneratedReport,
  GenomeSample,
} from '../types/genomics';
import allergySensitivityCatalog from '../marker-packs/allergy_sensitivity_catalog.json';
import sourceRegistry from '../marker-packs/source_registry.json';
import { getTierInfo } from './evidence';
import { contextLabels } from './contextIndicators';
import {
  buildLabRequestListText,
  deriveActionablePlan,
  deriveAllergySensitivityGuidance,
  type RecommendationItem,
} from './actionabilityEngine';
import {
  buildConditionCoverageSummaries,
  conditionDiagnosticCapabilityLabel,
  type ConditionEvidenceSummary,
} from './conditionEvidence';
import { getLaypersonTranslation, getSimpleFindingCopy } from './layperson';
import { formatGeneticSexLabel } from './uiLabels';
import {
  clinicalStateLabel,
  inheritanceModelLabel,
  interpretationClassLabel,
  normalizeFindingSemantics,
} from './findingSemantics';
import type { PersonalSafetyContext } from './personalSafetyContext';
import type { ProfileContext } from './profileContext';
import { populatedReproductiveIntake } from './reproductiveIntake';
import { selectedReproductiveContextOption } from './reproductiveContext';
import {
  buildReportReferenceRegistry,
  type ReportReference,
} from './reportReferences';
import { dedupeWarnings, classifyWarnings } from './warningTaxonomy';
import { groupGenomeWideConditionAssociations } from './genomewideConditionDiscovery';
import aiPromptPolicy from '../marker-packs/ai_prompt_policy.json';
import {
  buildAssertionKey,
  callabilityStateForResult,
  callabilityStateLabel,
  orientationStateForResult,
  orientationStateLabel,
} from './callability';
import { derivePrsReadiness } from './prsReadiness';

export type ReportExportAudience = 'personal' | 'clinician' | 'ai';

export interface ReportExportOptions {
  audience: ReportExportAudience;
  report: GeneratedReport;
  sample: GenomeSample;
  /**
   * Deprecated compatibility field. AI and clinician audiences always include
   * raw calls; Personal remains summary-oriented regardless of this value.
   */
  includeRawGenotypes?: boolean;
  /** Explicit user context; never inferred from the DNA report. */
  reproductiveContext?: string;
  /** Explicit profile context kept separate from genetic findings in the export. */
  personalSafetyContext?: PersonalSafetyContext;
  /** Canonical profile dossier; used by bundled exports and Connected Chat. */
  profileContext?: ProfileContext;
}

interface ExportFinding {
  referenceIds: string[];
  section: string;
  title: string;
  direction?: string;
  plainMeaning: string;
  whyItMatters?: string;
  nextStep: string;
  evidence: string;
  uncertainty: string;
  gene?: string;
  rsid?: string;
  variant?: string;
  genotype?: string;
  normalizedGenotype?: string | null;
  effectAllele?: string;
  effectCount?: number | null;
  severity?: string;
  assertionStatus?: string;
  applicability?: string;
  clinicalConfirmation?: string;
  conditionLabel?: string;
  interpretationClass?: string;
  inheritanceModel?: string;
  clinicalState?: string;
  technicalInterpretation?: string;
  claimBoundary?: string;
  callabilityState?: string;
  orientationState?: string;
  assertionKey?: string;
}

const PRIVACY_WARNING = aiPromptPolicy.export_disclosures.privacy_warning;
const AI_REVIEW_INSTRUCTIONS = aiPromptPolicy.export_disclosures.ai_review_instructions;

/** Technical handoffs always retain the calls that produced their findings. */
export function audienceRequiresRawGenotypes(audience: ReportExportAudience): boolean {
  return audience === 'clinician' || audience === 'ai';
}

function clean(value: unknown, fallback = 'Not recorded'): string {
  const text = String(value ?? '').trim();
  return text || fallback;
}

function safeFilePart(value: string): string {
  return clean(value, 'genome')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '_')
    .replace(/^_+|_+$/g, '')
    .slice(0, 64) || 'genome';
}

function nextStepFor(marker: EvaluatedMarker): string {
  if (marker.assertion_status === 'NotEvaluated' || marker.severity_class === 'not_evaluated') {
    return 'This assertion was not scored because its allele or assay rule is not represented by the current evaluator; review the technical coverage before drawing a conclusion.';
  }
  if (marker.clinical_confirmation_required || marker.severity_class === 'confirmation_required') {
    return 'Discuss whether confirmatory clinical testing is appropriate before making health decisions.';
  }
  if (marker.confirm_with.length > 0) {
    return `Discuss the measured follow-up listed in the report: ${marker.confirm_with.join('; ')}.`;
  }
  if (marker.effect_direction === 'context_dependent') {
    return 'Review this alongside symptoms, medications, diet, and other health context; DNA alone cannot determine its effect.';
  }
  return 'Use this as a discussion prompt and review it with personal history rather than treating it as a diagnosis.';
}

function uncertaintyFor(marker: EvaluatedMarker): string {
  if (marker.assertion_status === 'NotEvaluated' || marker.severity_class === 'not_evaluated') {
    return 'The raw call was not converted into a finding because this assertion lacks a matchable allele or assay-specific scoring rule.';
  }
  if (!marker.interpretation_allowed) {
    return 'The interpretation is blocked until the call or allele orientation is verified.';
  }
  if (marker.clinical_confirmation_required || marker.severity_class === 'confirmation_required') {
    return 'This consumer-array result is not sufficient for a clinical conclusion; confirmation may be needed.';
  }
  return 'This is an association or pathway context, not a diagnosis, current measurement, or guaranteed outcome.';
}

function findingFor(
  section: string,
  marker: EvaluatedMarker,
  referenceIds: string[],
  audience: ReportExportAudience,
  includeRawGenotypes: boolean,
): ExportFinding {
  const simple = getLaypersonTranslation(marker);
  const simpleCopy = getSimpleFindingCopy(marker, simple);
  const tier = getTierInfo(marker.evidence_tier);
  const notEvaluated = marker.assertion_status === 'NotEvaluated' || marker.severity_class === 'not_evaluated';
  const finding: ExportFinding = {
    referenceIds,
    section,
    title: audience === 'personal' ? simpleCopy.plain_title : `${marker.gene} ${clean(marker.variant_name, marker.rsid)}`,
    direction: notEvaluated ? 'Not evaluated' : simpleCopy.direction_label,
    plainMeaning: notEvaluated
      ? 'This assertion was not scored because the current evaluator does not have a matchable allele or assay-specific rule for it.'
      : simpleCopy.signal,
    whyItMatters: notEvaluated
      ? 'The row remains available for technical review and must not be treated as benign or negative.'
      : simpleCopy.why_it_matters,
    nextStep: notEvaluated
      ? nextStepFor(marker)
      : audience === 'personal' ? simpleCopy.review_action : nextStepFor(marker),
    evidence: `${tier.label} — ${tier.confidenceLabel}`,
    uncertainty: uncertaintyFor(marker),
  };

  if (audience !== 'personal') {
    const semantics = normalizeFindingSemantics(marker);
    finding.gene = marker.gene;
    finding.rsid = marker.rsid;
    finding.variant = marker.variant_name;
    finding.severity = marker.severity_class;
    finding.assertionStatus = marker.assertion_status;
    finding.assertionKey = marker.assertion_key || buildAssertionKey(marker);
    finding.callabilityState = callabilityStateLabel(
      marker.callability_state || callabilityStateForResult(marker.variant_type, marker.assertion_status),
    );
    finding.orientationState = orientationStateLabel(
      marker.orientation_state || orientationStateForResult(marker.assertion_status, marker.requires_orientation_verification),
    );
    finding.applicability = contextLabels(marker.context_tags, marker.sex_scope).join('; ')
      || 'All users unless context says otherwise';
    finding.clinicalConfirmation = marker.clinical_confirmation_required ? 'Discuss confirmation' : 'Not specifically required by this marker';
    finding.conditionLabel = semantics.condition_label || undefined;
    finding.interpretationClass = interpretationClassLabel(semantics.interpretation_class);
    finding.inheritanceModel = inheritanceModelLabel(semantics.inheritance_model);
    finding.clinicalState = clinicalStateLabel(semantics.clinical_state);
    finding.technicalInterpretation = marker.interpretation;
    const boundaries = dedupeWarnings(classifyWarnings([
      ...marker.do_not_claim,
      marker.raw_dna_limitation || '',
    ])).map((warning) => warning.text);
    finding.claimBoundary = boundaries.join('; ') || 'Interpret with the evidence and context shown in this report.';
    finding.effectAllele = marker.effect_allele;
    finding.effectCount = marker.effect_count;
    if (includeRawGenotypes) {
      finding.genotype = marker.user_genotype;
      finding.normalizedGenotype = marker.normalized_genotype;
    }
  }

  return finding;
}

function renderPersonalContext(options: ReportExportOptions): string {
  const profileContext = options.profileContext;
  const context = profileContext?.safety || options.personalSafetyContext;
  const selectedContext = selectedReproductiveContextOption(
    profileContext?.selectedReproductiveContext || options.reproductiveContext,
  )?.label;
  const lines = [
    `- Selected reproductive context: ${selectedContext || 'Not specified'}`,
    `- Medications (self-reported): ${context?.medications.map((value) => clean(value)).join('; ') || 'None recorded'}`,
    `- Supplements / OTC products (self-reported): ${context?.supplements.map((value) => clean(value)).join('; ') || 'None recorded'}`,
    `- Allergies / intolerances (self-reported): ${context?.allergies.map((value) => clean(value)).join('; ') || 'None recorded'}`,
    `- Symptoms and timing (self-reported): ${context?.symptoms.map((value) => clean(value)).join('; ') || 'None recorded'}`,
    `- Recent labs / clinician findings (self-reported): ${context?.labObservations.map((value) => clean(value)).join('; ') || 'None recorded'}`,
  ];
  if (profileContext) {
    const notes = profileContext.notes;
    if (notes.goals) lines.push(`- Goals (self-reported): ${clean(notes.goals)}`);
    if (notes.challenges) lines.push(`- Challenges and symptoms (self-reported): ${clean(notes.challenges)}`);
    if (notes.relevantBodySystems) lines.push(`- Relevant body systems / life context (self-reported): ${clean(notes.relevantBodySystems)}`);
    if (notes.diet) lines.push(`- Diet pattern (self-reported): ${clean(notes.diet)}`);
    if (notes.diagnoses) lines.push(`- Diagnoses or working diagnoses (self-reported): ${clean(notes.diagnoses)}`);
    if (notes.supportiveTests) lines.push(`- Supportive tests / clinician findings (self-reported): ${clean(notes.supportiveTests)}`);
    if (notes.reproductiveHormoneContext) lines.push(`- Additional hormone / reproductive notes (self-reported): ${clean(notes.reproductiveHormoneContext)}`);
  }
  const intake = populatedReproductiveIntake(context?.reproductiveIntake);
  if (intake.length > 0) {
    lines.push(...intake.map(({ field, value }) => `- ${field.label} (self-reported): ${clean(value)}`));
  }
  if (context?.cycleDiary?.length) {
    lines.push(`- Cycle or symptom diary: ${context.cycleDiary.length} self-reported observation${context.cycleDiary.length === 1 ? '' : 's'}`);
  }
  const dietary = context?.dietaryProfile;
  if (dietary) {
    lines.push(`- Confirmed food allergies: ${dietary.allergies_confirmed.map((value) => clean(value)).join('; ') || 'None recorded'}`);
    lines.push(`- Suspected food reactions: ${dietary.allergies_suspected.map((value) => clean(value)).join('; ') || 'None recorded'}`);
    lines.push(`- Explicit food exclusions: ${dietary.hard_exclusions.map((value) => clean(value)).join('; ') || 'None recorded'}`);
  }
  return [
    '## Explicit personal context',
    '',
    'The following information was supplied by the profile owner. It is not genetic evidence and should not be treated as a diagnosis or medication instruction.',
    '',
    ...lines,
  ].join('\n');
}

function renderConditionSummary(summary: ConditionEvidenceSummary, audience: ReportExportAudience): string {
  const lines = [
    `### ${summary.label}`,
    `- Relative signal: ${summary.relative_signal_label}`,
    `- Indicators: ${summary.matched_indicator_count} of ${summary.coded_indicator_count} aligned; ${summary.callable_indicator_count} callable`,
    `- What this points toward: ${summary.plain_meaning}`,
    `- Look into: ${summary.clinical_route}`,
    `- Evidence: ${summary.evidence_label}; ${summary.direction_summary}`,
    `- Clinical capability: ${conditionDiagnosticCapabilityLabel(summary.diagnostic_capability)}`,
  ];
  if (audience !== 'personal') {
    lines.push(
      `- Genes: ${summary.genes.join(', ') || 'Not recorded'}`,
      `- Matched markers: ${summary.rsids.join(', ') || 'Not recorded'}`,
      ...(summary.interpretation_classes?.length
        ? [`- Interpretation classes: ${summary.interpretation_classes.join(', ')}`]
        : []),
      ...(summary.inheritance_models?.length
        ? [`- Inheritance models: ${summary.inheritance_models.join(', ')}`]
        : []),
      ...(summary.clinical_states?.length
        ? [`- Clinical states: ${summary.clinical_states.join(', ')}`]
        : []),
      `- Reference IDs: ${summary.reference_ids.join(', ') || 'None recorded'}`,
    );
  }
  return lines.join('\n');
}

function renderConditionEvidence(report: GeneratedReport, audience: ReportExportAudience): string {
  const summaries = deriveActionablePlan(report).conditionEvidence;
  if (summaries.length === 0) {
    return '## Potential conditions & health patterns\n\nNo named condition-level DNA signal was matched in this report.';
  }
  return [
    '## Potential conditions & health patterns',
    '',
    'Named DNA signal groups to help prioritize personal or clinical review. Indicator counts describe coverage within this toolkit; they are not probabilities.',
    '',
    summaries.map((summary) => renderConditionSummary(summary, audience)).join('\n\n'),
  ].join('\n');
}

function renderCatalogAssociations(report: GeneratedReport): string {
  const associations = deriveActionablePlan(report).catalogAssociations;
  if (associations.length === 0) {
    return [
      '## Catalog-linked conditions, traits & medication responses',
      '',
      'No named links from the available local ClinVar, GWAS Catalog, or ClinGen records were found for called markers. This can reflect catalog coverage or sync limits; it is not a negative disease result.',
    ].join('\n');
  }
  return [
    '## Catalog-linked conditions, traits & medication responses',
    '',
    'Each link names its source and scope. ClinVar variant assertions, GWAS locus–trait statistics, ClinGen gene–disease validity, and pharmacogenomic response annotations are different evidence types; none by itself establishes a diagnosis, personal risk, or medication recommendation.',
    '',
    ...associations.slice(0, 30).flatMap((association) => [
      `### ${association.label}`,
      `- Relationship: ${association.association_is} (${association.association_scope}-level) · ${association.relationship_label}`,
      `- Source: ${association.source_type} · ${association.marker_count} called ${association.marker_count === 1 ? 'marker' : 'markers'}`,
      `- Evidence: ${association.evidence_summary}`,
      ...(association.clinical_significance ? [`- Clinical significance: ${association.clinical_significance}`] : []),
      ...(association.review_statuses?.length
        ? [`- Review status: ${association.review_statuses.join(', ')}`]
        : []),
      ...(association.allele_match ? [`- Allele match: ${association.allele_match}`] : []),
      ...(association.condition_specific_assertion_available === false
        ? ['- Granularity: ClinVar variant summary; the local index does not retain the condition-specific RCV assertion.']
        : []),
      ...(association.rcv_accessions?.length
        ? [`- RCV accessions on source row (not mapped per condition): ${association.rcv_accessions.join(', ')}`]
        : []),
      ...(association.classification ? [`- Gene–disease validity: ${association.classification}`] : []),
      ...(association.best_p_value != null ? [`- Best reported p-value: ${association.best_p_value}`] : []),
      ...(association.study_accessions?.length ? [`- Study accessions: ${association.study_accessions.join(', ')}`] : []),
      `- Genes: ${association.genes.join(', ') || 'Not recorded'}`,
      `- Markers: ${association.rsids.join(', ') || 'Not recorded'}`,
      `- Source record IDs: ${association.record_ids.join(', ') || 'Not recorded'}`,
      `- Reference IDs: ${association.reference_ids.join(', ') || 'Not recorded'}`,
      ...association.source_urls.slice(0, 3).map((url) => `- Source URL: ${url}`),
      '',
    ]),
    ...(associations.length > 30 ? [`Showing 30 of ${associations.length} links; the complete list is retained in structured JSON.`] : []),
  ].join('\n').trim();
}

function renderGenomewideConditionDiscovery(report: GeneratedReport): string {
  const discovery = report.genomewide_clinvar;
  if (!discovery) return '';
  const groups = groupGenomeWideConditionAssociations(discovery.associations);
  const header = [
    '## Potential disease associations from full-genome ClinVar scan',
    '',
    'Exact allele matches to local ClinVar condition-specific submissions. These are not diagnoses, personal-risk estimates, or a complete disease screen. This scan does not resolve inheritance, phase, penetrance, or clinical fit. Explicit somatic/oncogenic records are excluded; some records may not specify origin.',
    '',
  ];
  if (!discovery.local_index_ready) {
    return [...header, 'Local ClinVar variant and submission indexes were not both ready for this report.'].join('\n');
  }
  if (groups.length === 0) {
    return [
      ...header,
      `No qualifying exact-allele condition submissions were reported. ${discovery.genotypes_scanned.toLocaleString()} imported calls were available for scanning; this does not rule out a condition.`,
    ].join('\n');
  }
  return [
    ...header,
    `- Exact matched variants: ${discovery.exact_variant_count}`,
    `- Matched condition labels: ${groups.length}`,
    `- Allele/build handling: ${clean(discovery.allele_orientation)}`,
    '',
    ...groups.slice(0, 40).flatMap((group) => [
      `### ${clean(group.condition)}`,
      `- Taxonomy topic (navigation only): ${clean(group.categoryLabel)}`,
      `- Matched variants: ${group.variantCount}`,
      ...(group.conflicts ? ['- The ClinVar variant-wide summary contains a conflict, which may span conditions.'] : []),
      ...group.assertions.slice(0, 5).map((assertion) =>
        `- ${clean(assertion.rsid)}${assertion.gene_symbol ? ` (${clean(assertion.gene_symbol)})` : ''}: ${clean(assertion.association_is)} (${clean(assertion.association_scope)} scope); ${clean(assertion.clinical_significance)}; variant-wide summary ${clean(assertion.variant_summary_clinical_significance)}; ${clean(assertion.review_status)}; ${clean(assertion.origin_status)}; [${clean(assertion.scv_accession)}](${clean(assertion.source_url)})`
      ),
      ...(group.assertions.length > 5 ? [`- ${group.assertions.length - 5} additional SCV submissions are retained in the JSON report.`] : []),
      '',
    ]),
    ...(groups.length > 40 ? [`${groups.length - 40} additional condition labels are retained in the JSON report.`] : []),
    ...(discovery.omitted_association_count > 0
      ? [`The source association list was capped; ${discovery.omitted_association_count} matching submissions were omitted from this report payload.`]
      : []),
  ].join('\n');
}

function renderConditionCoverage(report: GeneratedReport): string {
  const summaries = buildConditionCoverageSummaries(report)
    .filter((summary) => summary.status !== 'not_observed');
  const lines = [
    '## Condition coverage',
    '',
    'These counts describe registered condition routes with at least one represented component. They are coverage metadata, not disease probabilities or negative results. The complete zero-coverage registry remains in the JSON handoff.',
    '',
  ];
  if (summaries.length === 0) {
    lines.push('No registered condition route had a represented component in this report.');
    return lines.join('\n').trim();
  }
  for (const summary of summaries) {
    lines.push(
      `### ${summary.label}`,
      `- Status: ${summary.status}`,
      `- Indicators: ${summary.available_indicator_count} available of ${summary.coded_indicator_count}; ${summary.callable_indicator_count} callable; ${summary.matched_indicator_count} aligned`,
      `- Not aligned: ${summary.non_aligned_indicator_count}; not present: ${summary.not_present_indicator_count}; unknown: ${summary.unknown_indicator_count}; blocked: ${summary.blocked_indicator_count}; unsupported: ${summary.not_callable_indicator_count}; absent from report: ${summary.missing_indicator_count}`,
      `- Clinical capability: ${conditionDiagnosticCapabilityLabel(summary.diagnostic_capability)}`,
      `- Route: ${summary.clinical_route}`,
      '',
    );
  }
  return lines.join('\n').trim();
}

function renderPersonalFinding(finding: ExportFinding, index: number): string {
  return [
    `### ${index}. ${finding.title}`,
    `- Area: ${finding.section}`,
    ...(finding.direction ? [`- Direction: ${finding.direction}`] : []),
    `- What this might mean: ${finding.plainMeaning}`,
    ...(finding.whyItMatters ? [`- Why it may matter: ${finding.whyItMatters}`] : []),
    `- Evidence context: ${finding.evidence}`,
    `- Next helpful step: ${finding.nextStep}`,
    `- References: ${finding.referenceIds.join(', ') || 'None recorded'}`,
  ].join('\n');
}

function renderTechnicalFinding(finding: ExportFinding, index: number): string {
  return [
    `### ${index}. ${finding.gene} — ${finding.rsid}`,
    `- Section: ${finding.section}`,
    `- Variant: ${finding.variant}`,
    ...(finding.conditionLabel ? [`- Condition/topic: ${finding.conditionLabel}`] : []),
    `- Interpretation class: ${finding.interpretationClass}`,
    `- Inheritance model: ${finding.inheritanceModel}`,
    `- Clinical state: ${finding.clinicalState}`,
    `- Applicability: ${finding.applicability}`,
    `- Severity class: ${finding.severity}`,
    `- Assertion status: ${finding.assertionStatus}`,
    `- Assertion key: ${finding.assertionKey}`,
    `- Callability: ${finding.callabilityState}`,
    `- Orientation: ${finding.orientationState}`,
    `- Evidence: ${finding.evidence}`,
    ...(finding.direction ? [`- Plain-language direction: ${finding.direction}`] : []),
    `- Effect allele / count: ${finding.effectAllele} / ${finding.effectCount ?? 'Not recorded'}`,
    `- Clinical confirmation: ${finding.clinicalConfirmation}`,
    ...(finding.genotype ? [`- Raw genotype call: ${finding.genotype}`] : []),
    ...(finding.normalizedGenotype ? [`- Normalized genotype: ${finding.normalizedGenotype}`] : []),
    `- Plain-language meaning: ${finding.plainMeaning}`,
    ...(finding.whyItMatters ? [`- Why it may matter: ${finding.whyItMatters}`] : []),
    `- Technical interpretation: ${finding.technicalInterpretation}`,
    `- Claim boundary: ${finding.claimBoundary}`,
    `- Next helpful step: ${finding.nextStep}`,
    `- Uncertainty: ${finding.uncertainty}`,
    `- Reference IDs: ${finding.referenceIds.join(', ') || 'None recorded'}`,
  ].join('\n');
}

function renderReferences(references: ReportReference[]): string {
  if (references.length === 0) return '## Source index\n\nNo sources were attached to the evaluated findings.';
  return [
    '## Source index',
    '',
    ...references.map((reference) => [
      `### ${reference.id} — ${reference.title}`,
      `- Organization: ${reference.organization}`,
      `- Date: ${reference.date}`,
      `- Evidence role: ${reference.evidenceRole}`,
      `- Direct link: ${reference.url || 'No direct link recorded in the local resource'}`,
    ].join('\n')),
  ].join('\n\n');
}

function renderRecommendationItems(
  heading: string,
  items: RecommendationItem[],
  includeProvenance: boolean,
): string[] {
  if (items.length === 0) return [];
  return [
    `### ${heading}`,
    ...items.map((item) => [
      `- **${item.name}** — ${item.why_it_appears}`,
      `  - Evidence: ${item.evidence_level}; relevant: ${item.when_relevant}`,
      ...(includeProvenance ? [
        `  - Basis topics: ${item.basis_topic_ids.join(', ') || 'Not recorded'}`,
        `  - Basis markers / genes: ${item.basis_marker_ids.join(', ') || 'Not recorded'} / ${item.basis_genes.join(', ') || 'Not recorded'}`,
      ] : []),
      ...(item.conflicts.length > 0 ? [`  - Profile conflicts: ${item.conflicts.join('; ')}`] : []),
    ].join('\n')),
  ];
}

function renderActionabilityRecommendations(report: GeneratedReport, audience: ReportExportAudience): string {
  const plan = deriveActionablePlan(report);
  const includeProvenance = audience !== 'personal';
  const sections = [
    '## DNA-linked recommendations',
    '',
    'These items are resource-linked discussion options. Their basis is retained so a reviewer can trace each item to the matched topic and marker IDs.',
    '',
    ...renderRecommendationItems('Food ideas', plan.diet.favorItems, includeProvenance),
    ...renderRecommendationItems('Foods to limit', plan.diet.avoidItems, includeProvenance),
    ...renderRecommendationItems('Supplements to consider', plan.supplements, includeProvenance),
    ...renderRecommendationItems('Supplements to avoid or confirm first', plan.supplementAvoid, includeProvenance),
    ...renderRecommendationItems('Activity prompts', plan.activity.recommendationItems, includeProvenance),
  ];
  if (audience !== 'personal' && plan.labTests.length > 0) {
    sections.push(
      '### Clinician request list',
      '',
      'DNA-linked follow-ups grouped by clinical question. Each item is a discussion prompt, not a universal order.',
      '',
      '```text',
      buildLabRequestListText(plan.labTests),
      '```',
    );
  }
  return sections.length > 3 ? sections.join('\n') : '## DNA-linked recommendations\n\nNo DNA-linked food, supplement, or activity recommendations were generated for this report.';
}

function renderPgxCoverage(report: GeneratedReport): string {
  const coverage = deriveActionablePlan(report).pgxGuidance.componentCoverage;
  if (coverage.length === 0) {
    return '## PGx component coverage\n\nNo curated PGx pathway components were present in this report.';
  }

  const lines = [
    '## PGx component coverage',
    '',
    'These are component-coverage records only. They do not assign a star allele, diplotype, metabolizer phenotype, medication choice, or dose.',
    '',
  ];
  for (const pathway of coverage) {
    lines.push(
      `### ${pathway.label}`,
      `- Coverage status: ${pathway.status}`,
      `- Callable components: ${pathway.callable_marker_ids.join(', ') || 'None recorded'}`,
      `- Components without a usable call: ${pathway.not_present_marker_ids.join(', ') || 'None recorded'}`,
      `- Blocked or unsupported components: ${[...pathway.blocked_marker_ids, ...pathway.not_callable_marker_ids].join(', ') || 'None recorded'}`,
      `- Required components absent from this report: ${pathway.missing_from_report_marker_ids.join(', ') || 'None recorded'}`,
      `- Required clinical inputs: ${pathway.required_inputs.join('; ')}`,
      `- Next step: ${pathway.clinical_next_step}`,
      '- Phenotype or dose from this export: not allowed',
      '',
    );
  }
  return lines.join('\n').trim();
}

function renderPrsReadiness(): string {
  const modules = derivePrsReadiness();
  return [
    '## PRS readiness',
    '',
    'PRS entries remain unscored model specifications. No numeric score was produced.',
    '',
    ...modules.map((module) => [
      `- **${module.trait}** — ${module.status}; policy: ${module.output_policy}`,
      `  - Required and currently missing inputs: ${module.missing_inputs.join('; ')}`,
    ].join('\n')),
  ].join('\n');
}

function renderAllergyGuidance(report: GeneratedReport): string {
  const guidance = deriveAllergySensitivityGuidance(report);
  const lines = [
    '## Allergy & sensitivity map',
    '',
    'Matched DNA pathways with the situations where they are most useful.',
    '',
    '### Matched pathways',
    '',
  ];

  if (guidance.dnaContexts.length > 0) {
    for (const context of guidance.dnaContexts) {
      lines.push(
        `- **${context.label}** — ${context.signal_label} (${context.evidence_label}; ${context.matched_marker_count} matched DNA ${context.matched_marker_count === 1 ? 'finding' : 'findings'})`,
        `  - What it points toward: ${context.summary}`,
        `  - Relevant when: ${context.relevance}`,
        `  - Examples: ${context.examples.join('; ')}`,
        `  - Matched genes: ${context.matched_genes.join(', ') || 'Not recorded'}`,
      );
    }
  } else {
    lines.push('- No active DNA-linked allergy pathway was matched in this report.');
  }

  if (guidance.medicationSafety.length > 0) {
    lines.push('', '### Medication alerts', '');
    for (const route of guidance.medicationSafety) {
      lines.push(
        `- **${route.label}** — ${route.signal_label} (${route.evidence_label}; ${route.matched_marker_count} matched DNA ${route.matched_marker_count === 1 ? 'finding' : 'findings'})`,
        `  - Medicines: ${route.medications.join(', ')}`,
        `  - What it means: ${route.summary}`,
        `  - Relevant when: ${route.relevance}`,
        `  - Matched genes: ${route.matched_genes.join(', ') || 'Not recorded'}`,
      );
    }
  }

  const followUps = Array.from(new Set([
    ...guidance.dnaContexts.map((context) => context.next_step),
    ...guidance.medicationSafety.map((route) => route.next_step),
  ])).filter(Boolean);
  if (followUps.length > 0) {
    lines.push('', '### Focused follow-up', '');
    for (const followUp of followUps.slice(0, 4)) lines.push(`- ${followUp}`);
  }

  const sourceRecords = sourceRegistry.sources as Record<string, { name?: string; url?: string }>;
  const sourceIds = Array.from(new Set([
    ...guidance.dnaContexts.flatMap((context) => context.sources),
    ...guidance.medicationSafety.flatMap((route) => route.sources),
  ]));
  lines.push('', '### Allergy source links', '');
  for (const sourceId of sourceIds) {
    const source = sourceRecords[sourceId];
    if (!source) continue;
    lines.push(source.url
      ? `- [${clean(source.name, sourceId)}](${source.url})`
      : `- ${clean(source.name, sourceId)} (local resource)`);
  }
  lines.push('', allergySensitivityCatalog.display.compact_footer);
  return lines.join('\n');
}

function renderImportProvenance(report: GeneratedReport): string[] {
  const provenance = report.import_provenance;
  if (!provenance) {
    return ['## Import provenance', '', 'No import provenance was recorded for this profile.'];
  }
  const diagnostics = provenance.diagnostics;
  return [
    '## Import provenance',
    '',
    aiPromptPolicy.export_disclosures.import_provenance_notice,
    '',
    `- Import ID: ${clean(provenance.import_id)}`,
    `- Source file: ${clean(provenance.source_file_name)}`,
    `- Source SHA-256: ${clean(provenance.source_file_sha256)}`,
    `- Format/vendor: ${clean(diagnostics.format)} / ${clean(diagnostics.vendor)} (${clean(diagnostics.delimiter)})`,
    `- Source build: ${clean(diagnostics.source_build)}`,
    `- Coordinate system: ${clean(diagnostics.coordinate_system)}`,
    `- Allele orientation: ${clean(diagnostics.allele_orientation)}`,
    `- Rows: ${diagnostics.accepted_rows} accepted; ${diagnostics.malformed_rows} malformed; ${diagnostics.duplicate_rows} duplicate`,
    `- Liftover: ${provenance.liftover_mapped_rows} mapped; ${provenance.liftover_unmapped_rows} unmapped`,
  ];
}

export function buildReportAudienceMarkdown(options: ReportExportOptions): string {
  const includeRawGenotypes = audienceRequiresRawGenotypes(options.audience);
  const registry = buildReportReferenceRegistry(options.report);
  const findings = options.report.sections.flatMap((section) =>
    section.markers
      .filter((marker) => options.audience !== 'personal' || !['benign', 'no_data'].includes(marker.severity_class))
      .map((marker) => findingFor(
        section.name,
        marker,
        registry.idsByMarker.get(`${section.name}:${marker.link_id}`) || [],
        options.audience,
        includeRawGenotypes,
      )),
  );
  const generatedAt = clean(options.report.generated_at);
  const header = [
    options.audience === 'personal' ? '# Personal Simple Genomics Report' :
      options.audience === 'clinician' ? '# Clinician Handoff — Genomics Report' : '# AI Review — Genomics Report',
    '',
    `- Profile: ${clean(options.sample.name)}`,
    `- Chromosome pattern: ${formatGeneticSexLabel(options.sample.genetic_sex)}`,
    `- Report generated: ${generatedAt}`,
    '- Data source: local consumer-array interpretation; not a clinical laboratory report',
    '',
    PRIVACY_WARNING,
  ];

  if (options.audience === 'personal') {
    return [
      ...header,
      '',
      '## How to use this report',
      '',
      'These are plain-language research prompts. They do not diagnose a condition, measure current hormone levels, or tell you to start, stop, or change a medication or supplement.',
      '',
      renderPersonalContext(options),
      '',
      renderConditionEvidence(options.report, options.audience),
      '',
      renderCatalogAssociations(options.report),
      '',
      renderGenomewideConditionDiscovery(options.report),
      '',
      renderAllergyGuidance(options.report),
      '',
      renderActionabilityRecommendations(options.report, options.audience),
      '',
      '## Findings',
      '',
      findings.length > 0 ? findings.map(renderPersonalFinding).join('\n\n') : 'No evaluated findings were available.',
      '',
      renderReferences(registry.references),
    ].join('\n');
  }

  const technicalBody = findings.length > 0
    ? findings.map(renderTechnicalFinding).join('\n\n')
    : 'No evaluated findings were available.';
  const audienceBoundary = options.audience === 'ai'
    ? ['## AI review instructions', '', AI_REVIEW_INSTRUCTIONS]
    : ['## Clinician handoff boundary', '', aiPromptPolicy.export_disclosures.clinician_handoff_boundary];

  return [
    ...header,
    '',
    ...audienceBoundary,
    '',
    ...renderImportProvenance(options.report),
    '',
    renderPersonalContext(options),
    '',
    renderConditionEvidence(options.report, options.audience),
    '',
    renderCatalogAssociations(options.report),
    '',
    renderGenomewideConditionDiscovery(options.report),
    '',
    renderConditionCoverage(options.report),
    '',
    renderAllergyGuidance(options.report),
    '',
    renderPgxCoverage(options.report),
    '',
    renderPrsReadiness(),
    '',
    renderActionabilityRecommendations(options.report, options.audience),
    '',
    '## Structured findings',
    '',
    technicalBody,
    '',
    renderReferences(registry.references),
  ].join('\n');
}

export function reportAudienceFilename(sampleName: string, audience: ReportExportAudience): string {
  return `${safeFilePart(sampleName)}_${audience}_report.md`;
}

export const reportExportPrivacyWarning = PRIVACY_WARNING;
