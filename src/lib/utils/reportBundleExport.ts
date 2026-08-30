import type { GeneratedReport, GenomeSample } from '../types/genomics';
import { buildLabRequestListText, deriveActionablePlan, deriveAllergySensitivityGuidance } from './actionabilityEngine';
import { CYCLE_DIARY_SCHEMA } from './cycleDiary';
import { buildReportAudienceMarkdown, type ReportExportAudience, type ReportExportOptions } from './reportAudienceExport';
import { buildReportReferenceRegistry } from './reportReferences';
import type { ProfileContext } from './profileContext';
import { formatGeneticSexLabel } from './uiLabels';
import { normalizeFindingSemantics } from './findingSemantics';

export interface ReportBundleFile {
  filename: string;
  content: string;
}

export interface ReportBundleOptions extends ReportExportOptions {
  profileContext: ProfileContext;
}

const PRIVACY_TEXT = `This ZIP was generated locally by Genomics Caddy. It contains sensitive health and genetic information. Share it only with the intended recipient.\n`;

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

function dnaAnalysisJson(options: ReportBundleOptions): string {
  const registry = buildReportReferenceRegistry(options.report);
  const includeRawGenotypes = options.includeRawGenotypes === true;
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
          rsid: marker.rsid,
          gene: marker.gene,
          variant_name: marker.variant_name,
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
    allergy_sensitivity: {
      matched_marker_count: allergy.matchedMarkerCount,
      dna_contexts: allergy.dnaContexts,
      medication_safety: allergy.medicationSafety,
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
        raw_genotypes_included: options.includeRawGenotypes === true,
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
