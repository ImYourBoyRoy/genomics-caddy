// ./src/lib/utils/pageSampleHandlers.ts
/**
 * Sample import, deletion, and report generation handlers for the main dashboard page.
 * Pure async functions that accept dependencies via params for testability.
 */

import {
  importGenome as apiImportGenome,
  getSamples,
  generateReport,
  deleteSample as apiDeleteSample,
  queryRsids,
  queryRegion,
} from "../api/tauri";
import type { GenomeSample, GeneratedReport, NormalizedReport, DbSnpRecord } from "../types/genomics";
import { mergedReportTemplateJson } from "./reportTemplate";

export interface ImportGenomeParams {
  filePath: string;
  sampleNameInput: string;
  onState: (patch: {
    isImporting?: boolean;
    importError?: string;
    importSuccess?: string;
    progressPercent?: number;
    progressStatus?: string;
    filePath?: string;
    sampleNameInput?: string;
  }) => void;
  refreshSamples: () => Promise<GenomeSample[]>;
  selectSample: (sample: GenomeSample) => void;
}

export async function runImportGenome(
  e: Event,
  { filePath, sampleNameInput, onState, refreshSamples, selectSample }: ImportGenomeParams,
): Promise<void> {
  e.preventDefault();
  if (!filePath || !sampleNameInput) {
    onState({ importError: "Please specify both file path and sample name." });
    return;
  }
  onState({
    isImporting: true,
    importError: "",
    importSuccess: "",
    progressPercent: 0,
    progressStatus: "Initializing ingestion...",
  });

  try {
    const sampleId = await apiImportGenome(filePath, sampleNameInput);
    onState({ importSuccess: `Successfully imported sample as ID: ${sampleId}!`, filePath: "", sampleNameInput: "" });
    const samples = await refreshSamples();
    const newSample = samples.find((s) => s.id === sampleId);
    if (newSample) selectSample(newSample);
  } catch (err: unknown) {
    onState({ importError: String(err) });
  } finally {
    onState({ isImporting: false });
  }
}

export interface WarmReportParams {
  sampleId: number;
  onState: (patch: {
    isGeneratingReport?: boolean;
    reportError?: string;
    generatedReport?: GeneratedReport | null;
    rawReport?: NormalizedReport | null;
  }) => void;
}

export async function warmReport({ sampleId, onState }: WarmReportParams): Promise<void> {
  onState({ isGeneratingReport: true, reportError: "" });
  try {
    const payload = await generateReport(sampleId, mergedReportTemplateJson());
    onState({ generatedReport: payload.report, rawReport: payload.raw });
  } catch (err: unknown) {
    onState({ reportError: "Report generation failed: " + String(err) });
    console.error(err);
  } finally {
    onState({ isGeneratingReport: false });
  }
}

export interface TriggerReportParams {
  selectedSample: GenomeSample | null;
  generatedReport: GeneratedReport | null;
  warmReportFn: (sampleId: number) => Promise<void>;
}

export async function triggerReport({
  selectedSample,
  generatedReport,
  warmReportFn,
}: TriggerReportParams): Promise<void> {
  if (!selectedSample) return;
  if (generatedReport) return;
  await warmReportFn(selectedSample.id);
}

export interface DeleteSampleParams {
  id: number;
  selectedSample: GenomeSample | null;
  confirm: (message: string, onConfirm: () => void | Promise<void>, title?: string) => void;
  alert: (message: string) => void;
  onState: (patch: {
    selectedSample?: GenomeSample | null;
    generatedReport?: GeneratedReport | null;
  }) => void;
  refreshSamples: () => Promise<void>;
}

export async function deleteSampleWithConfirm({
  id,
  selectedSample,
  confirm,
  alert,
  onState,
  refreshSamples,
}: DeleteSampleParams): Promise<void> {
  confirm(
    "Are you sure you want to delete this sample and all its genotypes?",
    async () => {
      try {
        await apiDeleteSample(id);
        if (selectedSample && selectedSample.id === id) {
          onState({ selectedSample: null, generatedReport: null });
        }
        await refreshSamples();
      } catch (err: unknown) {
        alert("Delete failed: " + String(err));
      }
    },
    "Delete Sample",
  );
}

export async function searchVariants(
  e: Event,
  {
    selectedSample,
    searchRsid,
    browseChr,
    browseStart,
    browseEnd,
    onState,
    alert,
  }: {
    selectedSample: GenomeSample | null;
    searchRsid: string;
    browseChr: string;
    browseStart: number;
    browseEnd: number;
    onState: (patch: { isBrowsing?: boolean; browserResults?: DbSnpRecord[] }) => void;
    alert: (message: string) => void;
  },
): Promise<void> {
  e.preventDefault();
  if (!selectedSample) {
    alert("Import or select a genome profile before searching.");
    return;
  }
  onState({ isBrowsing: true });
  try {
    if (searchRsid.trim()) {
      const results = await queryRsids(selectedSample.id, [searchRsid.trim()]);
      onState({ browserResults: results });
    } else {
      const results = await queryRegion(
        selectedSample.id,
        browseChr,
        Number(browseStart),
        Number(browseEnd),
      );
      onState({ browserResults: results });
    }
  } catch (err: unknown) {
    alert("Search failed: " + String(err));
  } finally {
    onState({ isBrowsing: false });
  }
}

export async function fetchSamples(): Promise<GenomeSample[]> {
  return getSamples();
}

export function computeReportMarkerCounts(generatedReport: GeneratedReport | null): {
  totalMarkersChecked: number;
  foundMarkersCount: number;
} {
  if (!generatedReport?.sections) {
    return { totalMarkersChecked: 0, foundMarkersCount: 0 };
  }
  let totalMarkersChecked = 0;
  let foundMarkersCount = 0;
  for (const sec of generatedReport.sections) {
    if (!sec.markers) continue;
    totalMarkersChecked += sec.markers.length;
    foundMarkersCount += sec.markers.filter(
      (m) => m.user_genotype !== "--" && !m.user_genotype.includes("-"),
    ).length;
  }
  return { totalMarkersChecked, foundMarkersCount };
}

export async function browseGenomeFile(
  selectFileFn: typeof import("../api/tauri").selectFile,
  onState: (patch: { filePath?: string; sampleNameInput?: string }) => void,
): Promise<void> {
  try {
    const selected = await selectFileFn();
    if (selected) {
      const parts = selected.split(/[\\/]/);
      const fileName = parts[parts.length - 1];
      onState({
        filePath: selected,
        sampleNameInput: fileName.replace(/\.[^/.]+$/, ""),
      });
    }
  } catch (e) {
    console.error("File selector error", e);
  }
}

export async function downloadReferenceChain(
  downloadFn: () => Promise<void>,
  checkFn: () => Promise<boolean>,
  onState: (patch: { isDownloadingChain?: boolean; isChainDownloaded?: boolean }) => void,
  alert: (message: string) => void,
): Promise<void> {
  onState({ isDownloadingChain: true });
  try {
    await downloadFn();
    onState({ isChainDownloaded: await checkFn() });
  } catch (err: unknown) {
    alert("Failed to download chain file: " + String(err));
  } finally {
    onState({ isDownloadingChain: false });
  }
}
