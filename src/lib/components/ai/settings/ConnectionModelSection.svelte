<!-- ./src/lib/components/ai/settings/ConnectionModelSection.svelte -->
<script lang="ts">
  import { onMount } from "svelte";
  import { getActiveOllamaModels } from "../../../api/tauri";

  interface Props {
    ollamaUrl: string;
    ollamaToken: string;
    selectedModel: string;
    models: string[];
    isScanning: boolean;
    scanError: string;
    scanModels: () => void;
    modelDetails: any;
    isVisionCapable: boolean;
    contextWindow: number;
    reviewModel: string;
    twoModelReview: boolean;
  }

  let {
    ollamaUrl = $bindable(),
    ollamaToken = $bindable(),
    selectedModel = $bindable(),
    models,
    isScanning,
    scanError,
    scanModels,
    modelDetails,
    isVisionCapable,
    contextWindow,
    reviewModel = $bindable(),
    twoModelReview = $bindable()
  }: Props = $props();

  // Telemetry state
  let activeModels = $state<any[]>([]);
  let activeModelsError = $state<string>("");
  let isPollingActive = $state<boolean>(false);
  let totalVramBytes = $derived(
    activeModels.reduce((sum, m) => sum + (m.size_vram || 0), 0)
  );
  let totalVramGb = $derived((totalVramBytes / 1e9).toFixed(1));

  async function refreshActiveModels() {
    if (!ollamaUrl) return;
    isPollingActive = true;
    try {
      const res = await getActiveOllamaModels(ollamaUrl, ollamaToken || undefined);
      activeModels = res.models || [];
      activeModelsError = "";
    } catch (e: any) {
      activeModelsError = e.message || String(e);
      activeModels = [];
    } finally {
      isPollingActive = false;
    }
  }

  // Reactively poll whenever the url/token is updated or scanned
  $effect(() => {
    if (ollamaUrl) {
      refreshActiveModels();
    }
  });

  onMount(() => {
    const interval = setInterval(refreshActiveModels, 15000);
    return () => clearInterval(interval);
  });
</script>

<details class="settings-details-group" open>
  <summary class="settings-details-summary">🔌 Connection &amp; Model</summary>
  <div class="settings-details-content">
    <div class="input-row">
      <label for="ollama-url">API Endpoint URL</label>
      <input id="ollama-url" type="text" bind:value={ollamaUrl} placeholder="http://localhost:11434" />
    </div>
    <div class="input-row">
      <label for="ollama-token">Auth Token / Bearer (Optional)</label>
      <input id="ollama-token" type="password" bind:value={ollamaToken} placeholder="Bearer ..." />
    </div>
    <button class="btn btn-secondary w-full" onclick={scanModels} disabled={isScanning}>
      {isScanning ? "Scanning..." : "🔌 Connect & Poll Models"}
    </button>

    {#if scanError}
      <div class="error-msg">{scanError}</div>
    {/if}

    {#if models.length > 0}
      <div class="input-row mt-2">
        <label for="model-selector">Primary LLM Model</label>
        <select id="model-selector" bind:value={selectedModel} class="w-full">
          {#each models as m}
            <option value={m}>{m}</option>
          {/each}
        </select>
        
        {#if modelDetails}
          <div class="model-capabilities-row mt-1">
            {#if isVisionCapable}
              <span class="capability-badge vision-badge">👁️ Multimodal (Vision)</span>
            {:else}
              <span class="capability-badge text-badge">✍️ Text-Only</span>
            {/if}
            <span class="capability-badge context-badge">📏 {contextWindow.toLocaleString()} context</span>
          </div>
        {/if}
      </div>

      <!-- Two-model Review Toggle -->
      <div style="margin-top: 10px; padding-top: 10px; border-top: 1px dashed rgba(255, 255, 255, 0.08); display: flex; flex-direction: column; gap: 8px;">
        <label style="display: flex; align-items: center; gap: 8px; cursor: pointer; font-size: 0.78rem;">
          <input type="checkbox" bind:checked={twoModelReview} style="cursor: pointer;" />
          <span class="highlight-text font-bold" style="color: var(--text-primary);">Two-Model Safety Review</span>
        </label>
        
        {#if twoModelReview}
          <div class="input-row">
            <label for="review-model-selector">Secondary Review Model</label>
            <select id="review-model-selector" bind:value={reviewModel} class="w-full">
              {#each models as m}
                <option value={m}>{m}</option>
              {/each}
            </select>
            <p style="font-size: 0.68rem; color: var(--text-secondary); line-height: 1.3; margin: 0;">
              🤖 Automatically cross-checks primary model drafts for clinical claims, dosages, or diagnoses.
            </p>
          </div>
        {/if}
      </div>

      <!-- Telemetry and VRAM advice -->
      <div class="telemetry-box mt-2">
        <div class="telemetry-header">
          <span class="telemetry-title">⚡ VRAM &amp; Active Models Telemetry</span>
          <button class="btn-refresh" onclick={refreshActiveModels} disabled={isPollingActive} type="button" title="Refresh active models">
            {isPollingActive ? "..." : "🔄"}
          </button>
        </div>
        
        {#if activeModelsError}
          <div class="telemetry-error font-mono">Ollama telemetry offline</div>
        {:else if activeModels.length === 0}
          <div class="telemetry-empty">
            <span class="status-indicator status-idle"></span>
            No models loaded in VRAM (Ollama is idle)
          </div>
        {:else}
          <div class="telemetry-stats">
            <div style="display: flex; justify-content: space-between; align-items: center; font-size: 0.7rem; color: var(--text-secondary);">
              <span class="vram-total font-mono font-bold" style="color: var(--accent);">{totalVramGb} GB VRAM used</span>
              <span class="vram-capacity font-mono">/ 48 GB (Dual P40 Capacity)</span>
            </div>
            <div class="vram-bar-bg">
              <div class="vram-bar-fill" style="width: {Math.min(100, (Number(totalVramGb) / 48.0) * 100)}%"></div>
            </div>
            <div class="active-models-list mt-1">
              {#each activeModels as m}
                <div class="active-model-item">
                  <span class="active-model-name font-mono">{m.name}</span>
                  <span class="active-model-size font-mono">{(m.size_vram / 1e9).toFixed(1)} GB VRAM</span>
                </div>
              {/each}
            </div>
          </div>
        {/if}
      </div>

      <!-- Educational Guidance Note -->
      <div class="telemetry-guidance mt-2">
        <div class="guidance-title">💡 Standard vs. Reasoning Models</div>
        <p class="guidance-text">
          Standard instruction models (like <strong>MedGemma</strong>) stream answers directly and do not show a "thinking" phase. If you wish to see step-by-step reasoning streams, select a reasoning model (like <strong>DeepSeek-R1</strong> or R1-distilled models).
        </p>
        {#if twoModelReview}
          <p class="guidance-text warning-text mt-1">
            ⚠️ <strong>VRAM Warning:</strong> Running primary and safety review models concurrently requires sufficient VRAM. If Ollama crashes with <em>HTTP 500</em>, reduce model sizes or context windows.
          </p>
        {/if}
      </div>
    {/if}
  </div>
</details>

<style>
  .input-row {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .input-row label {
    font-size: 0.75rem;
    color: var(--text-secondary);
  }

  .input-row input, .input-row select {
    background: rgba(0, 0, 0, 0.2);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    padding: 8px 12px;
    border-radius: 6px;
    font-size: 0.85rem;
  }

  .input-row input:focus, .input-row select:focus {
    outline: none;
    border-color: var(--accent);
  }

  .error-msg {
    color: var(--danger);
    font-size: 0.75rem;
    line-height: 1.3;
    padding: 6px;
    background: rgba(239, 68, 68, 0.1);
    border-radius: 4px;
    border: 1px solid rgba(239, 68, 68, 0.2);
    white-space: pre-line;
  }

  .model-capabilities-row {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    margin-top: 4px;
  }
  .capability-badge {
    font-size: 0.68rem;
    padding: 2px 6px;
    border-radius: 4px;
    font-weight: 500;
  }
  .vision-badge {
    background: rgba(16, 185, 129, 0.15);
    color: #34d399;
    border: 1px solid rgba(16, 185, 129, 0.3);
  }
  .text-badge {
    background: rgba(107, 114, 128, 0.15);
    color: #9ca3af;
    border: 1px solid rgba(107, 114, 128, 0.3);
  }
  .context-badge {
    background: rgba(245, 158, 11, 0.15);
    color: #fbbf24;
    border: 1px solid rgba(245, 158, 11, 0.3);
  }

  .w-full { width: 100%; }
  .mt-1 { margin-top: 4px; }
  .mt-2 { margin-top: 8px; }

  /* Telemetry Styles */
  .telemetry-box {
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    padding: 10px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .telemetry-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .telemetry-title {
    font-size: 0.72rem;
    font-weight: 600;
    color: var(--text-secondary);
  }
  .btn-refresh {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 0.75rem;
    padding: 0 4px;
    transition: opacity 0.2s;
  }
  .btn-refresh:hover {
    opacity: 0.8;
  }
  .telemetry-error {
    font-size: 0.68rem;
    color: var(--danger);
  }
  .telemetry-empty {
    font-size: 0.68rem;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .status-indicator {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    display: inline-block;
  }
  .status-idle {
    background: #3b82f6;
    box-shadow: 0 0 6px #3b82f6;
  }
  .telemetry-stats {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .vram-bar-bg {
    background: rgba(255, 255, 255, 0.05);
    height: 6px;
    border-radius: 3px;
    overflow: hidden;
    width: 100%;
  }
  .vram-bar-fill {
    background: linear-gradient(90deg, var(--accent) 0%, #a855f7 100%);
    height: 100%;
    border-radius: 3px;
    transition: width 0.3s ease;
  }
  .active-models-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .active-model-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.68rem;
    background: rgba(0, 0, 0, 0.15);
    padding: 4px 8px;
    border-radius: 4px;
  }
  .active-model-name {
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 150px;
  }
  .active-model-size {
    color: var(--text-secondary);
  }
  .telemetry-guidance {
    background: rgba(59, 130, 246, 0.04);
    border: 1px solid rgba(59, 130, 246, 0.15);
    border-radius: 6px;
    padding: 8px 10px;
  }
  .guidance-title {
    font-size: 0.72rem;
    font-weight: 600;
    color: #60a5fa;
    margin-bottom: 4px;
  }
  .guidance-text {
    font-size: 0.68rem;
    color: var(--text-secondary);
    line-height: 1.35;
    margin: 0;
  }
  .warning-text {
    color: #fb7185;
  }
  .font-mono {
    font-family: monospace;
  }
  .font-bold {
    font-weight: 700;
  }
</style>
