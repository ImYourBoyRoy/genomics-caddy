// ./src/lib/utils/chatProvider.ts
/**
 * Which backend answers Ask AI consultations: a local/remote Ollama endpoint or
 * the user's ChatGPT plan. Ollama stays configured either way because vector
 * research embeddings still use it.
 */

import { streamOllamaChat } from "../api/tauri";
import { streamChatgptChat } from "../api/chatgpt";
import type { ChatMessage } from "../types/agent";
import { ollamaRawDataDisclosure } from "./ollamaSettings";

export type ChatProvider = "ollama" | "chatgpt";

export const CHAT_PROVIDER_STORAGE_KEY = "genomics_chat_provider";

export const CHATGPT_RAW_DATA_DISCLOSURE =
  "Raw genotype calls in the selected context are sent to OpenAI (api.openai.com) using your ChatGPT plan and leave this device. Requests ask OpenAI not to store them; OpenAI's own terms still apply. No API key is used and ChatGPT conversations are not accessed.";

export function loadChatProvider(): ChatProvider {
  if (typeof localStorage === "undefined") return "ollama";
  return localStorage.getItem(CHAT_PROVIDER_STORAGE_KEY) === "chatgpt" ? "chatgpt" : "ollama";
}

export function saveChatProvider(provider: ChatProvider): void {
  if (typeof localStorage === "undefined") return;
  localStorage.setItem(CHAT_PROVIDER_STORAGE_KEY, provider);
}

export function chatProviderDisclosure(provider: ChatProvider, ollamaUrl: string): string {
  return provider === "chatgpt" ? CHATGPT_RAW_DATA_DISCLOSURE : ollamaRawDataDisclosure(ollamaUrl);
}

/** Route a streamed chat completion to the selected provider. */
export function streamChatCompletion(
  provider: ChatProvider,
  streamId: string,
  ollama: { url: string; token: string | undefined },
  model: string,
  messages: ChatMessage[],
  temperature: number,
  numPredict: number,
): Promise<void> {
  if (provider === "chatgpt") {
    // The ChatGPT plan route rejects temperature and max_output_tokens.
    return streamChatgptChat(streamId, model, messages);
  }
  return streamOllamaChat(streamId, ollama.url, ollama.token, model, messages, temperature, numPredict);
}
