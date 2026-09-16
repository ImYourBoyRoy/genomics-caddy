import prsRegistry from '../marker-packs/prs_registry.json';

export interface PrsReadinessRecord {
  id: string;
  trait: string;
  required_inputs: string[];
  missing_inputs: string[];
  output_policy: string;
  status: 'unscored';
  score: null;
}

/**
 * PRS modules remain explicit research/model specifications. This makes the
 * missing prerequisites visible to downstream reviewers without creating a
 * score from marker overlap or an incomplete weight file.
 */
export function derivePrsReadiness(): PrsReadinessRecord[] {
  return prsRegistry.prs_modules.map((module) => ({
    id: module.id,
    trait: module.trait,
    required_inputs: module.required_inputs,
    missing_inputs: module.required_inputs,
    output_policy: module.output_policy,
    status: 'unscored' as const,
    score: null,
  }));
}
