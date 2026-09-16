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
  inspectGenome,
} from "../api/tauri";
import type { GenomeSample, GeneratedReport, NormalizedReport, DbSnpRecord, GenomeImportPreview } from "../types/genomics";
import { mergedReportTemplateJson } from "./reportTemplate";
import { clearProfileScopedSessionState, clearProfileScopedStorage } from "./profileContext";
import { isCallableGenotype } from "./genotype";
import type { ImportPhase } from "./importProgress";

export interface ImportGenomeParams {
  filePath: string;
  sampleNameInput: string;
  existingSamples: GenomeSample[];
  selectedSampleId?: number | null;
  onConfirmationRequired: (onConfirm: () => void | Promise<void>, replacingExisting: boolean) => void;
  onState: (patch: {
    isImportPreparing?: boolean;
    isImporting?: boolean;
    importError?: string;
    importSuccess?: string;
    progressPercent?: number;
    progressStatus?: string;
    importPhase?: ImportPhase;
    importPreview?: GenomeImportPreview | null;
    importProfileName?: string;
    filePath?: string;
    sampleNameInput?: string;
  }) => void;
  refreshSamples: () => Promise<GenomeSample[]>;
  selectSample: (sample: GenomeSample) => void | Promise<void>;
}

export async function runImportGenome(
  e: Event,
  { filePath, sampleNameInput, existingSamples, selectedSampleId, onConfirmationRequired, onState, refreshSamples, selectSample }: ImportGenomeParams,
): Promise<void> {
  e.preventDefault();
  const normalizedName = sampleNameInput.trim();
  if (!filePath || !normalizedName) {
    onState({ importError: "Please specify both file path and sample name." });
    return;
  }
  const existing = existingSamples.find((sample) => sample.name.trim().toLowerCase() === normalizedName.toLowerCase());

  onState({
    isImportPreparing: true,
    importError: "",
    importSuccess: "",
    importPhase: "preview",
    importPreview: null,
    importProfileName: normalizedName,
    progressPercent: 0,
    progressStatus: "Checking the DNA export before writing…",
  });

  let preview: GenomeImportPreview;
  try {
    preview = await inspectGenome(filePath);
  } catch (err: unknown) {
    onState({
      isImportPreparing: false,
      importPhase: "error",
      progressStatus: "Import preview failed — no data was written.",
      importError: `Could not preview the DNA export. No data was written: ${String(err)}`,
    });
    return;
  }
  onState({
    isImportPreparing: false,
    importPhase: "awaiting-confirmation",
    importPreview: preview,
    progressPercent: 0,
    progressStatus: "Preview ready. No profile data has been written.",
  });
  const startImport = async (replaceExistingSampleId?: number): Promise<void> => {
    onState({
      isImportPreparing: false,
      isImporting: true,
      importError: "",
      importSuccess: "",
      importPhase: "reading",
      progressPercent: 0,
      progressStatus: replaceExistingSampleId == null
        ? "Starting local DNA import…"
        : "Preparing profile replacement…",
    });

    try {
      const sampleId = await apiImportGenome(filePath, normalizedName, replaceExistingSampleId);
      onState({
        importPhase: "profile",
        progressPercent: 100,
        progressStatus: "DNA records committed — loading the imported profile…",
      });
      if (replaceExistingSampleId != null) {
        clearProfileScopedSessionState(replaceExistingSampleId, selectedSampleId === replaceExistingSampleId);
      }
      const samples = await refreshSamples();
      const newSample = samples.find((s) => s.id === sampleId);
      if (!newSample) {
        throw new Error("The import completed, but the new profile was not returned when the profile list refreshed.");
      }

      onState({
        importPhase: "report",
        progressPercent: 100,
        progressStatus: "Profile loaded — preparing the trait report…",
      });
      await selectSample(newSample);

      const action = replaceExistingSampleId == null ? "Successfully imported" : "Successfully replaced";
      onState({
        importPhase: "ready",
        progressPercent: 100,
        progressStatus: "Profile and report ready.",
        importSuccess: `${action} profile as ID: ${sampleId}.`,
        filePath: "",
        sampleNameInput: "",
      });
      // Leave the completed state visible long enough for the final checkpoint
      // to paint before the workspace returns.
      await new Promise<void>((resolve) => setTimeout(resolve, 700));
    } catch (err: unknown) {
      onState({
        importPhase: "error",
        progressStatus: "Import stopped safely — review the error and retry.",
        importError: String(err),
      });
    } finally {
      onState({ isImporting: false });
    }
  };

  onState({ progressStatus: "Preview ready. Review the results, then choose an action below." });
  onConfirmationRequired(() => startImport(existing?.id), existing !== undefined);
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

export interface ReloadReportParams {
  selectedSample: GenomeSample | null;
  warmReportFn: (sampleId: number) => Promise<void>;
}

/** Rebuild an existing report after resource data changes. */
export async function reloadReport({
  selectedSample,
  warmReportFn,
}: ReloadReportParams): Promise<void> {
  if (!selectedSample) return;
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
  refreshSamples: () => Promise<GenomeSample[]>;
  selectSample: (sample: GenomeSample) => void | Promise<void>;
  removeSampleFromList: (sampleId: number) => void;
}

export async function deleteSampleWithConfirm({
  id,
  selectedSample,
  confirm,
  alert,
  onState,
  refreshSamples,
  selectSample,
  removeSampleFromList,
}: DeleteSampleParams): Promise<void> {
  confirm(
    "Are you sure you want to delete this profile, its genotype database, derived findings, chat history, and all profile-scoped Context/Diary data from this device?",
    async () => {
      let deleteError: unknown = null;
      try {
        await apiDeleteSample(id);
      } catch (err: unknown) {
        deleteError = err;
      }

      let refreshedProfiles: GenomeSample[] | null = null;
      let refreshError: unknown = null;
      try {
        refreshedProfiles = await refreshSamples();
      } catch (err: unknown) {
        refreshError = err;
      }

      const profileStillListed = refreshedProfiles?.some((profile) => profile.id === id) ?? false;
      const profileWasRemoved = deleteError === null || (refreshedProfiles !== null && !profileStillListed);
      if (!profileWasRemoved) {
        const reason = deleteError === null
          ? 'The profile is still present in the active profile list.'
          : String(deleteError);
        alert(`Delete failed: ${reason}${refreshError ? ` Profile list refresh also failed: ${String(refreshError)}` : ''}`);
        return;
      }

      const warnings: string[] = [];
      removeSampleFromList(id);
      if (deleteError === null && profileStillListed) {
        warnings.push('The delete operation completed, but the refreshed list still contained this profile; it was removed from the current view.');
      }
      if (deleteError !== null) {
        warnings.push(`The profile is no longer active, but cleanup reported: ${String(deleteError)}`);
      }
      try {
        clearProfileScopedStorage(id, selectedSample?.id === id);
      } catch (err: unknown) {
        warnings.push(`Profile-specific local settings could not all be cleared: ${String(err)}`);
      }

      if (selectedSample?.id === id) {
        onState({ selectedSample: null, generatedReport: null });
        const nextProfile = refreshedProfiles?.find((profile) => profile.id !== id);
        if (nextProfile) {
          try {
            await selectSample(nextProfile);
          } catch (err: unknown) {
            warnings.push(`The next profile could not be opened automatically: ${String(err)}`);
          }
        }
      }

      if (refreshError !== null) {
        warnings.push(`The profile was deleted, but the profile list could not be refreshed: ${String(refreshError)}`);
      }
      if (warnings.length > 0) {
        alert(warnings.join('\n'));
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
      (m) => isCallableGenotype(m.user_genotype),
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
