// ./src/lib/utils/ollamaSettings.test.ts
import { describe, expect, it, beforeEach, vi } from "vitest";
import {
  loadOllamaUrl,
  OLLAMA_URL_STORAGE_KEY,
  saveOllamaUrl,
} from "./ollamaSettings";

function mockLocalStorage() {
  const store = new Map<string, string>();
  vi.stubGlobal("localStorage", {
    getItem: (key: string) => store.get(key) ?? null,
    setItem: (key: string, value: string) => {
      store.set(key, value);
    },
    removeItem: (key: string) => {
      store.delete(key);
    },
    clear: () => {
      store.clear();
    },
  });
  return store;
}

describe("ollamaSettings", () => {
  beforeEach(() => {
    mockLocalStorage();
  });

  it("returns empty when storage is empty (no hardcoded host)", () => {
    expect(loadOllamaUrl()).toBe("");
  });

  it("persists and reloads URL", () => {
    saveOllamaUrl("http://192.168.1.10:11434");
    expect(localStorage.getItem(OLLAMA_URL_STORAGE_KEY)).toBe("http://192.168.1.10:11434");
    expect(loadOllamaUrl()).toBe("http://192.168.1.10:11434");
  });

  it("trims whitespace on save", () => {
    saveOllamaUrl("  http://127.0.0.1:11434  ");
    expect(loadOllamaUrl()).toBe("http://127.0.0.1:11434");
  });

  it("clears storage when saving empty", () => {
    saveOllamaUrl("http://127.0.0.1:11434");
    saveOllamaUrl("  ");
    expect(localStorage.getItem(OLLAMA_URL_STORAGE_KEY)).toBeNull();
  });
});
