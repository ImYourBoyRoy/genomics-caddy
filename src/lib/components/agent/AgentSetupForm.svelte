<!-- ./src/lib/components/agent/AgentSetupForm.svelte -->
<!--
Module Docstring:
Purpose: Advanced setup form for genomics research agent including dynamic target selectors.
Responsibilities:
- Configure standard Rsid/Gene/Topic search terms or Autonomous Discovery Scan.
- Delegate autonomous multi-variant scan logic to AutonomousScanSection.
- Manage selected models, API keys, and launch research submissions.
Key Inputs: Selected sample, scanned Ollama models, NCBI API keys.
Key Outputs: Submission events carrying target queries or checked discovered variants.
Operational Notes: Uses Svelte 5 runes. Fully typed. Strictly under 500 lines.
-->
<script lang="ts">
  import type { GenomeSample, GeneratedReport } from "../../types/genomics";
  import type { VariantEvidence } from "../../types/agent";
  import { dialogStore } from "../../utils/dialogState.svelte";
  import { formatGeneticSexLabel } from "../../utils/uiLabels";
  import AutonomousScanSection from "./AutonomousScanSection.svelte";

  interface Props {
    selectedSample: GenomeSample | null;
    generatedReport: GeneratedReport | null;
    models: string[];
    isScanning: boolean;
    ollamaUrl: string;
    ollamaToken: string;
    selectedModel: string;
    ncbiApiKey: string;
    queryType: "rsid" | "gene" | "topic" | "autonomous";
    queryText: string;
    onStartResearch: (options: {
      type: "rsid" | "gene" | "topic" | "autonomous";
      term: string;
      selectedVariants?: VariantEvidence[];
    }) => void;
  }

  let {
    selectedSample,
    generatedReport,
    models,
    isScanning,
    ollamaUrl,
    ollamaToken,
    selectedModel = $bindable(),
    ncbiApiKey = $bindable(),
    queryType = $bindable(),
    queryText = $bindable(),
    onStartResearch
  }: Props = $props();

  type DiscoveredVariant = VariantEvidence & { checked?: boolean };
  let discoveredVariants = $state<DiscoveredVariant[]>([]);
  let isScanningDb = $state(false);
  let scanProgressMsg = $state("");
  let currentProgress = $state(0);
  let totalToScan = $state(0);

  function handleFormSubmit(e: SubmitEvent) {
    e.preventDefault();
    if (!selectedSample) {
      dialogStore.alert("Please select a genome sample from the dashboard first.");
      return;
    }
    if (queryType === "autonomous") {
      const checkedVariants = discoveredVariants.filter(v => v.checked);
      if (checkedVariants.length === 0) {
        dialogStore.alert("Please select at least one discovered variant for analysis.");
        return;
      }
      onStartResearch({
        type: "autonomous",
        term: `Autonomous Discovery Scan (${checkedVariants.length} validated findings)`,
        selectedVariants: checkedVariants
      });
    } else {
      if (!queryText.trim()) return;
      onStartResearch({
        type: queryType,
        term: queryText.trim()
      });
    }
  }
</script>

<div class="setup-config card">
  <h3>🕵️ Configure Genomics Research Agent</h3>
  <p class="card-hint">
    Research single targets dynamically, or run an autonomous scan on the user's raw DNA sequence for co-occurring risk mapping.
  </p>

  <form onsubmit={handleFormSubmit} class="setup-form">
    <!-- Selected Profile Card with QC Badges -->
    {#if selectedSample}
      <div class="selected-sample-panel">
        <div class="sample-meta-row">
          <div class="sample-title">
            <span class="sample-icon">🧬</span>
            <div>
              <div class="sample-name">{selectedSample.name}</div>
              <div class="sample-date">Imported: {selectedSample.imported_at}</div>
            </div>
          </div>
          <span class="qc-badge {selectedSample.qc_status?.toLowerCase() || 'pass'}">
            QC: {selectedSample.qc_status || 'Pass'}
          </span>
        </div>

        <div class="qc-metrics-grid">
          <div class="metric-card">
            <span class="metric-label">Chromosome-call context</span>
            <span class="metric-value">{formatGeneticSexLabel(selectedSample.genetic_sex)}</span>
          </div>
          <div class="metric-card">
            <span class="metric-label">Call Rate</span>
            <span class="metric-value">
              {selectedSample.call_rate !== null && selectedSample.call_rate !== undefined
                ? `${(selectedSample.call_rate * 100).toFixed(2)}%`
                : 'N/A'}
            </span>
          </div>
          <div class="metric-card">
            <span class="metric-label">Ti/Tv Ratio</span>
            <span class="metric-value">
              {selectedSample.titv_ratio !== null && selectedSample.titv_ratio !== undefined
                ? selectedSample.titv_ratio.toFixed(2)
                : 'N/A'}
            </span>
          </div>
          <div class="metric-card">
            <span class="metric-label">Heterozygosity</span>
            <span class="metric-value">
              {selectedSample.heterozygosity_rate !== null && selectedSample.heterozygosity_rate !== undefined
                ? `${(selectedSample.heterozygosity_rate * 100).toFixed(2)}%`
                : 'N/A'}
            </span>
          </div>
        </div>
      </div>
    {:else}
      <div class="no-sample-warning">
        ⚠️ No genomic profile selected. Please select a profile from the dashboard first.
      </div>
    {/if}

    <!-- Target selector -->
    <div class="input-row">
      <label for="query-type-selector">Query Target Type</label>
      <select id="query-type-selector" bind:value={queryType}>
        <option value="autonomous">🚀 Autonomous Discovery Scan (Local Genome Scan)</option>
        <option value="rsid">rsID (Single Variant)</option>
        <option value="gene">Gene Symbol (e.g., MTHFR)</option>
        <option value="topic">Health Topic (RAG Semantic Search)</option>
      </select>
    </div>

    <!-- Conditional rendering based on mode -->
    {#if queryType === "autonomous"}
      <AutonomousScanSection
        {selectedSample}
        {ncbiApiKey}
        bind:discoveredVariants
        bind:isScanningDb
        bind:scanProgressMsg
        bind:currentProgress
        bind:totalToScan
      />
    {:else}
      <!-- Query Input -->
      <div class="input-row">
        <label for="query-input">Search Term</label>
        <input
          id="query-input"
          type="text"
          bind:value={queryText}
          placeholder={queryType === "rsid" ? "e.g. rs1801133" : queryType === "gene" ? "e.g. MTHFR" : "e.g. caffeine metabolism"}
          required
        />
      </div>
    {/if}

    <!-- Ollama Model selector -->
    <div class="input-row">
      <label for="agent-model-selector">Agent LLM Model</label>
      <select id="agent-model-selector" bind:value={selectedModel}>
        {#each models as m}
          <option value={m}>{m}</option>
        {/each}
      </select>
      {#if isScanning}
        <span class="scanning-text font-mono">Scanning Ollama...</span>
      {/if}
    </div>

    <!-- NCBI API Key -->
    <div class="input-row">
      <label for="ncbi-api-key">NCBI API Key (Optional)</label>
      <input
        id="ncbi-api-key"
        type="password"
        bind:value={ncbiApiKey}
        placeholder="e.g. 32-character key"
      />
      <p style="font-size: 0.65rem; color: var(--text-secondary); margin: 0; line-height: 1.35;">
        🔑 Speeds up queries to NCBI E-utilities.
      </p>
    </div>

    <button
      class="btn btn-primary w-full"
      type="submit"
      disabled={isScanning || isScanningDb || (queryType !== "autonomous" && !queryText.trim())}
    >
      🕵️ Start Agentic Research
    </button>
  </form>
</div>

<style>
  .setup-config {
    display: flex;
    flex-direction: column;
  }
  .setup-form {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .input-row {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .input-row label {
    font-size: 0.75rem;
    color: var(--text-secondary);
    font-weight: 500;
  }
  .input-row input, .input-row select {
    background: rgba(0, 0, 0, 0.2);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    padding: 10px 14px;
    border-radius: 6px;
    font-size: 0.88rem;
    width: 100%;
    box-sizing: border-box;
  }
  .input-row input:focus, .input-row select:focus {
    outline: none;
    border-color: var(--accent);
  }
  .w-full { width: 100%; }
  .scanning-text {
    font-size: 0.65rem;
    color: var(--accent);
    margin-top: 4px;
  }
  .font-mono { font-family: monospace; }

  /* QC Metrics & Selected Sample Panel */
  .selected-sample-panel {
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin-bottom: 4px;
  }
  .sample-meta-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
  }
  .sample-title {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-grow: 1;
  }
  .sample-icon {
    font-size: 1.35rem;
  }
  .sample-name {
    font-weight: 600;
    font-size: 0.9rem;
    color: var(--text-primary);
  }
  .sample-date {
    font-size: 0.72rem;
    color: var(--text-secondary);
  }
  .qc-badge {
    font-size: 0.65rem;
    font-weight: 700;
    padding: 2px 8px;
    border-radius: 10px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  .qc-badge.pass {
    background: rgba(16, 185, 129, 0.1);
    color: #10b981;
    border: 1px solid rgba(16, 185, 129, 0.25);
  }
  .qc-badge.warning {
    background: rgba(245, 158, 11, 0.1);
    color: #f59e0b;
    border: 1px solid rgba(245, 158, 11, 0.25);
  }
  .qc-badge.fail {
    background: rgba(239, 68, 68, 0.1);
    color: #ef4444;
    border: 1px solid rgba(239, 68, 68, 0.25);
  }
  .qc-metrics-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 6px;
  }
  @media (min-width: 480px) {
    .qc-metrics-grid {
      grid-template-columns: repeat(4, 1fr);
    }
  }
  .metric-card {
    background: rgba(0, 0, 0, 0.12);
    border: 1px solid rgba(255, 255, 255, 0.03);
    border-radius: 6px;
    padding: 6px;
    display: flex;
    flex-direction: column;
    align-items: center;
  }
  .metric-label {
    font-size: 0.6rem;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }
  .metric-value {
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--text-primary);
    font-family: monospace;
    margin-top: 1px;
  }
  .no-sample-warning {
    background: rgba(245, 158, 11, 0.06);
    color: #f59e0b;
    border: 1px solid rgba(245, 158, 11, 0.15);
    border-radius: 8px;
    padding: 10px;
    font-size: 0.78rem;
    text-align: center;
    margin-bottom: 4px;
  }
</style>
