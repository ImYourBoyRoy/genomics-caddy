// ./src/lib/utils/ollamaSettings.test.ts
import { describe, expect, it, beforeEach, vi } from "vitest";
import {
  classifyOllamaEndpoint,
  loadOllamaUrl,
  OLLAMA_URL_STORAGE_KEY,
  ollamaRawDataDisclosure,
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
    saveOllamaUrl("http://ollama.example.test:11434");
    expect(localStorage.getItem(OLLAMA_URL_STORAGE_KEY)).toBe("http://ollama.example.test:11434");
    expect(loadOllamaUrl()).toBe("http://ollama.example.test:11434");
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

  it("classifies local and remote endpoints for raw-data disclosure", () => {
    expect(classifyOllamaEndpoint("http://127.0.0.1:11434")).toBe("local");
    expect(classifyOllamaEndpoint("http://localhost:11434")).toBe("local");
    expect(classifyOllamaEndpoint("http://[::1]:11434")).toBe("local");
    expect(classifyOllamaEndpoint("http://ollama.example.test:11434")).toBe("remote");
    expect(classifyOllamaEndpoint("not a URL")).toBe("remote");
    expect(classifyOllamaEndpoint(" ")).toBe("unconfigured");
    expect(ollamaRawDataDisclosure("not a URL")).toContain("invalid");
    expect(ollamaRawDataDisclosure("http://ollama.example.test:11434")).toContain("leave this device");
  });
});
