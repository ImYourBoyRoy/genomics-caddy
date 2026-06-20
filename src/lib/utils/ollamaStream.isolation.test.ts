// ./src/lib/utils/ollamaStream.isolation.test.ts
import { describe, expect, it } from "vitest";
import {
  newOllamaStreamId,
  ollamaChunkEvent,
  ollamaDoneEvent,
} from "./ollamaStream";

describe("ollama stream isolation", () => {
  it("assigns distinct event namespaces per stream id", () => {
    const consultationId = newOllamaStreamId();
    const agentId = newOllamaStreamId();

    expect(consultationId).not.toBe(agentId);
    expect(ollamaChunkEvent(consultationId)).not.toBe(ollamaChunkEvent(agentId));
    expect(ollamaDoneEvent(consultationId)).not.toBe(ollamaDoneEvent(agentId));
  });

  it("never emits bare global event names", () => {
    const id = newOllamaStreamId();
    expect(ollamaChunkEvent(id)).not.toBe("ollama-chunk");
    expect(ollamaDoneEvent(id)).not.toBe("ollama-done");
    expect(ollamaChunkEvent(id)).toContain(id);
  });
});
