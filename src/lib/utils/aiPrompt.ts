// ./src/lib/utils/aiPrompt.ts
/**
 * AI system prompt construction and model intelligence utilities.
 *
 * Responsibilities:
 * - Assemble the full system prompt including genomic JSON context, user
 *   biohacking profile, date-aware bloodwork aging warnings, and custom
 *   instructions.
 * - Calculate context statistics (active vs total variants sent to LLM).
 * - Detect active finding categories to drive dynamic curated helper prompts.
 * - Classify models as reasoning-capable or vision-capable.
 *
 * Key Inputs:  GeneratedReport, GenomeSample, selectedPacks, userProfile.
 * Key Outputs: Fully assembled system prompt string, context stats, model flags.
 */

import type { GeneratedReport, GenomeSample, EvaluatedMarker } from "../types/genomics";

// ---------------------------------------------------------------------------
// Shared Interfaces (re-exported for convenience)
// ---------------------------------------------------------------------------

export interface UserBiohackingProfile {
  goals: string;
  challenges: string;
  diet: string;
  supplements: string;
  medications: string;
  bloodwork: string;
  diagnoses: string;
  supportiveTests: string;
  injectProfile: boolean;
}

export interface ContextStats {
  included: number;
  total: number;
}

export interface ActiveCategories {
  metabolicMethylation: boolean;
  histamineCaffeine: boolean;
  pgxDrug: boolean;
  clinicalConfirmation: boolean;
}

export interface CuratedQuestion {
  label: string;
  text: string;
}

// ---------------------------------------------------------------------------
// Default System Instructions
// ---------------------------------------------------------------------------

export const DEFAULT_INSTRUCTIONS = `\
- Role: You are Genomics Caddy AI, a private local assistant helping the user understand raw genomic data.
- Data Scope: Refer ONLY to the genomic variants explicitly provided in the JSON Context below. Do not speculate, generalize, or invent other findings. If a variant is not in the data, state that you do not have data for it.
- Scope Pushback: If the user asks about genes, variants, medical conditions, or drug responses NOT explicitly present in the JSON Context below, you MUST politely push back. Refuse to speculate, generalize, or answer from general knowledge. Explicitly state that you do not have genomic data for that query in their profile.
- Language: Use simple, layperson-friendly language while maintaining accuracy. Translate complex terms (e.g. use "copies of variant gene" instead of "homozygous"). Heavily rely on the 'layperson_summary' fields in the JSON.
- Safety: Always emphasize that this is raw consumer data and requires clinical confirmation. Recommend discussing all findings with a licensed medical professional.
- Format: Keep answers concise, direct, and structured. Use bullet points and headings for clarity. When discussing multiple variants, group them logically by category.`;

// ---------------------------------------------------------------------------
// System Prompt Builder
// ---------------------------------------------------------------------------

interface PromptBuildParams {
  selectedSample: GenomeSample;
  generatedReport: GeneratedReport;
  selectedPacks: Record<string, boolean>;
  onlyActiveFindings: boolean;
  userProfile: UserBiohackingProfile;
  systemInstructions: string;
  manifestPacks: { id: string; label: string }[];
  packsMap: Record<string, { name?: string }>;
  laypersonMap: Record<string, { simpleImpact: string; simpleMeaning: string }>;
}

/**
 * Build a slim variant payload for a single marker to include in context JSON.
 */
function buildMarkerPayload(
  m: EvaluatedMarker,
  laypersonMap: Record<string, { simpleImpact: string; simpleMeaning: string }>
) {
  const layperson = laypersonMap[m.rsid];
  return {
    rsid: m.rsid,
    gene: m.gene,
    variant_name: m.variant_name || undefined,
    genotype: m.user_genotype,
    effect_allele: m.effect_allele,
    effect_count: m.effect_count,
    direction: m.effect_direction,
    severity: m.severity_class,
    layperson_summary: layperson
      ? { simple_impact: layperson.simpleImpact, simple_meaning: layperson.simpleMeaning }
      : undefined,
    impact: m.impact,
    interpretation: m.interpretation,
  };
}

/**
 * Assemble the complete system prompt injected as the first message to the LLM.
 */
export function buildSystemPrompt(params: PromptBuildParams): string {
  const {
    selectedSample, generatedReport, selectedPacks, onlyActiveFindings,
    userProfile, systemInstructions, manifestPacks, packsMap, laypersonMap,
  } = params;

  // --- Genomic JSON Context ---
  const sectionsData = generatedReport.sections
    .map((sec) => {
      const pack = manifestPacks.find(
        (p) => p.label === sec.name || packsMap[p.id]?.name === sec.name
      );
      if (!pack || !selectedPacks[pack.id]) return null;

      const findings = sec.markers
        .filter((m) =>
          onlyActiveFindings
            ? m.effect_count > 0 && m.severity_class !== "no_data"
            : m.severity_class !== "no_data"
        )
        .map((m) => buildMarkerPayload(m, laypersonMap));

      return { section_name: sec.name, findings };
    })
    .filter(
      (s): s is NonNullable<typeof s> => s !== null && s.findings.length > 0
    );

  const payloadContext = {
    sample_name: selectedSample.name,
    genetic_sex: selectedSample.genetic_sex,
    sections: sectionsData,
  };

  // --- User Biohacking Profile ---
  let profileBlock = "";
  if (userProfile.injectProfile) {
    const lines: string[] = [];
    if (userProfile.goals.trim())           lines.push(`- Biohacking Goals: ${userProfile.goals.trim()}`);
    if (userProfile.challenges.trim())      lines.push(`- Current Challenges & Symptoms: ${userProfile.challenges.trim()}`);
    if (userProfile.diet.trim())            lines.push(`- Average Diet: ${userProfile.diet.trim()}`);
    if (userProfile.supplements.trim())     lines.push(`- Supplements List: ${userProfile.supplements.trim()}`);
    if (userProfile.medications.trim())     lines.push(`- Medications List: ${userProfile.medications.trim()}`);
    if (userProfile.bloodwork.trim())       lines.push(`- Blood Work History (DATED): ${userProfile.bloodwork.trim()}`);
    if (userProfile.diagnoses.trim())       lines.push(`- Diagnosis List: ${userProfile.diagnoses.trim()}`);
    if (userProfile.supportiveTests.trim()) lines.push(`- Supportive Tests: ${userProfile.supportiveTests.trim()}`);

    if (lines.length > 0) {
      profileBlock = `\n[USER PROFILE & GOALS MEMORY]
The user has provided the following personal biohacking and wellness context:
${lines.join("\n")}
Refer to this context (especially their goals, diet, blood work, or supplements) when explaining active genomic findings. Tailor your dietary, lifestyle, or supplement suggestions to align with their self-reported objectives and health profile.`;
    }
  }

  // --- Final Assembly ---
  const todayStr = new Date().toDateString();
  const instructions = systemInstructions.trim() || DEFAULT_INSTRUCTIONS;

  return `[SYSTEM INSTRUCTIONS]
- Today's Date: ${todayStr} (Pay close attention to dates on any user blood work, diagnoses, or supportive tests above. If any results are older than 1 year relative to Today's Date, explicitly warn the user that they are historical, and recommend obtaining updated testing to see their current values).
${instructions}
${profileBlock}

[JSON CONTEXT]
${JSON.stringify(payloadContext, null, 2)}`;
}

// ---------------------------------------------------------------------------
// Context Statistics
// ---------------------------------------------------------------------------

/**
 * Count how many found variants are included vs total found for the active packs.
 */
export function calculateContextStats(
  generatedReport: GeneratedReport,
  selectedPacks: Record<string, boolean>,
  onlyActiveFindings: boolean,
  manifestPacks: { id: string; label: string }[],
  packsMap: Record<string, { name?: string }>
): ContextStats {
  let included = 0;
  let total = 0;

  for (const sec of generatedReport.sections) {
    const pack = manifestPacks.find(
      (p) => p.label === sec.name || packsMap[p.id]?.name === sec.name
    );
    if (!pack || !selectedPacks[pack.id]) continue;

    for (const m of sec.markers) {
      const isFound = m.user_genotype !== "--" && !m.user_genotype.includes("-");
      if (isFound) {
        total++;
        if (onlyActiveFindings) {
          if (m.effect_count > 0) included++;
        } else {
          included++;
        }
      }
    }
  }
  return { included, total };
}

// ---------------------------------------------------------------------------
// Active Category Detection
// ---------------------------------------------------------------------------

/**
 * Detect which finding categories have at least one active variant.
 * Used to filter the curated helper prompt buttons dynamically.
 */
export function getActiveCategories(
  generatedReport: GeneratedReport,
  selectedPacks: Record<string, boolean>,
  manifestPacks: { id: string; label: string }[],
  packsMap: Record<string, { name?: string }>
): ActiveCategories {
  const cats: ActiveCategories = {
    metabolicMethylation: false,
    histamineCaffeine: false,
    pgxDrug: false,
    clinicalConfirmation: false,
  };

  for (const sec of generatedReport.sections) {
    const pack = manifestPacks.find(
      (p) => p.label === sec.name || packsMap[p.id]?.name === sec.name
    );
    if (!pack || !selectedPacks[pack.id]) continue;

    for (const m of sec.markers) {
      const isActive =
        m.effect_count > 0 &&
        m.user_genotype !== "--" &&
        !m.user_genotype.includes("-");
      if (!isActive) continue;

      const secName = sec.name;
      if (
        secName === "Metabolic Health & T2D" ||
        secName === "Nutrients & One-Carbon Methylation"
      ) {
        cats.metabolicMethylation = true;
      }
      if (
        m.rsid === "rs762551" ||
        m.gene === "AOC1" ||
        m.gene === "HNMT"
      ) {
        cats.histamineCaffeine = true;
      }
      if (secName === "Pharmacogenomics (PGx)") {
        cats.pgxDrug = true;
      }
      if (
        secName === "Cancer Risks (Confirmation Required)" ||
        m.clinical_confirmation_required
      ) {
        cats.clinicalConfirmation = true;
      }
    }
  }
  return cats;
}

// ---------------------------------------------------------------------------
// Dynamic Curated Questions
// ---------------------------------------------------------------------------

/**
 * Generate curated helper prompt buttons based on which finding categories
 * are active in the user's genomic profile.
 */
export function getDynamicQuestions(cats: ActiveCategories): CuratedQuestion[] {
  const list: CuratedQuestion[] = [];

  if (cats.metabolicMethylation) {
    list.push({
      label: "🍎 Metabolic & Methylation Summary",
      text: "Explain my active metabolic and nutrient methylation findings in simple, clear terms.",
    });
  }
  if (cats.histamineCaffeine) {
    list.push({
      label: "☕ Histamine & Caffeine Lifestyle",
      text: "What lifestyle or dietary topics should I discuss with my doctor based on my histamine and caffeine markers?",
    });
  }
  if (cats.pgxDrug) {
    list.push({
      label: "💊 PGx Drug Variations Guide",
      text: "Help me draft a simple summary of my active pharmacogenomic (PGx) variations to share with my doctor or pharmacist.",
    });
  }
  if (cats.clinicalConfirmation) {
    list.push({
      label: "⚠️ Clinical vs Standard Traits",
      text: "Explain the difference between variants requiring clinical confirmation (like high-stakes cancer markers) and standard traits.",
    });
  }
  if (list.length === 0) {
    list.push({
      label: "🧬 Genomic Overview",
      text: "Give me a high-level summary of the active marker findings in my profile and what they mean.",
    });
  }
  return list;
}

// ---------------------------------------------------------------------------
// Model Capability Detection
// ---------------------------------------------------------------------------

/** Known reasoning model name patterns. */
const REASONING_PATTERNS = [
  "deepseek", "r1", "think", "reasoning",
  "qwen3", "qwen2.5", "gemma3", "gemma4", "medgemma",
];

/** Known vision-capable model name patterns. */
const VISION_NAME_PATTERNS = [
  "llava", "vision", "minicpm", "moondream", "-vl",
  "gemma3", "gemma4", "medgemma",
  "qwen2.5", "qwen3.5", "qwen3.6",
  "ministral-3",
];

/**
 * Determine if a model name indicates a reasoning / extended thinking model.
 */
export function isReasoningModel(modelName: string): boolean {
  if (!modelName) return false;
  const lower = modelName.toLowerCase();
  return REASONING_PATTERNS.some((p) => lower.includes(p));
}

/**
 * Determine if a model is vision-capable from its name and metadata.
 *
 * Checks three signal sources:
 * 1. Model info keys containing ".vision.", ".mm.", or ".audio."
 * 2. Model families array containing "clip"
 * 3. Model name matching known vision model patterns
 */
export function isModelVisionCapable(
  modelName: string,
  details: {
    details?: { families?: string[]; family?: string };
    model_info?: Record<string, unknown>;
  } | null
): boolean {
  const nameLower = modelName.toLowerCase();

  // If we have actual model details from Ollama, trust them!
  if (details) {
    let hasVisionIndicator = false;
    
    // Check model_info keys for vision indicators (e.g. clip, mllama, vision, vl, projector)
    if (details.model_info) {
      for (const key of Object.keys(details.model_info)) {
        const keyLower = key.toLowerCase();
        if (
          keyLower.includes(".vision.") || 
          keyLower.includes(".mm.") || 
          keyLower.includes(".audio.") ||
          keyLower.includes("vision_projector") ||
          keyLower.includes("clip") ||
          keyLower.includes("mllama")
        ) {
          hasVisionIndicator = true;
          break;
        }
      }
    }
    
    // Check families metadata
    const families = details.details?.families || [];
    const family = details.details?.family || "";
    
    const isVisionFamily = (fam: string) => {
      const fLower = fam.toLowerCase();
      return (
        fLower.includes("clip") || 
        fLower.includes("mllama") || 
        fLower.includes("vision") ||
        fLower.includes("-vl") ||
        fLower.includes("vl")
      );
    };

    if (isVisionFamily(family) || families.some(isVisionFamily)) {
      hasVisionIndicator = true;
    }
    
    return hasVisionIndicator;
  }

  // Fallback to name pattern heuristics ONLY if details are unavailable
  return VISION_NAME_PATTERNS.some((p) => nameLower.includes(p));
}

/**
 * Extract the context window size from model metadata, defaulting to 4096.
 */
export function getContextWindow(
  modelInfo: Record<string, unknown> | undefined
): number {
  if (!modelInfo) return 4096;
  for (const key of Object.keys(modelInfo)) {
    if (key.endsWith(".context_length")) {
      const val = Number(modelInfo[key]);
      if (val > 0) return val;
    }
  }
  return 4096;
}

/**
 * Filter out non-chat models (embedding, reranker) from the model list.
 */
export function filterChatModels(modelNames: string[]): string[] {
  return modelNames.filter((name) => {
    const lower = name.toLowerCase();
    return (
      !lower.includes("embed") &&
      !lower.includes("embedding") &&
      !lower.includes("reranker") &&
      !lower.includes("bge-") &&
      !lower.includes("colbert")
    );
  });
}

/**
 * Auto-select the best available model from a list of chat models.
 * Prefers Gemma 4, then larger Gemma/Qwen variants, then falls back to first.
 */
export function autoSelectModel(
  models: string[],
  currentSelection: string
): string {
  if (currentSelection && models.includes(currentSelection)) {
    return currentSelection;
  }
  const preferred = [
    "gemma4:e4b", "gemma4:26b", "gemma4:31b",
    "qwen3.5:27b", "qwen3:32b",
    "llama3.1:latest",
  ];
  for (const p of preferred) {
    if (models.includes(p)) return p;
  }
  return models.length > 0 ? models[0] : "";
}
