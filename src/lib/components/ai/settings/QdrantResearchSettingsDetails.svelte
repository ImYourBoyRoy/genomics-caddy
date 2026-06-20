<!-- ./src/lib/components/ai/settings/QdrantResearchSettingsDetails.svelte -->
<script lang="ts">
  import { onMount } from "svelte";
  import {
    testQdrantConnection,
    getQdrantConfig,
    saveQdrantConfig,
    scanOllamaModels,
  } from "../../../api/tauri";
  import type { QdrantConfigUpdate } from "../../../types/research";
  import { DEFAULT_QDRANT_CONFIG, isEmbedModel } from "../../../types/research";
  import GnomadSetupPanel from "../../research/GnomadSetupPanel.svelte";

  interface Props {
    ollamaUrl?: string;
    ollamaToken?: string;
  }

  let {
    ollamaUrl = $bindable("http://localhost:11434"),
    ollamaToken = $bindable(""),
  }: Props = $props();

  let qdrantUrl = $state<string>(DEFAULT_QDRANT_CONFIG.url);
  let qdrantApiKey = $state<string>("");
  let qdrantApiKeySet = $state<boolean>(false);
  let qdrantCollection = $state<string>(DEFAULT_QDRANT_CONFIG.collection);
  let qdrantEmbedModel = $state<string>(DEFAULT_QDRANT_CONFIG.embedding_model);
  let qdrantGwasStrict = $state<boolean>(DEFAULT_QDRANT_CONFIG.gwas_strict);
  let qdrantAutoStart = $state<boolean>(DEFAULT_QDRANT_CONFIG.auto_start);
  let ncbiApiKey = $state<string>("");
  let ncbiApiKeySet = $state<boolean>(false);

  let qdrantTestStatus = $state<"idle" | "testing" | "ok" | "error">("idle");
  let qdrantTestMessage = $state<string>("");
  let qdrantTestVectors = $state<number | null>(null);
  let availableEmbedModels = $state<string[]>([]);
  let qdrantCollections = $state<string[]>([]);

  async function saveQdrantSettings() {
    try {
      const payload: QdrantConfigUpdate = {
        url: qdrantUrl.trim(),
        collection: qdrantCollection.trim(),
        embedding_model: qdrantEmbedModel.trim(),
        gwas_strict: qdrantGwasStrict,
        auto_start: qdrantAutoStart,
      };
      if (qdrantApiKey.trim()) payload.api_key = qdrantApiKey.trim();
      if (ncbiApiKey.trim()) payload.ncbi_api_key = ncbiApiKey.trim();
      await saveQdrantConfig(payload);
      if (qdrantApiKey.trim()) {
        qdrantApiKeySet = true;
        qdrantApiKey = "";
      }
      if (ncbiApiKey.trim()) {
        ncbiApiKeySet = true;
        ncbiApiKey = "";
      }
    } catch (e) {
      console.error("Failed to save Qdrant settings:", e);
    }
  }

  async function handleTestQdrant() {
    qdrantTestStatus = "testing";
    qdrantTestMessage = "";
    qdrantTestVectors = null;
    try {
      const result = await testQdrantConnection(
        qdrantUrl,
        qdrantApiKey || undefined,
        qdrantCollection || undefined
      );
      if (result.success) {
        qdrantTestStatus = "ok";
        qdrantTestVectors = result.vectors_count ?? 0;
        qdrantTestMessage = `Connected — ${qdrantTestVectors.toLocaleString()} vectors stored`;
        if (result.collections) qdrantCollections = result.collections;
      } else {
        qdrantTestStatus = "error";
        qdrantTestMessage = result.error || "Connection failed";
      }
    } catch (e: unknown) {
      qdrantTestStatus = "error";
      qdrantTestMessage = e instanceof Error ? e.message : String(e);
    }
  }

  onMount(async () => {
    try {
      const cfg = await getQdrantConfig();
      qdrantUrl = cfg.url;
      qdrantApiKeySet = cfg.api_key_set;
      qdrantCollection = cfg.collection;
      qdrantEmbedModel = cfg.embedding_model;
      qdrantGwasStrict = cfg.gwas_strict;
      qdrantAutoStart = cfg.auto_start;
      ncbiApiKeySet = cfg.ncbi_api_key_set;
    } catch (e) {
      console.warn("Failed to load Qdrant config in settings:", e);
    }

    try {
      const allModels = await scanOllamaModels(ollamaUrl, ollamaToken || undefined);
      availableEmbedModels = allModels.filter(isEmbedModel);
    } catch {
      availableEmbedModels = ["mxbai-embed-large", "nomic-embed-text"];
    }

    if (!availableEmbedModels.includes(qdrantEmbedModel)) {
      availableEmbedModels = [...availableEmbedModels, qdrantEmbedModel];
    }

    if (qdrantUrl) void handleTestQdrant();
  });
</script>

<details class="settings-details-group">
  <summary class="settings-details-summary">🔬 Research &amp; Vector DB</summary>
  <div class="settings-details-content">
    <div class="setting-row">
      <span class="section-label">🗄️ Qdrant Vector Database</span>
      <span class="help-text">Self-hosted Qdrant for autonomous marker research.</span>

      <div class="mt-1">
        <label class="help-text" for="qdrant-url">Server URL</label>
        <input id="qdrant-url" type="text" placeholder="http://localhost:6333" bind:value={qdrantUrl} onblur={saveQdrantSettings} class="mcp-input" />
      </div>

      <div class="mt-1">
        <label class="help-text" for="qdrant-api-key">API Key (optional)</label>
        <input
          id="qdrant-api-key"
          type="password"
          placeholder={qdrantApiKeySet ? "Configured (.env or keychain) — enter new value to replace" : "Set in .env (QDRANT_API_KEY) or enter here"}
          bind:value={qdrantApiKey}
          onblur={saveQdrantSettings}
          class="mcp-input"
        />
      </div>

      <div class="mt-1">
        <label class="help-text" for="qdrant-collection">Collection Name</label>
        <input list="settings-collections-list" id="qdrant-collection" type="text" placeholder="genomics_evidence" bind:value={qdrantCollection} onblur={saveQdrantSettings} class="mcp-input" />
        <datalist id="settings-collections-list">
          {#each qdrantCollections as name}
            <option value={name}>{name}</option>
          {/each}
        </datalist>
      </div>

      <div class="mt-1">
        <label class="help-text" for="ollama-url-input">Ollama Server URL</label>
        <input id="ollama-url-input" type="text" placeholder="http://localhost:11434" bind:value={ollamaUrl} class="mcp-input" />
      </div>

      <div class="mt-1">
        <label class="help-text" for="ollama-token-input">Ollama Token (optional)</label>
        <input id="ollama-token-input" type="password" placeholder="Leave blank if no token required" bind:value={ollamaToken} class="mcp-input" />
      </div>

      <div class="mt-1">
        <label class="help-text" for="qdrant-model">Embedding Model</label>
        <select id="qdrant-model" bind:value={qdrantEmbedModel} onchange={saveQdrantSettings} class="mcp-input">
          {#each availableEmbedModels as model}
            <option value={model}>{model} {isEmbedModel(model) ? "✨ (embed)" : ""}</option>
          {/each}
        </select>
      </div>

      <div class="qdrant-test-row mt-2">
        <button class="btn btn-secondary w-full" onclick={handleTestQdrant} disabled={qdrantTestStatus === "testing" || !qdrantUrl.trim()}>
          {qdrantTestStatus === "testing" ? "⏳ Testing..." : "🔌 Test Connection"}
        </button>
        {#if qdrantTestStatus === "ok"}
          <div class="status-msg success">{qdrantTestMessage}</div>
        {:else if qdrantTestStatus === "error"}
          <div class="status-msg error">{qdrantTestMessage}</div>
        {/if}
      </div>
    </div>

    <div class="setting-row section-divider mt-2">
      <span class="section-label">🧬 NCBI API Credentials</span>
      <span class="help-text">
        Speeds up PubMed searches 3x. Get a key at the
        <a href="https://www.ncbi.nlm.nih.gov/account/" target="_blank" rel="noreferrer" style="color: var(--accent); text-decoration: underline;">NCBI Account Portal</a>.
      </span>
      <div class="mt-1">
        <label class="help-text" for="ncbi-api-key">NCBI API Key</label>
        <input
          id="ncbi-api-key"
          type="password"
          placeholder={ncbiApiKeySet ? "Configured (.env or keychain) — enter new value to replace" : "NCBI API Key (optional — or NCBI_API_KEY in .env)"}
          bind:value={ncbiApiKey}
          onblur={saveQdrantSettings}
          class="mcp-input"
        />
      </div>
    </div>

    <GnomadSetupPanel compact />

    <div class="setting-row section-divider mt-2">
      <span class="section-label">📊 GWAS Significance Mode</span>
      <span class="help-text">Choose whether the research loop filters strictly or includes suggestive findings.</span>
      <div class="toggle-list mt-1">
        <label class="toggle-item">
          <input type="checkbox" bind:checked={qdrantGwasStrict} onchange={saveQdrantSettings} />
          <span>Strict GWAS Significance (p &lt; 5×10⁻⁸)</span>
        </label>
      </div>
    </div>
  </div>
</details>
