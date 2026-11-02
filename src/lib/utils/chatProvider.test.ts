// ./src/lib/utils/chatProvider.test.ts
/* Provider routing and disclosure tests for Ask AI. Run: pnpm test */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const store = new Map<string, string>();

vi.stubGlobal("localStorage", {
  getItem: (key: string) => store.get(key) ?? null,
  setItem: (key: string, value: string) => {
    store.set(key, value);
  },
  removeItem: (key: string) => {
    store.delete(key);
  },
  clear: () => store.clear(),
});

vi.mock("../api/tauri", () => ({
  streamOllamaChat: vi.fn(() => Promise.resolve()),
}));

vi.mock("../api/chatgpt", () => ({
  streamChatgptChat: vi.fn(() => Promise.resolve()),
}));

import { streamOllamaChat } from "../api/tauri";
import { streamChatgptChat } from "../api/chatgpt";
import {
  CHAT_PROVIDER_STORAGE_KEY,
  CHATGPT_RAW_DATA_DISCLOSURE,
  chatProviderDisclosure,
  loadChatProvider,
  saveChatProvider,
  streamChatCompletion,
} from "./chatProvider";

describe("chatProvider", () => {
  beforeEach(() => {
    store.clear();
    vi.clearAllMocks();
  });

  afterEach(() => {
    store.clear();
  });

  it("defaults to ollama and persists chatgpt when selected", () => {
    expect(loadChatProvider()).toBe("ollama");
    saveChatProvider("chatgpt");
    expect(loadChatProvider()).toBe("chatgpt");
    expect(store.get(CHAT_PROVIDER_STORAGE_KEY)).toBe("chatgpt");
  });

  it("discloses OpenAI as the ChatGPT destination and states no API key is used", () => {
    const text = chatProviderDisclosure("chatgpt", "http://127.0.0.1:11434");
    expect(text).toBe(CHATGPT_RAW_DATA_DISCLOSURE);
    expect(text).toMatch(/api\.openai\.com/);
    expect(text).toMatch(/leave this device/);
    expect(text).toMatch(/No API key is used/i);
  });

  it("routes streaming to ChatGPT without temperature/token knobs", async () => {
    await streamChatCompletion(
      "chatgpt",
      "stream-1",
      { url: "http://127.0.0.1:11434", token: undefined },
      "gpt-5",
      [{ role: "user", content: "hello" }],
      0.2,
      512,
    );
    expect(streamChatgptChat).toHaveBeenCalledWith("stream-1", "gpt-5", [
      { role: "user", content: "hello" },
    ]);
    expect(streamOllamaChat).not.toHaveBeenCalled();
  });

  it("keeps Ollama on the local streaming path", async () => {
    await streamChatCompletion(
      "ollama",
      "stream-2",
      { url: "http://127.0.0.1:11434", token: "tok" },
      "llama3",
      [{ role: "user", content: "hi" }],
      0,
      256,
    );
    expect(streamOllamaChat).toHaveBeenCalled();
    expect(streamChatgptChat).not.toHaveBeenCalled();
  });
});
