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
import { buildVectorResearchBlock, type VectorSearchMeta } from "./qdrantRag";
import { markerPacksStore } from "./markerPacksState.svelte";
import { buildSupportResourceContext } from "./supportResourceContext";

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
- Data Scope: The JSON CONTEXT has two complementary layers:
  1. sample_context.sections — curated trait report markers from marker packs (user genotype, effect direction, layperson summaries).
  2. vector_research — semantically retrieved enriched variants from the local Qdrant index (GWAS associations, ClinVar, gnomAD, gene mapping). When vector_research.status is "ok", you HAVE this data — use it and cite rsIDs. Do NOT ask the user for database access or claim you cannot parse vector data.
- Merging sources: When the same rsID appears in both layers, combine them — trait report for personal genotype/impact; vector_research for literature traits, gene provenance, and population frequency.
- No matches: If vector_research.status is "no_matches", answer from sample_context only and note that no indexed vector research matched the query.
- Scope Pushback: If the user asks about genes, variants, or conditions absent from BOTH sample_context AND vector_research, politely push back. Refuse to speculate from general knowledge.
- Menstrual/hormone safety: Do not describe the late-luteal or menstrual transition as an estrogen spike. In a natural ovulatory cycle, estradiol peaks before ovulation and estradiol/progesterone generally fall before bleeding; hormonal contraception can change the pattern. Use the exact medication name and active ingredients when available.
- Reproductive workup boundary: PMDD/PMS requires clinical symptom timing, and adenomyosis requires gynecologic evaluation and often imaging. DNA can provide limited research context but cannot diagnose either condition, measure current hormones, or justify medication changes.
- Sex and anatomy context: The genetic-sex label is a chromosome-call hint, not gender identity, anatomy, fertility, pregnancy status, or hormone status. Do not infer menstruation, pregnancy potential, prostate/testicular anatomy, or hormone therapy from it. Ask which body systems and life context are relevant; treat missing/uncertain sex context as unknown, not as XX or XY.
- Probability and actionability: Describe common SNPs as probabilistic modifiers with small or conditional effects. Separate evidence strength, personal genotype matching, phenotype/lab support, and clinical actionability. More markers increase coverage, not certainty. Food, supplement, activity, and medication ideas must be conditional discussion points; never recommend starting/stopping a medication or supplement, a restrictive diet, or a high-intensity activity from raw DNA alone. Include what to avoid or verify when a rule has a safety concern.
- Language: Use simple, layperson-friendly language while maintaining accuracy. Heavily rely on layperson_summary fields when present.
- Safety: Always emphasize that this is raw consumer data and requires clinical confirmation. Recommend discussing all findings with a licensed medical professional.
- Format: Keep answers concise, direct, and structured. Use bullet points and headings. Group variants by category. Cite rsIDs when referencing vector research hits.`;

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

export type ConsultationMode =
  | "general"
  | "pgx"
  | "nutrients"
  | "metabolic"
  | "sleep"
  | "brain_mood"
  | "joints"
  | "thyroid_autoimmune"
  | "cardiovascular"
  | "hormones_reproductive";

export const CONSULTATION_MODES: Record<ConsultationMode, { label: string; icon: string; instructions: string }> = {
  general: {
    label: "General Consultation",
    icon: "🧬",
    instructions: "Focus on providing a broad, balanced overview of the user's genomic profile. Explain basic inheritance concepts, help the user prioritize which packs/findings they might want to discuss first with a healthcare provider, and keep all insights structured."
  },
  pgx: {
    label: "Pharmacogenomics (PGx)",
    icon: "💊",
    instructions: "Focus strictly on drug metabolism, transport, and safety/efficacy markers (e.g., CYP450, DPYD). You MUST explain that drug responses are highly variable. Highlight any markers requiring clinical confirmation (such as DPYD rs55886062) and remind the user to never alter medications or dosages without consulting a licensed physician or pharmacist."
  },
  nutrients: {
    label: "Nutrients & Methylation",
    icon: "🍎",
    instructions: "Focus on MTHFR, COMT, PEMT, and other nutrient/methylation markers. Explain the one-carbon cycle, how these genes influence nutrient requirements (e.g. folate, B12, choline), and suggest dietary foods (no supplement dosages) that support these pathways. Emphasize discussing supplement changes with a doctor."
  },
  metabolic: {
    label: "Metabolic Health & T2D",
    icon: "🏃",
    instructions: "Focus on blood sugar regulation, insulin sensitivity, lipid transport (e.g. APOE, FTO, TCF7L2), and type 2 diabetes markers. Discuss how these genetic predispositions interact with diet, exercise, circadian rhythms, and stress. Do not diagnose metabolic syndrome."
  },
  sleep: {
    label: "Sleep & Circadian Rhythms",
    icon: "🌙",
    instructions: "Focus on circadian rhythm genes (CLOCK, PER2, MTNR1B) and sleep duration/quality markers. Explain how light exposure, sleep hygiene, and sleep timing can be optimized based on genetic predispositions. Do not diagnose sleep apnea or insomnia."
  },
  brain_mood: {
    label: "Brain & Mood (Neuropsych)",
    icon: "🧠",
    instructions: "Focus on neurotransmitter synthesis, transport, and receptors (e.g., COMT, MAOA, DRD2, 5-HTTLPR). Discuss how these relate to focus, mood, stress response, and caffeine/alcohol sensitivity. Strictly avoid diagnosing clinical depression, ADHD, or anxiety disorders."
  },
  joints: {
    label: "Joints & Connective Tissue",
    icon: "🦴",
    instructions: "Focus on collagen structure, joint mobility, tendon/ligament integrity (e.g. COL1A1, COL5A1), and bone density markers. Discuss training volume, recovery protocols, joint-supporting nutrition, and injury prevention, emphasizing consult with physical therapists or sports medicine doctors."
  },
  thyroid_autoimmune: {
    label: "Thyroid & Autoimmune Context",
    icon: "🛡️",
    instructions: "Focus on thyroid hormone conversion (e.g. DIO1, DIO2), inflammation pathways, and immune sensitivity markers. Discuss environmental triggers, gut health/dietary links, and thyroid cofactors (selenium, iodine) from a high-level perspective. Strictly avoid diagnosing autoimmune diseases."
  },
  cardiovascular: {
    label: "Cardiovascular Health",
    icon: "❤️",
    instructions: "Focus on vascular integrity, blood pressure regulation, lipid profile (e.g. NOS3, ACE, LPA, APOE), and nitric oxide production. Discuss cardiovascular lifestyle habits, aerobic conditioning, salt sensitivity, and dietary fats. Do not diagnose cardiovascular disease or prescribe statins."
  },
  hormones_reproductive: {
    label: "Menstrual Cycle & Hormone Context",
    icon: "🌸",
    instructions: "Focus on menstrual-cycle physiology, PMDD/PMS symptom timing, estrogen/progesterone/progestin distinctions, reproductive hormone metabolism, and adenomyosis/endometriosis workup boundaries. Correct the assumption that estrogen normally spikes at the end of the luteal phase; in a natural ovulatory cycle estradiol and progesterone generally fall before bleeding. Treat cyclic irritability or avoidance as a symptom-timing lead, not a diagnosis. Require the exact contraceptive product and active ingredients before discussing medication context, never recommend medication changes from raw DNA, and direct suspected adenomyosis toward gynecologic evaluation and appropriate imaging."
  }
};

interface PromptBuildParams {
  selectedSample: GenomeSample;
  generatedReport: GeneratedReport;
  selectedPacks: Record<string, boolean>;
  onlyActiveFindings: boolean;
  contextMode: AiContextMode;
  consultationMode: ConsultationMode;
  userProfile: UserBiohackingProfile;
  systemInstructions: string;
  laypersonMap: Record<string, { simpleImpact: string; simpleMeaning: string }>;
  qdrantHits?: QdrantHit[];
  vectorSearchMeta?: VectorSearchMeta;
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
    evidence_tier: m.evidence_tier,
    sex_scope: m.sex_scope || undefined,
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
    contextMode = "active_findings",
    consultationMode = "general",
    userProfile, systemInstructions, laypersonMap,
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
        genetic_sex: selectedSample.genetic_sex,
        profile: userProfile.injectProfile ? userProfile : undefined,
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
        genetic_sex: selectedSample.genetic_sex,
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
        sections: sectionsData
      },
      support_resources: supportResources,
      rules: [
        "Consultation is educational only and does not substitute for medical professional consultation.",
        "Use sample_context.sections for curated trait report markers AND vector_research for semantically retrieved enriched evidence when present.",
        "Use support_resources as the operational safety layer: ask the listed phenotype questions, use the listed lab overlays, respect callability limits, and apply food/medication safety priorities before genotype context.",
        "When vector_research.status is ok, cite rsIDs from vector_research.hits — do not claim lack of database access.",
        "Use simple, layperson-friendly language. Heavily rely on layperson_summary fields when present.",
        "If a gene or condition is absent from both sample_context and vector_research, politely push back and refuse to speculate.",
        "Recommend discussing all findings and changes with a licensed clinician."
      ],
      forbidden_actions: [
        "Do not diagnose diseases or conditions.",
        "Do not prescribe treatments or recommend specific drug/supplement dosages.",
        "Do not ask the user how to access Qdrant or vector databases — the data is already in vector_research when status is ok.",
        "Do not speculate from general genetic knowledge when data is absent from both context layers."
      ],
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
- Today's Date: ${todayStr} (Pay close attention to dates on any user blood work, diagnoses, or supportive tests above. If any results are older than 1 year relative to Today's Date, explicitly warn the user that they are historical, and recommend obtaining updated testing to see their current values).
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
  const cats: ActiveCategories = {
    metabolicMethylation: false,
    histamineCaffeine: false,
    pgxDrug: false,
    clinicalConfirmation: false,
  };

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
