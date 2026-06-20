// ./src/lib/utils/ollamaSettings.ts
/**
 * Shared Ollama connection URL persistence for AI consultation and research panels.
 */

export const OLLAMA_URL_STORAGE_KEY = "genomics_ollama_url";
export const DEFAULT_OLLAMA_URL = "http://localhost:11434";

export function loadOllamaUrl(fallback: string = DEFAULT_OLLAMA_URL): string {
  if (typeof localStorage === "undefined") return fallback;
  return localStorage.getItem(OLLAMA_URL_STORAGE_KEY) || fallback;
}

export function saveOllamaUrl(url: string): void {
  if (typeof localStorage === "undefined") return;
  localStorage.setItem(OLLAMA_URL_STORAGE_KEY, url.trim());
}
