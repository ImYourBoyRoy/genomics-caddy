// ./src/lib/utils/aiAssistantPreferences.ts
/**
 * Load persisted AI consultation UI preferences from localStorage.
 */

import { DEFAULT_INSTRUCTIONS, type AiContextMode, type ConsultationMode } from "./aiPrompt";
import type { UserBiohackingProfile } from "./aiPrompt";

export interface AiAssistantPreferences {
  showThinkingProcess: boolean;
  autoCollapseThinking: boolean;
  includeTraceInExport: boolean;
  contextMode: AiContextMode;
  consultationMode: ConsultationMode;
  reviewModel: string;
  twoModelReview: boolean;
  userProfile: Partial<UserBiohackingProfile>;
  systemInstructions: string;
}

export function loadAiAssistantPreferences(
  baseProfile: UserBiohackingProfile,
): AiAssistantPreferences {
  let userProfile: Partial<UserBiohackingProfile> = {};
  try {
    userProfile = JSON.parse(localStorage.getItem("genomics_user_biohacking_profile") || "{}");
  } catch {
    userProfile = {};
  }
  return {
    showThinkingProcess: localStorage.getItem("genomics_show_thinking_process") !== "false",
    autoCollapseThinking: localStorage.getItem("genomics_auto_collapse_thinking") !== "false",
    includeTraceInExport: localStorage.getItem("genomics_include_trace_in_export") === "true",
    contextMode: (localStorage.getItem("genomics_context_mode") || "active_findings") as AiContextMode,
    consultationMode: (localStorage.getItem("genomics_consultation_mode") || "general") as ConsultationMode,
    reviewModel: localStorage.getItem("genomics_review_model") || "",
    twoModelReview: localStorage.getItem("genomics_two_model_review") === "true",
    userProfile: { ...baseProfile, ...userProfile },
    systemInstructions: localStorage.getItem("genomics_system_instructions") || DEFAULT_INSTRUCTIONS,
  };
}

export function persistAiAssistantPreferences(prefs: {
  showThinkingProcess: boolean;
  autoCollapseThinking: boolean;
  includeTraceInExport: boolean;
  contextMode: AiContextMode;
  consultationMode: ConsultationMode;
  reviewModel: string;
  twoModelReview: boolean;
}): void {
  const entries: Record<string, string | boolean> = {
    show_thinking_process: prefs.showThinkingProcess,
    auto_collapse_thinking: prefs.autoCollapseThinking,
    include_trace_in_export: prefs.includeTraceInExport,
    context_mode: prefs.contextMode,
    consultation_mode: prefs.consultationMode,
    review_model: prefs.reviewModel,
    two_model_review: prefs.twoModelReview,
  };
  for (const [key, value] of Object.entries(entries)) {
    localStorage.setItem(`genomics_${key}`, String(value));
  }
}
