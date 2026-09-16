<!-- ./src/lib/components/import/GenomeImportPanel.svelte -->
<script lang="ts">
  import Tooltip from '../common/Tooltip.svelte';

  /*
  Module Docstring:
  Purpose: DNA ingestion panel for selecting and processing raw genomic data.
  Responsibilities:
  - Render file path browser and text input for sample nickname.
  - Supports the validated TXT/CSV/TSV/ZIP import formats and pre-write preview.
  - Display progress bar and state notifications during file import.
  - Trigger callbacks for file browsing and genome importing.
  Key Inputs: filePath, sampleNameInput, isImportPreparing, isImporting, progressPercent, progressStatus, importError, importSuccess.
  Key Outputs: Ingestion form and progress indicators.
  Operational Notes: Form action calls onImportGenome.
  */

  interface Props {
    filePath: string;
    sampleNameInput: string;
    isImportPreparing?: boolean;
    isImporting: boolean;
    progressPercent: number;
    progressStatus: string;
    importError: string;
    importSuccess: string;
    disabled?: boolean;
    onBrowseFile: () => void;
    onImportGenome: (e: Event) => void;
  }

  let {
    filePath = $bindable(),
    sampleNameInput = $bindable(),
    isImportPreparing = false,
    isImporting,
    progressPercent,
    progressStatus,
    importError,
    importSuccess,
    disabled = false,
    onBrowseFile,
    onImportGenome
  }: Props = $props();
</script>

<div class="import-card card">
  <h3>Add DNA profile</h3>
  <p class="import-card-hint">Import a local .txt, .csv, .tsv, or .zip DNA export. Existing profile names require replacement confirmation; use a new name to keep both profiles.</p>
  <form onsubmit={onImportGenome}>
    <div class="form-group">
        <label for="file-path">DNA export</label>
      <div class="file-input-wrapper">
        <Tooltip
          label="Selected DNA file"
          description={filePath || 'No genome file selected yet.'}
          interactiveChildren={true}
          interactiveClickBehavior="dismiss"
        >
          <input id="file-path" type="text" placeholder="Select txt/zip file..." bind:value={filePath} readonly disabled={disabled || isImportPreparing || isImporting} aria-label={filePath || 'No genome file selected'} />
        </Tooltip>
        <button type="button" class="btn btn-primary btn-sm" onclick={onBrowseFile} disabled={disabled || isImportPreparing || isImporting}>Browse...</button>
      </div>
    </div>
    <div class="form-group">
      <label for="sample-name">Profile name</label>
      <input id="sample-name" type="text" placeholder="My Genome" bind:value={sampleNameInput} disabled={disabled || isImportPreparing || isImporting} />
    </div>
    <button type="submit" class="btn btn-accent btn-block" disabled={isImporting || isImportPreparing || disabled}>
      {isImportPreparing ? "Checking file…" : isImporting ? "Processing…" : "Import Genome"}
    </button>

    {#if isImporting || isImportPreparing}
      <div class="progress-container" role="status" aria-live="polite">
        <div class="progress-bar">
          <div class="progress-fill" style="width: {Math.max(progressPercent, isImportPreparing ? 8 : 0)}%"></div>
        </div>
        <div class="progress-status">
          <strong class="progress-phase">{isImportPreparing ? "Checking export" : "Importing DNA"}</strong>
          <span>{isImportPreparing ? progressStatus || "Checking the DNA export…" : `${progressPercent}% — ${progressStatus}`}</span>
        </div>
      </div>
    {/if}

    {#if importError}
      <div class="error-msg">{importError}</div>
    {/if}
    {#if importSuccess}
      <div class="success-msg">{importSuccess}</div>
    {/if}
  </form>
</div>
