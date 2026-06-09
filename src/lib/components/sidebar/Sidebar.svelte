<!-- ./src/lib/components/sidebar/Sidebar.svelte -->
<script lang="ts">
  import GenomeImportPanel from '../import/GenomeImportPanel.svelte';
  import SampleList from '../samples/SampleList.svelte';
  import type { GenomeSample } from '../../types/genomics';

  /*
  Module Docstring:
  Purpose: Sidebar panel container aggregating assembly settings, file import, and profile lists.
  Responsibilities:
  - Render branding header.
  - Render Liftover assembly status and download button.
  - Mount GenomeImportPanel and SampleList components, forwarding state and callbacks.
  Key Inputs: state variables and callbacks.
  Key Outputs: Structured sidebar component.
  Operational Notes: Uses glassmorphic backdrop filter styling.
  */

  interface Props {
    isChainDownloaded: boolean;
    isDownloadingChain: boolean;
    filePath: string;
    sampleNameInput: string;
    isImporting: boolean;
    progressPercent: number;
    progressStatus: string;
    importError: string;
    importSuccess: string;
    samples: GenomeSample[];
    selectedSample: GenomeSample | null;
    onDownloadChain: () => void;
    onBrowseFile: () => void;
    onImportGenome: (e: Event) => void;
    onSelectSample: (sample: GenomeSample) => void;
    onDeleteSample: (id: number) => void;
  }

  let {
    isChainDownloaded,
    isDownloadingChain,
    filePath = $bindable(),
    sampleNameInput = $bindable(),
    isImporting,
    progressPercent,
    progressStatus,
    importError,
    importSuccess,
    samples,
    selectedSample,
    onDownloadChain,
    onBrowseFile,
    onImportGenome,
    onSelectSample,
    onDeleteSample
  }: Props = $props();
</script>

<aside class="sidebar">
  <div class="brand">
    <img src="/logo.png" alt="Genomics Caddy Logo" class="brand-logo" />
    <h2>Genomics Caddy</h2>
  </div>

  <!-- Chain Status Indicator -->
  <div class="chain-status-card card">
    <h4>Liftover Assembly</h4>
    {#if isChainDownloaded}
      <div class="badge success">🟢 GRCh38 Active</div>
    {:else}
      <div class="badge warning">⚠️ GRCh37 Only</div>
      <p class="card-hint">Liftover chain file is missing. Import will not map to GRCh38 coordinates.</p>
      <button class="btn btn-primary btn-sm" onclick={onDownloadChain} disabled={isDownloadingChain}>
        {isDownloadingChain ? "Downloading..." : "Download Chain"}
      </button>
    {/if}
  </div>

  <!-- Import DNA Form -->
  <GenomeImportPanel
    bind:filePath
    bind:sampleNameInput
    {isImporting}
    {progressPercent}
    {progressStatus}
    {importError}
    {importSuccess}
    {onBrowseFile}
    {onImportGenome}
  />

  <!-- Active Profiles -->
  <SampleList
    {samples}
    {selectedSample}
    {onSelectSample}
    {onDeleteSample}
  />
</aside>
