// ./src/lib/utils/consultationSession.svelte.ts
/**
 * Consultation Session Reactive Store for Svelte 5.
 * Purpose: Manage active and historical chat sessions reactively.
 * Key Inputs: Selected genome sample, current AI models.
 * Key Outputs: Reactive lists of sessions, active session ID, session creation/deletion logic.
 * Operational Notes: Leverages Svelte 5's class-based reactive state.
 */

import { createNewSession, type ChatSession } from "./chatSession";
import { getChatSessions, saveChatSession, deleteChatSession } from "../api/tauri";
import type { GenomeSample } from "../types/genomics";

export class ConsultationSessionStore {
  sessions = $state<ChatSession[]>([]);
  currentSessionId = $state<string | null>(null);

  constructor() {
    // sessions will be loaded asynchronously during load()
  }

  async load(selectedSample: GenomeSample | null, selectedModel: string, models: string[], manifestPacks: any[]) {
    const pid = selectedSample ? selectedSample.id : null;
    try {
      this.sessions = await getChatSessions(pid);
    } catch (e) {
      console.error("Failed to load chat sessions from database:", e);
      this.sessions = [];
    }
    const activeId = localStorage.getItem(`genomics_active_session_id_${pid}`);
    const found = activeId ? this.sessions.find(s => s.id === activeId && s.sampleId === pid) : null;
    if (found) {
      this.currentSessionId = found.id;
      localStorage.setItem("genomics_active_session_id", found.id);
    } else {
      const ps = this.sessions.filter(s => s.sampleId === pid);
      if (ps.length > 0) {
        this.currentSessionId = ps[0].id;
        localStorage.setItem(`genomics_active_session_id_${pid}`, ps[0].id);
        localStorage.setItem("genomics_active_session_id", ps[0].id);
      } else {
        await this.startNew(selectedSample, selectedModel, models, manifestPacks);
      }
    }
  }

  async startNew(selectedSample: GenomeSample | null, selectedModel: string, models: string[], manifestPacks: any[]) {
    const pid = selectedSample ? selectedSample.id : null;
    const ns = createNewSession({ sampleName: selectedSample?.name || "Guest", sampleId: pid, selectedModel, models, manifestPacks });
    this.sessions = [ns, ...this.sessions];
    this.currentSessionId = ns.id;
    localStorage.setItem(`genomics_active_session_id_${pid}`, ns.id);
    localStorage.setItem("genomics_active_session_id", ns.id);
    try {
      await saveChatSession(ns);
    } catch (e) {
      console.error("Failed to save new chat session to database:", e);
    }
  }

  async delete(id: string, selectedSample: GenomeSample | null, selectedModel: string, models: string[], manifestPacks: any[]) {
    this.sessions = this.sessions.filter(s => s.id !== id);
    try {
      await deleteChatSession(id);
    } catch (e) {
      console.error("Failed to delete chat session from database:", e);
    }
    if (this.currentSessionId === id) {
      const pid = selectedSample ? selectedSample.id : null;
      const ps = this.sessions.filter(s => s.sampleId === pid);
      if (ps.length > 0) {
        this.currentSessionId = ps[0].id;
        localStorage.setItem(`genomics_active_session_id_${pid}`, ps[0].id);
        localStorage.setItem("genomics_active_session_id", ps[0].id);
      } else {
        await this.startNew(selectedSample, selectedModel, models, manifestPacks);
      }
    }
  }

  async saveTitle() {
    if (!this.currentSessionId) return;
    const current = this.sessions.find(s => s.id === this.currentSessionId);
    if (current) {
      try {
        await saveChatSession(current);
      } catch (e) {
        console.error("Failed to auto-save chat session to database:", e);
      }
    }
  }
}
