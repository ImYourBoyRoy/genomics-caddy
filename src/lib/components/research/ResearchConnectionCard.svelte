<!-- ./src/lib/components/research/ResearchConnectionCard.svelte -->
<script lang="ts">
  import { onMount } from "svelte";
  import ActivityPulse from "../common/loading/ActivityPulse.svelte";
  import {
    testQdrantConnection,
    createQdrantCollection,
    purgeQdrantCollection,
    saveQdrantConfig,
    scanOllamaModels,
    saveOllamaToken,
  } from "../../api/tauri";
  import type {
    QdrantConfigPublic,
    QdrantConfigUpdate,
    QdrantConnectionStatus,
    ConnectionActivity,
  } from "../../types/research";
  import { isEmbedModel, DEFAULT_CONNECTION_ACTIVITY } from "../../types/research";
  import {
    shouldAutoCheckConnections,
    withInvokeTimeout,
    connectionCheckTimeoutMs,
  } from "../../utils/researchConnection";
  import { saveOllamaUrl } from "../../utils/ollamaSettings";
  import ResearchConnectionEditForm from "./ResearchConnectionEditForm.svelte";

  interface Props {
    config: QdrantConfigPublic;
    ollamaUrl: string;
    ollamaToken?: string;
    connectionStatus?: QdrantConnectionStatus | null;
    connectionActivity?: ConnectionActivity;
    ollamaStatus?: 'untested' | 'testing' | 'live' | 'dead';
    isConfigLoaded?: boolean;
    selectedSampleId?: number | null;
    disabled?: boolean;
    onConnectionStatus?: (status: QdrantConnectionStatus | null) => void;
    onLog?: (message: string) => void;
    onConfigUpdated?: () => void;
    onJobReset?: () => void;
  }

  let {
    config = $bindable(),
    ollamaUrl = $bindable(),
    ollamaToken = $bindable(""),
    connectionStatus = $bindable(null),
    connectionActivity = $bindable({ ...DEFAULT_CONNECTION_ACTIVITY }),
    ollamaStatus = $bindable<'untested' | 'testing' | 'live' | 'dead'>('untested'),
    isConfigLoaded = false,
    selectedSampleId = null,
    disabled = false,
    onConnectionStatus,
    onLog,
    onConfigUpdated,
    onJobReset,
  }: Props = $props();

  let isTesting = $state(false);
  let isCreating = $state(false);
  let isPurging = $state(false);
  let purgeConfirm = $state(false);

  let isEditing = $state(false);
  let editQdrantUrl = $state("");
  let editCollection = $state("");
  let editEmbedModel = $state("");
  let editApiKey = $state("");
  let editOllamaUrl = $state("");
  let editOllamaToken = $state("");
  let editNamedVectors = $state(false);
  let availableEmbedModels = $state<string[]>([]);
  let totalOllamaModels = $state(0);
  let isScanningModels = $state(false);

  function log(msg: string) {
    onLog?.(msg);
  }

  function setStatus(status: QdrantConnectionStatus | null) {
    connectionStatus = status;
    onConnectionStatus?.(status);
  }

  function setActivity(phase: ConnectionActivity["phase"], message: string) {
    connectionActivity = { phase, message };
  }

  async function handleTestConnection(explicitTest = false) {
    const url = isEditing ? editQdrantUrl : config.url;
    const timeoutMs = connectionCheckTimeoutMs(url, explicitTest);
    const status = await withInvokeTimeout(
      testQdrantConnection(
        url,
        isEditing ? (editApiKey.trim() || undefined) : undefined,
        isEditing ? editCollection : config.collection
      ),
      timeoutMs,
      "Qdrant connection check"
    );
    setStatus(status);
    if (status.success) {
      log(
        status.collection_exists
          ? `Qdrant Connected — ${status.vectors_count?.toLocaleString() ?? 0} vectors in '${isEditing ? editCollection : config.collection}'`
          : `Qdrant Connected — collection '${isEditing ? editCollection : config.collection}' not created yet`
      );
    } else {
      log(`Qdrant Connection failed: ${status.error || "unknown error"}`);
    }
    return status;
  }

  async function handleTestOllama(explicitTest = false) {
    ollamaStatus = "testing";
    const url = isEditing ? editOllamaUrl : ollamaUrl;
    const timeoutMs = connectionCheckTimeoutMs(url, explicitTest);
    try {
      const models = await withInvokeTimeout(
        scanOllamaModels(url, (isEditing ? editOllamaToken : ollamaToken) || undefined),
        timeoutMs,
        "Ollama model scan"
      );
      if (models && models.length > 0) {
        ollamaStatus = "live";
        totalOllamaModels = models.length;
        const embedCount = models.filter(isEmbedModel).length;
        log(
          `Ollama Connected — ${models.length} models total (${embedCount} embedding-capable for vector research). Chat/review models are configured in AI Assistant settings.`
        );
        return true;
      }
      ollamaStatus = "dead";
      log("Ollama Connection failed: No models returned.");
      return false;
    } catch (e: any) {
      ollamaStatus = "dead";
      log(`Ollama Connection failed: ${e.message || String(e)}`);
      return false;
    }
  }

  async function runConnectionChecks(options: { quiet?: boolean; explicit?: boolean } = {}) {
    if (isTesting) return;
    isTesting = true;
    const explicit = options.explicit ?? !options.quiet;
    if (!options.quiet) {
      setStatus(null);
    }

    try {
      setActivity("checking-qdrant", "Pinging Qdrant server…");
      await new Promise((r) => setTimeout(r, 0));
      const qdrant = await handleTestConnection(explicit);

      setActivity("checking-ollama", "Scanning Ollama embedding models…");
      await new Promise((r) => setTimeout(r, 0));
      await handleTestOllama(explicit);

      if (qdrant.success) {
        setActivity("ready", "Connections verified");
      } else {
        setActivity("error", qdrant.error || "Qdrant connection failed");
      }
    } catch (e: any) {
      setActivity("error", e.message || String(e));
    } finally {
      isTesting = false;
    }
  }

  async function handleCreateCollection() {
    isCreating = true;
    try {
      log(`Creating Qdrant collection '${config.collection}' via Ollama embed probe...`);
      const status = await createQdrantCollection(ollamaUrl);
      setStatus(status);
      if (status.success && status.collection_exists) {
        log(`Collection ready — ${status.vectors_count?.toLocaleString() ?? 0} vectors`);
      } else if (status.success) {
        log("Collection created (empty).");
      } else {
        log(`Create failed: ${status.error || "unknown error"}`);
      }
    } catch (e: any) {
      log(`Create failed: ${e.message || String(e)}`);
    } finally {
      isCreating = false;
    }
  }

  async function handlePurgeCollection() {
    if (!purgeConfirm) {
      purgeConfirm = true;
      log("Click Purge again to confirm deleting all vectors in this collection.");
      return;
    }
    isPurging = true;
    purgeConfirm = false;
    try {
      const result = await purgeQdrantCollection(selectedSampleId ?? undefined);
      log(result.message);
      setStatus({
        success: true,
        collection_exists: false,
        vectors_count: 0,
      });
      if (result.job_reset) {
        onJobReset?.();
      }
    } catch (e: any) {
      log(`Purge failed: ${e.message || String(e)}`);
    } finally {
      isPurging = false;
    }
  }

  // Edit methods
  function startEditing() {
    editQdrantUrl = config.url;
    editCollection = config.collection;
    editEmbedModel = config.embedding_model;
    editApiKey = "";
    editOllamaUrl = ollamaUrl;
    editOllamaToken = ollamaToken;
    editNamedVectors = config.named_vectors_enabled;
    isEditing = true;
    void refreshModels();
  }

  function cancelEditing() {
    isEditing = false;
  }

  async function refreshModels() {
    isScanningModels = true;
    try {
      const models = await scanOllamaModels(editOllamaUrl, editOllamaToken || undefined);
      totalOllamaModels = models.length;
      availableEmbedModels = models.filter(isEmbedModel);
      if (editEmbedModel && !availableEmbedModels.includes(editEmbedModel)) {
        availableEmbedModels = [...availableEmbedModels, editEmbedModel];
      }
    } catch (e) {
      console.warn("Failed to scan Ollama models:", e);
      availableEmbedModels = ["mxbai-embed-large", "nomic-embed-text"];
      if (editEmbedModel && !availableEmbedModels.includes(editEmbedModel)) {
        availableEmbedModels = [...availableEmbedModels, editEmbedModel];
      }
    } finally {
      isScanningModels = false;
    }
  }

  async function handleSave() {
    try {
      const payload: QdrantConfigUpdate = {
        url: editQdrantUrl.trim(),
        collection: editCollection.trim(),
        embedding_model: editEmbedModel.trim(),
        gwas_strict: config.gwas_strict,
        auto_start: config.auto_start,
        named_vectors_enabled: editNamedVectors,
      };
      if (editApiKey.trim()) {
        payload.api_key = editApiKey.trim();
      }

      await saveQdrantConfig(payload);

      // Save Ollama settings
      ollamaUrl = editOllamaUrl.trim();
      saveOllamaUrl(ollamaUrl);

      ollamaToken = editOllamaToken.trim();
      await saveOllamaToken(ollamaToken || undefined);

      isEditing = false;
      log("Connection settings saved.");
      onConfigUpdated?.();
      
      // Re-run connection status tests
      void runConnectionChecks();
    } catch (e: any) {
      log(`Failed to save settings: ${e.message || String(e)}`);
    }
  }

  let hasRunInitialTests = $state(false);

  let autoCheckOnOpen = $derived(
    shouldAutoCheckConnections(config.url, ollamaUrl, config.auto_start)
  );

  // Auto-check remote hosts (Tailscale/LAN); skip auto-ping for localhost unless auto_start is enabled.
  $effect(() => {
    if (isConfigLoaded && !hasRunInitialTests) {
      hasRunInitialTests = true;
      if (autoCheckOnOpen) {
        setTimeout(() => {
          void runConnectionChecks({ quiet: true, explicit: false });
        }, 120);
      } else {
        setActivity("idle", "Click Test Connections when your Qdrant URL is configured.");
      }
    }
  });
</script>

<div class="glass-card research-connection-card">
  <div class="card-header-row">
    <h2 class="card-title">Vector DB Connection</h2>
    {#if !isEditing}
      <button class="edit-settings-btn" onclick={startEditing} disabled={disabled} aria-label="Edit Connection Settings">⚙️ Edit Settings</button>
    {/if}
  </div>

  {#if connectionActivity.phase !== "idle" && connectionActivity.phase !== "ready"}
    <div class="activity-strip activity-{connectionActivity.phase}" aria-live="polite">
      <ActivityPulse
        message={connectionActivity.message}
        accent={connectionActivity.phase === "error" ? "var(--danger)" : "#8b5cf6"}
        isError={connectionActivity.phase === "error"}
        maxWidth="100%"
      />
    </div>
  {/if}

  {#if isEditing}
    <ResearchConnectionEditForm
      bind:editQdrantUrl
      bind:editCollection
      bind:editEmbedModel
      bind:editApiKey
      bind:editOllamaUrl
      bind:editOllamaToken
      bind:editNamedVectors
      {config}
      {connectionStatus}
      {ollamaStatus}
      {availableEmbedModels}
      {totalOllamaModels}
      {isScanningModels}
      onCancel={cancelEditing}
      onSave={handleSave}
      onRefreshModels={refreshModels}
      onOllamaUrlChange={() => { void refreshModels(); void handleTestOllama(); }}
    />
  {:else}
    <div class="connection-body">
      <div class="readout-section">
        <div class="settings-group-title">Qdrant</div>
        <dl class="config-readout">
          <div>
            <dt>Server</dt>
            <dd class="val-with-status">
              <span class="status-indicator-dot {connectionStatus?.success ? 'live' : connectionStatus ? 'dead' : 'untested'}" title="Qdrant status"></span>
              <span class="url-text">{config.url}</span>
            </dd>
          </div>
          <div>
            <dt>Collection</dt>
            <dd>{config.collection}</dd>
          </div>
          <div>
            <dt>API Key</dt>
            <dd>{config.api_key_set ? "Stored securely" : "Not configured"}</dd>
          </div>
          <div>
            <dt>Named vectors</dt>
            <dd>{config.named_vectors_enabled ? "Enabled" : "Default only"}</dd>
          </div>
        </dl>
      </div>

      <div class="readout-section">
        <div class="settings-group-title">Ollama</div>
        <dl class="config-readout">
          <div>
            <dt>Server</dt>
            <dd class="val-with-status">
              <span class="status-indicator-dot {ollamaStatus}" title="Ollama status"></span>
              <span class="url-text">{ollamaUrl}</span>
            </dd>
          </div>
          <div>
            <dt>Embed model</dt>
            <dd>{config.embedding_model}</dd>
          </div>
        </dl>
      </div>

      <div class="connection-footer">
        {#if !connectionStatus && !isTesting}
          <p class="connection-hint">
            {#if autoCheckOnOpen}
              Remote server configured — connection check runs automatically on open.
            {:else}
              Localhost is not pinged automatically — run Test Connections or enable auto-start in AI settings.
            {/if}
          </p>
        {/if}

        <div class="btn-row">
          <button
            class="btn btn-secondary"
            onclick={() => { void runConnectionChecks({ explicit: true }); }}
            disabled={disabled || isTesting || !config.url.trim()}
          >
            {isTesting ? connectionActivity.message : "Test Connections"}
          </button>
          {#if !connectionStatus?.collection_exists}
            <button
              class="btn btn-primary"
              onclick={handleCreateCollection}
              disabled={disabled || isCreating || !connectionStatus?.success}
            >
              {isCreating ? "Creating..." : "Create Collection"}
            </button>
          {/if}
          <button
            class="btn btn-danger"
            onclick={handlePurgeCollection}
            disabled={disabled || isPurging || !connectionStatus?.collection_exists}
          >
            {isPurging ? "Purging..." : purgeConfirm ? "Confirm Purge" : "Purge Collection"}
          </button>
        </div>

        {#if connectionStatus}
          <div
            class="connection-status"
            class:success={connectionStatus.success}
            class:error={!connectionStatus.success}
          >
            {#if connectionStatus.success}
              <div class="status-summary">
                <span class="indicator success"></span>
                <span>Qdrant Connected</span>
              </div>
              <p class="status-detail">
                {#if connectionStatus.collection_exists}
                  Collection <strong>{config.collection}</strong> —
                  <strong>{connectionStatus.vectors_count?.toLocaleString() ?? 0}</strong> vectors indexed.
                {:else}
                  Server reachable. Collection <strong>{config.collection}</strong> missing —
                  create it before starting a sweep.
                {/if}
              </p>
              {#if ollamaStatus === "live"}
                <p class="status-detail ollama-ok">Ollama verified — embedding models available.</p>
              {:else if ollamaStatus === "dead"}
                <p class="status-detail error-text">Ollama unreachable — fix the embed server before sweeping.</p>
              {:else if ollamaStatus === "untested"}
                <p class="status-detail">Ollama not verified — run Test Connections.</p>
              {/if}
            {:else}
              <div class="status-summary">
                <span class="indicator error"></span>
                <span>Qdrant Connection Failed</span>
              </div>
              <p class="status-detail error-text">
                {connectionStatus.error || "Unknown error connecting to Qdrant."}
              </p>
            {/if}
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>
