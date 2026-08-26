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
