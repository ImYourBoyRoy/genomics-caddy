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
import evidencePolicy from "../marker-packs/evidence_policy.json";

// ---------------------------------------------------------------------------
// Evidence Tier
// ---------------------------------------------------------------------------

export interface TierInfo {
  label: string;
  description: string;
  colorClass: string;
  /** Evidence strength is not the same as personal certainty or disease probability. */
  confidenceLabel: string;
}

export function getTierInfo(tier: string): TierInfo {
  const normalized = String(tier || "").trim();
  const band = normalized.charAt(0).toUpperCase();
  const display = (evidencePolicy.display.tiers as Record<string, {
    label: string;
    description: string;
    color_class: string;
    confidence_label: string;
  }>)[band] || evidencePolicy.display.unknown_tier;
  return {
    label: display.label,
    description: display.description,
    colorClass: display.color_class,
    confidenceLabel: display.confidence_label,
  };
}

/** Keep the primary Simple card label short; the full context remains in the tooltip. */
export function getSimpleTierLabel(tier: string): string {
  const band = String(tier || '').trim().charAt(0).toUpperCase();
  const labels: Record<string, string> = {
    A: 'Higher evidence',
    B: 'Moderate evidence',
    C: 'Limited evidence',
    D: 'Research only',
    E: 'Safety boundary',
  };
  return labels[band] || 'Evidence context';
}

/**
 * Add a visible claim frame beside every interpretation. Evidence strength,
 * genotype matching, and clinical actionability are separate dimensions.
 */
export function getClaimFrame(
  tier: string,
  clinicalConfirmationRequired = false,
  interpretationAllowed = true
): string {
  const frames = evidencePolicy.display.claim_frames;
  if (!interpretationAllowed) {
    return frames.interpretation_blocked;
  }
  if (clinicalConfirmationRequired || String(tier || "").startsWith("A_")) {
    return frames.clinical_confirmation;
  }
  const band = String(tier || "").trim().charAt(0).toUpperCase();
  if (band === "B" || band === "C" || band === "D" || band === "E") return frames[band];
  return frames.unknown;
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
  const display = evidencePolicy.display.directions[direction] || evidencePolicy.display.directions.unknown;
  return {
    label: display.label,
    plainLabel: display.plain_label,
    description: display.description,
    colorClass: display.color_class,
  };
}

// ---------------------------------------------------------------------------
// Severity Class (for VariantCard styling)
// ---------------------------------------------------------------------------

export interface SeverityInfo {
  cssClass: string;
  emoji: string;
  glyph: string;
  label: string;
  plainLabel: string;
  legendLabel: string;
  legendDescription: string;
  description: string;
}

export function getSeverityInfo(severityClass: SeverityClass): SeverityInfo {
  const display = evidencePolicy.display.severity[severityClass] || evidencePolicy.display.severity.benign;
  return {
    cssClass: display.css_class,
    emoji: display.emoji,
    glyph: display.glyph,
    label: display.label,
    plainLabel: display.plain_label,
    legendLabel: display.legend_label,
    legendDescription: display.legend_description,
    description: display.description,
  };
}

/** Resolve biological-applicability scope wording without implying identity or anatomy. */
export function getScopeLabel(scope: string): string {
  const normalized = String(scope || '').trim();
  const label = (evidencePolicy.display.scope_labels as Record<string, string>)[normalized];
  return label || normalized.replaceAll('_', ' ');
}

// ---------------------------------------------------------------------------
// Section Summary (plain-language)
// ---------------------------------------------------------------------------

export function getSectionSummaryParts(summary: SectionSummary): string[] {
  const parts: string[] = [];

  // Check if new detailed counts are available
  if (summary.active_risk_marker_count !== undefined) {
    if (summary.active_risk_marker_count > 0) {
      const markerWord = summary.active_risk_marker_count === 1 ? "risk variant" : "risk variants";
      parts.push(`${summary.active_risk_marker_count} ${markerWord}`);
    }
    if (summary.active_protective_marker_count > 0) {
      const markerWord = summary.active_protective_marker_count === 1 ? "protective variant" : "protective variants";
      parts.push(`${summary.active_protective_marker_count} ${markerWord}`);
    }
    if (summary.active_trait_marker_count > 0) {
      const markerWord = summary.active_trait_marker_count === 1 ? "trait marker" : "trait markers";
      parts.push(`${summary.active_trait_marker_count} ${markerWord}`);
    }
    if (summary.active_context_marker_count > 0) {
      const markerWord = summary.active_context_marker_count === 1 ? "context variant" : "context variants";
      parts.push(`${summary.active_context_marker_count} ${markerWord}`);
    }
    if (summary.blocked_unverified_count > 0) {
      parts.push(`${summary.blocked_unverified_count} blocked (unverified)`);
    }
    if (summary.confirmation_required_count > 0) {
      parts.push(`${summary.confirmation_required_count} require validation`);
    }
  } else {
    // Fallback to legacy count mapping
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
