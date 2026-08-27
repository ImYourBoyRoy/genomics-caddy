<!-- ./src/lib/components/research/ResearchConnectionEditForm.svelte -->
<!--
  Edit settings form for Vector DB / Ollama connection configuration.
  Bindable fields; save/test logic lives in the parent card via callbacks.
-->
<script lang="ts">
  import type { QdrantConfigPublic, QdrantConnectionStatus } from "../../types/research";
  import "$lib/styles/components/research-connection-card.css";
  import Tooltip from "../common/Tooltip.svelte";

  interface Props {
    editQdrantUrl: string;
    editCollection: string;
    editEmbedModel: string;
    editApiKey: string;
    editOllamaUrl: string;
    editOllamaToken: string;
    editNamedVectors: boolean;
    config: QdrantConfigPublic;
    connectionStatus?: QdrantConnectionStatus | null;
    ollamaStatus: "untested" | "testing" | "live" | "dead";
    availableEmbedModels: string[];
    totalOllamaModels: number;
    isScanningModels: boolean;
    onCancel: () => void;
    onSave: () => void;
    onRefreshModels: () => void;
    onOllamaUrlChange: () => void;
  }

  let {
    editQdrantUrl = $bindable(),
    editCollection = $bindable(),
    editEmbedModel = $bindable(),
    editApiKey = $bindable(),
    editOllamaUrl = $bindable(),
    editOllamaToken = $bindable(),
    editNamedVectors = $bindable(),
    config,
    connectionStatus = null,
    ollamaStatus,
    availableEmbedModels,
    totalOllamaModels,
    isScanningModels,
    onCancel,
    onSave,
    onRefreshModels,
    onOllamaUrlChange,
  }: Props = $props();
</script>

<div class="settings-form">
  <div class="settings-group-title">📦 Qdrant Settings</div>
  <div class="form-grid">
    <div class="form-group-custom">
      <label for="qdrant-url" class="label-with-status">
        <span>Qdrant Server URL</span>
        <Tooltip
          label="Qdrant status"
          description={connectionStatus?.success ? "Qdrant is connected." : connectionStatus ? "The last Qdrant connection check failed." : "Qdrant has not been checked yet."}
          triggerClass="status-tooltip-trigger"
        >
          <span class="status-indicator-dot {connectionStatus?.success ? 'live' : connectionStatus ? 'dead' : 'untested'}"></span>
        </Tooltip>
      </label>
      <input id="qdrant-url" type="text" bind:value={editQdrantUrl} placeholder="e.g. http://127.0.0.1:6333" />
    </div>

    <div class="form-group-custom">
      <label for="qdrant-api-key">Qdrant API Key</label>
      <input
        id="qdrant-api-key"
        type="password"
        bind:value={editApiKey}
        placeholder={config.api_key_set ? "•••••••• (Keep existing)" : "Enter API key..."}
      />
    </div>

    <div class="form-group-custom">
      <label for="qdrant-collection">Collection Name</label>
      <input
        list="card-collections-list"
        id="qdrant-collection"
        type="text"
        bind:value={editCollection}
        placeholder="genomics_caddy"
      />
      <datalist id="card-collections-list">
        {#if connectionStatus?.collections}
          {#each connectionStatus.collections as name}
            <option value={name}>{name}</option>
          {/each}
        {/if}
      </datalist>
    </div>
  </div>

  <hr class="section-divider" />

  <div class="settings-group-title">🧠 Ollama Settings</div>
  <div class="form-grid">
    <div class="form-group-custom">
      <label for="ollama-url" class="label-with-status">
        <span>Ollama Server URL</span>
        <Tooltip
          label="Ollama status"
          description={ollamaStatus === "live" ? "Ollama is available." : ollamaStatus === "dead" ? "The last Ollama model check failed." : ollamaStatus === "testing" ? "Ollama is being checked." : "Ollama has not been checked yet."}
          triggerClass="status-tooltip-trigger"
        >
          <span class="status-indicator-dot {ollamaStatus}"></span>
        </Tooltip>
      </label>
      <input
        id="ollama-url"
        type="text"
        bind:value={editOllamaUrl}
        onchange={onOllamaUrlChange}
        placeholder="e.g. http://127.0.0.1:11434"
      />
    </div>

    <div class="form-group-custom">
      <label for="ollama-token">Ollama Token (optional)</label>
      <input id="ollama-token" type="password" bind:value={editOllamaToken} placeholder="Enter token..." />
    </div>

    <div class="form-group-custom">
      <label for="qdrant-model">Embedding Model (vector research only)</label>
      <p class="field-hint">
        Showing {availableEmbedModels.length} of {totalOllamaModels || "…"} Ollama models — only embedding models appear here.
        LLM chat and validator models are set under AI Assistant → Settings.
      </p>
      <div class="select-row">
        <select id="qdrant-model" bind:value={editEmbedModel}>
          {#each availableEmbedModels as model}
            <option value={model}>{model}</option>
          {/each}
        </select>
        <Tooltip interactiveChildren interactiveClickBehavior="dismiss" label="Refresh models" description="Refresh the available embedding models from Ollama.">
          <button
            class="refresh-btn"
            type="button"
            onclick={onRefreshModels}
            disabled={isScanningModels}
          >
            {isScanningModels ? "⏳" : "🔄"}
          </button>
        </Tooltip>
      </div>
    </div>

    <label class="checkbox-row">
      <input type="checkbox" bind:checked={editNamedVectors} />
      <span>Enable named vectors (trait / gene / evidence / actionability)</span>
    </label>
  </div>

  <div class="form-actions mt-3">
    <button class="btn btn-secondary" type="button" onclick={onCancel}>Cancel</button>
    <button
      class="btn btn-primary"
      type="button"
      onclick={onSave}
      disabled={!editQdrantUrl.trim() || !editCollection.trim()}
    >
      Save & Test
    </button>
  </div>
</div>
