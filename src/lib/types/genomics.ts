// ./src/lib/types/genomics.ts
/*
Module Docstring:
Purpose: Centralized TypeScript interface definitions for genetic data structures.
Responsibilities:
- Expose typed models for genomics reports, variant definitions, and search results.
- Define severity classification and section summary types for direction-aware scoring.
- Facilitate type safety across Svelte components.
Key Inputs: None.
Key Outputs: Declared interfaces.
Operational Notes: Matches the Rust backend structs in report.rs and db.rs.
*/

export type EffectDirection = "risk" | "protective" | "context_dependent" | "trait" | "unknown" | "not_applicable" | "no_claim";

export type VariantType = "snp" | "indel" | "repeat" | "cnv" | "hla" | "haplotype" | "star_allele" | "gene_panel";

export type SeverityClass =
  | "high_risk"
  | "moderate_risk"
  | "low_risk"       // weak-tier risk marker (Tier D/E)
  | "protective"
  | "trait"
  | "context_dependent"
  | "confirmation_required"
  | "no_data"
  | "benign";

export interface MarkerSource {
  name: string;
  url?: string;
  accessed?: string;
  evidence_type?: string;
  conflict_of_interest?: string;
  notes?: string;
}

/** A citation derived from the local reference database (ClinVar, GWAS, evidence_library). */
export interface EnrichedSource {
  source_type: string;   // "ClinVar", "GWAS", "PharmGKB", "EvidenceLibrary"
  citation: string;      // Human-readable citation, e.g. "ClinVar: Pathogenic (expert panel)"
  details?: string;
  url?: string;
}

export interface MarkerDefinition {
  rsid: string;
  gene: string;
  variant_name?: string | null;
  effect_allele: string;
  impact: string;
  evidence_tier: string;
  interpretation: string;
  do_not_claim: string[];
  confirm_with: string[];
  effect_direction: EffectDirection;
  raw_dna_limitation?: string | null;
  clinical_confirmation_required?: boolean | null;
  variant_type?: VariantType | null;
  sources?: MarkerSource[] | null;
  expected_plus_alleles?: string[] | null;
  strand?: string | null;
  source_build?: string | null;
  hgvs?: string | null;
  allele_orientation_verified?: boolean | null;
  orientation_source?: string | null;
  interpretation_blocked_if_unverified?: boolean | null;
}

export interface SectionDefinition {
  name: string;
  markers: MarkerDefinition[];
}

export interface ReportTemplate {
  title: string;
  description: string;
  sections: SectionDefinition[];
}

export interface CanonicalVariant {
  rsid: string;
  gene: string;
  variant_name?: string | null;
  chromosome?: string | null;
  position_grch37?: number | null;
  position_grch38?: number | null;
  variant_type?: string | null;
}

export type CallStatus = "Found" | "NoData" | "NotInRawFile";

export interface UserCall {
  user_genotype: string;
  normalized_genotype?: string | null;
  call_status: CallStatus;
  source_build?: string | null;
}

export type AssertionStatus =
  | "Verified"
  | "NoData"
  | "NotInRawFile"
  | "BlockedRawCall"
  | "UnverifiedOrientation"
  | "OrientationMismatch"
  | "AmbiguousAlleles"
  | "NotEvaluated";

export interface VariantCategoryLink {
  link_id: string;
  rsid: string;
  category_id: string;
  category_label: string;
  impact: string;
  evidence_tier: string;
  interpretation: string;
  effect_direction: EffectDirection;
  effect_allele: string;
  expected_plus_alleles?: string[] | null;
  severity_class: string;
  effect_count?: number | null;
  assertion_status: AssertionStatus;
  requires_orientation_verification: boolean;
  interpretation_allowed: boolean;
  do_not_claim: string[];
  confirm_with: string[];
  raw_dna_limitation?: string | null;
  clinical_confirmation_required?: boolean | null;
  interpretation_blocked_if_unverified?: boolean | null;
  sources: MarkerSource[];
}

export interface ClinVarAnnotation {
  clinical_significance: string;
  conditions?: string | null;
  review_status?: string | null;
}

export interface GwasHit {
  trait_name: string;
  p_value_string: string;
  neg_log10_p: number;
  association_count?: number | null;
  p_value_underflow?: boolean | null;
}

export interface DbsnpAnnotation {
  rsid: string;
  chromosome: string;
  position_grch37?: number | null;
  position_grch38?: number | null;
  ref_allele: string;
  alt_alleles: string[];
  plus_strand_alleles?: string[] | null;
  source_build?: string | null;
}

export interface PopulationAnnotation {
  allele_frequency: number;
  rarity_bucket: string;
}

export interface PharmGkbAnnotation {
  drug: string;
  phenotype: string;
  evidence_level: string;
}

export interface ClinGenAnnotation {
  gene_symbol: string;
  disease_label: string;
  classification: string;
}

export interface ManeAnnotation {
  gene_symbol: string;
  ensembl_transcript: string;
  refseq_transcript: string;
  mane_status: string;
}

export interface VariantEnrichment {
  clinvar?: ClinVarAnnotation | null;
  gwas_hits: GwasHit[];
  dbsnp?: DbsnpAnnotation | null;
  population?: PopulationAnnotation | null;
  pharmgkb?: PharmGkbAnnotation | null;
  clingen?: ClinGenAnnotation | null;
  mane?: ManeAnnotation | null;
  db_enriched_sources: EnrichedSource[];
}

/** Direction-aware summary statistics for a report section. */
export interface SectionSummary {
  risk_effect_count: number;
  risk_possible: number;
  protective_effect_count: number;
  protective_possible: number;
  trait_count: number;
  context_dependent_count: number;
  no_data_count: number;
  confirmation_required_count: number;
  total_markers: number;
  show_percent_score: boolean;
  all_require_confirmation: boolean;
  active_marker_count: number;
  active_risk_marker_count: number;
  active_protective_marker_count: number;
  active_trait_marker_count: number;
  active_context_marker_count: number;
  blocked_unverified_count: number;
  benign_modifier_count: number;
}

export interface NormalizedSection {
  section_id: string;
  name: string;
  link_ids: string[];
  section_signal_score: number;
  summary: SectionSummary;
}

export interface NormalizedReport {
  schema_version: string;
  export_format: string;
  generated_at: string;
  title: string;
  description: string;
  overall_signal_score: number;
  variants: Record<string, CanonicalVariant>;
  user_calls: Record<string, UserCall>;
  category_links: Record<string, VariantCategoryLink>;
  enrichment: Record<string, VariantEnrichment>;
  sections: NormalizedSection[];
}

/** Flattened display-ready model computed once per report/category tab change. */
export interface DisplayMarker {
  link_id: string;
  rsid: string;
  gene: string;
  variant_name: string;
  chromosome: string;
  position: number | null;
  user_genotype: string;
  normalized_genotype: string | null;
  effect_allele: string;
  effect_count: number | null;
  impact: string;
  evidence_tier: string;
  interpretation: string;
  effect_direction: EffectDirection;
  severity_class: SeverityClass;
  assertion_status: AssertionStatus;
  interpretation_allowed: boolean;
  sources: MarkerSource[];
  db_enriched_sources: EnrichedSource[];
  clinvar_significance: string | null;
  clinvar_conditions: string | null;
  clinvar_review_status: string | null;
  gwas_top_trait: string | null;
  gwas_best_pvalue: number | null;
  gwas_association_count: number | null;
  population_af: number | null;
  population_rarity: string | null;
  confirm_with: string[];
  do_not_claim: string[];
  clinical_confirmation_required?: boolean;
  raw_dna_limitation?: string;
  pharmgkb?: PharmGkbAnnotation | null;
  clingen?: ClinGenAnnotation | null;
  mane?: ManeAnnotation | null;
}

export interface EvaluatedMarker extends DisplayMarker {}

export interface EvaluatedSection {
  name: string;
  markers: EvaluatedMarker[];
  section_signal_score: number;
  summary: SectionSummary;
}

export interface GeneratedReport {
  schema_version: string;
  export_format: string;
  generated_at: string;
  title: string;
  description: string;
  overall_signal_score: number;
  variants: Record<string, CanonicalVariant>;
  user_calls: Record<string, UserCall>;
  category_links: Record<string, VariantCategoryLink>;
  enrichment: Record<string, VariantEnrichment>;
  sections: EvaluatedSection[];
}

export interface ReportPayload {
  report: GeneratedReport;
  raw: NormalizedReport;
}

export interface GenomeSample {
  id: number;
  name: string;
  genetic_sex: string;
  imported_at: string;
  qc_status?: string | null;
  call_rate?: number | null;
  titv_ratio?: number | null;
  heterozygosity_rate?: number | null;
}

export type DataDirMode =
  | "env_override"
  | "app_layout"
  | "legacy_project";

export interface AppPaths {
  project_root: string;
  app_layout_dir: string;
  data_dir: string;
  data_dir_mode: DataDirMode;
  db_path: string;
  chain_path: string;
  references_dir: string;
  raw_downloads_dir: string;
  env_path?: string;
}

/** Startup payload after a single DB open + migration pass. */
export interface AppBootstrapStatus {
  data_dir: string;
  db_path: string;
  chain_path: string;
  chain_present: boolean;
  env_path: string;
  sample_count: number;
  genotype_count: number;
  discovered_findings_count: number;
  gwas_reference_count: number;
  evidence_library_count: number;
  samples: GenomeSample[];
}

export interface DiscoveredFindingSummary {
  rsid: string;
  gene?: string | null;
  user_genotype?: string | null;
  interpretation_status: string;
  clinvar_clinical_significance?: string | null;
}

export interface VectorPromotedFinding {
  rsid: string;
  gene?: string | null;
  user_genotype?: string | null;
  trait_summary: string;
  trait_categories: string[];
  significance_score: number;
  gwas_best_pvalue?: number | null;
  enrichment_version: string;
  promoted_at: number;
}

export interface DbSnpRecord {
  sample_id: number;
  rsid: string;
  chromosome: string;
  position_grch37: number;
  position_grch38: number | null;
  allele1: string;
  allele2: string;
}
