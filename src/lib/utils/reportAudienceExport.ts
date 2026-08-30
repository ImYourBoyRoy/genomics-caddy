import type {
  EvaluatedMarker,
  GeneratedReport,
  GenomeSample,
} from '../types/genomics';
import allergySensitivityCatalog from '../marker-packs/allergy_sensitivity_catalog.json';
import sourceRegistry from '../marker-packs/source_registry.json';
import { getScopeLabel, getTierInfo } from './evidence';
import {
  buildLabRequestListText,
  deriveActionablePlan,
  deriveAllergySensitivityGuidance,
  type RecommendationItem,
} from './actionabilityEngine';
import { getLaypersonTranslation, getSimpleFindingTitle } from './layperson';
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

export type ReportExportAudience = 'personal' | 'clinician' | 'ai';

export interface ReportExportOptions {
  audience: ReportExportAudience;
  report: GeneratedReport;
  sample: GenomeSample;
  /** Raw calls are opt-in for every audience; personal exports default to false. */
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
  plainMeaning: string;
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
}

const PRIVACY_WARNING =
  '> Privacy: This file was generated locally. Treat it as sensitive health and genetic information; share it only with the intended recipient. DNA calls are not a diagnosis.';

const AI_REVIEW_INSTRUCTIONS = [
  'Do not diagnose, assign disease probability, infer missing facts, or treat association markers as proof of a condition.',
  'Separate raw genotype calls, self-reported context, measured clinical data, and research interpretations.',
  'Do not infer current hormone levels, anatomy, pregnancy status, medication composition, or treatment response from DNA alone.',
  'Treat medication, supplement, pregnancy, and symptom guidance as clinician-discussion prompts only.',
].join(' ');

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
  const tier = getTierInfo(marker.evidence_tier);
  const finding: ExportFinding = {
    referenceIds,
    section,
    title: audience === 'personal' ? getSimpleFindingTitle(simple.simpleImpact) : `${marker.gene} ${clean(marker.variant_name, marker.rsid)}`,
    plainMeaning: simple.simpleMeaning,
    nextStep: nextStepFor(marker),
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
    finding.applicability = marker.sex_scope ? getScopeLabel(marker.sex_scope) : 'All users unless context says otherwise';
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

function renderPersonalFinding(finding: ExportFinding, index: number): string {
  return [
    `### ${index}. ${finding.title}`,
    `- Area: ${finding.section}`,
    `- What this might mean: ${finding.plainMeaning}`,
    `- Evidence context: ${finding.evidence}`,
    `- Next helpful step: ${finding.nextStep}`,
    `- Uncertainty: ${finding.uncertainty}`,
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
    `- Evidence: ${finding.evidence}`,
    `- Effect allele / count: ${finding.effectAllele} / ${finding.effectCount ?? 'Not recorded'}`,
    `- Clinical confirmation: ${finding.clinicalConfirmation}`,
    ...(finding.genotype ? [`- Raw genotype call: ${finding.genotype}`] : []),
    ...(finding.normalizedGenotype ? [`- Normalized genotype: ${finding.normalizedGenotype}`] : []),
    `- Plain-language meaning: ${finding.plainMeaning}`,
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

export function buildReportAudienceMarkdown(options: ReportExportOptions): string {
  const includeRawGenotypes = options.includeRawGenotypes === true;
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
    `- Sex: ${formatGeneticSexLabel(options.sample.genetic_sex)}`,
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
    : ['## Clinician handoff boundary', '', 'Please verify clinically important findings with history, examination, validated laboratory testing, and current authoritative guidance. Do not use consumer-array calls alone for diagnosis or treatment decisions.'];

  return [
    ...header,
    '',
    ...audienceBoundary,
    '',
    renderPersonalContext(options),
    '',
    renderAllergyGuidance(options.report),
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
