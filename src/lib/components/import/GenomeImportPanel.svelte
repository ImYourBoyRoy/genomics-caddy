<!-- ./src/lib/components/import/GenomeImportPanel.svelte -->
<script lang="ts">
  /*
  Module Docstring:
  Purpose: DNA ingestion panel for selecting and processing raw genomic data.
  Responsibilities:
  - Render file path browser and text input for sample nickname.
  - Display progress bar and state notifications during file import.
  - Trigger callbacks for file browsing and genome importing.
  Key Inputs: filePath, sampleNameInput, isImporting, progressPercent, progressStatus, importError, importSuccess.
  Key Outputs: Ingestion form and progress indicators.
  Operational Notes: Form action calls onImportGenome.
  */

  interface Props {
    filePath: string;
    sampleNameInput: string;
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
  <h3>Ingest DNA Export</h3>
  <form onsubmit={onImportGenome}>
    <div class="form-group">
      <label for="file-path">File Path (.txt or .zip)</label>
      <div class="file-input-wrapper">
        <input id="file-path" type="text" placeholder="Select txt/zip file..." bind:value={filePath} readonly disabled={disabled} />
        <button type="button" class="btn btn-primary btn-sm" onclick={onBrowseFile} disabled={disabled}>Browse...</button>
      </div>
    </div>
    <div class="form-group">
      <label for="sample-name">Sample Nickname</label>
      <input id="sample-name" type="text" placeholder="My Genome" bind:value={sampleNameInput} disabled={disabled} />
    </div>
    <button type="submit" class="btn btn-accent btn-block" disabled={isImporting || disabled}>
      {isImporting ? "Processing..." : "Import Genome"}
    </button>

    {#if isImporting}
      <div class="progress-container">
        <div class="progress-bar">
          <div class="progress-fill" style="width: {progressPercent}%"></div>
        </div>
        <div class="progress-status">{progressPercent}% - {progressStatus}</div>
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
