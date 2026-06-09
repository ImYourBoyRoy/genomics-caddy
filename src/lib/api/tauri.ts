// ./src/lib/api/tauri.ts
/*
Module Docstring:
Purpose: Tauri command invocation wrapper API layer.
Responsibilities:
- Encapsulate Tauri invoke calls into typed asynchronous functions.
- Handle type conversions between TypeScript and Tauri Rust payloads.
Key Inputs: Functional parameters matching Tauri commands.
Key Outputs: Strongly typed promises wrapping genomic and app state.
Operational Notes: Exposes clean promise interfaces for Svelte components.
*/

import { invoke } from "@tauri-apps/api/core";
import type { AppPaths, GenomeSample, DbSnpRecord, GeneratedReport } from "../types/genomics";

export async function selectFile(): Promise<string | null> {
  return invoke<string | null>("select_file");
}

export async function saveReportJson(content: string, defaultFilename: string): Promise<boolean> {
  return invoke<boolean>("save_report_json", { content, defaultFilename });
}

export async function getAppPaths(): Promise<AppPaths> {
  return invoke<AppPaths>("get_app_paths");
}

export async function importGenome(filePath: string, sampleName: string): Promise<number> {
  return invoke<number>("import_genome", { filePath, sampleName });
}

export async function getSamples(): Promise<GenomeSample[]> {
  return invoke<GenomeSample[]>("get_samples");
}

export async function queryRsids(sampleId: number, rsids: string[]): Promise<DbSnpRecord[]> {
  return invoke<DbSnpRecord[]>("query_rsids", { sampleId, rsids });
}

export async function queryRegion(
  sampleId: number,
  chromosome: string,
  start: number,
  end: number
): Promise<DbSnpRecord[]> {
  return invoke<DbSnpRecord[]>("query_region", { sampleId, chromosome, start, end });
}

export async function generateReport(sampleId: number, templateJson: string): Promise<GeneratedReport> {
  return invoke<GeneratedReport>("generate_report", { sampleId, templateJson });
}

export async function deleteSample(sampleId: number): Promise<void> {
  return invoke<void>("delete_sample", { sampleId });
}

export async function checkChainStatus(): Promise<boolean> {
  return invoke<boolean>("check_chain_status");
}

export async function downloadChain(): Promise<void> {
  return invoke<void>("download_chain_file");
}

export async function scanOllamaModels(url: string, token: string | undefined): Promise<string[]> {
  return invoke<string[]>("scan_ollama_models", { url, token });
}

export async function streamOllamaChat(
  url: string,
  token: string | undefined,
  model: string,
  messages: any[],
  temperature?: number,
  numPredict?: number
): Promise<void> {
  return invoke<void>("stream_ollama_chat", { url, token, model, messages, temperature, numPredict });
}

export async function showOllamaModel(
  url: string,
  token: string | undefined,
  name: string
): Promise<any> {
  return invoke<any>("show_ollama_model", { url, token, name });
}

export async function getCurrentExe(): Promise<string> {
  return invoke<string>("get_current_exe");
}

export async function getMcpTools(): Promise<any[]> {
  return invoke<any[]>("get_mcp_tools");
}

export interface EvidenceRecord {
  rsid: string;
  gene: string;
  evidence_text: string;
  source_citation: string;
  has_embedding: boolean;
  similarity: number | null;
}

export async function listEvidenceSources(): Promise<string[]> {
  return invoke<string[]>("list_evidence_sources");
}

export async function getEvidenceForMarker(rsid: string): Promise<EvidenceRecord[]> {
  return invoke<EvidenceRecord[]>("get_evidence_for_marker", { rsid });
}

export async function searchEvidence(
  query: string,
  ollamaUrl?: string,
  ollamaToken?: string
): Promise<EvidenceRecord[]> {
  return invoke<EvidenceRecord[]>("search_evidence", {
    query,
    ollamaUrl: ollamaUrl || undefined,
    ollamaToken: ollamaToken || undefined,
  });
}

export async function getChatSessions(sampleId: number | null): Promise<any[]> {
  return invoke<any[]>("get_chat_sessions", { sampleId });
}

export async function saveChatSession(session: any): Promise<void> {
  return invoke<void>("save_chat_session", { session });
}

export async function deleteChatSession(sessionId: string): Promise<void> {
  return invoke<void>("delete_chat_session", { sessionId });
}
