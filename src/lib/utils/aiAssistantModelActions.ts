// ./src/lib/utils/aiAssistantModelActions.ts
/**
 * Ollama model scan and detail loading for AiAssistantPanel.
 */

import { scanOllamaModels, showOllamaModel, saveOllamaToken } from "../api/tauri";
import {
  autoSelectModel,
  filterChatModels,
  getContextWindow,
  isModelVisionCapable,
} from "./aiPrompt";
import { chatgptListModels, chatgptStatus } from "../api/chatgpt";
import { persistOllamaUrl } from "./ollamaSettings";
import type { ChatProvider } from "./chatProvider";

/** Models available to the connected ChatGPT account (plan usage route). */
export async function scanChatGptModels(
  currentModel: string,
): Promise<{ models: string[]; selectedModel: string; scanError: string }> {
  try {
    const connection = await chatgptStatus();
    if (!connection.connected || !connection.sharing) {
      // Not an error: the settings card prompts for sign-in.
      return { models: [], selectedModel: "", scanError: "" };
    }
    const models = (await chatgptListModels()).map((m) => m.slug);
    return {
      models,
      selectedModel: models.includes(currentModel) ? currentModel : (models[0] ?? ""),
      scanError: models.length ? "" : "No models are available for this ChatGPT connection.",
    };
  } catch (err: unknown) {
    return {
      models: [],
      selectedModel: "",
      scanError: err instanceof Error ? err.message : String(err),
    };
  }
}

export async function scanChatModels(
  ollamaUrl: string,
  ollamaToken: string,
  currentModel: string,
): Promise<{ models: string[]; selectedModel: string; scanError: string }> {
  if (!ollamaUrl.trim()) {
    return {
      models: [],
      selectedModel: currentModel,
      scanError: "Set an Ollama URL under Advanced → Connections before scanning models.",
    };
  }
  await persistOllamaUrl(ollamaUrl);
  await saveOllamaToken(ollamaToken || undefined);
  try {
    const models = filterChatModels(await scanOllamaModels(ollamaUrl, ollamaToken || undefined));
    return {
      models,
      selectedModel: autoSelectModel(models, currentModel),
      scanError: "",
    };
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : String(err);
    let scanError = `Failed to connect: ${message}`;
    if (!ollamaUrl.includes("localhost") && !ollamaUrl.includes("127.0.0.1")) {
      scanError +=
        "\n\n💡 Remote Connection Tips:\n1. Ensure Ollama is running on the remote host.\n2. Set OLLAMA_HOST=0.0.0.0 before starting Ollama.\n3. Verify port 11434 is open in the firewall.";
    }
    return { models: [], selectedModel: currentModel, scanError };
  }
}

export async function loadOllamaModelDetails(
  ollamaUrl: string,
  ollamaToken: string,
  selectedModel: string,
  provider: ChatProvider = "ollama",
): Promise<{
  modelDetails: unknown;
  isVisionCapable: boolean;
  contextWindow: number;
}> {
  if (!selectedModel) {
    return { modelDetails: null, isVisionCapable: false, contextWindow: 4096 };
  }
  if (provider === "chatgpt") {
    // Hosted models: no local model card, images accepted by the Responses route.
    return { modelDetails: null, isVisionCapable: true, contextWindow: 4096 };
  }
  try {
    const details = await showOllamaModel(ollamaUrl, ollamaToken || undefined, selectedModel);
    return {
      modelDetails: details,
      isVisionCapable: isModelVisionCapable(selectedModel, details),
      contextWindow: getContextWindow(details.model_info),
    };
  } catch {
    return {
      modelDetails: null,
      isVisionCapable: isModelVisionCapable(selectedModel, null),
      contextWindow: 4096,
    };
  }
}
