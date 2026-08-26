import cycleSupport from '../marker-packs/cycle_support_guidance.json';

export interface ReproductivePersonalContextLike {
  reproductiveIntake?: Record<string, unknown> | null;
  cycleDiary?: readonly unknown[] | null;
}

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
  return cycleSupportDomainsForContextIds(option ? [option.id] : []);
}

/**
 * Resolve specific reproductive routes from explicitly supplied profile text.
 * The keyword lists live in cycle_support_guidance.json so adding a context
 * does not require a second TypeScript routing table. This is relevance
 * routing only; it never infers anatomy, identity, hormones, or a diagnosis.
 */
export function reproductiveContextIdsForProfileText(profileContext?: string | null): string[] {
  const normalizedText = String(profileContext || '').replace(/\s+/g, ' ').trim().toLowerCase();
  if (!normalizedText) return [];

  const matchedIds = new Set<string>();
  for (const option of cycleSupport.context_options) {
    const keywords = (option.profile_keywords || []).filter((keyword) => String(keyword).trim() !== '');
    if (keywords.some((keyword) => normalizedText.includes(String(keyword).trim().toLowerCase()))) {
      matchedIds.add(option.id);
    }
  }

  return cycleSupport.context_options
    .filter((option) => matchedIds.has(option.id))
    .map((option) => option.id);
}

/**
 * Resolve context options from explicitly supplied intake fields and diary
 * entries. This is a routing aid only: it never infers anatomy, identity,
 * fertility, pregnancy, hormones, or a diagnosis from DNA or from free text.
 */
export function reproductiveContextIdsForPersonalContext(
  personalContext?: ReproductivePersonalContextLike,
): string[] {
  const populatedFieldIds = new Set(
    Object.entries(personalContext?.reproductiveIntake || {})
      .filter(([, value]) => String(value ?? '').trim() !== '')
      .map(([fieldId]) => fieldId),
  );
  const contextIds = new Set<string>();

  for (const group of cycleSupport.intake_schema.groups) {
    if (!group.field_ids.some((fieldId) => populatedFieldIds.has(fieldId))) continue;
    for (const contextId of group.context_ids || []) contextIds.add(contextId);
  }

  if ((personalContext?.cycleDiary?.length || 0) > 0) {
    for (const contextId of cycleSupport.diary_schema.context_ids || []) contextIds.add(contextId);
  }

  const knownContextIds = new Set(cycleSupport.context_options.map((option) => option.id));
  return cycleSupport.context_options
    .filter((option) => contextIds.has(option.id) && knownContextIds.has(option.id))
    .map((option) => option.id);
}

export function hasReproductivePersonalContext(
  personalContext?: ReproductivePersonalContextLike,
): boolean {
  return Object.values(personalContext?.reproductiveIntake || {})
    .some((value) => String(value ?? '').trim() !== '')
    || (personalContext?.cycleDiary?.length || 0) > 0;
}

/**
 * An explicit selector takes precedence. If it is blank, only populated
 * resource-declared intake/diary context can activate reproductive support.
 */
export function activeReproductiveContextIds(
  reproductiveContext?: string | null,
  personalContext?: ReproductivePersonalContextLike,
): string[] {
  const selected = selectedReproductiveContextOption(reproductiveContext);
  return selected ? [selected.id] : reproductiveContextIdsForPersonalContext(personalContext);
}

export function cycleSupportDomainsForContextIds(
  contextIds: readonly string[],
): typeof cycleSupport.domains {
  if (contextIds.length === 0) return [];
  const domainsById = new Map(cycleSupport.domains.map((domain) => [domain.id, domain]));
  const selectedDomainIds: string[] = [];
  const seen = new Set<string>();
  for (const option of cycleSupport.context_options) {
    if (!contextIds.includes(option.id)) continue;
    for (const domainId of option.domain_ids) {
      if (seen.has(domainId)) continue;
      seen.add(domainId);
      selectedDomainIds.push(domainId);
    }
  }
  return selectedDomainIds.flatMap((domainId) => {
    const domain = domainsById.get(domainId);
    return domain ? [domain] : [];
  });
}

export function cycleSupportDomainsForPersonalContext(
  personalContext?: ReproductivePersonalContextLike,
): typeof cycleSupport.domains {
  return cycleSupportDomainsForContextIds(reproductiveContextIdsForPersonalContext(personalContext));
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

/**
 * Identify sections eligible for explicit reproductive-context prioritization.
 * Section keywords and marker membership are both resource-authored; this
 * helper never infers a person's anatomy, identity, fertility, or hormones.
 */
export function reproductiveSectionHasContext(
  sectionName: string,
  markerIds: string[] = [],
): boolean {
  const normalized = String(sectionName || '').toLowerCase();
  const keywordMatch = cycleSupport.context_keywords.some((keyword) =>
    normalized.includes(String(keyword).toLowerCase())
  );
  if (keywordMatch) return true;
  return markerIds.some((rsid) => reproductiveMarkerContextIds(rsid).length > 0);
}
