// ./src/lib/utils/aiAssistantSendActions.ts
/**
 * Consultation prompt send/stop actions for AiAssistantPanel.
 */

import { cancelOllamaStream } from "../api/tauri";
import type { ChatMessage } from "../types/agent";
import type { GeneratedReport, GenomeSample } from "../types/genomics";
import type { QdrantHit } from "../types/research";
import {
  type AiContextMode,
  type ConsultationMode,
  type UserBiohackingProfile,
} from "./aiPrompt";
import { runConsultationTurn } from "./consultationChat";

export interface SendConsultationPromptParams {
  customPrompt?: string;
  promptText: string;
  isChatting: boolean;
  selectedSample: GenomeSample | null;
  selectedModel: string;
  generatedReport: GeneratedReport | null;
  attachedImages: { name: string; base64: string; previewUrl: string }[];
  messages: ChatMessage[];
  ollamaUrl: string;
  ollamaToken: string;
  temperature: number;
  maxTokens: number;
  extendedThinking: boolean;
  contextWindow: number;
  useVectorResearch: boolean;
  twoModelReview: boolean;
  reviewModel: string;
  selectedPacks: Record<string, boolean>;
  onlyActiveFindings: boolean;
  contextMode: AiContextMode;
  consultationMode: ConsultationMode;
  userProfile: UserBiohackingProfile;
  systemInstructions: string;
  alert: (message: string) => void;
  onExportModal: () => void;
  onPromptInspector: () => void;
  onState: (patch: {
    promptText?: string;
    attachedImages?: { name: string; base64: string; previewUrl: string }[];
    messages?: ChatMessage[];
    isChatting?: boolean;
    lastVectorQuery?: string;
    lastVectorHits?: QdrantHit[];
    lastVectorError?: string;
    sessionPromptTokens?: number;
    sessionResponseTokens?: number;
  }) => void;
  getSessionPromptTokens: () => number;
  getSessionResponseTokens: () => number;
  stopListeners: () => void;
  stopReviewListeners: () => void;
  registerMainListener: (stop: () => void) => void;
  registerReviewListener: (stop: () => void) => void;
}

export async function sendConsultationPrompt(params: SendConsultationPromptParams): Promise<void> {
  if (params.customPrompt === "TRIGGER_EXPORT_MODAL") {
    params.onExportModal();
    return;
  }
  if (params.customPrompt === "TRIGGER_CONTEXT_INSPECTOR") {
    params.onPromptInspector();
    return;
  }
  const text = params.customPrompt || params.promptText.trim();
  if (!text || params.isChatting || !params.selectedSample) return;
  if (!params.selectedModel) {
    params.alert("Please configure a connection and select an LLM model.");
    return;
  }
  if (!params.generatedReport) {
    params.alert("Report calculations are still loading. Please wait a moment.");
    return;
  }

  const userMsg: ChatMessage = { role: "user", content: text };
  if (params.attachedImages.length > 0) {
    userMsg.images = params.attachedImages.map((img) => img.base64);
  }
  let messages = [...params.messages, userMsg];
  for (const img of params.attachedImages) {
    if (img.previewUrl.startsWith("blob:")) URL.revokeObjectURL(img.previewUrl);
  }
  params.onState({
    attachedImages: [],
    promptText: params.customPrompt ? params.promptText : "",
    messages,
    isChatting: true,
  });
  messages = [...messages, { role: "assistant", content: "" }];
  const assistantIndex = messages.length - 1;
  params.onState({ messages });

  params.stopListeners();
  params.stopReviewListeners();

  try {
    await runConsultationTurn(
      {
        text,
        selectedSample: params.selectedSample,
        generatedReport: params.generatedReport,
        messages,
        assistantIndex,
        ollamaUrl: params.ollamaUrl,
        ollamaToken: params.ollamaToken,
        selectedModel: params.selectedModel,
        temperature: params.temperature,
        maxTokens: params.maxTokens,
        extendedThinking: params.extendedThinking,
        contextWindow: params.contextWindow,
        useVectorResearch: params.useVectorResearch,
        twoModelReview: params.twoModelReview,
        reviewModel: params.reviewModel,
        selectedPacks: params.selectedPacks,
        onlyActiveFindings: params.onlyActiveFindings,
        contextMode: params.contextMode,
        consultationMode: params.consultationMode,
        userProfile: params.userProfile,
        systemInstructions: params.systemInstructions,
      },
      {
        onMessages: (next) => {
          messages = next;
          params.onState({ messages: next });
        },
        onVectorResult: (hits, query, error) => {
          params.onState({
            lastVectorQuery: query,
            lastVectorHits: hits,
            lastVectorError: error || "",
          });
        },
        onTokenUsage: (prompt, response) => {
          params.onState({
            sessionPromptTokens: params.getSessionPromptTokens() + (prompt || 0),
            sessionResponseTokens: params.getSessionResponseTokens() + (response || 0),
          });
        },
        onChattingDone: () => params.onState({ isChatting: false }),
        registerMainListeners: params.registerMainListener,
        registerReviewListeners: params.registerReviewListener,
      },
    );
    if (!(params.twoModelReview && params.reviewModel)) {
      params.onState({ isChatting: false });
    }
  } catch (err: unknown) {
    params.stopListeners();
    params.stopReviewListeners();
    const errMsg = err instanceof Error ? err.message : String(err);
    if (messages[assistantIndex].content === "") {
      messages[assistantIndex].content = `Error connecting to AI: ${errMsg}`;
    } else {
      messages[assistantIndex].content += `\n\n*[Error during stream: ${errMsg}]*`;
    }
    params.onState({ messages: [...messages], isChatting: false });
  }
}

export function stopConsultationGeneration(
  messages: ChatMessage[],
  stopListeners: () => void,
  stopReviewListeners: () => void,
  onState: (patch: { messages: ChatMessage[]; isChatting: boolean }) => void,
): void {
  void cancelOllamaStream();
  stopListeners();
  stopReviewListeners();
  const next = [...messages];
  const last = next.length - 1;
  if (last >= 0 && next[last].role === "assistant") {
    if (next[last].safetyReview === "Reviewing response safety...") {
      next[last].safetyReview = "*[Review stopped by user]*";
    }
    next[last].content += "\n\n*[Consultation response stopped by user]*";
  }
  onState({ messages: next, isChatting: false });
}
