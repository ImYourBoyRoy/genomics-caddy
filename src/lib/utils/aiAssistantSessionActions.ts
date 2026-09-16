// ./src/lib/utils/aiAssistantSessionActions.ts
/**
 * Consultation session load/create/delete actions for AiAssistantPanel.
 */

import type { ChatMessage } from "../types/agent";
import type { GenomeSample } from "../types/genomics";
import type { AiContextMode, ConsultationMode } from "./aiPrompt";
import type { ConsultationSessionStore } from "./consultationSession.svelte";

export interface ConsultationSessionSnapshot {
  messages: ChatMessage[];
  selectedPacks: Record<string, boolean>;
  onlyActiveFindings: boolean;
  temperature: number;
  selectedModel: string;
  maxTokens: number;
  extendedThinking: boolean;
  consultationMode: ConsultationMode;
  contextMode: AiContextMode;
}

export function readConsultationSession(
  id: string,
  sessionStore: ConsultationSessionStore,
  selectedSample: GenomeSample | null,
): ConsultationSessionSnapshot | null {
  const session = sessionStore.sessions.find((x) => x.id === id);
  if (!session) return null;
  sessionStore.currentSessionId = id;
  const pid = selectedSample ? selectedSample.id : null;
  localStorage.setItem(`genomics_active_session_id_${pid}`, id);
  localStorage.setItem("genomics_active_session_id", id);
  return {
    messages: session.messages || [],
    selectedPacks: { ...session.selectedPacks },
    onlyActiveFindings: session.onlyActiveFindings,
    temperature: session.temperature,
    selectedModel: session.selectedModel,
    maxTokens: session.maxTokens || 2048,
    extendedThinking: session.extendedThinking || false,
    consultationMode: (session.consultationMode || "general") as ConsultationMode,
    contextMode: (session.contextMode || "active_findings") as AiContextMode,
  };
}

export async function startNewConsultationSession(
  sessionStore: ConsultationSessionStore,
  selectedSample: GenomeSample | null,
  selectedModel: string,
  models: string[],
  packs: { id: string }[],
): Promise<void> {
  await sessionStore.startNew(selectedSample, selectedModel, models, packs);
}

export async function saveConsultationSessionTitle(
  sessionStore: ConsultationSessionStore,
  session: { title: string },
): Promise<void> {
  if (session.title.trim()) await sessionStore.saveTitle();
}

export async function deleteConsultationSession(
  id: string,
  sessionStore: ConsultationSessionStore,
  selectedSample: GenomeSample | null,
  selectedModel: string,
  models: string[],
  packs: { id: string }[],
): Promise<void> {
  await sessionStore.delete(id, selectedSample, selectedModel, models, packs);
}
