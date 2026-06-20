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
import { saveOllamaUrl } from "./ollamaSettings";

export async function scanChatModels(
  ollamaUrl: string,
  ollamaToken: string,
  currentModel: string,
): Promise<{ models: string[]; selectedModel: string; scanError: string }> {
  saveOllamaUrl(ollamaUrl);
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
): Promise<{
  modelDetails: unknown;
  isVisionCapable: boolean;
  contextWindow: number;
}> {
  if (!selectedModel) {
    return { modelDetails: null, isVisionCapable: false, contextWindow: 4096 };
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
