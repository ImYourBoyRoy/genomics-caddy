// ./src/lib/types/agent.ts
/*
Module Docstring:
Purpose: TypeScript interface definitions for Genomics Research Agent and Evidence-Grade Validation.
Responsibilities:
- Define types for VariantEvidence and its matching enums (AlleleMatchStatus, OrientationStatus, InterpretationStatus).
- Define types for deterministic safety checks and QA audits.
Key Inputs: None.
Key Outputs: Declared interfaces and type definitions.
Operational Notes: Matches the Rust backend types defined in agent.rs.
*/

export type AlleleMatchStatus =
  | "matched"
  | "not_matched"
  | "no_data"
  | "orientation_unverified"
  | "ambiguous";

export type OrientationStatus =
  | "verified"
  | "unverified"
  | "conflicting"
  | "not_required";

export type InterpretationStatus =
  | "active_clinical"
  | "active_research"
  | "benign"
  | "modifier_only"
  | "not_active_for_user"
  | "blocked_unverified"
  | "conflicting"
  | "insufficient_evidence"
  | "mechanism_context_only";

export interface VariantEvidence {
  rsid: string;
  gene?: string;
  user_genotype?: string;
  user_alleles?: string[];

  matched_effect_allele?: string | null;
  clinically_relevant_allele?: string | null;

  allele_match_status: AlleleMatchStatus;
  orientation_status: OrientationStatus;

  genome_build?: "GRCh37" | "GRCh38" | "both" | "unknown";
  hgvs?: string | null;

  clinvar_variation_id?: string | null;
  clinvar_clinical_significance?: string | null;
  clinvar_review_status?: string | null;
  clinvar_condition?: string | null;
  clinvar_conflict_status?: string | null;

  evidence_source_ids: string[];

  interpretation_status: InterpretationStatus;

  allowed_language: string;
  forbidden_language: string[];

  requires_clinical_confirmation: boolean;
  safe_for_ai_context: boolean;

  notes?: string[];
}

export interface SafetyFinding {
  rule_id: string;
  severity: "warning" | "fail";
  message: string;
  excerpt?: string;
}

export interface SafetyCheckResult {
  passed: boolean;
  severity: "pass" | "warning" | "fail";
  findings: SafetyFinding[];
}

export interface SearchSourceResult {
  sourceType: 'pubmed' | 'clinvar' | 'dbsnp' | 'web' | 'qdrant';
  title: string;
  url: string;
  snippet?: string;
  author?: string;
  year?: string;
}

export interface ChatMessage {
  role: 'user' | 'assistant' | 'system';
  content: string;
  fullContent?: string;
  images?: string[];
  safetyReview?: string;
  safetyAudit?: SafetyCheckResult;
  isSearching?: boolean;
  searchStatus?: string;
  retrievedSources?: SearchSourceResult[];
}
