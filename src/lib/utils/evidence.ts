// ./src/lib/utils/evidence.ts
/*
Module Docstring:
Purpose: Evidence tier, direction, and severity display utility.
Responsibilities:
- Resolve human-readable badges, color classes, and plain-language labels.
- Generate user-friendly section summary text (no medical jargon).
- Map severity_class strings to CSS class names for card styling.
Key Inputs: Evidence tier codes, effect directions, severity classes, and section summaries.
Key Outputs: Display-ready objects (labels, CSS classes, explanation strings).
Operational Notes: All user-facing text should be understandable without a medical background.
*/

import type { EffectDirection, SeverityClass, SectionSummary } from "../types/genomics";

// ---------------------------------------------------------------------------
// Evidence Tier
// ---------------------------------------------------------------------------

export interface TierInfo {
  label: string;
  description: string;
  colorClass: string;
}

export function getTierInfo(tier: string): TierInfo {
  switch (tier) {
    case "A_clinical_guideline":
      return {
        label: "Tier A — Clinical Guideline",
        description: "Supported by established medical guidelines (e.g. CPIC).",
        colorClass: "tier-a"
      };
    case "B_replicated_common_marker":
      return {
        label: "Tier B — Well-Studied",
        description: "Confirmed across multiple large-scale studies.",
        colorClass: "tier-b"
      };
    case "C_biohacker_hypothesis":
      return {
        label: "Tier C — Preliminary",
        description: "Limited evidence. Treat as hypothesis, not fact.",
        colorClass: "tier-c"
      };
    case "D_research_only":
      return {
        label: "Tier D — Research Only",
        description: "Early research. Not validated for clinical use.",
        colorClass: "tier-d"
      };
    default:
      return {
        label: "Tier D — Research Only",
        description: "Early research. Not validated for clinical use.",
        colorClass: "tier-d"
      };
  }
}

// ---------------------------------------------------------------------------
// Effect Direction
// ---------------------------------------------------------------------------

export interface DirectionInfo {
  label: string;
  plainLabel: string;
  description: string;
  colorClass: string;
}

export function getDirectionInfo(direction: EffectDirection): DirectionInfo {
  switch (direction) {
    case "risk":
      return {
        label: "Risk Modifier",
        plainLabel: "May increase risk",
        description: "This variant is associated with a higher likelihood of the described trait or condition.",
        colorClass: "direction-risk"
      };
    case "protective":
      return {
        label: "Protective Modifier",
        plainLabel: "May be beneficial",
        description: "This variant is associated with reduced risk or improved function.",
        colorClass: "direction-protective"
      };
    case "context_dependent":
      return {
        label: "Context-Dependent",
        plainLabel: "Depends on context",
        description: "The effect of this variant depends on other factors like diet, medication, or other genes.",
        colorClass: "direction-context_dependent"
      };
    case "trait":
      return {
        label: "Trait Marker",
        plainLabel: "Describes a trait",
        description: "This variant is linked to a physical or behavioral characteristic, not a disease risk.",
        colorClass: "direction-trait"
      };
    case "unknown":
    default:
      return {
        label: "Not Yet Classified",
        plainLabel: "Effect unclear",
        description: "The direction of this variant's effect has not been definitively established.",
        colorClass: "direction-unknown"
      };
  }
}

// ---------------------------------------------------------------------------
// Severity Class (for VariantCard styling)
// ---------------------------------------------------------------------------

export interface SeverityInfo {
  cssClass: string;
  emoji: string;
  label: string;
  description: string;
}

export function getSeverityInfo(severityClass: SeverityClass): SeverityInfo {
  switch (severityClass) {
    case "high_risk":
      return {
        cssClass: "signal-high-risk",
        emoji: "🔴",
        label: "Two risk alleles detected",
        description: "Both copies of this gene carry the variant associated with increased risk."
      };
    case "moderate_risk":
      return {
        cssClass: "signal-moderate-risk",
        emoji: "🟡",
        label: "One risk allele detected",
        description: "One copy carries the risk variant. Effect is typically smaller than two copies."
      };
    case "protective":
      return {
        cssClass: "signal-protective",
        emoji: "🟢",
        label: "Protective variant detected",
        description: "This variant is associated with a beneficial or protective effect."
      };
    case "trait":
      return {
        cssClass: "signal-trait",
        emoji: "🔵",
        label: "Trait variant detected",
        description: "This variant describes a characteristic, not a health risk."
      };
    case "context_dependent":
      return {
        cssClass: "signal-context",
        emoji: "🟣",
        label: "Context-dependent variant",
        description: "This variant's effect depends on other factors like medications, diet, or environment."
      };
    case "confirmation_required":
      return {
        cssClass: "signal-confirm",
        emoji: "⚠️",
        label: "Clinical confirmation needed",
        description: "Consumer DNA tests can produce false positives on rare variants. A clinical lab test is needed to confirm."
      };
    case "no_data":
      return {
        cssClass: "signal-nodata",
        emoji: "⚪",
        label: "No data available",
        description: "Your DNA test did not include or could not read this position."
      };
    case "benign":
    default:
      return {
        cssClass: "signal-benign",
        emoji: "○",
        label: "Variant not detected",
        description: "Your genotype at this position matches the standard reference. The effect allele being tested was not found."
      };
  }
}

// ---------------------------------------------------------------------------
// Section Summary (plain-language)
// ---------------------------------------------------------------------------

/**
 * Generate a human-readable summary string for a section.
 * Used when show_percent_score is false (e.g. cancer, PGx sections).
 */
export function getSectionSummaryParts(summary: SectionSummary): string[] {
  const parts: string[] = [];

  if (summary.risk_effect_count > 0) {
    const alleleWord = summary.risk_effect_count === 1 ? "allele" : "alleles";
    parts.push(`${summary.risk_effect_count} risk ${alleleWord} found`);
  }
  if (summary.protective_effect_count > 0) {
    const alleleWord = summary.protective_effect_count === 1 ? "allele" : "alleles";
    parts.push(`${summary.protective_effect_count} protective ${alleleWord}`);
  }
  if (summary.trait_count > 0) {
    const markerWord = summary.trait_count === 1 ? "trait marker" : "trait markers";
    parts.push(`${summary.trait_count} ${markerWord}`);
  }
  if (summary.context_dependent_count > 0) {
    parts.push(`${summary.context_dependent_count} context-dependent`);
  }
  if (summary.confirmation_required_count > 0) {
    parts.push(`${summary.confirmation_required_count} need clinical confirmation`);
  }
  if (summary.no_data_count > 0) {
    parts.push(`${summary.no_data_count} not tested`);
  }

  if (parts.length === 0) {
    parts.push("No notable findings");
  }

  return parts;
}

// ---------------------------------------------------------------------------
// Clinical Warning
// ---------------------------------------------------------------------------

export function getClinicalWarning(gene: string): string {
  return `🔒 Clinical Confirmation Required: Consumer-grade DNA tests can sometimes report false positives on rare variants. A clinical lab test (CLIA/CAP certified) is needed to confirm any findings for ${gene}.`;
}
