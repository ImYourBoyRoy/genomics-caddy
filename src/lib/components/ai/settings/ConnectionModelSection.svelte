<!-- ./src/lib/components/ai/settings/ConnectionModelSection.svelte -->
<script lang="ts">
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
            <select id="review-model-selector" bind:value={reviewModel} class="w-full" style="background: rgba(0, 0, 0, 0.3); border: 1px solid var(--border-color); color: var(--text-primary); padding: 6px; border-radius: 4px; font-size: 0.78rem;">
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
</style>
