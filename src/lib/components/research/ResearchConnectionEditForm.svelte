<!-- ./src/lib/components/research/ResearchConnectionEditForm.svelte -->
<!--
  Edit settings form for Vector DB / Ollama connection configuration.
  Bindable fields; save/test logic lives in the parent card via callbacks.
-->
<script lang="ts">
  import type { QdrantConfigPublic, QdrantConnectionStatus } from "../../types/research";

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
        <span
          class="status-indicator-dot {connectionStatus?.success ? 'live' : connectionStatus ? 'dead' : 'untested'}"
          title="Qdrant status"
        ></span>
      </label>
      <input id="qdrant-url" type="text" bind:value={editQdrantUrl} placeholder="http://localhost:6333" />
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
        <span class="status-indicator-dot {ollamaStatus}" title="Ollama status"></span>
      </label>
      <input
        id="ollama-url"
        type="text"
        bind:value={editOllamaUrl}
        onchange={onOllamaUrlChange}
        placeholder="http://localhost:11434"
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
        <button
          class="refresh-btn"
          type="button"
          onclick={onRefreshModels}
          disabled={isScanningModels}
          title="Refresh models from Ollama"
        >
          {isScanningModels ? "⏳" : "🔄"}
        </button>
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

<style src="../../styles/components/research-connection-card.css"></style>
