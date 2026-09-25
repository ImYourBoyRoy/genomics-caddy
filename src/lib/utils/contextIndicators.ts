import evidencePolicy from '../marker-packs/evidence_policy.json';
import { getScopeLabel } from './evidence';

export interface ContextIndicator {
  id: string;
  label: string;
  shortLabel: string;
  group: string;
  description: string;
}

type ContextIndicatorRecord = {
  label: string;
  short_label: string;
  group: string;
  description: string;
};

const authoredIndicators = evidencePolicy.display.context_indicators as Record<string, ContextIndicatorRecord>;

const SCOPE_CONTEXT_TAGS: Record<string, string[]> = {
  menstrual_cycle_context: ['menstrual_cycle'],
  ovarian_context: ['ovarian'],
  uterine_context: ['uterine'],
  androgen_reproductive_context: ['androgen_reproductive'],
  all_bodies_preconception_fertility_context: ['preconception', 'fertility'],
  all_bodies_hormone_therapy_context: ['hormone_therapy'],
  all_bodies_pregnancy_lactation_context: ['pregnancy', 'lactation'],
  all_bodies_menopause_context: ['menopause'],
};

function uniqueNonEmpty(values: readonly (string | null | undefined)[]): string[] {
  return Array.from(new Set(values.map((value) => String(value || '').trim()).filter(Boolean)));
}

function authoredIndicator(id: string): ContextIndicator | null {
  const definition = authoredIndicators[id];
  if (!definition) return null;
  return {
    id,
    label: definition.label,
    shortLabel: definition.short_label,
    group: definition.group,
    description: definition.description,
  };
}

function fallbackScopeIndicator(scope: string): ContextIndicator {
  const label = getScopeLabel(scope);
  return {
    id: `scope:${scope}`,
    label,
    shortLabel: label.replace(/\s+context$/i, '').replace(/^All-bodies\s+/i, ''),
    group: 'Biological scope',
    description: 'This is an explicit biological applicability hint authored for the marker. It does not establish anatomy, reproductive status, or current hormone levels.',
  };
}

/**
 * Resolve explicit marker context tags, falling back to the legacy biological
 * scope field for older packs. The fallback is intentionally display-only:
 * it never infers a person's current state from DNA.
 */
export function getContextIndicators(
  contextTags?: readonly (string | null | undefined)[] | null,
  scope?: string | null,
): ContextIndicator[] {
  const authoredTags = uniqueNonEmpty(contextTags || []);
  const fallbackTags = scope && scope !== 'all' ? (SCOPE_CONTEXT_TAGS[scope] || []) : [];
  const tags = authoredTags.length > 0 ? authoredTags : fallbackTags;
  const indicators = tags.map(authoredIndicator).filter((item): item is ContextIndicator => item !== null);

  if (indicators.length > 0) return indicators;
  if (scope && scope !== 'all') return [fallbackScopeIndicator(scope)];
  return [];
}

export function contextLabels(
  contextTags?: readonly (string | null | undefined)[] | null,
  scope?: string | null,
): string[] {
  return getContextIndicators(contextTags, scope).map((indicator) => indicator.label);
}
