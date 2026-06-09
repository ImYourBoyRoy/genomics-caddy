// ./src/lib/utils/chatSession.ts
/**
 * Consultation Chat Session Serialization & Persistence Helpers.
 * Purpose: Handle reading/writing of chat session histories from local storage.
 * Key Inputs: ChatSession objects, local storage access keys.
 * Key Outputs: Serialized session strings, new ChatSession templates.
 * Operational Notes: Keeps AI panel state thin by isolating persistence logic.
 */

export interface ChatSession {
  id: string;
  title: string;
  messages: any[];
  timestamp: number;
  sampleId: number | null;
  selectedPacks: Record<string, boolean>;
  onlyActiveFindings: boolean;
  temperature: number;
  selectedModel: string;
  maxTokens?: number;
  extendedThinking?: boolean;
}

/**
 * Load all consultation chat sessions from localStorage.
 */
export function loadSessionsFromLocalStorage(): ChatSession[] {
  const saved = localStorage.getItem("genomics_chat_sessions");
  if (!saved) return [];
  try {
    return JSON.parse(saved);
  } catch (e) {
    console.error("Failed to parse chat sessions from local storage:", e);
    return [];
  }
}

/**
 * Save the entire chat sessions array to localStorage.
 */
export function saveSessionsToLocalStorage(sessions: ChatSession[]): void {
  localStorage.setItem("genomics_chat_sessions", JSON.stringify(sessions));
}

/**
 * Factory function to create a new empty consultation session.
 */
export function createNewSession(params: {
  sampleName: string;
  sampleId: number | null;
  selectedModel: string;
  models: string[];
  manifestPacks: { id: string }[];
}): ChatSession {
  const { sampleName, sampleId, selectedModel, models, manifestPacks } = params;
  const title = `Consultation with ${sampleName} - ${new Date().toLocaleDateString()} ${new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}`;
  
  const packs: Record<string, boolean> = {};
  for (const pack of manifestPacks) {
    packs[pack.id] = true;
  }

  return {
    id: Date.now().toString(),
    title,
    messages: [],
    timestamp: Date.now(),
    sampleId,
    selectedPacks: packs,
    onlyActiveFindings: true,
    temperature: 0.0,
    selectedModel: selectedModel || (models.length > 0 ? models[0] : ""),
    maxTokens: 2048,
    extendedThinking: false,
  };
}
