import { describe, expect, it } from 'vitest';
import {
  reproductiveMarkerContextIds,
  reproductiveMarkerContextRank,
  selectedReproductiveContextOption,
} from './reproductiveContext';

describe('reproductive marker context routing', () => {
  it('prioritizes menstrual-cycle markers without hiding other findings', () => {
    expect(reproductiveMarkerContextRank('PANEL_PMDD_OVARIAN_STEROID_SENSITIVITY', 'menstrual_cycle')).toBe(3);
    expect(reproductiveMarkerContextRank('rs2234693', 'menstrual_cycle')).toBe(2);
    expect(reproductiveMarkerContextRank('PANEL_AR_CAG_REPEAT_CALLOUT', 'menstrual_cycle')).toBe(0);
    expect(reproductiveMarkerContextIds('PANEL_PMDD_OVARIAN_STEROID_SENSITIVITY')).toContain('menstrual_cycle');
  });

  it('routes the same reproductive pack symmetrically across contexts', () => {
    expect(reproductiveMarkerContextRank('PANEL_AR_CAG_REPEAT_CALLOUT', 'androgen_reproductive')).toBe(3);
    expect(reproductiveMarkerContextRank('rs523349', 'androgen_reproductive')).toBe(3);
    expect(reproductiveMarkerContextRank('PANEL_PMDD_OVARIAN_STEROID_SENSITIVITY', 'androgen_reproductive')).toBe(0);
    expect(reproductiveMarkerContextRank('PANEL_ADENOMYOSIS_RESEARCH_GAP', 'uterine_pelvic')).toBe(3);
  });

  it('keeps unknown or unselected context non-directive', () => {
    expect(selectedReproductiveContextOption('')).toBeUndefined();
    expect(reproductiveMarkerContextRank('rs2234693', '')).toBe(0);
    expect(reproductiveMarkerContextRank('future_marker', 'menstrual_cycle')).toBe(1);
  });
});
