import { describe, expect, it } from 'vitest';
import {
  classifyWarning,
  dedupeWarnings,
  isWarningVisibleInAudience,
  selectWarningsForAudience,
} from './warningTaxonomy';

describe('warning taxonomy', () => {
  it('prioritizes concrete safety routes over broad caveat language', () => {
    expect(classifyWarning('Abacavir hypersensitivity route')).toBe('actionable_safety');
    expect(classifyWarning('Confirm clinical HLA typing before prescribing')).toBe('clinical_review');
    expect(classifyWarning('This association is probabilistic')).toBe('general_interpretation');
    expect(classifyWarning('Reference catalog update available')).toBe('operational_status');
    expect(classifyWarning('This file was generated locally; share it only with the intended recipient')).toBe('legal_privacy');
  });

  it('deduplicates repeated text while preserving first-seen order', () => {
    const warnings = selectWarningsForAudience([
      'Confirm clinical testing.',
      'confirm clinical testing',
      'This association is probabilistic.',
    ], 'clinical');
    expect(warnings.map((warning) => warning.text)).toEqual([
      'Confirm clinical testing.',
      'This association is probabilistic.',
    ]);
  });

  it('keeps simple mode focused and retains concrete routes', () => {
    const warnings = selectWarningsForAudience([
      'This is not a diagnosis.',
      'Avoid abacavir when the clinical hypersensitivity route is confirmed.',
      'Review measured ferritin and transferrin saturation.',
    ], 'simple');
    expect(warnings.map((warning) => warning.kind)).toEqual([
      'actionable_safety',
      'clinical_review',
    ]);
  });

  it('routes one-time interpretation and legal copy to their shared surfaces', () => {
    expect(isWarningVisibleInAudience('general_interpretation', 'footer')).toBe(true);
    expect(isWarningVisibleInAudience('general_interpretation', 'simple')).toBe(false);
    expect(isWarningVisibleInAudience('legal_privacy', 'legal')).toBe(true);
    expect(isWarningVisibleInAudience('legal_privacy', 'clinical')).toBe(false);
    expect(dedupeWarnings([])).toEqual([]);
  });
});
