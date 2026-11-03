// ./src/lib/utils/consultationContextPack.ts
/*
Purpose: Unified ask-scoped consultation context packer for Ollama and ChatGPT.
Context modes define the candidate pool; ranking + hard budgets choose what is
sent. Message layout is cache-stable (static policy → session → turn → history → ask).
Usage: buildConsultationPack(...) then toProviderMessages(pack).
*/

import type { ChatMessage } from "../types/agent";
import type { GeneratedReport, GenomeSample } from "../types/genomics";
import type { QdrantHit } from "../types/research";
import aiPromptPolicy from "../marker-packs/ai_prompt_policy.json";
import {
  buildCandidatePool,
  buildMarkerPayload,
  CONSULTATION_MODES,
  DEFAULT_INSTRUCTIONS,
  type AiContextMode,
  type CandidateFinding,
  type ConsultationMode,
  type MarkerPayload,
  type UserBiohackingProfile,
} from "./aiPrompt";
import type { ChatProvider } from "./chatProvider";
import type { PersonalSafetyContext } from "./personalSafetyContext";
import type { LaypersonTranslation } from "./layperson";
import { buildVectorResearchBlock, type VectorSearchMeta } from "./qdrantRag";
import { buildSupportResourceContext } from "./supportResourceContext";
import {
  buildCatalogAssociationSummaries,
  buildConditionCoverageSummaries,
  buildConditionEvidenceSummaries,
  getConditionCoverageGaps,
} from "./conditionEvidence";
import { markerPacksStore } from "./markerPacksState.svelte";

/** Rough token estimate; chars/4 is good enough until a real tokenizer is wired. */
export function estimateTokens(text: string): number {
  return Math.ceil(Math.max(0, text.length) / 4);
}

export interface ConsultationBudget {
  /** Soft target for turn JSON (findings + vector + support summaries). */
  turnContextChars: number;
  /** Max characters of prior chat history (excluding current ask). */
  historyChars: number;
  /** Max prior turns after budget trim. */
  historyMaxTurns: number;
  /** Instructions must stay tiny (especially ChatGPT Responses API). */
  instructionsMaxChars: number;
}

export const CONSULTATION_BUDGETS: Record<ChatProvider, ConsultationBudget> = {
  chatgpt: {
    turnContextChars: 160_000,
    historyChars: 24_000,
    historyMaxTurns: 12,
    instructionsMaxChars: 16_000,
  },
  ollama: {
    turnContextChars: 400_000,
    historyChars: 48_000,
    historyMaxTurns: 24,
    instructionsMaxChars: 32_000,
  },
};

export type ContextDropReason = "mode_filter" | "low_rank" | "budget" | "history_trim" | "mode_remap";

export interface ContextBudgetReport {
  provider: ChatProvider;
  contextMode: AiContextMode;
  effectiveMode: AiContextMode;
  poolSize: number;
  sentFindings: number;
  droppedFindings: number;
  vectorHits: number;
  historyTurnsKept: number;
  historyTurnsDropped: number;
  estimatedTokens: {
    instructions: number;
    session: number;
    turn: number;
    history: number;
    ask: number;
    total: number;
  };
  charCounts: {
    instructions: number;
    session: number;
    turn: number;
    history: number;
    ask: number;
  };
  notices: string[];
  dropCounts: Partial<Record<ContextDropReason, number>>;
}

export interface RankedFinding extends CandidateFinding {
  score: number;
  rankReasons: string[];
}

export interface ConsultationPack {
  instructions: string;
  sessionBlock: string;
  turnBlock: string;
  history: ChatMessage[];
  ask: ChatMessage;
  report: ContextBudgetReport;
  /** Inspector-friendly reconstruction of what would be sent. */
  inspectorText: string;
}

export interface BuildConsultationPackInput {
  askText: string;
  askImages?: string[];
  selectedSample: GenomeSample;
  generatedReport: GeneratedReport;
  selectedPacks: Record<string, boolean>;
  contextMode: AiContextMode;
  consultationMode: ConsultationMode;
  userProfile: UserBiohackingProfile;
  reproductiveContext?: string;
  personalSafetyContext?: PersonalSafetyContext;
  systemInstructions: string;
  laypersonMap: Record<string, LaypersonTranslation>;
  qdrantHits?: QdrantHit[];
  vectorSearchMeta?: VectorSearchMeta;
  /** Prior messages excluding the current user ask. */
  history: ChatMessage[];
  provider: ChatProvider;
  /** Optional Ollama context window to scale the ollama turn budget. */
  contextWindow?: number;
  pinnedLinkIds?: string[];
}

function tokenizeAsk(ask: string): string[] {
  return ask
    .toLowerCase()
    .split(/[^a-z0-9_]+/i)
    .map((t) => t.trim())
    .filter((t) => t.length >= 3);
}

function payloadSearchText(payload: MarkerPayload, sectionName: string): string {
  const parts = [
    sectionName,
    payload.gene,
    payload.rsid,
    payload.variant_name,
    payload.impact,
    payload.interpretation,
    payload.layperson_summary?.simple_impact,
    payload.layperson_summary?.simple_meaning,
    payload.layperson_summary?.why_it_matters,
    ...(payload.context_tags ?? []),
  ];
  return parts.filter(Boolean).join(" ").toLowerCase();
}

/** Score pool findings for this ask. Deterministic ties by rsid then gene. */
export function rankCandidates(
  pool: CandidateFinding[],
  ask: string,
  vectorHits: QdrantHit[] = [],
  pinnedLinkIds: string[] = [],
): RankedFinding[] {
  const terms = tokenizeAsk(ask);
  const hitGenes = new Set(
    vectorHits
      .flatMap((h) => [
        h.gene,
        ...(h.gene_candidates ?? []).map((c) => c.symbol ?? c.gene ?? c.name),
      ])
      .filter(Boolean)
      .map((g) => String(g).toUpperCase()),
  );
  const hitRsids = new Set(
    vectorHits.map((h) => String(h.rsid || "").toUpperCase()).filter(Boolean),
  );
  const pinned = new Set(pinnedLinkIds);

  return pool
    .map((row) => {
      const reasons: string[] = [];
      let score = 0;
      const text = payloadSearchText(row.payload, row.section_name);
      const gene = String(row.payload.gene || "").toUpperCase();
      const rsid = String(row.payload.rsid || "").toUpperCase();
      const linkId = String(row.payload.link_id || "");

      if (pinned.has(linkId)) {
        score += 1000;
        reasons.push("pinned");
      }
      for (const term of terms) {
        if (text.includes(term)) {
          score += 4;
          reasons.push(`term:${term}`);
        }
      }
      if (rsid && hitRsids.has(rsid)) {
        score += 40;
        reasons.push("vector_rsid");
      }
      const geneTokens = gene.split(/[\s/]+/).filter(Boolean);
      if (geneTokens.some((g) => hitGenes.has(g))) {
        score += 25;
        reasons.push("vector_gene");
      }
      const effect = row.payload.effect_count ?? 0;
      if (effect > 0) {
        score += Math.min(10, effect * 2);
        reasons.push("effect");
      }
      if (row.payload.claim_boundaries?.clinical_confirmation_required) {
        score += 8;
        reasons.push("clinical");
      }
      // Stable baseline so low-overlap findings still have relative order.
      score += 1;
      return { ...row, score, rankReasons: reasons };
    })
    .sort((a, b) => {
      if (b.score !== a.score) return b.score - a.score;
      const r = String(a.payload.rsid || "").localeCompare(String(b.payload.rsid || ""));
      if (r !== 0) return r;
      return String(a.payload.gene || "").localeCompare(String(b.payload.gene || ""));
    });
}

function groupFindings(findings: RankedFinding[]): Array<{
  pack_id?: string;
  section_name: string;
  findings: MarkerPayload[];
}> {
  const bySection = new Map<string, { pack_id?: string; section_name: string; findings: MarkerPayload[] }>();
  for (const row of findings) {
    const key = String(row.pack_id ?? row.section_name);
    const existing = bySection.get(key);
    if (existing) existing.findings.push(row.payload);
    else {
      bySection.set(key, {
        pack_id: row.pack_id,
        section_name: row.section_name,
        findings: [row.payload],
      });
    }
  }
  return [...bySection.values()];
}

function resolveBudget(provider: ChatProvider, contextWindow?: number): ConsultationBudget {
  const base = { ...CONSULTATION_BUDGETS[provider] };
  if (provider === "ollama" && contextWindow && contextWindow > 0) {
    // Leave room for output (~1/4) and instructions/history; use ~1/2 of window as turn chars.
    const approxChars = Math.floor(contextWindow * 4 * 0.45);
    base.turnContextChars = Math.max(80_000, Math.min(base.turnContextChars, approxChars));
  }
  return base;
}

function buildStaticInstructions(
  systemInstructions: string,
  consultationMode: ConsultationMode,
  todayStr: string,
): string {
  const instructions = systemInstructions.trim() || DEFAULT_INSTRUCTIONS;
  const modeInfo = CONSULTATION_MODES[consultationMode] || CONSULTATION_MODES.general;
  return `[SYSTEM INSTRUCTIONS]
- Today's Date: ${todayStr} (${aiPromptPolicy.date_warning})
${instructions}

[SPECIALTY CONSULTATION MODE: ${modeInfo.label}]
${modeInfo.instructions}

[CONDITION EVIDENCE]
${aiPromptPolicy.condition_evidence_instructions}

[RULES]
${aiPromptPolicy.payload_rules.join("\n")}

[FORBIDDEN ACTIONS]
${aiPromptPolicy.forbidden_actions.join("\n")}`;
}

function trimHistory(
  history: ChatMessage[],
  budget: ConsultationBudget,
): { kept: ChatMessage[]; dropped: number } {
  const usable = history.filter((m) => m.role === "user" || m.role === "assistant");
  const kept: ChatMessage[] = [];
  let chars = 0;
  for (let i = usable.length - 1; i >= 0; i -= 1) {
    const msg = usable[i];
    const content = msg.fullContent || msg.content || "";
    if (kept.length >= budget.historyMaxTurns) break;
    if (chars + content.length > budget.historyChars && kept.length > 0) break;
    kept.unshift({
      role: msg.role,
      content,
      images: msg.images,
    });
    chars += content.length;
  }
  return { kept, dropped: Math.max(0, usable.length - kept.length) };
}

function supportSummary(
  packIds: string[],
  consultationMode: ConsultationMode,
  reproductiveContext: string | undefined,
  personalSafetyContext: PersonalSafetyContext | undefined,
  userProfile: UserBiohackingProfile,
): unknown {
  const full = buildSupportResourceContext({
    packIds,
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
        ].join(" ")
      : undefined,
  });
  // Compact: keep operational keys, drop long prose blobs when present.
  if (!full || typeof full !== "object") return full;
  const copy = { ...(full as unknown as Record<string, unknown>) };
  return copy;
}

function fitFindingsToBudget(
  ranked: RankedFinding[],
  buildTurnJson: (findings: RankedFinding[], includeHeavy: boolean) => string,
  budgetChars: number,
): { selected: RankedFinding[]; includeHeavy: boolean; droppedForBudget: number } {
  // Try heavy support first with progressive finding counts.
  const attempts: Array<{ includeHeavy: boolean; limit: number | null }> = [
    { includeHeavy: true, limit: null },
    { includeHeavy: false, limit: null },
    { includeHeavy: false, limit: 250 },
    { includeHeavy: false, limit: 100 },
    { includeHeavy: false, limit: 40 },
    { includeHeavy: false, limit: 15 },
  ];

  for (const attempt of attempts) {
    const slice =
      attempt.limit == null ? ranked : ranked.slice(0, Math.min(attempt.limit, ranked.length));
    const json = buildTurnJson(slice, attempt.includeHeavy);
    if (json.length <= budgetChars) {
      return {
        selected: slice,
        includeHeavy: attempt.includeHeavy,
        droppedForBudget: Math.max(0, ranked.length - slice.length),
      };
    }
  }

  // Last resort: binary search finding count with compact payload.
  let lo = 0;
  let hi = Math.min(ranked.length, 40);
  let best: RankedFinding[] = [];
  while (lo <= hi) {
    const mid = Math.floor((lo + hi) / 2);
    const slice = ranked.slice(0, mid);
    const json = buildTurnJson(slice, false);
    if (json.length <= budgetChars) {
      best = slice;
      lo = mid + 1;
    } else {
      hi = mid - 1;
    }
  }
  return {
    selected: best,
    includeHeavy: false,
    droppedForBudget: Math.max(0, ranked.length - best.length),
  };
}

/**
 * Build a provider-ready consultation pack: small instructions, session + turn
 * context blocks, trimmed history, and a budget report.
 */
export function buildConsultationPack(input: BuildConsultationPackInput): ConsultationPack {
  const provider = input.provider;
  const budget = resolveBudget(provider, input.contextWindow);
  const notices: string[] = [];
  const dropCounts: Partial<Record<ContextDropReason, number>> = {};
  const todayStr = new Date().toDateString();

  let effectiveMode = input.contextMode;
  if (input.contextMode === "developer_raw_json" && provider === "chatgpt") {
    effectiveMode = "active_findings";
    notices.push(
      "Developer raw JSON is not sent to ChatGPT; using active findings from the candidate pool.",
    );
    dropCounts.mode_remap = (dropCounts.mode_remap ?? 0) + 1;
  }

  const pool = buildCandidatePool(
    input.generatedReport,
    input.selectedPacks,
    effectiveMode === "developer_raw_json" ? "full_selected" : effectiveMode,
    input.laypersonMap,
  );

  // Callable markers in the same pack scope that the mode filter excluded.
  const packScopedCallable = buildCandidatePool(
    input.generatedReport,
    input.selectedPacks,
    effectiveMode === "active_findings" || effectiveMode === "developer_raw_json"
      ? "full_selected"
      : "full_selected",
    input.laypersonMap,
  ).filter((row) => {
    if (effectiveMode === "active_findings" || effectiveMode === "developer_raw_json") return true;
    return input.selectedPacks[row.pack_id ?? ""] === true || !row.pack_id;
  });
  const modeFiltered = Math.max(0, packScopedCallable.length - pool.length);
  if (modeFiltered > 0) dropCounts.mode_filter = modeFiltered;

  const ranked = rankCandidates(
    pool,
    input.askText,
    input.qdrantHits ?? [],
    input.pinnedLinkIds ?? [],
  );

  const sharedReproductive = input.userProfile.injectProfile ? input.reproductiveContext : undefined;
  const sharedSafety = input.userProfile.injectProfile ? input.personalSafetyContext : undefined;

  const buildTurnJson = (findings: RankedFinding[], includeHeavy: boolean): string => {
    const packIds = [
      ...new Set(findings.map((f) => f.pack_id).filter((id): id is string => typeof id === "string")),
    ];
    const turn = {
      sample_context: {
        sections: groupFindings(findings),
        raw_genotypes_included: true,
        raw_genotype_policy: aiPromptPolicy.raw_genotype_policy,
        condition_evidence: includeHeavy
          ? buildConditionEvidenceSummaries(input.generatedReport)
          : undefined,
        catalog_associations: includeHeavy
          ? buildCatalogAssociationSummaries(input.generatedReport)
          : undefined,
        condition_coverage: includeHeavy
          ? buildConditionCoverageSummaries(input.generatedReport)
          : undefined,
        condition_coverage_gaps: includeHeavy ? getConditionCoverageGaps() : undefined,
      },
      support_resources: includeHeavy
        ? supportSummary(
            packIds.length
              ? packIds
              : markerPacksStore.manifest.packs.map((p) => p.id),
            input.consultationMode,
            sharedReproductive,
            sharedSafety,
            input.userProfile,
          )
        : undefined,
      vector_research: buildVectorResearchBlock(
        input.qdrantHits ?? [],
        input.vectorSearchMeta ?? { query: input.askText, enabled: false },
      ),
    };
    return JSON.stringify(turn);
  };

  // Developer raw JSON on Ollama: try full raw_report if it fits; else remap.
  let turnJson = "";
  let selected: RankedFinding[] = [];
  let includeHeavy = false;
  let droppedForBudget = 0;

  if (effectiveMode === "developer_raw_json" && provider === "ollama") {
    const rawTurn = JSON.stringify({
      sample_context: {
        raw_report: input.generatedReport,
        raw_genotypes_included: true,
        raw_genotype_policy: aiPromptPolicy.raw_genotype_policy,
      },
      vector_research: buildVectorResearchBlock(
        input.qdrantHits ?? [],
        input.vectorSearchMeta ?? { query: input.askText, enabled: false },
      ),
    });
    if (rawTurn.length <= budget.turnContextChars) {
      turnJson = rawTurn;
      notices.push("Developer raw JSON included for local Ollama (fits budget).");
    } else {
      effectiveMode = "active_findings";
      notices.push(
        "Developer raw JSON exceeded the local budget; remapped to ranked active findings.",
      );
      dropCounts.mode_remap = (dropCounts.mode_remap ?? 0) + 1;
      const remappedPool = buildCandidatePool(
        input.generatedReport,
        input.selectedPacks,
        "active_findings",
        input.laypersonMap,
      );
      const remappedRanked = rankCandidates(
        remappedPool,
        input.askText,
        input.qdrantHits ?? [],
        input.pinnedLinkIds ?? [],
      );
      const fit = fitFindingsToBudget(remappedRanked, buildTurnJson, budget.turnContextChars);
      selected = fit.selected;
      includeHeavy = fit.includeHeavy;
      droppedForBudget = fit.droppedForBudget;
      turnJson = buildTurnJson(selected, includeHeavy);
    }
  } else {
    const fit = fitFindingsToBudget(ranked, buildTurnJson, budget.turnContextChars);
    selected = fit.selected;
    includeHeavy = fit.includeHeavy;
    droppedForBudget = fit.droppedForBudget;
    turnJson = buildTurnJson(selected, includeHeavy);
  }

  if (droppedForBudget > 0) {
    dropCounts.budget = (dropCounts.budget ?? 0) + droppedForBudget;
    notices.push(`Omitted ${droppedForBudget} lower-ranked pool finding(s) to fit the context budget.`);
  }
  const lowRank = Math.max(0, ranked.length - selected.length - droppedForBudget);
  // When we sliced for budget, those are budget drops; remaining unselected after a full fit attempt with includeHeavy false are also budget.
  if (!includeHeavy && selected.length < ranked.length && droppedForBudget === 0) {
    // handled above
  }
  void lowRank;
  if (ranked.length > selected.length) {
    dropCounts.low_rank = Math.max(
      0,
      ranked.length - selected.length - (dropCounts.budget ?? 0),
    );
  }

  let instructions = buildStaticInstructions(
    input.systemInstructions,
    input.consultationMode,
    todayStr,
  );
  if (instructions.length > budget.instructionsMaxChars) {
    instructions = instructions.slice(0, budget.instructionsMaxChars - 80) + "\n…[instructions truncated]";
    notices.push("Static instructions were truncated to the provider instructions budget.");
  }

  const sessionObject = {
    app_context: {
      name: "Genomics Caddy",
      date: todayStr,
      provider,
      context_mode: effectiveMode,
      consultation_mode: input.consultationMode,
    },
    sample_context: {
      sample_name: input.selectedSample.name,
      chromosome_call_context: input.selectedSample.genetic_sex,
      import_provenance: input.generatedReport.import_provenance ?? null,
      profile: input.userProfile.injectProfile
        ? {
            goals: input.userProfile.goals.trim() || undefined,
            challenges: input.userProfile.challenges.trim() || undefined,
            relevantBodySystems: input.userProfile.relevantBodySystems.trim() || undefined,
            reproductiveHormoneContext:
              input.userProfile.reproductiveHormoneContext.trim() || undefined,
            diet: input.userProfile.diet.trim() || undefined,
            supplements: input.userProfile.supplements.trim() || undefined,
            medications: input.userProfile.medications.trim() || undefined,
            bloodwork: input.userProfile.bloodwork.trim() || undefined,
            diagnoses: input.userProfile.diagnoses.trim() || undefined,
            supportiveTests: input.userProfile.supportiveTests.trim() || undefined,
          }
        : undefined,
      personal_safety_context: sharedSafety,
    },
  };
  const sessionBlock = `[SESSION CONTEXT]\n${JSON.stringify(sessionObject)}`;
  const turnBlock = `[TURN CONTEXT]\n${turnJson}`;

  const { kept: historyKept, dropped: historyDropped } = trimHistory(input.history, budget);
  if (historyDropped > 0) {
    dropCounts.history_trim = historyDropped;
    notices.push(`Trimmed ${historyDropped} earlier chat turn(s) to fit the history budget.`);
  }

  const ask: ChatMessage = {
    role: "user",
    content: input.askText,
    images: input.askImages,
  };

  const charCounts = {
    instructions: instructions.length,
    session: sessionBlock.length,
    turn: turnBlock.length,
    history: historyKept.reduce((n, m) => n + (m.content?.length ?? 0), 0),
    ask: ask.content.length,
  };
  const estimatedTokens = {
    instructions: estimateTokens(instructions),
    session: estimateTokens(sessionBlock),
    turn: estimateTokens(turnBlock),
    history: estimateTokens(historyKept.map((m) => m.content).join("\n")),
    ask: estimateTokens(ask.content),
    total: 0,
  };
  estimatedTokens.total =
    estimatedTokens.instructions +
    estimatedTokens.session +
    estimatedTokens.turn +
    estimatedTokens.history +
    estimatedTokens.ask;

  const report: ContextBudgetReport = {
    provider,
    contextMode: input.contextMode,
    effectiveMode,
    poolSize: pool.length,
    sentFindings: selected.length,
    droppedFindings: Math.max(0, pool.length - selected.length),
    vectorHits: input.qdrantHits?.length ?? 0,
    historyTurnsKept: historyKept.length,
    historyTurnsDropped: historyDropped,
    estimatedTokens,
    charCounts,
    notices,
    dropCounts,
  };

  const inspectorText = [
    instructions,
    "",
    sessionBlock,
    "",
    turnBlock,
    "",
    `[BUDGET REPORT]\n${JSON.stringify(report, null, 2)}`,
    "",
    ...historyKept.map((m) => `${m.role.toUpperCase()}: ${m.content}`),
    "",
    `USER: ${ask.content}`,
  ].join("\n");

  return {
    instructions,
    sessionBlock,
    turnBlock,
    history: historyKept,
    ask,
    report,
    inspectorText,
  };
}

/** Convert a pack into provider chat messages (cache-stable order). */
export function toProviderMessages(pack: ConsultationPack): ChatMessage[] {
  return [
    { role: "system", content: pack.instructions },
    {
      role: "user",
      content: `Reference session context for this consultation (not a user question):\n\n${pack.sessionBlock}`,
    },
    {
      role: "user",
      content: `Reference turn context for this consultation (not a user question):\n\n${pack.turnBlock}`,
    },
    ...pack.history,
    pack.ask,
  ];
}

/** One-line chrome notice from a budget report. */
export function formatBudgetNotice(report: ContextBudgetReport): string {
  const parts = [
    `Sending ${report.sentFindings} of ${report.poolSize} pool findings`,
    `~${report.estimatedTokens.total.toLocaleString()} tokens`,
    `${report.vectorHits} vector hit${report.vectorHits === 1 ? "" : "s"}`,
    `${report.historyTurnsKept} history turn${report.historyTurnsKept === 1 ? "" : "s"}`,
  ];
  if (report.notices[0]) parts.push(report.notices[0]);
  return parts.join(" · ");
}

// Re-export for tests that build payloads without going through the pool helper.
export { buildMarkerPayload };
