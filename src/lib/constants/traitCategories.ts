// ./src/lib/constants/traitCategories.ts
/** GWAS trait taxonomy keys aligned with Rust crossmap.rs */

export interface TraitCategoryOption {
  id: string;
  label: string;
  queryHint: string;
}

export const TRAIT_CATEGORIES: TraitCategoryOption[] = [
  { id: "bone_density", label: "Bone density", queryHint: "bone mineral density osteoporosis fracture" },
  { id: "cancer_risk", label: "Cancer risk", queryHint: "cancer carcinoma tumor malignancy" },
  { id: "connective_tissue", label: "Connective tissue", queryHint: "collagen joint ligament ehlers" },
  { id: "cardiovascular", label: "Cardiovascular", queryHint: "heart stroke blood pressure cholesterol thrombo" },
  { id: "metabolic", label: "Metabolic", queryHint: "diabetes glucose insulin obesity" },
  { id: "neuropsych", label: "Brain & mood", queryHint: "depression anxiety mood neuroticism" },
  { id: "sleep", label: "Sleep", queryHint: "insomnia circadian chronotype" },
  { id: "thyroid_autoimmune", label: "Thyroid & autoimmune", queryHint: "thyroid hashimoto autoimmune tsh" },
  { id: "nutrients", label: "Nutrients", queryHint: "vitamin folate methylation iron" },
  { id: "pharmacogenomics", label: "Pharmacogenomics", queryHint: "drug warfarin cyp statin response" },
];

export type VariantNavTarget = "report" | "map" | "browser" | "evidence";
