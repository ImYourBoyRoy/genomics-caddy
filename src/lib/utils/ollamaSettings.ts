// ./src/lib/utils/ollamaSettings.ts
/**
 * Shared Ollama connection URL persistence for AI consultation and research panels.
 * Precedence: SQLite (UI save) → `.env` bootstrap (via Tauri) → localStorage mirror → empty (must configure).
 */

import { getOllamaServiceConfig, saveOllamaUrl as saveOllamaUrlBackend } from "../api/tauri";

export const OLLAMA_URL_STORAGE_KEY = "genomics_ollama_url";

/** Example-only placeholder for empty inputs — never used as a live default. */
export const OLLAMA_URL_PLACEHOLDER = "e.g. http://127.0.0.1:11434 or http://your-host:11434";

export function loadOllamaUrl(fallback: string = ""): string {
  if (typeof localStorage === "undefined") return fallback;
  return localStorage.getItem(OLLAMA_URL_STORAGE_KEY) || fallback;
}

export function saveOllamaUrl(url: string): void {
  if (typeof localStorage === "undefined") return;
  const trimmed = url.trim();
  if (!trimmed) {
    localStorage.removeItem(OLLAMA_URL_STORAGE_KEY);
    return;
  }
  localStorage.setItem(OLLAMA_URL_STORAGE_KEY, trimmed);
}

/** Persist to SQLite (via Tauri) and mirror into localStorage for fast UI reads. */
export async function persistOllamaUrl(url: string): Promise<void> {
  const trimmed = url.trim();
  await saveOllamaUrlBackend(trimmed);
  saveOllamaUrl(trimmed);
}

/** Resolve Ollama URL at app startup: SQLite (UI-saved) → `.env` bootstrap → empty. */
export async function resolveInitialOllamaUrl(): Promise<string> {
  try {
    const cfg = await getOllamaServiceConfig();
    if (cfg.url?.trim()) {
      saveOllamaUrl(cfg.url);
      return cfg.url.trim();
    }
  } catch (e) {
    console.warn("Could not load Ollama service config from backend:", e);
  }
  return loadOllamaUrl("");
}

export const LOCAL_OLLAMA_URL = "http://127.0.0.1:11434";
export const LOCAL_QDRANT_URL = "http://127.0.0.1:6333";
