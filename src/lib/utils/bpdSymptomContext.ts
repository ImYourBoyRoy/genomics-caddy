import bpdSymptomContext from '../marker-packs/bpd_symptom_context.json';
import type { EvaluatedMarker, GeneratedReport } from '../types/genomics';
import { isCallableGenotype } from './genotype';
import { isVerifiedAssertionStatus } from './reportStatuses';

export type BpdSymptomContextResource = typeof bpdSymptomContext;

export interface BpdSymptomContextDomain {
  id: string;
  label: string;
  summary: string;
  prompts: string[];
  marker_genes?: string[];
  marker_rsids?: string[];
  always_show_if_context?: boolean;
}

export interface BpdSymptomContextMatch extends BpdSymptomContextDomain {
  matchedMarkerLinkIds: string[];
  matchedGenes: string[];
}

export interface BpdSymptomContextView {
  display: BpdSymptomContextResource['display'];
  domains: BpdSymptomContextMatch[];
  matchedMarkerLinkIds: string[];
  matchedGenes: string[];
  matchedMarkerCount: number;
  matchedDomainCount: number;
  hasPathwayContext: boolean;
  /** True only when one of the authored BPD GWAS loci matched. */
  hasBpdLocusContext: boolean;
}

export const BPD_SYMPTOM_RESEARCH_CONTEXT_ID = 'bpd_symptom_research_context';
const BPD_GWAS_DOMAIN_ID = 'recent_bpd_gwas_loci';
const NEUROPSYCH_SECTION_NAME = 'neurotransmitter & mood resiliency';

function normalize(value: unknown): string {
  return String(value || '').trim().toLowerCase();
}

function markerGeneSymbols(marker: EvaluatedMarker): string[] {
  return String(marker.gene || '')
    .split(/[\s/]+/)
    .map((gene) => gene.trim().toUpperCase())
    .filter(Boolean);
}

function activeNeuropsychMarkers(report: GeneratedReport): EvaluatedMarker[] {
  return report.sections
    .filter((section) => normalize(section.name) === NEUROPSYCH_SECTION_NAME)
    .flatMap((section) => section.markers)
    .filter((marker) => (
      isVerifiedAssertionStatus(marker.assertion_status)
      && marker.interpretation_allowed
      && isCallableGenotype(marker.user_genotype)
      && (marker.effect_count ?? 0) > 0
    ));
}

function matchesDomain(marker: EvaluatedMarker, domain: BpdSymptomContextDomain): boolean {
  const markerRsid = normalize(marker.rsid);
  const rsidMatch = (domain.marker_rsids || []).some((rsid) => normalize(rsid) === markerRsid);
  if (rsidMatch) return true;

  const markerGenes = new Set(markerGeneSymbols(marker));
  return (domain.marker_genes || []).some((gene) => markerGenes.has(String(gene).trim().toUpperCase()));
}

function uniqueStrings(values: readonly string[]): string[] {
  return Array.from(new Set(values.filter(Boolean)));
}

/**
 * Build a genotype-free symptom crosswalk from active, callable neuropsych
 * rows. The result contains marker link IDs only for navigation; it never
 * exposes a genotype or computes a BPD score. `hasPathwayContext` means a
 * transdiagnostic neuropsych pathway was active; `hasBpdLocusContext` is
 * reserved for the authored BPD GWAS loci.
 */
export function buildBpdSymptomContext(report: GeneratedReport): BpdSymptomContextView {
  const markers = activeNeuropsychMarkers(report);
  const resourceDomains = bpdSymptomContext.domains as readonly BpdSymptomContextDomain[];
  const matchedDomains = resourceDomains.reduce<BpdSymptomContextMatch[]>((result, domain) => {
      const matchedMarkers = markers.filter((marker) => matchesDomain(marker, domain));
      const shouldShow = matchedMarkers.length > 0
        || (domain.always_show_if_context === true && markers.length > 0);
      if (!shouldShow) return result;

      result.push({
        ...domain,
        matchedMarkerLinkIds: uniqueStrings(matchedMarkers.map((marker) => marker.link_id)),
        matchedGenes: uniqueStrings(matchedMarkers.flatMap(markerGeneSymbols)),
      });
      return result;
    }, []);

  const matchedMarkerLinkIds = uniqueStrings(matchedDomains.flatMap((domain) => domain.matchedMarkerLinkIds));
  const matchedGenes = uniqueStrings(matchedDomains.flatMap((domain) => domain.matchedGenes));
  const hasBpdLocusContext = matchedDomains.some((domain) =>
    domain.id === BPD_GWAS_DOMAIN_ID && domain.matchedMarkerLinkIds.length > 0,
  );

  return {
    display: bpdSymptomContext.display,
    domains: matchedDomains,
    matchedMarkerLinkIds,
    matchedGenes,
    matchedMarkerCount: matchedMarkerLinkIds.length,
    matchedDomainCount: matchedDomains.length,
    hasPathwayContext: matchedDomains.length > 0,
    hasBpdLocusContext,
  };
}
