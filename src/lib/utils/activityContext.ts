import activityGuardrails from '../marker-packs/activity_guardrails.json';
import { selectedReproductiveContextOption } from './reproductiveContext';
import type { EvaluatedMarker } from '../types/genomics';
import type { PersonalSafetyContext } from './personalSafetyContext';

type ActivityRoutingFields = {
  context_ids?: readonly string[];
  personal_context_keywords?: readonly string[];
};

function routingFields(
  domain: typeof activityGuardrails.domains[number],
): ActivityRoutingFields {
  return domain as unknown as ActivityRoutingFields;
}

/** Flatten explicitly supplied phenotype and medication context for routing only. */
export function activityPersonalContextText(context?: PersonalSafetyContext): string {
  if (!context) return '';
  return [
    ...context.medications,
    ...context.supplements,
    ...context.allergies,
    ...context.symptoms,
    ...context.labObservations,
    ...Object.values(context.reproductiveIntake || {}),
    ...(context.cycleDiary || []).flatMap((entry) => Object.values(entry.values)),
  ].join(' ').toLowerCase();
}

/** Match activity guardrails using only resource-authored context signals. */
export function activityDomainMatchesPersonalContext(
  domain: typeof activityGuardrails.domains[number],
  personalContextText: string,
  reproductiveContext?: string,
): boolean {
  const routing = routingFields(domain);
  const selectedContextId = selectedReproductiveContextOption(reproductiveContext)?.id;
  if (selectedContextId && routing.context_ids?.includes(selectedContextId)) return true;

  return (routing.personal_context_keywords || []).some((keyword) =>
    personalContextText.includes(String(keyword).toLowerCase())
  );
}

/** Match report-derived activity context plus explicit personal context. */
export function activityDomainMatches(
  domain: typeof activityGuardrails.domains[number],
  markers: EvaluatedMarker[],
  sectionNames: string[],
  personalContextText: string,
  reproductiveContext?: string,
): boolean {
  const context = [
    ...sectionNames,
    ...markers.map((marker) => `${marker.gene} ${marker.variant_name || ''} ${marker.sex_scope || ''}`),
  ].join(' ').toLowerCase();
  const keywords = Array.isArray(domain.section_keywords) ? domain.section_keywords : [];
  return keywords.some((keyword) => context.includes(String(keyword).toLowerCase()))
    || activityDomainMatchesPersonalContext(domain, personalContextText, reproductiveContext);
}
