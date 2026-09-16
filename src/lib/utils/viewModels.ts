// ./src/lib/utils/viewModels.ts
/*
Module Docstring:
Purpose: Derived view-model selector for genomics reports.
Responsibilities:
- Perform O(1) relational joins on normalized backend reports to build flat DisplayMarker objects.
- Cache/precompute DisplayMarker arrays once per report/category tab change.
- Provide robust fallback values for missing fields to prevent UI runtime crashes.
Key Inputs: GeneratedReport, sectionId.
Key Outputs: DisplayMarker[].
Operational Notes: Tailored for Svelte 5 derived state integration.
*/

import type { GeneratedReport, NormalizedReport, DisplayMarker } from '../types/genomics';
import { buildReferenceEntries } from './reportReferences';
import {
  normalizeAssertionStatus,
  normalizeReportStatuses,
} from './reportStatuses';
import {
  buildAssertionKey,
  callabilityStateForResult,
  normalizeCallabilityState,
  orientationStateForResult,
} from './callability';

export function deriveDisplayMarkers(report: NormalizedReport | null | undefined, sectionId: string): DisplayMarker[] {
  if (!report || !report.sections || !report.variants || !report.user_calls || !report.category_links) {
    return [];
  }

  const section = report.sections.find(s => s.section_id === sectionId);
  if (!section) {
    return [];
  }

  const displayMarkers: DisplayMarker[] = [];
  const enrichmentMap = report.enrichment || {};

  for (const linkId of section.link_ids) {
    const link = report.category_links[linkId];
    if (!link) {
      continue;
    }

    const rsid = link.rsid;
    const variant = report.variants[rsid] || {
      rsid,
      gene: '',
      variant_name: null,
      chromosome: null,
      position_grch37: null,
      position_grch38: null,
      variant_type: null
    };

    const call = report.user_calls[rsid] || {
      user_genotype: '--',
      normalized_genotype: null,
      call_status: 'NoData',
      source_build: null
    };

    const enrichment = enrichmentMap[rsid] || {
      clinvar: null,
      clinvar_annotations: [],
      gwas_hits: [],
      dbsnp: null,
      population: null,
      pharmgkb_annotations: [],
      clingen_annotations: [],
      mane_annotations: [],
      db_enriched_sources: []
    };

    const clinvarAnnotations = enrichment.clinvar_annotations?.length
      ? enrichment.clinvar_annotations
      : enrichment.clinvar
        ? [enrichment.clinvar]
        : [];
    const pharmgkbAnnotations = enrichment.pharmgkb_annotations?.length
      ? enrichment.pharmgkb_annotations
      : enrichment.pharmgkb
        ? [enrichment.pharmgkb]
        : [];
    const clingenAnnotations = enrichment.clingen_annotations?.length
      ? enrichment.clingen_annotations
      : enrichment.clingen
        ? [enrichment.clingen]
        : [];
    const maneAnnotations = enrichment.mane_annotations?.length
      ? enrichment.mane_annotations
      : enrichment.mane
        ? [enrichment.mane]
        : [];

    const explicitReferenceIds = [
      ...(link.reference_ids || []),
      ...(enrichment.reference_ids || [])
    ];
    const reference_ids = explicitReferenceIds.length > 0
      ? [...new Set(explicitReferenceIds)]
      : buildReferenceEntries(link.sources || [], enrichment.db_enriched_sources || [])
        .map(reference => reference.id);

    const assertion_status = normalizeAssertionStatus(link.assertion_status);
    const callability_state = normalizeCallabilityState(link.callability_state)
      ?? callabilityStateForResult(link.variant_type ?? variant.variant_type, assertion_status);
    const orientation_state = link.orientation_state
      ?? orientationStateForResult(assertion_status, link.requires_orientation_verification);
    const assertion_key = link.assertion_key || buildAssertionKey({
      rsid: link.rsid,
      gene: link.gene || variant.gene,
      variant_name: link.variant_name || variant.variant_name,
      variant_type: link.variant_type ?? variant.variant_type,
      source_build: link.source_build,
      hgvs: link.hgvs,
      expected_plus_alleles: link.expected_plus_alleles,
      effect_allele: link.effect_allele,
      category_id: link.category_id,
      sex_scope: link.sex_scope,
      clinical_semantics: link.clinical_semantics,
      reference_ids: link.reference_ids,
      sources: link.sources,
    });

    // Determine position based on source build (GRCh38 preferred, fallback to 37)
    let position = variant.position_grch38 ?? variant.position_grch37 ?? null;
    if (enrichment.dbsnp) {
      position = enrichment.dbsnp.position_grch38 ?? enrichment.dbsnp.position_grch37 ?? position;
    }

    displayMarkers.push({
      link_id: linkId,
      assertion_key,
      rsid: link.rsid,
      gene: link.gene || variant.gene || '',
      variant_name: link.variant_name || variant.variant_name || rsid,
      variant_type: link.variant_type ?? variant.variant_type ?? null,
      source_build: link.source_build ?? null,
      hgvs: link.hgvs ?? null,
      expected_plus_alleles: link.expected_plus_alleles ?? null,
      category_id: link.category_id,
      chromosome: variant.chromosome || enrichment.dbsnp?.chromosome || '',
      position,
      user_genotype: call.user_genotype,
      normalized_genotype: call.normalized_genotype ?? null,
      effect_allele: link.effect_allele,
      effect_count: link.effect_count ?? null,
      impact: link.impact,
      evidence_tier: link.evidence_tier,
      interpretation: link.interpretation,
      effect_direction: link.effect_direction,
      severity_class: link.severity_class as any,
      assertion_status,
      callability_state,
      orientation_state,
      requires_orientation_verification: link.requires_orientation_verification,
      interpretation_allowed: link.interpretation_allowed,
      sources: link.sources || [],
      db_enriched_sources: enrichment.db_enriched_sources || [],
      reference_ids,
      clinvar_significance: clinvarAnnotations[0]?.clinical_significance ?? null,
      clinvar_conditions: clinvarAnnotations[0]?.conditions ?? null,
      clinvar_review_status: clinvarAnnotations[0]?.review_status ?? null,
      gwas_top_trait: enrichment.gwas_hits?.[0]?.trait_name ?? null,
      gwas_best_pvalue: enrichment.gwas_hits?.[0] ? parseFloat(enrichment.gwas_hits[0].p_value_string) : null,
      gwas_association_count: enrichment.gwas_hits?.[0]?.association_count ?? null,
      population_af: enrichment.population?.allele_frequency ?? null,
      population_rarity: enrichment.population?.rarity_bucket ?? null,
      confirm_with: link.confirm_with || [],
      do_not_claim: link.do_not_claim || [],
      clinical_confirmation_required: link.clinical_confirmation_required ?? undefined,
      sex_scope: link.sex_scope ?? undefined,
      raw_dna_limitation: link.raw_dna_limitation ?? undefined,
      clinical_semantics: link.clinical_semantics ?? null,
      pharmgkb: pharmgkbAnnotations[0] ?? null,
      pharmgkb_annotations: pharmgkbAnnotations,
      clingen: clingenAnnotations[0] ?? null,
      clingen_annotations: clingenAnnotations,
      mane: maneAnnotations[0] ?? null,
      mane_annotations: maneAnnotations
    });
  }

  return displayMarkers;
}

export function denormalizeReport(report: NormalizedReport): GeneratedReport {
  const normalized = normalizeReportStatuses(report);
  return {
    ...normalized,
    sections: normalized.sections.map(sec => ({
      name: sec.name,
      section_id: sec.section_id,
      section_signal_score: sec.section_signal_score,
      summary: sec.summary,
      markers: deriveDisplayMarkers(normalized, sec.section_id)
    }))
  };
}
