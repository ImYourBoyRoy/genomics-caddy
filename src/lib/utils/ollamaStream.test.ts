// ./src/lib/utils/ollamaStream.test.ts
import { describe, expect, it } from "vitest";
import {
  newOllamaStreamId,
  ollamaChunkEvent,
  ollamaDoneEvent,
} from "./ollamaStream";

describe("ollamaStream", () => {
  it("builds namespaced event strings", () => {
    expect(ollamaChunkEvent("abc-123")).toBe("ollama-chunk:abc-123");
    expect(ollamaDoneEvent("abc-123")).toBe("ollama-done:abc-123");
  });

  it("generates unique stream ids", () => {
    const a = newOllamaStreamId();
    const b = newOllamaStreamId();
    expect(a).not.toBe(b);
    expect(a.length).toBeGreaterThan(10);
  });
});
