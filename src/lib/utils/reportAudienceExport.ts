import type {
  EnrichedSource,
  EvaluatedMarker,
  GeneratedReport,
  GenomeSample,
  MarkerSource,
} from '../types/genomics';
import { getScopeLabel, getTierInfo } from './evidence';
import { getLaypersonTranslation } from './layperson';
import type { PersonalSafetyContext } from './personalSafetyContext';
import { populatedReproductiveIntake } from './reproductiveIntake';
import { selectedReproductiveContextOption } from './reproductiveContext';

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
}

interface ReportReference {
  id: string;
  title: string;
  organization: string;
  date: string;
  evidenceRole: string;
  url: string | null;
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

function sourceKey(source: MarkerSource | EnrichedSource): string {
  const url = 'name' in source ? source.url : source.url;
  if (url?.trim()) {
    return `url:${url.trim().replace(/\/$/, '').toLowerCase()}`;
  }
  if ('name' in source) {
    return [source.name, source.url || '', source.evidence_type || '', source.notes || '']
      .map((value) => clean(value))
      .join('|')
      .toLowerCase();
  }
  return [source.source_type, source.citation, source.url || '', source.details || '']
    .map((value) => clean(value))
    .join('|')
    .toLowerCase();
}

function sourceRecord(
  source: MarkerSource | EnrichedSource,
  id: string,
): ReportReference {
  if ('name' in source) {
    return {
      id,
      title: clean(source.name, 'Reference'),
      organization: clean(source.name, 'Not specified'),
      date: clean(source.accessed, 'Access date not recorded'),
      evidenceRole: clean(source.evidence_type, 'Marker-pack reference'),
      url: source.url || null,
    };
  }
  return {
    id,
    title: clean(source.citation, 'Catalog reference'),
    organization: clean(source.source_type, 'Local reference catalog'),
    date: 'Local catalog record',
    evidenceRole: clean(source.details, 'Catalog evidence'),
    url: source.url || null,
  };
}

function buildReferenceRegistry(report: GeneratedReport): {
  references: ReportReference[];
  idsByMarker: Map<string, string[]>;
} {
  const references: ReportReference[] = [];
  const idsByKey = new Map<string, string>();
  const idsByMarker = new Map<string, string[]>();

  for (const section of report.sections) {
    for (const marker of section.markers) {
      const markerKey = `${section.name}:${marker.link_id}`;
      const markerIds: string[] = [];
      for (const source of [...marker.sources, ...marker.db_enriched_sources]) {
        const key = sourceKey(source);
        let id = idsByKey.get(key);
        if (!id) {
          id = `REF-${String(references.length + 1).padStart(3, '0')}`;
          idsByKey.set(key, id);
          references.push(sourceRecord(source, id));
        }
        markerIds.push(id);
      }
      idsByMarker.set(markerKey, [...new Set(markerIds)]);
    }
  }

  return { references, idsByMarker };
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
    title: audience === 'personal' ? simple.simpleImpact : `${marker.gene} ${clean(marker.variant_name, marker.rsid)}`,
    plainMeaning: simple.simpleMeaning,
    nextStep: nextStepFor(marker),
    evidence: `${tier.label} — ${tier.confidenceLabel}`,
    uncertainty: uncertaintyFor(marker),
  };

  if (audience !== 'personal') {
    finding.gene = marker.gene;
    finding.rsid = marker.rsid;
    finding.variant = marker.variant_name;
    finding.severity = marker.severity_class;
    finding.assertionStatus = marker.assertion_status;
    finding.applicability = marker.sex_scope ? getScopeLabel(marker.sex_scope) : 'All users unless context says otherwise';
    finding.clinicalConfirmation = marker.clinical_confirmation_required ? 'Discuss confirmation' : 'Not specifically required by this marker';
    finding.technicalInterpretation = marker.interpretation;
    finding.claimBoundary = marker.do_not_claim.join('; ') || marker.raw_dna_limitation || 'Do not treat as diagnostic.';
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
  const context = options.personalSafetyContext;
  const selectedContext = selectedReproductiveContextOption(options.reproductiveContext)?.label;
  const lines = [
    `- Selected reproductive context: ${selectedContext || 'Not specified'}`,
    `- Medications (self-reported): ${context?.medications.map((value) => clean(value)).join('; ') || 'None recorded'}`,
    `- Supplements / OTC products (self-reported): ${context?.supplements.map((value) => clean(value)).join('; ') || 'None recorded'}`,
    `- Allergies / intolerances (self-reported): ${context?.allergies.map((value) => clean(value)).join('; ') || 'None recorded'}`,
    `- Symptoms and timing (self-reported): ${context?.symptoms.map((value) => clean(value)).join('; ') || 'None recorded'}`,
    `- Recent labs / clinician findings (self-reported): ${context?.labObservations.map((value) => clean(value)).join('; ') || 'None recorded'}`,
  ];
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

export function buildReportAudienceMarkdown(options: ReportExportOptions): string {
  const includeRawGenotypes = options.includeRawGenotypes === true;
  const registry = buildReferenceRegistry(options.report);
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
    `- Chromosome-call context: ${clean(options.sample.genetic_sex)}`,
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
