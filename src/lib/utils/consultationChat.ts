// ./src/lib/utils/consultationChat.ts
/**
 * Ollama consultation streaming: RAG retrieval, primary response, optional safety review.
 */

import { listen } from "@tauri-apps/api/event";
import {
  streamOllamaChat,
  showOllamaModel,
  searchAssociationsHybrid,
} from "../api/tauri";
import type { ChatMessage } from "../types/agent";
import type { QdrantHit, VectorResearchDiagnostics } from "../types/research";
import type { GenomeSample, GeneratedReport } from "../types/genomics";
import { evidenceCardsToQdrantHits } from "./qdrantRag";
import {
  buildSystemPrompt,
  DEFAULT_INSTRUCTIONS,
  getContextWindow,
  isReasoningModel as checkReasoningModel,
  type AiContextMode,
  type ConsultationMode,
  type UserBiohackingProfile,
} from "./aiPrompt";
import { stripThinkingTokens } from "./chatParser";
import { LAYPERSON_MAP } from "./layperson";
import {
  newOllamaStreamId,
  subscribeOllamaStream,
  type OllamaDonePayload,
} from "./ollamaStream";
export interface ConsultationTurnInput {
  text: string;
  selectedSample: GenomeSample;
  generatedReport: GeneratedReport;
  messages: ChatMessage[];
  assistantIndex: number;
  ollamaUrl: string;
  ollamaToken: string;
  selectedModel: string;
  temperature: number;
  maxTokens: number;
  extendedThinking: boolean;
  contextWindow: number;
  useVectorResearch: boolean;
  vectorDiagnostics?: VectorResearchDiagnostics | null;
  twoModelReview: boolean;
  reviewModel: string;
  selectedPacks: Record<string, boolean>;
  onlyActiveFindings: boolean;
  contextMode: AiContextMode;
  consultationMode: ConsultationMode;
  userProfile: UserBiohackingProfile;
  systemInstructions: string;
}

export interface ConsultationTurnCallbacks {
  onMessages: (messages: ChatMessage[]) => void;
  onVectorResult: (hits: QdrantHit[], query: string, error?: string) => void;
  onTokenUsage: (promptTokens: number, responseTokens: number) => void;
  onChattingDone: () => void;
  registerMainListeners: (stop: () => void) => void;
  registerReviewListeners: (stop: () => void) => void;
}

function outputTokenLimit(
  model: string,
  contextWindow: number,
  maxTokens: number,
  extendedThinking: boolean,
): number {
  const defaultLimit = checkReasoningModel(model)
    ? Math.max(4096, Math.min(8192, Math.floor(contextWindow / 4)))
    : Math.max(2048, Math.min(4096, Math.floor(contextWindow / 8)));
  return extendedThinking
    ? Math.max(8192, Math.min(16384, Math.floor(contextWindow / 2)))
    : Math.min(defaultLimit, maxTokens);
}

export async function runConsultationTurn(
  input: ConsultationTurnInput,
  callbacks: ConsultationTurnCallbacks,
): Promise<void> {
  const {
    text,
    selectedSample,
    generatedReport,
    messages,
    assistantIndex,
    ollamaUrl,
    ollamaToken,
    selectedModel,
    temperature,
    maxTokens,
    extendedThinking,
    contextWindow,
    useVectorResearch,
    vectorDiagnostics,
    twoModelReview,
    reviewModel,
    selectedPacks,
    onlyActiveFindings,
    contextMode,
    consultationMode,
    userProfile,
    systemInstructions,
  } = input;

  let qdrantHits: QdrantHit[] = [];
  let vectorSearchError = "";

  if (useVectorResearch) {
    if (vectorDiagnostics?.embedding_model_mismatch) {
      vectorSearchError =
        `Index embedding model (${vectorDiagnostics.index_embedding_model ?? "unknown"}) ` +
        `does not match settings (${vectorDiagnostics.embedding_model}). ` +
        "Run re-embed stale vectors or re-sweep before consultation RAG.";
      callbacks.onVectorResult([], text, vectorSearchError);
    } else {
      try {
        const cards = await searchAssociationsHybrid(
          {
            sample_id: selectedSample.id,
            query: text,
            min_data_quality: 0.25,
            limit: 8,
          },
          ollamaUrl,
        );
        qdrantHits = evidenceCardsToQdrantHits(cards);
        callbacks.onVectorResult(qdrantHits, text);
      } catch (err: unknown) {
        vectorSearchError = err instanceof Error ? err.message : String(err);
        callbacks.onVectorResult([], text, vectorSearchError);
        console.warn("Failed to retrieve hybrid vector context for LLM prompt:", err);
      }
    }
  } else {
    callbacks.onVectorResult([], text);
  }

  const chatHistory: ChatMessage[] = messages.slice(0, -1).map((m) => {
    const msgObj: ChatMessage = {
      role: m.role,
      content: m.fullContent || m.content,
    };
    if (m.images && m.images.length > 0) msgObj.images = m.images;
    return msgObj;
  });

  const systemPromptWithRag = buildSystemPrompt({
    selectedSample,
    generatedReport,
    selectedPacks,
    onlyActiveFindings,
    contextMode,
    consultationMode,
    userProfile,
    systemInstructions: systemInstructions || DEFAULT_INSTRUCTIONS,
    laypersonMap: LAYPERSON_MAP,
    qdrantHits,
    vectorSearchMeta: {
      query: text,
      enabled: useVectorResearch,
      error: vectorSearchError || undefined,
    },
  });

  const payloadMessages: ChatMessage[] = [
    { role: "system", content: systemPromptWithRag },
    ...chatHistory,
  ];
  let currentMessages = [...messages];

  const mainStreamId = newOllamaStreamId();
  let stopMain: (() => void) | null = null;

  const limit = outputTokenLimit(selectedModel, contextWindow, maxTokens, extendedThinking);

  try {
    stopMain = await subscribeOllamaStream(mainStreamId, {
      onChunk: (chunk) => {
        currentMessages[assistantIndex].content += chunk;
        currentMessages = [...currentMessages];
        callbacks.onMessages(currentMessages);
      },
      onDone: (payload: OllamaDonePayload | null) => {
        if (payload?.prompt_eval_count) callbacks.onTokenUsage(payload.prompt_eval_count, 0);
        if (payload?.eval_count) callbacks.onTokenUsage(0, payload.eval_count);
      },
    });
    callbacks.registerMainListeners(() => {
      stopMain?.();
      stopMain = null;
    });

    await streamOllamaChat(
      mainStreamId,
      ollamaUrl,
      ollamaToken || undefined,
      selectedModel,
      payloadMessages,
      temperature,
      limit,
    );
  } finally {
    stopMain?.();
    stopMain = null;
  }

  if (!twoModelReview || !reviewModel) {
    callbacks.onChattingDone();
    return;
  }

  currentMessages[assistantIndex].safetyReview = "Reviewing response safety...";
  callbacks.onMessages([...currentMessages]);

  const reviewPrompt = `You are a medical safety auditor. Review the following genomic consultation draft for any clinical overclaiming, dosing advice, or diagnosing assertions. Output your safety corrections, warnings, or notes to the patient.

Draft Response to Review:
"""
${stripThinkingTokens(currentMessages[assistantIndex].content)}
"""`;

  const reviewStreamId = newOllamaStreamId();
  let stopReview: (() => void) | null = null;

  try {
    const reviewDetails = await showOllamaModel(ollamaUrl, ollamaToken || undefined, reviewModel).catch(() => null);
    const reviewCtx = reviewDetails ? getContextWindow(reviewDetails.model_info) : 4096;
    const reviewLimit = outputTokenLimit(reviewModel, reviewCtx, maxTokens, false);

    stopReview = await subscribeOllamaStream(reviewStreamId, {
      onChunk: (chunk) => {
        if (currentMessages[assistantIndex].safetyReview === "Reviewing response safety...") {
          currentMessages[assistantIndex].safetyReview = "";
        }
        currentMessages[assistantIndex].safetyReview =
          (currentMessages[assistantIndex].safetyReview || "") + chunk;
        currentMessages = [...currentMessages];
        callbacks.onMessages(currentMessages);
      },
      onDone: (payload: OllamaDonePayload | null) => {
        if (payload?.prompt_eval_count) callbacks.onTokenUsage(payload.prompt_eval_count, 0);
        if (payload?.eval_count) callbacks.onTokenUsage(0, payload.eval_count);
      },
    });
    callbacks.registerReviewListeners(() => {
      stopReview?.();
      stopReview = null;
    });

    let reviewTimeoutId: ReturnType<typeof setTimeout> | null = null;
    const reviewPromise = streamOllamaChat(
      reviewStreamId,
      ollamaUrl,
      ollamaToken || undefined,
      reviewModel,
      [{ role: "user", content: reviewPrompt }],
      0.0,
      reviewLimit,
    );
    const timeoutPromise = new Promise<never>((_, reject) => {
      reviewTimeoutId = setTimeout(
        () => reject(new Error("Safety review timed out (server busy or loading review model)")),
        60_000,
      );
    });

    try {
      await Promise.race([reviewPromise, timeoutPromise]);
    } finally {
      if (reviewTimeoutId) clearTimeout(reviewTimeoutId);
    }

    callbacks.onChattingDone();
  } catch (revError: unknown) {
    currentMessages[assistantIndex].safetyReview = `⚠️ Safety Review Failed: ${
      revError instanceof Error ? revError.message : String(revError)
    }`;
    callbacks.onMessages([...currentMessages]);
    callbacks.onChattingDone();
  } finally {
    stopReview?.();
    stopReview = null;
  }
}
