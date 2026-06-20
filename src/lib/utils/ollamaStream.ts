// ./src/lib/utils/ollamaStream.ts
/**
 * Namespaced Ollama stream events — each stream uses a unique ID so concurrent
 * consultation and agent runs cannot cross-contaminate chunks.
 */

import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface OllamaDonePayload {
  prompt_eval_count?: number;
  eval_count?: number;
  cancelled?: boolean;
}

export function newOllamaStreamId(): string {
  return crypto.randomUUID();
}

export function ollamaChunkEvent(streamId: string): string {
  return `ollama-chunk:${streamId}`;
}

export function ollamaDoneEvent(streamId: string): string {
  return `ollama-done:${streamId}`;
}

export interface OllamaStreamHandlers {
  onChunk: (text: string) => void;
  onDone: (payload: OllamaDonePayload | null) => void;
}

/** Subscribe to a single Ollama stream; returns an unsubscribe function. */
export async function subscribeOllamaStream(
  streamId: string,
  handlers: OllamaStreamHandlers,
): Promise<() => void> {
  const chunkEvent = ollamaChunkEvent(streamId);
  const doneEvent = ollamaDoneEvent(streamId);

  const [unlistenChunk, unlistenDone] = await Promise.all([
    listen<string>(chunkEvent, (event) => {
      handlers.onChunk(event.payload);
    }),
    listen<OllamaDonePayload>(doneEvent, (event) => {
      handlers.onDone(event.payload ?? null);
    }),
  ]);

  const stop: UnlistenFn = () => {
    unlistenChunk();
    unlistenDone();
  };
  return stop;
}

/**
 * Run an async action with guaranteed listener cleanup.
 */
export async function withOllamaStream<T>(
  streamId: string,
  handlers: OllamaStreamHandlers,
  action: () => Promise<T>,
): Promise<T> {
  const stop = await subscribeOllamaStream(streamId, handlers);
  try {
    return await action();
  } finally {
    stop();
  }
}
