import { describe, expect, it } from 'vitest';
import researchTaxonomy from '../marker-packs/research_taxonomy.json';
import { TRAIT_CATEGORIES } from './traitCategories';

describe('shared research taxonomy', () => {
  it('drives the discovery category UI from the resource file', () => {
    expect(TRAIT_CATEGORIES).toHaveLength(researchTaxonomy.categories.length);
    expect(TRAIT_CATEGORIES.find((category) => category.id === 'hormones_reproductive')).toEqual({
      id: 'hormones_reproductive',
      label: 'Hormone & reproductive',
      queryHint: 'menstrual cycle pmdd menopause fertility estrogen progesterone testosterone prostate',
    });
  });

  it('keeps routing coverage symmetric across body and health domains', () => {
    const ids = new Set(TRAIT_CATEGORIES.map((category) => category.id));
    for (const id of [
      'hormones_reproductive',
      'allergy_atopy_mast_cell',
      'kidney_fluid_electrolytes',
      'pain_migraine_sensory',
      'skin_hair_dermatology',
      'respiratory_airway',
    ]) {
      expect(ids.has(id)).toBe(true);
    }
  });
});
