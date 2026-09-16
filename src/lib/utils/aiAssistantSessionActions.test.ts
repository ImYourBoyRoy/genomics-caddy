import { beforeEach, describe, expect, it } from "vitest";
import { createNewSession, type ChatSession } from "./chatSession";
import { readConsultationSession } from "./aiAssistantSessionActions";
import type { ConsultationSessionStore } from "./consultationSession.svelte";

function makeStorage() {
  const values = new Map<string, string>();
  return {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => values.set(key, value),
    removeItem: (key: string) => values.delete(key),
    clear: () => values.clear(),
  };
}

describe("AI assistant session context persistence", () => {
  beforeEach(() => {
    Object.defineProperty(globalThis, "localStorage", {
      configurable: true,
      value: makeStorage(),
    });
  });

  it("restores the saved raw-genotype context mode", () => {
    const session = createNewSession({
      sampleName: "Fixture",
      sampleId: 7,
      selectedModel: "fixture-model",
      models: ["fixture-model"],
      manifestPacks: [{ id: "metabolic" }],
    });
    session.contextMode = "developer_raw_json";
    const store = {
      sessions: [session],
      currentSessionId: null,
    } as unknown as ConsultationSessionStore;

    const snapshot = readConsultationSession("" + session.id, store, null);

    expect(snapshot?.contextMode).toBe("developer_raw_json");
    expect(store.currentSessionId).toBe(session.id);
  });

  it("defaults sessions created before context mode existed to active findings", () => {
    const session = createNewSession({
      sampleName: "Legacy fixture",
      sampleId: null,
      selectedModel: "fixture-model",
      models: ["fixture-model"],
      manifestPacks: [],
    });
    const legacySession = { ...session, contextMode: undefined } as ChatSession;
    const store = {
      sessions: [legacySession],
      currentSessionId: null,
    } as unknown as ConsultationSessionStore;

    const snapshot = readConsultationSession(session.id, store, null);

    expect(snapshot?.contextMode).toBe("active_findings");
  });
});
