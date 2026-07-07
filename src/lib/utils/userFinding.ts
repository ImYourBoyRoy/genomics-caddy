// ./src/lib/utils/userFinding.ts
/**
 * Converts normalized evidence cards into user-facing finding summaries.
 * This is the presentation translation layer that lets dynamically scanned
 * findings read like curated report findings without hiding provenance.
 */

import type { EvidenceCard } from "../types/research";

export type UserFindingImpact =
  | "clinical_review"
  | "wellness_relevant"
  | "interpretable_research"
  | "interesting_lead"
  | "low_confidence";

export interface UserFacingFinding {
  title: string;
  plainEnglishMeaning: string;
  impact: UserFindingImpact;
  impactLabel: string;
  impactDescription: string;
  confidenceLabel: string;
  safetyBoundary: string;
  primaryNextStep: string;
  whySurfaced: string[];
  suggestedNextSteps: string[];
  evidenceBadges: string[];
}

function pct(value: number | undefined): number {
  return Math.round((value ?? 0) * 100);
}

function cleanLabel(value: string | undefined): string {
  return value?.replace(/_/g, " ").trim() || "";
}

function impactForCard(card: EvidenceCard): UserFindingImpact {
  if (card.clinical_actionability_score >= 0.35) return "clinical_review";
  if (card.wellness_actionability_score >= 0.35 && card.data_quality_score >= 0.45) {
    return "wellness_relevant";
  }
  if (card.has_direction && card.data_quality_score >= 0.4) return "interpretable_research";
  if (card.data_quality_score < 0.35 || card.missing_field_count >= 3) return "low_confidence";
  return "interesting_lead";
}

function confidenceForCard(card: EvidenceCard): string {
  if (card.data_quality_score >= 0.75 && card.has_direction) return "Higher confidence";
  if (card.data_quality_score >= 0.55) return "Moderate confidence";
  if (card.data_quality_score >= 0.35) return "Limited confidence";
  return "Low confidence";
}

function impactLabel(impact: UserFindingImpact): string {
  switch (impact) {
    case "clinical_review":
      return "Clinical review queue";
    case "wellness_relevant":
      return "Wellness research lead";
    case "interpretable_research":
      return "Interpretable research";
    case "low_confidence":
      return "Needs better evidence";
    default:
      return "Background research";
  }
}

function impactDescription(impact: UserFindingImpact): string {
  switch (impact) {
    case "clinical_review":
      return "Prioritize provenance review before treating this as personally meaningful.";
    case "wellness_relevant":
      return "Potentially useful for personal context after source and phenotype review.";
    case "interpretable_research":
      return "Direction is available, but the scan is still a research aid rather than advice.";
    case "low_confidence":
      return "Metadata or source coverage is thin; use this mainly to guide follow-up.";
    default:
      return "Useful as context when it clusters with stronger findings.";
  }
}

function titleForCard(card: EvidenceCard): string {
  const gene = card.gene_symbol ? `${card.gene_symbol} ` : "";
  const trait = card.primary_trait || cleanLabel(card.trait_category) || "indexed association";
  return `${gene}${card.rsid}: ${trait}`;
}

function meaningForCard(card: EvidenceCard): string {
  const genotype = card.genotype ? `Your genotype is ${card.genotype}. ` : "";
  const gene = card.gene_symbol ? `This marker is mapped near ${card.gene_symbol}. ` : "";
  const direction = card.has_direction
    ? `${card.directionality_label}. `
    : "The scan found evidence, but the personal direction is not known from the current data. ";
  const trait = card.primary_trait || cleanLabel(card.trait_category) || "this trait area";
  return `${genotype}${gene}${direction}Treat this as a lead about ${trait}, not as a diagnosis or standalone health instruction.`;
}

function whySurfaced(card: EvidenceCard): string[] {
  const reasons: string[] = [];
  if (card.has_gwas) reasons.push("Matched one or more GWAS associations.");
  if (card.has_clinvar) reasons.push("Has ClinVar context.");
  if (card.has_pgs) reasons.push("Overlaps PGS Catalog context.");
  if (card.has_pharmgkb) reasons.push("Has pharmacogenomics context.");
  if (card.has_reactome) reasons.push("Connected to a pathway source.");
  if (card.has_direction) reasons.push("Effect direction could be interpreted for your genotype.");
  if (card.source_count > 1) reasons.push(`Aggregated ${card.source_count} evidence sources.`);
  if (card.quality_flags.length > 0) {
    reasons.push(`Quality flags: ${card.quality_flags.slice(0, 3).map(cleanLabel).join(", ")}.`);
  }
  return reasons.length > 0 ? reasons : ["Indexed because it matched your scan scope and genotype data."];
}

function nextSteps(card: EvidenceCard, impact: UserFindingImpact): string[] {
  const steps: string[] = [];
  if (impact === "clinical_review") {
    steps.push("Review primary sources and ClinVar details before drawing any clinical conclusion.");
    steps.push("Discuss clinically relevant concerns with a qualified professional if this matches your health context.");
  } else if (impact === "wellness_relevant") {
    steps.push("Compare this finding with your actual labs, symptoms, lifestyle, and family context.");
    steps.push("Use the AI consultation or evidence packet export to summarize supporting sources.");
  } else if (impact === "low_confidence") {
    steps.push("Treat as a research lead until missing fields are backfilled or sources improve.");
    steps.push("Prioritize variants with higher data quality and known direction first.");
  } else {
    steps.push("Open details to inspect source provenance and directionality limitations.");
    steps.push("Search for similar findings to see if the signal clusters with related genes or traits.");
  }
  if (card.verification_ideas.length > 0) {
    steps.push(card.verification_ideas[0]);
  }
  return Array.from(new Set(steps)).slice(0, 4);
}

function badgesForCard(card: EvidenceCard): string[] {
  const badges: string[] = [];
  badges.push(`Data quality ${pct(card.data_quality_score)}%`);
  if (card.wellness_actionability_score > 0) {
    badges.push(`Wellness relevance ${pct(card.wellness_actionability_score)}%`);
  }
  if (card.clinical_actionability_score > 0) {
    badges.push(`Clinical review signal ${pct(card.clinical_actionability_score)}%`);
  }
  badges.push(card.has_direction ? "Direction known" : "Direction unknown");
  if (card.source_count > 0) badges.push(`${card.source_count} evidence sources`);
  return badges;
}

export function userFindingFromEvidenceCard(card: EvidenceCard): UserFacingFinding {
  const impact = impactForCard(card);
  const suggestedNextSteps = nextSteps(card, impact);
  const safetyBoundary =
    impact === "clinical_review"
      ? "Clinical review item — this app does not diagnose or recommend treatment."
      : "Research and education only — verify against primary sources and real-world phenotype.";

  return {
    title: titleForCard(card),
    plainEnglishMeaning: meaningForCard(card),
    impact,
    impactLabel: impactLabel(impact),
    impactDescription: impactDescription(impact),
    confidenceLabel: confidenceForCard(card),
    safetyBoundary,
    primaryNextStep: suggestedNextSteps[0] ?? "Open details and review source provenance before acting on this finding.",
    whySurfaced: whySurfaced(card),
    suggestedNextSteps,
    evidenceBadges: badgesForCard(card),
  };
}
