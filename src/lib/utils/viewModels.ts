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
      gwas_hits: [],
      dbsnp: null,
      population: null,
      db_enriched_sources: []
    };

    // Determine position based on source build (GRCh38 preferred, fallback to 37)
    let position = variant.position_grch38 ?? variant.position_grch37 ?? null;
    if (enrichment.dbsnp) {
      position = enrichment.dbsnp.position_grch38 ?? enrichment.dbsnp.position_grch37 ?? position;
    }

    displayMarkers.push({
      link_id: linkId,
      rsid: link.rsid,
      gene: variant.gene || '',
      variant_name: variant.variant_name || rsid,
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
      assertion_status: link.assertion_status,
      interpretation_allowed: link.interpretation_allowed,
      sources: link.sources || [],
      db_enriched_sources: enrichment.db_enriched_sources || [],
      clinvar_significance: enrichment.clinvar?.clinical_significance ?? null,
      clinvar_conditions: enrichment.clinvar?.conditions ?? null,
      clinvar_review_status: enrichment.clinvar?.review_status ?? null,
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
      pharmgkb: enrichment.pharmgkb ?? null,
      clingen: enrichment.clingen ?? null,
      mane: enrichment.mane ?? null
    });
  }

  return displayMarkers;
}

export function denormalizeReport(report: NormalizedReport): GeneratedReport {
  return {
    ...report,
    sections: report.sections.map(sec => ({
      name: sec.name,
      section_id: sec.section_id,
      section_signal_score: sec.section_signal_score,
      summary: sec.summary,
      markers: deriveDisplayMarkers(report, sec.section_id)
    }))
  };
}
