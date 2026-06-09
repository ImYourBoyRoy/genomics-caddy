// ./src/lib/utils/consultationSession.svelte.ts
/**
 * Consultation Session Reactive Store for Svelte 5.
 * Purpose: Manage active and historical chat sessions reactively.
 * Key Inputs: Selected genome sample, current AI models.
 * Key Outputs: Reactive lists of sessions, active session ID, session creation/deletion logic.
 * Operational Notes: Leverages Svelte 5's class-based reactive state.
 */

import { loadSessionsFromLocalStorage, saveSessionsToLocalStorage, createNewSession, type ChatSession } from "./chatSession";
import type { GenomeSample } from "../types/genomics";

export class ConsultationSessionStore {
  sessions = $state<ChatSession[]>([]);
  currentSessionId = $state<string | null>(null);

  constructor() {
    this.sessions = loadSessionsFromLocalStorage();
  }

  load(selectedSample: GenomeSample | null, selectedModel: string, models: string[], manifestPacks: any[]) {
    this.sessions = loadSessionsFromLocalStorage();
    const pid = selectedSample ? selectedSample.id : null;
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
        this.startNew(selectedSample, selectedModel, models, manifestPacks);
      }
    }
  }

  startNew(selectedSample: GenomeSample | null, selectedModel: string, models: string[], manifestPacks: any[]) {
    const pid = selectedSample ? selectedSample.id : null;
    const ns = createNewSession({ sampleName: selectedSample?.name || "Guest", sampleId: pid, selectedModel, models, manifestPacks });
    this.sessions = [ns, ...this.sessions];
    this.currentSessionId = ns.id;
    localStorage.setItem(`genomics_active_session_id_${pid}`, ns.id);
    localStorage.setItem("genomics_active_session_id", ns.id);
    saveSessionsToLocalStorage(this.sessions);
  }

  delete(id: string, selectedSample: GenomeSample | null, selectedModel: string, models: string[], manifestPacks: any[]) {
    this.sessions = this.sessions.filter(s => s.id !== id);
    saveSessionsToLocalStorage(this.sessions);
    if (this.currentSessionId === id) {
      const pid = selectedSample ? selectedSample.id : null;
      const ps = this.sessions.filter(s => s.sampleId === pid);
      if (ps.length > 0) {
        this.currentSessionId = ps[0].id;
        localStorage.setItem(`genomics_active_session_id_${pid}`, ps[0].id);
        localStorage.setItem("genomics_active_session_id", ps[0].id);
      } else {
        this.startNew(selectedSample, selectedModel, models, manifestPacks);
      }
    }
  }

  saveTitle() {
    saveSessionsToLocalStorage(this.sessions);
  }
}
