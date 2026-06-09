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

export type EffectDirection = "risk" | "protective" | "context_dependent" | "trait" | "unknown";

export type VariantType = "snp" | "indel" | "repeat" | "cnv" | "hla" | "haplotype" | "star_allele" | "gene_panel";

export type SeverityClass =
  | "high_risk"
  | "moderate_risk"
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

export interface MarkerDefinition {
  rsid: string;
  gene: string;
  variant_name?: string;
  effect_allele: string;
  impact: string;
  evidence_tier: string;
  interpretation: string;
  do_not_claim: string[];
  confirm_with: string[];
  effect_direction: EffectDirection;
  raw_dna_limitation?: string;
  clinical_confirmation_required?: boolean;
  variant_type?: VariantType;
  sources?: MarkerSource[];
  expected_plus_alleles?: string[];
  strand?: string;
  source_build?: string;
  hgvs?: string;
  allele_orientation_verified?: boolean;
  orientation_source?: string;
  interpretation_blocked_if_unverified?: boolean;
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

export interface EvaluatedMarker {
  rsid: string;
  gene: string;
  variant_name?: string;
  user_genotype: string;
  effect_allele: string;
  effect_count: number;
  impact: string;
  evidence_tier: string;
  interpretation: string;
  do_not_claim: string[];
  confirm_with: string[];
  effect_direction: EffectDirection;
  raw_dna_limitation?: string;
  clinical_confirmation_required?: boolean;
  variant_type?: VariantType;
  sources?: MarkerSource[];
  severity_class: SeverityClass;
  expected_plus_alleles?: string[];
  strand?: string;
  source_build?: string;
  hgvs?: string;
  allele_orientation_verified?: boolean;
  orientation_source?: string;
  interpretation_blocked_if_unverified?: boolean;
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
  active_marker_count: number;
  active_risk_marker_count: number;
  active_protective_marker_count: number;
  active_trait_marker_count: number;
  active_context_marker_count: number;
  blocked_unverified_count: number;
  benign_modifier_count: number;
}

export interface EvaluatedSection {
  name: string;
  markers: EvaluatedMarker[];
  /** Risk-direction-only signal score (0-100%). */
  section_signal_score: number;
  /** Direction-aware tallies for the section. */
  summary: SectionSummary;
}

export interface GeneratedReport {
  title: string;
  description: string;
  sections: EvaluatedSection[];
  /** Risk-direction-only overall signal score. */
  overall_signal_score: number;
}

export interface GenomeSample {
  id: number;
  name: string;
  genetic_sex: string;
  imported_at: string;
}

export interface AppPaths {
  db_path: string;
  chain_path: string;
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
