<!-- ./src/lib/components/ai/settings/InferenceSettingsSection.svelte -->
<script lang="ts">
  import type { AiContextMode, ConsultationMode } from "../../../utils/aiPrompt";

  interface Props {
    temperature: number;
    maxTokens: number;
    extendedThinking: boolean;
    showThinkingProcess: boolean;
    autoCollapseThinking: boolean;
    includeTraceInExport: boolean;
    contextMode: AiContextMode;
    consultationMode: ConsultationMode;
    sessionPromptTokens: number;
    sessionResponseTokens: number;
    sessionTotalTokens: number;
    contextWindow: number;
    selectedModel: string;
  }

  let {
    temperature = $bindable(),
    maxTokens = $bindable(),
    extendedThinking = $bindable(),
    showThinkingProcess = $bindable(),
    autoCollapseThinking = $bindable(),
    includeTraceInExport = $bindable(),
    contextMode = $bindable(),
    consultationMode = $bindable(),
    sessionPromptTokens,
    sessionResponseTokens,
    sessionTotalTokens,
    contextWindow,
    selectedModel
  }: Props = $props();
</script>

<details class="settings-details-group">
  <summary class="settings-details-summary">⚙️ Inference Settings</summary>
  <div class="settings-details-content">
    <div class="slider-header">
      <label for="temperature-slider">Model Temperature</label>
      <span class="slider-value font-mono">{temperature.toFixed(1)}</span>
    </div>
    <input id="temperature-slider" type="range" min="0.0" max="1.0" step="0.1" bind:value={temperature} class="slider-input" />
    <p class="card-hint">
      {#if temperature === 0.0}
        <span class="highlight-text font-bold">Deterministic (0.0):</span> Recommended. Adheres strictly to DNA data.
      {:else}
        <span class="highlight-text font-bold">{temperature <= 0.4 ? "Factual" : "Creative"} ({temperature.toFixed(1)}):</span> More natural flow.
      {/if}
    </p>

    <!-- Context Mode Selector -->
    <div style="margin-top: 10px; padding-top: 10px; border-top: 1px dashed rgba(255,255,255,0.08);">
      <label for="context-mode-select" class="highlight-text font-bold" style="font-size: 0.78rem; display: block; margin-bottom: 4px;">AI Context Mode</label>
      <select id="context-mode-select" bind:value={contextMode} class="settings-select">
        <option value="active_findings">Active findings (All packs)</option>
        <option value="active_context_dependent">Active + context-dependent</option>
        <option value="selected_pack_active">Selected pack active</option>
        <option value="full_selected">Full selected packs</option>
        <option value="clinical_checklist">Clinical confirmation checklist</option>
        <option value="evidence_audit">Evidence audit (RAG)</option>
        <option value="developer_raw_json">Developer raw JSON</option>
      </select>
    </div>

    <!-- Specialty Consultation Mode Selector -->
    <div style="margin-top: 10px; padding-top: 10px; border-top: 1px dashed rgba(255,255,255,0.08);">
      <label for="consultation-mode-select" class="highlight-text font-bold" style="font-size: 0.78rem; display: block; margin-bottom: 4px;">Specialty Mode</label>
      <select id="consultation-mode-select" bind:value={consultationMode} class="settings-select">
        <option value="general">🧬 General Consultation</option>
        <option value="pgx">💊 Pharmacogenomics (PGx)</option>
        <option value="nutrients">🍎 Nutrients &amp; Methylation</option>
        <option value="metabolic">🏃 Metabolic Health &amp; T2D</option>
        <option value="sleep">🌙 Sleep &amp; Circadian Rhythms</option>
        <option value="brain_mood">🧠 Brain &amp; Mood (Neuropsych)</option>
        <option value="joints">🦴 Joints &amp; Connective Tissue</option>
        <option value="thyroid_autoimmune">🛡️ Thyroid &amp; Autoimmune Context</option>
        <option value="cardiovascular">❤️ Cardiovascular Health</option>
        <option value="hormones_reproductive">🌸 Hormone &amp; Reproductive Context</option>
      </select>
    </div>

    <div class="flex-row justify-between items-center mt-3 border-t pt-3">
      <label class="extended-thinking-toggle">
        <input type="checkbox" bind:checked={extendedThinking} />
        <span class="highlight-text font-bold">Extended Thinking (Reasoning)</span>
      </label>
    </div>
    
    {#if !extendedThinking}
      <div class="slider-header mt-2">
        <label for="max-tokens-slider">Max Output Tokens</label>
        <span class="slider-value font-mono">{maxTokens.toLocaleString()}</span>
      </div>
      <input id="max-tokens-slider" type="range" min="512" max="8192" step="256" bind:value={maxTokens} class="slider-input" />
    {:else}
      <div class="thinking-mode-active-info">
        🧠 Extended thinking active (max 8,192 tokens). Required for reasoning models (DeepSeek R1, Qwen).
      </div>
    {/if}

    <!-- Thinking visibility & collapse settings -->
    <div style="display: flex; flex-direction: column; gap: 8px; margin-top: 8px; padding-top: 8px; border-top: 1px dashed rgba(255,255,255,0.08);">
      <label class="extended-thinking-toggle">
        <input type="checkbox" bind:checked={showThinkingProcess} />
        <span class="highlight-text">Show thinking process</span>
      </label>
      <label class="extended-thinking-toggle">
        <input type="checkbox" bind:checked={autoCollapseThinking} />
        <span class="highlight-text">Auto-collapse thoughts on completion</span>
      </label>
      <label class="extended-thinking-toggle">
        <input type="checkbox" bind:checked={includeTraceInExport} />
        <span class="highlight-text">Include trace in copy/exports</span>
      </label>
    </div>

    <!-- Session Token Tracker Widget -->
    <div class="token-stats-container mt-3 border-t pt-3">
      <div class="token-stat-row">
        <span>Prompt:</span>
        <span class="font-mono">{sessionPromptTokens.toLocaleString()}</span>
      </div>
      <div class="token-stat-row">
        <span>Response:</span>
        <span class="font-mono">{sessionResponseTokens.toLocaleString()}</span>
      </div>
      <div class="token-stat-row total-row">
        <span>Total:</span>
        <span class="font-mono font-bold">{sessionTotalTokens.toLocaleString()}</span>
      </div>
      
      {#if selectedModel}
        {@const pct = Math.min((sessionTotalTokens / contextWindow) * 100, 100)}
        <div class="token-gauge-bar">
          <div class="token-gauge-fill" style="width: {pct}%" class:warning-gauge={pct > 75}></div>
        </div>
        <div class="token-gauge-labels">
          <span>0</span>
          <span>Context Limit: {contextWindow.toLocaleString()}</span>
        </div>
        {#if pct > 75}
          <div class="token-warning-box">
            ⚠️ Context utilization is high. Consider clearing chat history to maintain factual responses.
          </div>
        {/if}
      {/if}
    </div>
  </div>
</details>

<style>
  .slider-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .slider-value {
    font-size: 0.8rem;
    background: rgba(88, 80, 236, 0.15);
    color: #a5b4fc;
    padding: 2px 6px;
    border-radius: 4px;
    border: 1px solid rgba(88, 80, 236, 0.3);
    font-weight: 600;
  }

  .slider-input {
    width: 100%;
    margin-top: 4px;
    background: rgba(0, 0, 0, 0.3);
    height: 6px;
    border-radius: 3px;
    outline: none;
    -webkit-appearance: none;
    appearance: none;
    cursor: pointer;
  }

  .slider-input::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--accent);
    cursor: pointer;
    transition: transform 0.1s;
  }

  .slider-input::-webkit-slider-thumb:hover {
    transform: scale(1.2);
  }

  .extended-thinking-toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    font-size: 0.8rem;
    color: var(--text-primary);
  }
  .extended-thinking-toggle input {
    cursor: pointer;
  }

  .thinking-mode-active-info {
    font-size: 0.72rem;
    background: rgba(88, 80, 236, 0.1);
    border: 1px solid rgba(88, 80, 236, 0.25);
    color: #a5b4fc;
    padding: 8px 10px;
    border-radius: 6px;
    line-height: 1.4;
    margin-top: 4px;
  }

  /* Token Stats Widget */
  .token-stats-container {
    display: flex;
    flex-direction: column;
    gap: 6px;
    background: rgba(0, 0, 0, 0.15);
    border: 1px solid var(--border-color);
    padding: 10px;
    border-radius: 6px;
  }
  .token-stat-row {
    display: flex;
    justify-content: space-between;
    font-size: 0.78rem;
    color: var(--text-secondary);
  }
  .token-stat-row.total-row {
    color: var(--text-primary);
    border-top: 1px solid rgba(255, 255, 255, 0.08);
    padding-top: 4px;
    margin-top: 2px;
  }
  .token-gauge-bar {
    background: rgba(255, 255, 255, 0.05);
    height: 6px;
    border-radius: 3px;
    margin-top: 8px;
    overflow: hidden;
  }
  .token-gauge-fill {
    background: var(--accent);
    height: 100%;
    border-radius: 3px;
    transition: width 0.3s ease;
  }
  .token-gauge-fill.warning-gauge {
    background: var(--danger);
  }
  .token-gauge-labels {
    display: flex;
    justify-content: space-between;
    font-size: 0.65rem;
    color: #6b7280;
    margin-top: 2px;
  }
  .token-warning-box {
    margin-top: 8px;
    font-size: 0.72rem;
    color: #fca5a5;
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.25);
    padding: 6px 8px;
    border-radius: 4px;
    line-height: 1.3;
  }

  .card-hint {
    font-size: 0.72rem;
    color: var(--text-secondary);
    line-height: 1.3;
    margin: 0;
  }

  .mt-2 { margin-top: 8px; }
  .mt-3 { margin-top: 12px; }
  .pt-3 { padding-top: 12px; }
  .border-t { border-top: 1px solid var(--border-color); }
  .highlight-text { color: var(--text-primary); }
  .font-bold { font-weight: bold; }
  .font-mono { font-family: monospace; }
  .flex-row { display: flex; flex-direction: row; }
  .justify-between { justify-content: space-between; }
  .items-center { align-items: center; }
</style>
