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

/** Explicit interpretation semantics; these labels never turn an association into a diagnosis. */
export type FindingInterpretationClass =
  | "susceptibility_context"
  | "carrier_possibility"
  | "clinically_actionable_variant"
  | "research_context"
  | "protective_context"
  | "trait_context"
  | "unknown";

export type FindingInheritanceModel =
  | "autosomal_dominant"
  | "autosomal_recessive"
  | "x_linked"
  | "y_linked"
  | "mitochondrial"
  | "unknown";

export type FindingClinicalState =
  | "clinically_confirmed"
  | "carrier_possibility"
  | "unknown"
  | "not_applicable";

/** Optional pack-authored semantics. Missing fields remain unknown. */
export interface ClinicalSemantics {
  condition_label?: string | null;
  interpretation_class?: FindingInterpretationClass | null;
  inheritance_model?: FindingInheritanceModel | null;
  clinical_state?: FindingClinicalState | null;
}

export type VariantType = "snp" | "indel" | "repeat" | "cnv" | "hla" | "haplotype" | "star_allele" | "gene_panel";

/**
 * Optional applicability hint for explicitly sex-linked or reproductive markers.
 * This is biological/genomic context, not gender identity or anatomy. Missing
 * values mean the marker is intended for all users unless the pack says otherwise.
 */
export type MarkerSexScope =
  | "all"
  | "xx_reproductive"
  | "xy_reproductive"
  | "x_linked"
  | "y_linked"
  | "menstrual_cycle_context"
  | "ovarian_context"
  | "uterine_context"
  | "androgen_reproductive_context"
  | string;

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

/** A citation derived from the local reference database (ClinVar, GWAS, ClinPGx, evidence_library). */
export interface EnrichedSource {
  source_type: string;   // "ClinVar", "GWAS", "ClinPGx" (formerly PharmGKB), "EvidenceLibrary"
  citation: string;      // Human-readable citation, e.g. "ClinVar: Pathogenic (expert panel)"
  details?: string;
  url?: string;
}

/** Serialized report-level reference metadata from the Rust report generator. */
export interface ReportReferenceRecord {
  id: string;
  title: string;
  organization: string;
  date: string;
  evidence_role: string;
  url: string | null;
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
  sex_scope?: MarkerSexScope | null;
  variant_type?: VariantType | null;
  sources?: MarkerSource[] | null;
  expected_plus_alleles?: string[] | null;
  strand?: string | null;
  source_build?: string | null;
  hgvs?: string | null;
  allele_orientation_verified?: boolean | null;
  orientation_source?: string | null;
  interpretation_blocked_if_unverified?: boolean | null;
  /** Explicit disease/inheritance semantics; never inferred from a gene name. */
  clinical_semantics?: ClinicalSemantics | null;
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
  sex_scope?: MarkerSexScope | null;
  interpretation_blocked_if_unverified?: boolean | null;
  clinical_semantics?: ClinicalSemantics | null;
  sources: MarkerSource[];
  /** Stable IDs into the report-level reference registry. */
  reference_ids?: string[];
}

export interface ClinVarAnnotation {
  clinical_significance: string;
  conditions?: string | null;
  review_status?: string | null;
  rsid?: string | null;
  allele_id?: string | null;
  variation_id?: string | null;
  gene_symbol?: string | null;
  name?: string | null;
  phenotype_ids?: string | null;
  assembly?: string | null;
  chromosome?: string | null;
  start?: number | null;
  stop?: number | null;
  last_evaluated?: string | null;
  number_submitters?: number | null;
  reference_allele?: string | null;
  alternate_allele?: string | null;
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
  /** Release label from allele source (typically gnomAD). */
  source_build?: string | null;
  /** Explicit provenance — allele/AF chips come from gnomAD cache, not offline dbSNP merge JSON. */
  allele_source?: string;
}

export interface PopulationAnnotation {
  allele_frequency: number;
  rarity_bucket: string;
  source_mode?: string | null;
  dataset?: string | null;
  source_build?: string | null;
  popmax?: number | null;
  popmax_population?: string | null;
  faf95_popmax?: number | null;
  homozygote_count?: number | null;
}

export interface PharmGkbAnnotation {
  drug: string;
  phenotype: string;
  evidence_level: string;
  gene?: string | null;
}

export interface ClinGenAnnotation {
  gene_symbol: string;
  disease_label: string;
  classification: string;
  hgnc_id?: string | null;
  mode_of_inheritance?: string | null;
  report_url?: string | null;
}

export interface ManeAnnotation {
  gene_symbol: string;
  ensembl_transcript: string;
  refseq_transcript: string;
  mane_status: string;
  grch38_coordinates?: string | null;
}

export interface VariantEnrichment {
  clinvar?: ClinVarAnnotation | null;
  clinvar_annotations?: ClinVarAnnotation[];
  gwas_hits: GwasHit[];
  dbsnp?: DbsnpAnnotation | null;
  population?: PopulationAnnotation | null;
  pharmgkb?: PharmGkbAnnotation | null;
  pharmgkb_annotations?: PharmGkbAnnotation[];
  clingen?: ClinGenAnnotation | null;
  clingen_annotations?: ClinGenAnnotation[];
  mane?: ManeAnnotation | null;
  mane_annotations?: ManeAnnotation[];
  db_enriched_sources: EnrichedSource[];
  /** Stable IDs into the report-level reference registry. */
  reference_ids?: string[];
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
  /** Deduplicated source metadata from the Rust report generator. */
  references?: ReportReferenceRecord[];
}

/** Flattened display-ready model computed once per report/category tab change. */
export interface DisplayMarker {
  link_id: string;
  rsid: string;
  gene: string;
  variant_name: string;
  /** Resource-defined marker class used for category-aware plain-language copy. */
  variant_type?: string | null;
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
  /** Stable IDs for all sources attached to this finding. */
  reference_ids?: string[];
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
  sex_scope?: MarkerSexScope;
  raw_dna_limitation?: string;
  clinical_semantics?: ClinicalSemantics | null;
  pharmgkb?: PharmGkbAnnotation | null;
  pharmgkb_annotations?: PharmGkbAnnotation[];
  clingen?: ClinGenAnnotation | null;
  clingen_annotations?: ClinGenAnnotation[];
  mane?: ManeAnnotation | null;
  mane_annotations?: ManeAnnotation[];
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
  /** Deduplicated source metadata from the normalized report payload. */
  references?: ReportReferenceRecord[];
  /** Explicit when ClinVar/dbSNP catalogs are missing or empty (never silent). */
  catalog_warnings?: string[];
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
