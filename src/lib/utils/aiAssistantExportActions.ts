// ./src/lib/utils/aiAssistantExportActions.ts
/**
 * Clipboard copy and markdown export actions for the AI consultation export modal.
 */

import { saveReportJson } from "../api/tauri";
import type { ChatMessage } from "../types/agent";
import type { GenomeSample, GeneratedReport } from "../types/genomics";
import { buildStandardMarkdown, buildClinicalHandoffMarkdown } from "./aiExport";
import { stripThinkingTokens } from "./chatParser";
import type { UserBiohackingProfile } from "./aiPrompt";

export interface CopyToClipboardParams {
  text: string;
  includeTraceInExport: boolean;
  onSuccess: () => void;
}

export function copyMessageToClipboard({
  text,
  includeTraceInExport,
  onSuccess,
}: CopyToClipboardParams): void {
  navigator.clipboard
    .writeText(includeTraceInExport ? text : stripThinkingTokens(text))
    .then(onSuccess)
    .catch((err) => console.error("Failed to copy text: ", err));
}

export interface ExportConversationParams {
  type: "standard" | "clinical";
  messages: ChatMessage[];
  selectedSample: GenomeSample | null;
  selectedModel: string;
  userProfile: UserBiohackingProfile;
  currentSystemPrompt: string;
  generatedReport: GeneratedReport | null;
  includeTraceInExport: boolean;
  alert: (message: string) => void;
}

export async function exportConsultationMarkdown({
  type,
  messages,
  selectedSample,
  selectedModel,
  userProfile,
  currentSystemPrompt,
  generatedReport,
  includeTraceInExport,
  alert,
}: ExportConversationParams): Promise<void> {
  if (!messages.length) {
    alert("No conversation history to export.");
    return;
  }
  const md =
    type === "clinical"
      ? buildClinicalHandoffMarkdown(
          messages,
          selectedSample,
          selectedModel,
          userProfile,
          currentSystemPrompt,
          generatedReport,
          includeTraceInExport,
        )
      : buildStandardMarkdown(messages, selectedSample, selectedModel, includeTraceInExport);
  const cleanName = (selectedSample ? selectedSample.name : "genome").replace(/[^a-zA-Z0-9]/g, "_");
  try {
    if (await saveReportJson(md, `${cleanName}_consultation_${type}_${Date.now()}.md`)) {
      alert("Chat conversation exported and saved successfully!");
    }
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : String(err);
    alert("Failed to save chat export: " + message);
  }
}
