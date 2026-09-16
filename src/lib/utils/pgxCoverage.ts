import pgxDiplotypeGuidance from '../marker-packs/pgx_diplotype_guidance.json';
import type { EvaluatedMarker } from '../types/genomics';
import { callabilityStateForResult } from './callability';
import { isCallableGenotype } from './genotype';

export type PgxCoverageStatus = 'not_observed' | 'incomplete' | 'component_coverage_only';

export interface PgxComponentCoverage {
  id: string;
  label: string;
  gene_symbols: string[];
  required_marker_ids: string[];
  observed_marker_ids: string[];
  callable_marker_ids: string[];
  not_present_marker_ids: string[];
  blocked_marker_ids: string[];
  not_callable_marker_ids: string[];
  missing_from_report_marker_ids: string[];
  observed_marker_link_ids: string[];
  required_inputs: string[];
  clinical_next_step: string;
  status: PgxCoverageStatus;
  phenotype_or_dose_allowed: false;
}

function unique(values: readonly string[]): string[] {
  return [...new Set(values.filter(Boolean))];
}

function markerState(marker: EvaluatedMarker): 'callable' | 'not_present' | 'blocked' | 'not_callable' | 'unknown' {
  return marker.callability_state
    || callabilityStateForResult(marker.variant_type, marker.assertion_status);
}

function markerMatchesGene(
  gene: typeof pgxDiplotypeGuidance.genes[number],
  marker: EvaluatedMarker,
): boolean {
  const symbols = marker.gene.toUpperCase().split(/[\s/]+/).filter(Boolean);
  return gene.marker_ids.includes(marker.rsid)
    || gene.gene_symbols.some((symbol) => symbols.includes(symbol.toUpperCase()));
}

/**
 * Report component coverage only. A fully observed set of array components is
 * still not a clinical diplotype or phenotype, so this function never returns
 * a clinical-ready status or a medication recommendation.
 */
export function derivePgxComponentCoverage(
  markers: readonly EvaluatedMarker[],
): PgxComponentCoverage[] {
  return pgxDiplotypeGuidance.genes
    .filter((gene) => markers.some((marker) => markerMatchesGene(gene, marker)))
    .map((gene) => {
      const requiredMarkerIds = unique(gene.marker_ids);
      const matching = markers.filter((marker) => markerMatchesGene(gene, marker));
      const byRsid = new Map<string, EvaluatedMarker[]>();
      for (const marker of matching) {
        const rows = byRsid.get(marker.rsid) || [];
        rows.push(marker);
        byRsid.set(marker.rsid, rows);
      }

      const observedMarkerIds = requiredMarkerIds.filter((id) => byRsid.has(id));
      const missingFromReportMarkerIds = requiredMarkerIds.filter((id) => !byRsid.has(id));
      const callableMarkerIds: string[] = [];
      const notPresentMarkerIds: string[] = [];
      const blockedMarkerIds: string[] = [];
      const notCallableMarkerIds: string[] = [];

      for (const id of observedMarkerIds) {
        const rows = byRsid.get(id) || [];
        const states = rows.map(markerState);
        if (states.includes('callable') && rows.some((marker) =>
          markerState(marker) === 'callable'
          && marker.assertion_status === 'Verified'
          && marker.interpretation_allowed
          && isCallableGenotype(marker.user_genotype),
        )) {
          callableMarkerIds.push(id);
        } else if (states.includes('blocked')) {
          blockedMarkerIds.push(id);
        } else if (states.includes('not_callable')) {
          notCallableMarkerIds.push(id);
        } else if (states.includes('not_present') || states.includes('unknown')) {
          notPresentMarkerIds.push(id);
        }
      }

      const status: PgxCoverageStatus = matching.length === 0
        ? 'not_observed'
        : missingFromReportMarkerIds.length === 0
          && notPresentMarkerIds.length === 0
          && blockedMarkerIds.length === 0
          && notCallableMarkerIds.length === 0
          ? 'component_coverage_only'
          : 'incomplete';

      return {
        id: gene.id,
        label: gene.label,
        gene_symbols: gene.gene_symbols,
        required_marker_ids: requiredMarkerIds,
        observed_marker_ids: observedMarkerIds,
        callable_marker_ids: unique(callableMarkerIds),
        not_present_marker_ids: unique(notPresentMarkerIds),
        blocked_marker_ids: unique(blockedMarkerIds),
        not_callable_marker_ids: unique(notCallableMarkerIds),
        missing_from_report_marker_ids: missingFromReportMarkerIds,
        observed_marker_link_ids: unique(
          matching
            .filter((marker) => observedMarkerIds.includes(marker.rsid))
            .map((marker) => marker.link_id),
        ),
        required_inputs: gene.required_inputs,
        clinical_next_step: gene.clinical_next_step,
        status,
        phenotype_or_dose_allowed: false,
      };
    });
}
