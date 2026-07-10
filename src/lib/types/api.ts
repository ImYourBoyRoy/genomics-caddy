// ./src/lib/types/api.ts
/**
 * Typed IPC payloads for Tauri commands in tauri.ts.
 */

import type { ChatMessage } from "./agent";

export interface OllamaModelDetails {
  modelfile?: string;
  parameters?: string;
  template?: string;
  details?: {
    parent_model?: string;
    format?: string;
    family?: string;
    families?: string[];
    parameter_size?: string;
    quantization_level?: string;
  };
  model_info?: Record<string, unknown>;
  license?: string;
}

export interface ActiveOllamaModel {
  name: string;
  model: string;
  size: number;
  digest: string;
  expires_at?: string;
  size_vram?: number;
}

export interface McpToolParam {
  name: string;
  type: string;
  required?: boolean;
  description?: string;
}

export interface McpToolDefinition {
  name: string;
  description: string;
  params: McpToolParam[];
}

export interface ExternalApiCacheResponse {
  url: string;
  body: unknown;
  cached: boolean;
  fetched_at?: number;
}

/** Narrow unknown JSON from fetchExternalApi for property access. */
export function asApiJson(value: unknown): Record<string, unknown> | null {
  if (value && typeof value === "object" && !Array.isArray(value)) {
    return value as Record<string, unknown>;
  }
  return null;
}

/** Persisted consultation session shape (matches Rust DbChatSession). */
export interface DbChatSession {
  id: string;
  title: string;
  messages: ChatMessage[];
  timestamp: number;
  sampleId: number | null;
  selectedPacks: Record<string, boolean>;
  onlyActiveFindings: boolean;
  temperature: number;
  selectedModel: string;
  maxTokens?: number;
  extendedThinking?: boolean;
  consultationMode?: string;
}
