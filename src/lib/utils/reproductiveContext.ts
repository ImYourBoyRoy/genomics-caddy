import cycleSupport from '../marker-packs/cycle_support_guidance.json';

export type ReproductiveContextOption = typeof cycleSupport.context_options[number];

export const REPRODUCTIVE_CONTEXT_STORAGE_PREFIX = 'genomics_reproductive_context:';

export function reproductiveContextStorageKey(sampleId?: number | null): string {
  return `${REPRODUCTIVE_CONTEXT_STORAGE_PREFIX}${sampleId ?? 'unknown'}`;
}

export function selectedReproductiveContextOption(
  reproductiveContext?: string | null,
): ReproductiveContextOption | undefined {
  const normalized = String(reproductiveContext || '').trim().toLowerCase();
  if (!normalized || normalized === 'not_specified' || normalized === 'none_or_unknown') return undefined;
  return cycleSupport.context_options.find((option) => option.id.toLowerCase() === normalized);
}

export function normalizeReproductiveContext(reproductiveContext?: string | null): string {
  return selectedReproductiveContextOption(reproductiveContext)?.id || '';
}

export function loadReproductiveContext(sampleId?: number | null): string {
  if (typeof localStorage === 'undefined') return '';
  try {
    return normalizeReproductiveContext(localStorage.getItem(reproductiveContextStorageKey(sampleId)));
  } catch {
    return '';
  }
}

export function saveReproductiveContext(sampleId: number | null | undefined, reproductiveContext?: string | null): void {
  if (typeof localStorage === 'undefined') return;
  try {
    localStorage.setItem(
      reproductiveContextStorageKey(sampleId),
      normalizeReproductiveContext(reproductiveContext) || 'none_or_unknown',
    );
  } catch {
    // localStorage may be unavailable or full; the in-memory selection remains usable.
  }
}

export function cycleSupportDomainsForContext(
  reproductiveContext?: string | null,
): typeof cycleSupport.domains {
  const option = selectedReproductiveContextOption(reproductiveContext);
  if (!option) return [];
  const domainIds = new Set(option.domain_ids);
  return cycleSupport.domains.filter((domain) => domainIds.has(domain.id));
}

/**
 * Rank a reproductive marker for an explicitly selected life/body context.
 *
 * This is a relevance hint, not a biological eligibility test. A marker can
 * remain visible even when its context rank is low; the report must never use
 * this function to infer anatomy, identity, fertility, pregnancy, or hormone
 * status from DNA.
 */
export function reproductiveMarkerContextRank(
  rsid: string,
  reproductiveContext?: string | null,
): number {
  const option = selectedReproductiveContextOption(reproductiveContext);
  if (!option) return 0;

  const markerId = String(rsid || '').trim();
  const markerContexts = cycleSupport.marker_contexts as Record<string, string[]>;
  const sharedMarkers = markerContexts.shared_reproductive || [];
  const selectedMarkers = markerContexts[option.id] || [];
  const isShared = sharedMarkers.includes(markerId);
  const isSelected = selectedMarkers.includes(markerId);

  // A marker authored for the selected context is the strongest match. Shared
  // reproductive biology remains useful in every selected reproductive route.
  if (isSelected) return 3;
  if (isShared) return 2;

  // Unmapped markers remain visible but are placed after explicitly mapped
  // findings; a mapped marker for another context is placed last.
  const isMapped = Object.values(markerContexts).some((ids) => ids.includes(markerId));
  return isMapped ? 0 : 1;
}

export function reproductiveMarkerContextIds(rsid: string): string[] {
  const markerId = String(rsid || '').trim();
  const markerContexts = cycleSupport.marker_contexts as Record<string, string[]>;
  return Object.entries(markerContexts)
    .filter(([, ids]) => ids.includes(markerId))
    .map(([contextId]) => contextId);
}
