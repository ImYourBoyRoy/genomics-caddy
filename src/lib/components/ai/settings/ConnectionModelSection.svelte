<!-- ./src/lib/components/ai/settings/ConnectionModelSection.svelte -->
<script lang="ts">
  import { onMount } from "svelte";
  import {
    discoverOllamaModels,
    getActiveOllamaModels,
    type InferenceHostProfile,
    type OllamaModelInsight,
  } from "../../../api/tauri";
  import { isReasoningModel, isModelVisionCapable } from "../../../utils/aiPrompt";
  import { OLLAMA_URL_PLACEHOLDER, ollamaRawDataDisclosure } from "../../../utils/ollamaSettings";

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

  let activeModels = $state<any[]>([]);
  let activeModelsError = $state<string>("");
  let isPollingActive = $state<boolean>(false);
  let host = $state<InferenceHostProfile | null>(null);
  let insights = $state<OllamaModelInsight[]>([]);

  let totalVramBytes = $derived(
    activeModels.reduce((sum, m) => sum + (m.size_vram || 0), 0)
  );
  let totalVramGb = $derived((totalVramBytes / 1e9).toFixed(1));
  let capacityBytes = $derived(
    host?.accel_bytes ||
      (host?.unified_memory ? host.system_ram_bytes : null) ||
      (totalVramBytes > 0 ? totalVramBytes : null)
  );
  let capacityLabel = $derived(
    capacityBytes
      ? `${(capacityBytes / 1e9).toFixed(1)} GiB ${host?.unified_memory ? "unified/shared" : "observed accel"}`
      : "capacity unknown — load a model to observe"
  );
  let barPct = $derived(
    capacityBytes && capacityBytes > 0
      ? Math.min(100, (totalVramBytes / capacityBytes) * 100)
      : totalVramBytes > 0
        ? 35
        : 0
  );

  function hintFor(name: string): string {
    return insights.find((m) => m.name === name)?.load_hint || "";
  }

  async function refreshActiveModels() {
    if (!ollamaUrl.trim()) return;
    isPollingActive = true;
    try {
      const [res, report] = await Promise.all([
        getActiveOllamaModels(ollamaUrl, ollamaToken || undefined),
        discoverOllamaModels(ollamaUrl, ollamaToken || undefined).catch(() => null),
      ]);
      activeModels = res.models || [];
      activeModelsError = "";
      if (report) {
        host = report.host;
        insights = report.models;
      }
    } catch (e: any) {
      activeModelsError = e.message || String(e);
      activeModels = [];
    } finally {
      isPollingActive = false;
    }
  }

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
    <p style="font-size: 0.72rem; color: var(--text-secondary); margin: 0 0 8px; line-height: 1.4;">
      Prefer <strong>Advanced → Connections</strong> for full Ollama/Qdrant setup, host posture, and GPU vs CPU model ranking.
    </p>
    <div class="input-row">
      <label for="ollama-url">API Endpoint URL</label>
      <input id="ollama-url" type="text" bind:value={ollamaUrl} placeholder={OLLAMA_URL_PLACEHOLDER} />
    </div>
    <div class="input-row">
      <label for="ollama-token">Auth Token / Bearer (Optional)</label>
      <input id="ollama-token" type="password" bind:value={ollamaToken} placeholder="Bearer ..." />
    </div>
    <p class="raw-data-disclosure" role="note">🔒 {ollamaRawDataDisclosure(ollamaUrl)} {#if ollamaUrl.trim()}<span>Endpoint: <code>{ollamaUrl.trim()}</code></span>{/if}</p>
    <button class="btn btn-secondary w-full" onclick={scanModels} disabled={isScanning || !ollamaUrl.trim()}>
      {isScanning ? "Scanning..." : "🔌 Connect & Poll Models"}
    </button>

    {#if scanError}
      <div class="error-msg">{scanError}</div>
    {/if}

    {#if host}
      <div class="telemetry-box mt-2">
        <div class="telemetry-header">
          <span class="telemetry-title">Host posture: {host.posture}</span>
        </div>
        <div class="telemetry-empty" style="flex-direction: column; align-items: flex-start; gap: 4px;">
          <span>backends: {host.accel_backends.join(", ") || "none"} · unified={host.unified_memory ? "yes" : "no"}</span>
          {#each host.notes.slice(0, 3) as note}
            <span style="opacity: 0.85;">{note}</span>
          {/each}
        </div>
      </div>
    {/if}

    {#if models.length > 0}
      <div class="input-row mt-2">
        <label for="model-selector">Primary LLM Model</label>
        <select id="model-selector" bind:value={selectedModel} class="w-full">
          {#each models as m}
            <option value={m}>
              {m}
              {hintFor(m) ? ` · ${hintFor(m)}` : ""}
              {isReasoningModel(m) ? " 🧠" : ""}{isModelVisionCapable(m, null) ? " 👁️" : ""}
            </option>
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
            {#if isReasoningModel(selectedModel)}
              <span class="capability-badge reasoning-badge">🧠 Reasoning</span>
            {/if}
            {#if hintFor(selectedModel)}
              <span class="capability-badge context-badge">{hintFor(selectedModel)}</span>
            {/if}
          </div>
        {/if}
      </div>

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
                <option value={m}>{m} {hintFor(m) ? `· ${hintFor(m)}` : ""} {isReasoningModel(m) ? "🧠" : ""}{isModelVisionCapable(m, null) ? " 👁️" : ""}</option>
              {/each}
            </select>
            <p style="font-size: 0.68rem; color: var(--text-secondary); line-height: 1.3; margin: 0;">
              Prefer two models with on_gpu / likely_gpu / fits_unified hints.
            </p>
          </div>
        {/if}
      </div>

      <div class="telemetry-box mt-2">
        <div class="telemetry-header">
          <span class="telemetry-title">VRAM &amp; Active Models Telemetry</span>
          <button class="btn-refresh" onclick={refreshActiveModels} disabled={isPollingActive} type="button" aria-label="Refresh active models">
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
              <span class="vram-total font-mono font-bold" style="color: var(--accent);">{totalVramGb} GB in use</span>
              <span class="vram-capacity font-mono">/ {capacityLabel}</span>
            </div>
            <div class="vram-bar-bg">
              <div class="vram-bar-fill" style="width: {barPct}%"></div>
            </div>
            <div class="active-models-list mt-1">
              {#each activeModels as m}
                <div class="active-model-item">
                  <span class="active-model-name font-mono">{m.name}</span>
                  <span class="active-model-size font-mono">
                    {m.size_vram > 0 ? `${(m.size_vram / 1e9).toFixed(1)} GB VRAM` : "CPU / system RAM"}
                  </span>
                </div>
              {/each}
            </div>
          </div>
        {/if}
      </div>

      <details class="guidance-details mt-2">
        <summary class="guidance-summary">About GPU vs CPU model placement</summary>
        <div class="guidance-content">
          <p class="guidance-text">
            Load hints come from live Ollama telemetry and host discovery (CUDA/ROCm/Metal, Apple Silicon unified memory,
            AMD AI-class APU signals) — not a fixed GPU model list. If size_vram is 0 while loaded, it is on the CPU path.
          </p>
          {#if twoModelReview}
            <p class="guidance-text warning-text mt-1">
              Two-model review needs accelerator headroom. Prefer likely_gpu / fits_unified / on_gpu models.
            </p>
          {/if}
        </div>
      </details>
    {/if}
  </div>
</details>

<style>
  .raw-data-disclosure {
    margin: 0.45rem 0 0.65rem;
    color: var(--text-secondary);
    font-size: 0.72rem;
    line-height: 1.45;
  }

  .raw-data-disclosure span {
    display: block;
    margin-top: 0.2rem;
    overflow-wrap: anywhere;
  }

  .raw-data-disclosure code {
    color: var(--text-primary);
    font-size: 0.68rem;
  }

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
  .reasoning-badge {
    background: rgba(139, 92, 246, 0.15);
    color: #a78bfa;
    border: 1px solid rgba(139, 92, 246, 0.3);
  }

  .guidance-details {
    background: rgba(59, 130, 246, 0.02);
    border: 1px solid rgba(59, 130, 246, 0.15);
    border-radius: 6px;
    overflow: hidden;
  }
  .guidance-summary {
    font-size: 0.72rem;
    font-weight: 600;
    color: #60a5fa;
    cursor: pointer;
    padding: 6px 8px;
    user-select: none;
    list-style: none;
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: rgba(0, 0, 0, 0.1);
  }
  .guidance-summary::-webkit-details-marker {
    display: none;
  }
  .guidance-summary::after {
    content: "▶";
    font-size: 0.55rem;
    transition: transform 0.2s;
    opacity: 0.7;
  }
  .guidance-details[open] .guidance-summary::after {
    transform: rotate(90deg);
  }
  .guidance-content {
    padding: 8px 10px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    border-top: 1px solid rgba(59, 130, 246, 0.1);
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
