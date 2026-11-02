// ./src/lib/api/chatgpt.ts
/**
 * Tauri bindings for "Continue with ChatGPT" (ChatGPT plan usage).
 * Tokens stay in the Rust backend / OS credential manager; the webview only
 * ever receives the connection summary below.
 */

import { invoke } from "@tauri-apps/api/core";
import type { ChatMessage } from "../types/agent";

export interface ChatGptStatus {
  connected: boolean;
  /** True only when the user granted ChatGPT plan usage. */
  sharing: boolean;
  email: string | null;
  name: string | null;
  expires_at: number | null;
}

export interface ChatGptModel {
  slug: string;
  display_name: string;
}

export const chatgptStatus = () => invoke<ChatGptStatus>("chatgpt_status");
export const chatgptSignIn = () => invoke<ChatGptStatus>("chatgpt_sign_in");
export const chatgptCancelSignIn = () => invoke<void>("chatgpt_cancel_sign_in");
export const chatgptSignOut = () => invoke<ChatGptStatus>("chatgpt_sign_out");
export const chatgptOpenUsage = () => invoke<void>("chatgpt_open_usage");
export const chatgptListModels = () => invoke<ChatGptModel[]>("chatgpt_list_models");

export const streamChatgptChat = (streamId: string, model: string, messages: ChatMessage[]) =>
  invoke<void>("stream_chatgpt_chat", { streamId, model, messages });
