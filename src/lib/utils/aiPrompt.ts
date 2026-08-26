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
import type { QdrantHit } from "../types/research";
import aiPromptHelpers from "../marker-packs/ai_prompt_helpers.json";
import aiPromptPolicy from "../marker-packs/ai_prompt_policy.json";
import consultationModes from "../marker-packs/consultation_modes.json";
import { buildVectorResearchBlock, type VectorSearchMeta } from "./qdrantRag";
import { markerPacksStore } from "./markerPacksState.svelte";
import { buildSupportResourceContext } from "./supportResourceContext";
import { getLaypersonTranslation, type LaypersonTranslation } from "./layperson";
import type { PersonalSafetyContext } from "./personalSafetyContext";

// ---------------------------------------------------------------------------
// Shared Interfaces (re-exported for convenience)
// ---------------------------------------------------------------------------

export interface UserBiohackingProfile {
  goals: string;
  challenges: string;
  relevantBodySystems: string;
  reproductiveHormoneContext: string;
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

export type ActiveCategories = Record<string, boolean>;

export interface CuratedQuestion {
  label: string;
  text: string;
}

// ---------------------------------------------------------------------------
// Default System Instructions
// ---------------------------------------------------------------------------

export const DEFAULT_INSTRUCTIONS = aiPromptPolicy.default_instructions.join("\n");

// ---------------------------------------------------------------------------
// System Prompt Builder
// ---------------------------------------------------------------------------

export type AiContextMode =
  | "active_findings"
  | "active_context_dependent"
  | "selected_pack_active"
  | "full_selected"
  | "clinical_checklist"
  | "evidence_audit"
  | "developer_raw_json";

export type ConsultationMode = typeof consultationModes.modes[number]["id"];

export interface ConsultationModeInfo {
  label: string;
  icon: string;
  instructions: string;
}

/** User-facing mode metadata is resource-authored so labels and safety focus cannot drift from routing. */
export const CONSULTATION_MODES = Object.fromEntries(
  consultationModes.modes.map(({ id, label, icon, instructions }) => [id, { label, icon, instructions }]),
) as Record<ConsultationMode, ConsultationModeInfo>;

type AiPromptHelper = typeof aiPromptHelpers.helpers[number];

const AI_PROMPT_HELPERS = aiPromptHelpers.helpers as AiPromptHelper[];

interface PromptBuildParams {
  selectedSample: GenomeSample;
  generatedReport: GeneratedReport;
  selectedPacks: Record<string, boolean>;
  onlyActiveFindings: boolean;
  contextMode: AiContextMode;
  consultationMode: ConsultationMode;
  userProfile: UserBiohackingProfile;
  reproductiveContext?: string;
  personalSafetyContext?: PersonalSafetyContext;
  systemInstructions: string;
  laypersonMap: Record<string, { simpleImpact: string; simpleMeaning: string }>;
  qdrantHits?: QdrantHit[];
  vectorSearchMeta?: VectorSearchMeta;
}

/**
 * Build a slim variant payload for a single marker to include in context JSON.
 */
export function buildMarkerPayload(
  m: EvaluatedMarker,
  laypersonMap: Record<string, LaypersonTranslation>
) {
  const layperson = getLaypersonTranslation(m, laypersonMap);
  return {
    rsid: m.rsid,
    gene: m.gene,
    variant_name: m.variant_name || undefined,
    genotype: m.user_genotype,
    effect_allele: m.effect_allele,
    effect_count: m.effect_count,
    direction: m.effect_direction,
    evidence_tier: m.evidence_tier,
    sex_scope: m.sex_scope || undefined,
    severity: m.severity_class,
    assertion_status: m.assertion_status,
    interpretation_allowed: m.interpretation_allowed,
    claim_boundaries: {
      do_not_claim: m.do_not_claim,
      confirm_with: m.confirm_with,
      raw_dna_limitation: m.raw_dna_limitation || undefined,
      clinical_confirmation_required: m.clinical_confirmation_required === true,
    },
    source_names: (m.sources || []).map((source) => source.name),
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
    contextMode = "active_findings",
    consultationMode = "general",
    userProfile, systemInstructions, laypersonMap,
    reproductiveContext,
    personalSafetyContext,
    qdrantHits = [],
    vectorSearchMeta,
  } = params;

  const todayStr = new Date().toDateString();

  // --- Genomic JSON Context ---
  let sectionsData: any[] = [];

  if (contextMode !== "developer_raw_json") {
    sectionsData = generatedReport.sections
      .map((sec) => {
        const pack = markerPacksStore.manifest.packs.find(
          (p) => p.label === sec.name || markerPacksStore.packs[p.id]?.name === sec.name
        );
        
        const isPackSelected = pack ? selectedPacks[pack.id] : false;
        
        // Skip pack if it's not selected and we are NOT in global active findings mode
        if (contextMode !== "active_findings" && !isPackSelected) {
          return null;
        }

        const filteredMarkers = sec.markers.filter((m) => {
          const isMissing = m.severity_class === "no_data" || m.user_genotype === "--" || m.user_genotype.includes("-");
          if (isMissing) return false;

          switch (contextMode) {
            case "active_findings":
            case "selected_pack_active":
              return (m.effect_count ?? 0) > 0;
            case "active_context_dependent":
              return (m.effect_count ?? 0) > 0 || m.severity_class === "context_dependent";
            case "full_selected":
              return true;
            case "clinical_checklist":
              return m.clinical_confirmation_required === true || 
                     m.severity_class === "confirmation_required" ||
                     m.severity_class === "high_risk" ||
                     m.severity_class === "moderate_risk";
            case "evidence_audit":
              return (m.effect_count ?? 0) > 0;
            default:
              return true;
          }
        });

        if (filteredMarkers.length === 0) return null;

        const findings = filteredMarkers.map((m) => buildMarkerPayload(m, laypersonMap));
        return { pack_id: pack?.id, section_name: sec.name, findings };
      })
      .filter((s): s is NonNullable<typeof s> => s !== null);
  }

  const supportPackIds = contextMode === "developer_raw_json"
    ? markerPacksStore.manifest.packs.map((pack) => pack.id)
    : sectionsData
      .map((section) => section.pack_id)
      .filter((packId): packId is string => typeof packId === "string");
  const supportResources = buildSupportResourceContext({
    packIds: supportPackIds,
    consultationMode,
    reproductiveContext,
    personalSafetyContext,
    profileContext: userProfile.injectProfile
      ? [
          userProfile.goals,
          userProfile.challenges,
          userProfile.relevantBodySystems,
          userProfile.reproductiveHormoneContext,
          userProfile.bloodwork,
          userProfile.diagnoses,
          userProfile.supportiveTests,
        ].join(' ')
      : undefined,
  });

  // --- Construct Payload JSON ---
  let payloadContext: any = {};
  
  if (contextMode === "developer_raw_json") {
    payloadContext = {
      app_context: {
        name: "Genomics Caddy",
        version: "1.0.0",
        date: todayStr,
        mode: "Developer raw JSON"
      },
      sample_context: {
        sample_name: selectedSample.name,
        chromosome_call_context: selectedSample.genetic_sex,
        profile: userProfile.injectProfile ? userProfile : undefined,
        personal_safety_context: userProfile.injectProfile ? personalSafetyContext : undefined,
        raw_report: generatedReport,
        support_resources: supportResources
      }
    };
  } else {
    const modeLabel = {
      active_findings: "Active findings (All packs)",
      active_context_dependent: "Active + context-dependent",
      selected_pack_active: "Selected pack active",
      full_selected: "Full selected packs",
      clinical_checklist: "Clinical confirmation checklist",
      evidence_audit: "Evidence audit"
    }[contextMode] || contextMode;

    payloadContext = {
      app_context: {
        name: "Genomics Caddy",
        version: "1.0.0",
        date: todayStr,
        mode: modeLabel
      },
      sample_context: {
        sample_name: selectedSample.name,
        chromosome_call_context: selectedSample.genetic_sex,
        profile: userProfile.injectProfile ? {
          goals: userProfile.goals.trim() || undefined,
          challenges: userProfile.challenges.trim() || undefined,
          relevantBodySystems: userProfile.relevantBodySystems.trim() || undefined,
          reproductiveHormoneContext: userProfile.reproductiveHormoneContext.trim() || undefined,
          diet: userProfile.diet.trim() || undefined,
          supplements: userProfile.supplements.trim() || undefined,
          medications: userProfile.medications.trim() || undefined,
          bloodwork: userProfile.bloodwork.trim() || undefined,
          diagnoses: userProfile.diagnoses.trim() || undefined,
          supportiveTests: userProfile.supportiveTests.trim() || undefined
        } : undefined,
        personal_safety_context: userProfile.injectProfile ? personalSafetyContext : undefined,
        sections: sectionsData
      },
      support_resources: supportResources,
      rules: aiPromptPolicy.payload_rules,
      forbidden_actions: aiPromptPolicy.forbidden_actions,
      vector_research: buildVectorResearchBlock(qdrantHits, vectorSearchMeta ?? {
        query: "",
        enabled: false,
      }),
    };
  }

  // --- Final Assembly ---
  const instructions = systemInstructions.trim() || DEFAULT_INSTRUCTIONS;
  const modeInfo = CONSULTATION_MODES[consultationMode] || CONSULTATION_MODES.general;
  const specialtyInstructions = `[SPECIALTY CONSULTATION MODE: ${modeInfo.label}]
${modeInfo.instructions}`;

  return `[SYSTEM INSTRUCTIONS]
- Today's Date: ${todayStr} (${aiPromptPolicy.date_warning})
${instructions}

${specialtyInstructions}

[JSON CONTEXT]
${JSON.stringify(payloadContext, null, 2)}`;
}

// ---------------------------------------------------------------------------
// Context Statistics
// ---------------------------------------------------------------------------

export function calculateContextStats(
  generatedReport: GeneratedReport,
  selectedPacks: Record<string, boolean>,
  contextMode: AiContextMode
): ContextStats {
  let included = 0;
  let total = 0;

  for (const sec of generatedReport.sections) {
    const pack = markerPacksStore.manifest.packs.find(
      (p) => p.label === sec.name || markerPacksStore.packs[p.id]?.name === sec.name
    );
    const isPackSelected = pack ? selectedPacks[pack.id] : false;
    
    // active_findings scans all packs, others only selected packs
    if (contextMode !== "active_findings" && !isPackSelected) continue;

    for (const m of sec.markers) {
      const isFound = m.user_genotype !== "--" && !m.user_genotype.includes("-");
      if (isFound) {
        total++;
        switch (contextMode) {
          case "active_findings":
          case "selected_pack_active":
          case "evidence_audit":
            if ((m.effect_count ?? 0) > 0) included++;
            break;
          case "active_context_dependent":
            if ((m.effect_count ?? 0) > 0 || m.severity_class === "context_dependent") included++;
            break;
          case "full_selected":
            included++;
            break;
          case "clinical_checklist":
            if (m.clinical_confirmation_required === true || 
                m.severity_class === "confirmation_required" ||
                m.severity_class === "high_risk" ||
                m.severity_class === "moderate_risk") {
              included++;
            }
            break;
          case "developer_raw_json":
            included++;
            break;
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
  selectedPacks: Record<string, boolean>
): ActiveCategories {
  const cats: ActiveCategories = Object.fromEntries(
    AI_PROMPT_HELPERS.map((helper) => [helper.id, false]),
  );
  const activeFindings: Array<{ packId: string; marker: EvaluatedMarker }> = [];

  for (const sec of generatedReport.sections) {
    const pack = markerPacksStore.manifest.packs.find(
      (p) => p.label === sec.name || markerPacksStore.packs[p.id]?.name === sec.name
    );
    if (!pack || !selectedPacks[pack.id]) continue;

    for (const m of sec.markers) {
      const isActive =
        (m.effect_count ?? 0) > 0 &&
        m.user_genotype !== "--" &&
        !m.user_genotype.includes("-");
      if (!isActive) continue;
      activeFindings.push({ packId: pack.id, marker: m });
    }
  }

  for (const helper of AI_PROMPT_HELPERS) {
    if (helper.fallback) continue;
    cats[helper.id] = activeFindings.some(({ packId, marker }) => {
      const geneSymbols = new Set(
        String(marker.gene || '')
          .split(/[\s/]+/)
          .map((gene) => gene.trim().toUpperCase())
          .filter(Boolean),
      );
      const packMatch = helper.pack_ids?.includes(packId) ?? false;
      const geneMatch = helper.gene_symbols?.some((gene) =>
        geneSymbols.has(String(gene).trim().toUpperCase())
      ) ?? false;
      const rsidMatch = helper.rsids?.some((rsid) =>
        String(rsid).trim().toLowerCase() === String(marker.rsid || '').trim().toLowerCase()
      ) ?? false;
      const clinicalMatch = helper.requires_clinical_confirmation === true
        && marker.clinical_confirmation_required === true;
      return packMatch || geneMatch || rsidMatch || clinicalMatch;
    });
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
  const list = AI_PROMPT_HELPERS
    .filter((helper) => !helper.fallback && cats[helper.id])
    .map(({ label, text }) => ({ label, text }));
  if (list.length > 0) return list;

  const fallback = AI_PROMPT_HELPERS.find((helper) => helper.fallback);
  return fallback ? [{ label: fallback.label, text: fallback.text }] : [];
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
