// ./src/lib/utils/ollamaSettings.ts
/**
 * Shared Ollama connection URL persistence for AI consultation and research panels.
 * Precedence: `.env` (via Tauri) → localStorage (UI save) → localhost default.
 */

import { getOllamaServiceConfig } from "../api/tauri";

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

/** Resolve Ollama URL at app startup: `.env` wins, then localStorage, then localhost. */
export async function resolveInitialOllamaUrl(): Promise<string> {
  try {
    const cfg = await getOllamaServiceConfig();
    if (cfg.from_env) {
      saveOllamaUrl(cfg.url);
      return cfg.url;
    }
  } catch (e) {
    console.warn("Could not load Ollama service config from backend:", e);
  }
  return loadOllamaUrl();
}
