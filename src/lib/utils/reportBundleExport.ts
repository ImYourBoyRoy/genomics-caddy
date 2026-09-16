import type { GeneratedReport, GenomeSample } from '../types/genomics';
import { buildLabRequestListText, deriveActionablePlan, deriveAllergySensitivityGuidance } from './actionabilityEngine';
import { CYCLE_DIARY_SCHEMA } from './cycleDiary';
import {
  audienceRequiresRawGenotypes,
  buildReportAudienceMarkdown,
  type ReportExportAudience,
  type ReportExportOptions,
} from './reportAudienceExport';
import { buildReportReferenceRegistry } from './reportReferences';
import type { ProfileContext } from './profileContext';
import { formatGeneticSexLabel } from './uiLabels';
import { normalizeFindingSemantics } from './findingSemantics';
import { buildConditionCoverageSummaries, getConditionCoverageGaps } from './conditionEvidence';
import { derivePrsReadiness } from './prsReadiness';
import {
  buildAssertionKey,
  callabilityPolicyForVariantType,
  callabilityStateForResult,
  orientationStateForResult,
} from './callability';
import aiPromptPolicy from '../marker-packs/ai_prompt_policy.json';

export interface ReportBundleFile {
  filename: string;
  content: string;
}

export interface ReportBundleOptions extends ReportExportOptions {
  profileContext: ProfileContext;
}

const PRIVACY_TEXT = `${aiPromptPolicy.export_disclosures.bundle_privacy_notice}\n`;

function safeName(value: string): string {
  return String(value || 'genome')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '_')
    .replace(/^_+|_+$/g, '')
    .slice(0, 64) || 'genome';
}

function csvCell(value: unknown): string {
  const text = String(value ?? '');
  return /[",\n\r]/.test(text) ? `"${text.replace(/"/g, '""')}"` : text;
}

function diaryCsv(profileContext: ProfileContext): string {
  const fields = CYCLE_DIARY_SCHEMA.fields;
  const header = fields.map((field) => csvCell(field.id)).join(',');
  const rows = (profileContext.safety.cycleDiary || []).map((entry) =>
    fields.map((field) => csvCell(entry.values[field.id] || '')).join(','),
  );
  return [header, ...rows, ''].join('\n');
}

function contextJson(sample: GenomeSample, profileContext: ProfileContext): string {
  return JSON.stringify({
    schema_version: 1,
    profile: {
      sample_id: sample.id,
      sample_name: sample.name,
      chromosome_call_context: formatGeneticSexLabel(sample.genetic_sex),
    },
    selected_reproductive_context: profileContext.selectedReproductiveContext || null,
    notes: profileContext.notes,
    safety: profileContext.safety,
  }, null, 2) + '\n';
}

function diaryPayload(profileContext: ProfileContext): Record<string, unknown> {
  return {
    schema_version: 1,
    fields: CYCLE_DIARY_SCHEMA.fields.map((field) => ({
      id: field.id,
      label: field.label,
      input_type: field.input_type,
    })),
    entries: (profileContext.safety.cycleDiary || []).map((entry) => ({
      id: entry.id,
      values: entry.values,
    })),
  };
}

function dnaAnalysisJson(options: ReportBundleOptions): string {
  const registry = buildReportReferenceRegistry(options.report);
  const includeRawGenotypes = audienceRequiresRawGenotypes(options.audience);
  const allergy = deriveAllergySensitivityGuidance(options.report);
  const actionable = deriveActionablePlan(options.report);
  const sections = options.report.sections.map((section) => ({
    name: section.name,
    markers: section.markers
      .filter((marker) => options.audience !== 'personal' || !['benign', 'no_data'].includes(marker.severity_class))
      .map((marker) => {
        const semantics = normalizeFindingSemantics(marker);
        const result: Record<string, unknown> = {
          link_id: marker.link_id,
          assertion_key: marker.assertion_key || buildAssertionKey(marker),
          rsid: marker.rsid,
          gene: marker.gene,
          variant_name: marker.variant_name,
          variant_type: marker.variant_type,
          callability_policy: callabilityPolicyForVariantType(marker.variant_type),
          callability_state: marker.callability_state || callabilityStateForResult(marker.variant_type, marker.assertion_status),
          orientation_state: marker.orientation_state || orientationStateForResult(marker.assertion_status, marker.requires_orientation_verification),
          effect_allele: marker.effect_allele,
          effect_count: marker.effect_count,
          effect_direction: marker.effect_direction,
          evidence_tier: marker.evidence_tier,
          severity_class: marker.severity_class,
          assertion_status: marker.assertion_status,
          sex_scope: marker.sex_scope,
          interpretation_allowed: marker.interpretation_allowed,
          clinical_confirmation_required: marker.clinical_confirmation_required,
          clinical_semantics: semantics,
          claim_boundaries: {
            do_not_claim: marker.do_not_claim,
            confirm_with: marker.confirm_with,
            raw_dna_limitation: marker.raw_dna_limitation,
          },
          reference_ids: registry.idsByMarker.get(`${section.name}:${marker.link_id}`) || [],
          sources: (marker.sources || []).map((source) => source.name),
        };
        if (includeRawGenotypes) {
          result.user_genotype = marker.user_genotype;
          result.normalized_genotype = marker.normalized_genotype;
        }
        return result;
      }),
  }));
  return JSON.stringify({
    schema_version: 1,
    audience: options.audience,
    generated_at: options.report.generated_at,
    raw_genotypes_included: includeRawGenotypes,
    import_provenance: includeRawGenotypes ? options.report.import_provenance ?? null : null,
    allergy_sensitivity: {
      matched_marker_count: allergy.matchedMarkerCount,
      dna_contexts: allergy.dnaContexts,
      medication_safety: allergy.medicationSafety,
    },
    condition_evidence: actionable.conditionEvidence,
    condition_coverage: buildConditionCoverageSummaries(options.report),
    condition_coverage_gaps: getConditionCoverageGaps(),
    pgx: {
      policy: actionable.pgxGuidance.policy,
      component_coverage: actionable.pgxGuidance.componentCoverage,
      do_not_do: actionable.pgxGuidance.doNotDo,
    },
    prs: {
      principle: 'PRS modules are registries, not scored markers.',
      modules: derivePrsReadiness(),
    },
    recommendations: {
      food: {
        favor: actionable.diet.favorItems,
        avoid: actionable.diet.avoidItems,
      },
      supplements: {
        consider: actionable.supplements,
        avoid_or_confirm_first: actionable.supplementAvoid,
      },
      activity: actionable.activity.recommendationItems,
      labs: actionable.labTests,
      clinician_request_list: buildLabRequestListText(actionable.labTests),
    },
    sections,
    references: registry.references,
  }, null, 2) + '\n';
}

/**
 * Single-file AI handoff. It keeps the report, entered context, and diary
 * together so a user can attach one JSON file to a model without reconstructing
 * the six-file bundle. Raw calls follow the same explicit audience policy as
 * the AI ZIP bundle.
 */
export function buildAiReviewJson(options: ReportBundleOptions): string {
  const dnaAnalysis = JSON.parse(dnaAnalysisJson({ ...options, audience: 'ai' })) as Record<string, unknown>;
  return JSON.stringify({
    schema_version: 1,
    export_kind: 'ai_review_json',
    audience: 'ai',
    generated_at: options.report.generated_at,
    profile: {
      sample_id: options.sample.id,
      sample_name: options.sample.name,
      chromosome_call_context: formatGeneticSexLabel(options.sample.genetic_sex),
    },
    raw_genotypes_included: audienceRequiresRawGenotypes('ai'),
    privacy: aiPromptPolicy.export_disclosures.bundle_privacy_notice,
    review_instructions: [aiPromptPolicy.export_disclosures.ai_review_instructions],
    dna_analysis: dnaAnalysis,
    context: JSON.parse(contextJson(options.sample, options.profileContext)),
    diary: diaryPayload(options.profileContext),
  }, null, 2) + '\n';
}

export function buildReportBundleFiles(options: ReportBundleOptions): ReportBundleFile[] {
  const prefix = safeName(options.sample.name);
  const generatedAt = new Date().toISOString();
  const fileNames = ['report.md', 'dna_analysis.json', 'context.json', 'diary.csv', 'manifest.json', 'PRIVACY.txt'];
  return [
    { filename: 'report.md', content: buildReportAudienceMarkdown(options) },
    { filename: 'dna_analysis.json', content: dnaAnalysisJson(options) },
    { filename: 'context.json', content: contextJson(options.sample, options.profileContext) },
    { filename: 'diary.csv', content: diaryCsv(options.profileContext) },
    {
      filename: 'manifest.json',
      content: JSON.stringify({
        schema_version: 1,
        profile_id: options.sample.id,
        profile_name: options.sample.name,
        chromosome_call_context: formatGeneticSexLabel(options.sample.genetic_sex),
        audience: options.audience,
        generated_at: generatedAt,
        raw_genotypes_included: audienceRequiresRawGenotypes(options.audience),
        import_provenance: audienceRequiresRawGenotypes(options.audience) ? options.report.import_provenance ?? null : null,
        context_included: true,
        files: fileNames,
      }, null, 2) + '\n',
    },
    { filename: 'PRIVACY.txt', content: PRIVACY_TEXT },
  ];
}

export function reportBundleFilename(sampleName: string, audience: ReportExportAudience): string {
  return `${safeName(sampleName)}_${audience}_bundle.zip`;
}

export const reportBundlePrivacyText = PRIVACY_TEXT;
