<!-- ./src/lib/components/settings/ConnectionsPanel.svelte -->
<script lang="ts">
  /*
  Purpose: Advanced Connections hub for Ollama + multi-provider vector DB.
  Responsibilities: endpoints, localhost reset, provider capability matrix,
  model Install / Update / Update all with progress, Ollama library links.
  */

  import { onMount, onDestroy } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import {
    checkOllamaUpdate,
    checkQdrantUpdate,
    deleteOllamaModel,
    discoverOllamaModels,
    getOllamaServiceConfig,
    getQdrantConfig,
    probeInferenceHost,
    probeLocalhostServices,
    probeVectorProvider,
    pullOllamaModel,
    saveOllamaToken,
    saveQdrantConfig,
    testQdrantConnection,
    type InferenceHostProfile,
    type LocalhostServiceStatus,
    type OllamaModelInsight,
    type ServiceUpdateCheck,
  } from "../../api/tauri";
  import type { QdrantConfigPublic, QdrantConnectionStatus } from "../../types/research";
  import { DEFAULT_QDRANT_CONFIG, isEmbedModel } from "../../types/research";
  import {
    LOCAL_OLLAMA_URL,
    LOCAL_QDRANT_URL,
    OLLAMA_URL_PLACEHOLDER,
    persistOllamaUrl,
    resolveInitialOllamaUrl,
  } from "../../utils/ollamaSettings";
  import {
    connectionCheckTimeoutMs,
    withInvokeTimeout,
  } from "../../utils/researchConnection";
  import "$lib/styles/components/connections-panel.css";

  interface Props {
    ollamaUrl?: string;
    ollamaToken?: string;
    onOllamaUrlChange?: (url: string) => void;
    onOllamaTokenChange?: (token: string) => void;
  }

  type VectorProviderId = "qdrant" | "pinecone" | "chroma" | "weaviate";

  const OLLAMA_LIBRARY_URL = "https://ollama.com/library";
  const OLLAMA_SEARCH_URL = "https://ollama.com/search";
  const SUGGESTED_EMBED = ["mxbai-embed-large", "nomic-embed-text", "bge-m3"];
  const SUGGESTED_CHAT = ["qwen2.5:14b", "llama3.2", "mistral"];

  const PROVIDER_META: Record<
    VectorProviderId,
    {
      label: string;
      urlLabel: string;
      urlPlaceholder: string;
      collectionLabel: string;
      collectionHint: string;
      needsCollection: boolean;
      needsApiKey: boolean;
      dense: boolean;
      namedVectors: boolean;
      hosting: string;
    }
  > = {
    qdrant: {
      label: "Qdrant",
      urlLabel: "Qdrant URL",
      urlPlaceholder: "http://qdrant.local:6333 or http://127.0.0.1:6333",
      collectionLabel: "Collection",
      collectionHint: "Existing collection name (created from Research or Connections).",
      needsCollection: true,
      needsApiKey: false,
      dense: true,
      namedVectors: true,
      hosting: "Self-host / LAN / cloud. Full research: dense + named vectors + payload indexes.",
    },
    pinecone: {
      label: "Pinecone",
      urlLabel: "Pinecone index host",
      urlPlaceholder: "https://….svc.…pinecone.io",
      collectionLabel: "Index label (optional)",
      collectionHint: "Index is selected by the host URL. Optional label for your notes.",
      needsCollection: false,
      needsApiKey: true,
      dense: true,
      namedVectors: false,
      hosting: "Managed cloud. Dense research sweeps and semantic search supported.",
    },
    chroma: {
      label: "Chroma",
      urlLabel: "Chroma URL",
      urlPlaceholder: "http://127.0.0.1:8000",
      collectionLabel: "Collection",
      collectionHint: "Chroma collection name for genomics evidence vectors.",
      needsCollection: true,
      needsApiKey: false,
      dense: true,
      namedVectors: false,
      hosting: "Self-host or cloud. Dense research path (no named vectors).",
    },
    weaviate: {
      label: "Weaviate",
      urlLabel: "Weaviate URL",
      urlPlaceholder: "http://127.0.0.1:8080",
      collectionLabel: "Class name",
      collectionHint: "Weaviate class (e.g. GenomicsEvidence). Created on ensure if missing.",
      needsCollection: true,
      needsApiKey: false,
      dense: true,
      namedVectors: false,
      hosting: "Self-host or cloud. Dense research path (no named vectors).",
    },
  };

  let {
    ollamaUrl = $bindable(""),
    ollamaToken = $bindable(""),
    onOllamaUrlChange,
    onOllamaTokenChange,
  }: Props = $props();

  let config = $state<QdrantConfigPublic>({ ...DEFAULT_QDRANT_CONFIG });
  let qdrantUrl = $state("");
  let collection = $state("");
  let embedModel = $state("");
  let apiKey = $state("");
  let namespace = $state("");
  let namedVectors = $state(false);
  let vectorProvider = $state<VectorProviderId>("qdrant");

  let qdrantStatus = $state<QdrantConnectionStatus | null>(null);
  let ollamaStatus = $state<"untested" | "testing" | "live" | "dead">("untested");
  let host = $state<InferenceHostProfile | null>(null);
  let models = $state<OllamaModelInsight[]>([]);
  let localhost = $state<LocalhostServiceStatus | null>(null);
  let envOllamaHint = $state<string | null>(null);
  let busy = $state(false);
  let saveMsg = $state("");
  let errorMsg = $state("");
  let pullName = $state("");
  let pullProgress = $state("");
  let updateAllProgress = $state<{ current: number; total: number; model: string } | null>(null);
  let ollamaUpdate = $state<ServiceUpdateCheck | null>(null);
  let qdrantUpdate = $state<ServiceUpdateCheck | null>(null);
  let providerProbe = $state("");
  let monitorTimer: ReturnType<typeof setInterval> | null = null;
  let unlistenPull: (() => void) | null = null;

  let providerMeta = $derived(PROVIDER_META[vectorProvider]);
  let embedModels = $derived(models.filter((m) => m.role === "embed" || isEmbedModel(m.name)));
  let chatModels = $derived(models.filter((m) => m.role !== "embed" && !isEmbedModel(m.name)));
  let needsSetup = $derived(!ollamaUrl.trim() && !qdrantUrl.trim());
  let researchReady = $derived(providerMeta.dense);
  let usingLocalhost = $derived(
    ollamaUrl.trim().includes("127.0.0.1") ||
      ollamaUrl.trim().includes("localhost") ||
      qdrantUrl.trim().includes("127.0.0.1") ||
      qdrantUrl.trim().includes("localhost")
  );
  let showLocalhostMissing = $derived(
    usingLocalhost &&
      !!localhost &&
      (!localhost.ollama_reachable || (vectorProvider === "qdrant" && !localhost.qdrant_reachable))
  );

  function fmtGiB(bytes?: number | null): string {
    if (!bytes || bytes <= 0) return "—";
    return `${(bytes / 1e9).toFixed(1)} GiB`;
  }

  function postureLabel(p?: string): string {
    switch (p) {
      case "unified_memory":
        return "Unified memory";
      case "discrete_or_dedicated_gpu":
        return "Dedicated GPU";
      case "remote_managed":
        return "Remote host";
      case "cpu_only_local":
        return "Local CPU-only";
      default:
        return p || "Unknown";
    }
  }

  function parseProvider(raw?: string | null): VectorProviderId {
    const p = (raw || "qdrant").trim().toLowerCase();
    if (p === "pinecone" || p === "chroma" || p === "weaviate") return p;
    return "qdrant";
  }

  async function openExternal(url: string) {
    try {
      await openUrl(url);
    } catch {
      try {
        window.open(url, "_blank", "noopener,noreferrer");
      } catch {
        /* ignore */
      }
    }
  }

  async function loadSaved() {
    try {
      const [cfg, url, svc] = await Promise.all([
        getQdrantConfig(),
        resolveInitialOllamaUrl(),
        getOllamaServiceConfig().catch(() => null),
      ]);
      config = cfg;
      qdrantUrl = cfg.url || "";
      collection = cfg.collection || "";
      embedModel = cfg.embedding_model || "";
      namedVectors = !!cfg.named_vectors_enabled;
      namespace = cfg.namespace || "";
      vectorProvider = parseProvider(cfg.vector_provider);
      ollamaUrl = url;
      onOllamaUrlChange?.(url);
      envOllamaHint = svc?.env_url?.trim() || null;
    } catch (e: any) {
      errorMsg = e?.message || String(e);
    }
  }

  async function refreshLocalhost() {
    localhost = await probeLocalhostServices();
  }

  async function refreshOllamaDiscovery(explicit = false) {
    const url = ollamaUrl.trim();
    if (!url) {
      ollamaStatus = "dead";
      models = [];
      return;
    }
    ollamaStatus = "testing";
    try {
      const report = await withInvokeTimeout(
        discoverOllamaModels(url, ollamaToken || undefined),
        connectionCheckTimeoutMs(url, explicit),
        "Ollama discovery"
      );
      host = report.host;
      models = report.models;
      ollamaStatus = report.ollama_reachable ? "live" : "dead";
      if (!report.ollama_reachable) errorMsg = report.error || "Ollama unreachable";
      if (!embedModel && embedModels.length) embedModel = embedModels[0].name;
    } catch (e: any) {
      ollamaStatus = "dead";
      models = [];
      errorMsg = e?.message || String(e);
    }
  }

  async function refreshVector(explicit = false) {
    const url = qdrantUrl.trim();
    if (!url) {
      qdrantStatus = null;
      return;
    }
    if (vectorProvider === "qdrant") {
      try {
        qdrantStatus = await withInvokeTimeout(
          testQdrantConnection(url, apiKey.trim() || undefined, collection.trim() || undefined),
          connectionCheckTimeoutMs(url, explicit),
          "Qdrant check"
        );
        providerProbe = "";
      } catch (e: any) {
        qdrantStatus = {
          success: false,
          collection_exists: false,
          error: e?.message || String(e),
        };
      }
      return;
    }
    try {
      const probe = await probeVectorProvider(vectorProvider, url, apiKey.trim() || undefined);
      providerProbe = String(probe.note || (probe.reachable ? "Reachable" : "Unreachable"));
      qdrantStatus = {
        success: !!probe.reachable,
        collection_exists: !!probe.collection_exists,
        vectors_count: typeof probe.vectors_count === "number" ? probe.vectors_count : undefined,
        collections: Array.isArray(probe.collections) ? (probe.collections as string[]) : undefined,
        error: probe.reachable ? undefined : providerProbe,
      };
    } catch (e: any) {
      providerProbe = e?.message || String(e);
      qdrantStatus = { success: false, collection_exists: false, error: providerProbe };
    }
  }

  async function runMonitor(explicit = false) {
    if (busy) return;
    busy = true;
    if (explicit) errorMsg = "";
    try {
      host = await probeInferenceHost(ollamaUrl.trim() || undefined);
      await Promise.all([refreshOllamaDiscovery(explicit), refreshVector(explicit), refreshLocalhost()]);
    } finally {
      busy = false;
    }
  }

  async function handleSave() {
    busy = true;
    saveMsg = "";
    errorMsg = "";
    try {
      if (!qdrantUrl.trim()) throw new Error("Set a vector DB URL before saving.");
      if (providerMeta.needsCollection && !collection.trim()) {
        throw new Error(`Set a ${providerMeta.collectionLabel.toLowerCase()} before saving.`);
      }
      if (providerMeta.needsApiKey && !apiKey.trim() && !config.api_key_set) {
        throw new Error(`${providerMeta.label} requires an API key.`);
      }
      if (vectorProvider !== "qdrant") {
        namedVectors = false;
      }
      await saveQdrantConfig({
        url: qdrantUrl.trim(),
        collection: collection.trim() || "genomics_evidence",
        embedding_model: embedModel.trim(),
        gwas_strict: config.gwas_strict,
        auto_start: config.auto_start,
        named_vectors_enabled: namedVectors,
        vector_provider: vectorProvider,
        namespace: namespace.trim(),
        ...(apiKey.trim() ? { api_key: apiKey.trim() } : {}),
      });
      await persistOllamaUrl(ollamaUrl.trim());
      await saveOllamaToken(ollamaToken.trim() || undefined);
      onOllamaUrlChange?.(ollamaUrl.trim());
      onOllamaTokenChange?.(ollamaToken.trim());
      apiKey = "";
      envOllamaHint = null;
      saveMsg = "Saved. UI settings now override .env until you clear them.";
      busy = false;
      await runMonitor(true);
    } catch (e: any) {
      errorMsg = e?.message || String(e);
      saveMsg = "";
      busy = false;
    }
  }

  async function resetToLocalhost() {
    ollamaUrl = LOCAL_OLLAMA_URL;
    qdrantUrl = LOCAL_QDRANT_URL;
    vectorProvider = "qdrant";
    namespace = "";
    await refreshLocalhost();
    if (localhost && (!localhost.ollama_reachable || !localhost.qdrant_reachable)) {
      errorMsg = [
        !localhost.ollama_reachable ? localhost.ollama_error : null,
        !localhost.qdrant_reachable ? localhost.qdrant_error : null,
        ...localhost.notes,
      ]
        .filter(Boolean)
        .join(" · ");
    } else {
      errorMsg = "";
      saveMsg = "Filled localhost URLs — click Save & Verify to persist.";
    }
  }

  async function adoptEnvOllama() {
    if (!envOllamaHint) return;
    ollamaUrl = envOllamaHint;
    await persistOllamaUrl(ollamaUrl);
    envOllamaHint = null;
    saveMsg = "Adopted OLLAMA_URL from .env and saved.";
    await runMonitor(true);
  }

  async function handlePull(name: string, asUpdate = false) {
    const model = name.trim();
    if (!model || !ollamaUrl.trim()) return;
    busy = true;
    pullProgress = asUpdate ? `Updating ${model}…` : `Pulling ${model}…`;
    errorMsg = "";
    try {
      await pullOllamaModel(ollamaUrl.trim(), model, ollamaToken || undefined);
      pullProgress = `${asUpdate ? "Updated" : "Installed"} ${model}`;
      pullName = "";
      busy = false;
      await refreshOllamaDiscovery(true);
    } catch (e: any) {
      errorMsg = e?.message || String(e);
      pullProgress = "";
      busy = false;
    }
  }

  async function handleUpdateAll() {
    if (!ollamaUrl.trim() || models.length === 0) return;
    if (
      !confirm(
        `Re-pull all ${models.length} models from the Ollama registry? This can take a while and will stream progress per model.`
      )
    ) {
      return;
    }
    busy = true;
    errorMsg = "";
    const list = models.map((m) => m.name);
    let failed = 0;
    for (let i = 0; i < list.length; i++) {
      const model = list[i];
      updateAllProgress = { current: i + 1, total: list.length, model };
      pullProgress = `Updating ${i + 1}/${list.length}: ${model}`;
      try {
        await pullOllamaModel(ollamaUrl.trim(), model, ollamaToken || undefined);
      } catch (e: any) {
        failed += 1;
        errorMsg = `Failed on ${model}: ${e?.message || String(e)}`;
      }
    }
    updateAllProgress = null;
    pullProgress =
      failed === 0
        ? `Updated all ${list.length} models`
        : `Finished with ${failed} failure(s) of ${list.length}`;
    busy = false;
    await refreshOllamaDiscovery(true);
  }

  async function handleDelete(name: string) {
    if (!confirm(`Delete model "${name}" from the Ollama server? This cannot be undone.`)) return;
    busy = true;
    try {
      await deleteOllamaModel(ollamaUrl.trim(), name, ollamaToken || undefined);
      await refreshOllamaDiscovery(true);
      saveMsg = `Deleted ${name}`;
    } catch (e: any) {
      errorMsg = e?.message || String(e);
    } finally {
      busy = false;
    }
  }

  async function handleCheckUpdates() {
    busy = true;
    errorMsg = "";
    try {
      const tasks: Promise<void>[] = [];
      if (ollamaUrl.trim()) {
        tasks.push(
          checkOllamaUpdate(ollamaUrl.trim(), ollamaToken || undefined).then((r) => {
            ollamaUpdate = r;
          })
        );
      }
      if (qdrantUrl.trim() && vectorProvider === "qdrant") {
        tasks.push(
          checkQdrantUpdate(qdrantUrl.trim(), apiKey.trim() || undefined).then((r) => {
            qdrantUpdate = r;
          })
        );
      }
      await Promise.all(tasks);
    } catch (e: any) {
      errorMsg = e?.message || String(e);
    } finally {
      busy = false;
    }
  }

  onMount(() => {
    void (async () => {
      unlistenPull = await listen<Record<string, unknown>>("ollama:pull_progress", (ev) => {
        const s = ev.payload?.status;
        const model = ev.payload?.model;
        const completed = ev.payload?.completed;
        const total = ev.payload?.total;
        const modelLabel = typeof model === "string" && model ? `${model}: ` : "";
        if (typeof s === "string") {
          const pct =
            completed && total
              ? ` (${Math.round((Number(completed) / Number(total)) * 100)}%)`
              : "";
          pullProgress = `${modelLabel}${s}${pct}`;
        }
      });
      await loadSaved();
      await runMonitor(false);
      monitorTimer = setInterval(() => void runMonitor(false), 30000);
    })();
  });

  onDestroy(() => {
    if (monitorTimer) clearInterval(monitorTimer);
    unlistenPull?.();
  });
</script>

<section class="connections-panel" aria-labelledby="connections-title">
  <header class="connections-hero">
    <div>
      <p class="connections-kicker">Advanced · Connections</p>
      <h3 id="connections-title">Ollama &amp; vector database</h3>
      <p class="connections-lead">
        Saved settings in the app win over <code>.env</code>. Point Ollama and your vector store at
        localhost, LAN (e.g. a home server), Tailscale, or cloud — Qdrant is not required.
      </p>
    </div>
    <div class="connections-actions hero-actions">
      <button type="button" class="btn btn-secondary btn-sm" onclick={resetToLocalhost} disabled={busy}>
        Reset to localhost
      </button>
      <button type="button" class="btn btn-secondary btn-sm" onclick={() => runMonitor(true)} disabled={busy}>
        Re-scan
      </button>
      <button type="button" class="btn btn-primary btn-sm" onclick={handleSave} disabled={busy}>
        Save &amp; Verify
      </button>
    </div>
  </header>

  {#if needsSetup}
    <div class="connections-empty">
      No URLs saved yet. Enter endpoints below, or <strong>Reset to localhost</strong> then save.
    </div>
  {/if}

  {#if envOllamaHint}
    <div class="connections-banner warn">
      <span
        ><code>.env</code> has a different Ollama URL: <code>{envOllamaHint}</code> (not applied — your
        saved setting wins).</span
      >
      <button type="button" class="btn btn-secondary btn-xs" onclick={adoptEnvOllama}>Adopt .env</button>
    </div>
  {/if}

  {#if showLocalhostMissing}
    <div class="connections-banner warn">
      <div>
        <strong>Localhost check:</strong>
        {#if localhost && !localhost.ollama_reachable}Ollama missing on {LOCAL_OLLAMA_URL}. {/if}
        {#if vectorProvider === "qdrant" && localhost && !localhost.qdrant_reachable}
          Qdrant missing on {LOCAL_QDRANT_URL}.
        {/if}
        Point Connections at a remote/LAN host, or install/start the missing service.
      </div>
    </div>
  {/if}

  {#if errorMsg}
    <div class="connections-alert" role="alert">{errorMsg}</div>
  {/if}

  <div class="conn-status-row">
    <span class="conn-pill {ollamaStatus}">Ollama · {ollamaStatus}</span>
    <span class="conn-pill {qdrantStatus?.success ? 'live' : qdrantStatus ? 'dead' : 'untested'}">
      {providerMeta.label} · {qdrantStatus?.success ? "live" : qdrantStatus ? "dead" : "untested"}
      {#if qdrantStatus?.collection_exists} · index ready{/if}
    </span>
    {#if saveMsg}<span class="conn-pill live">{saveMsg}</span>{/if}
    {#if pullProgress}<span class="conn-pill testing">{pullProgress}</span>{/if}
  </div>

  <div class="connections-grid">
    <div class="connections-card">
      <h4>1 · Inference (Ollama)</h4>
      <p class="field-hint">
        Models come from the Ollama registry on the host you point at — not from this app’s install.
      </p>
      <div class="field">
        <label for="conn-ollama-url">Ollama URL</label>
        <input
          id="conn-ollama-url"
          type="url"
          bind:value={ollamaUrl}
          placeholder={OLLAMA_URL_PLACEHOLDER}
          autocomplete="off"
        />
      </div>
      <div class="field">
        <label for="conn-ollama-token">Ollama token (optional)</label>
        <input id="conn-ollama-token" type="password" bind:value={ollamaToken} placeholder="Bearer …" />
      </div>
      <div class="field">
        <label for="conn-embed">Default embedding model</label>
        <select id="conn-embed" bind:value={embedModel}>
          {#if embedModel && !embedModels.some((m) => m.name === embedModel)}
            <option value={embedModel}>{embedModel}</option>
          {/if}
          {#each embedModels as m}
            <option value={m.name}>{m.name} · {m.load_hint}</option>
          {/each}
        </select>
        <p class="field-hint">Used for research vector indexing. Pick an embed-capable tag.</p>
      </div>
    </div>

    <div class="connections-card">
      <h4>2 · Vector store</h4>
      <div class="field">
        <label for="conn-provider">Provider</label>
        <select id="conn-provider" bind:value={vectorProvider}>
          {#each Object.entries(PROVIDER_META) as [id, meta]}
            <option value={id}>{meta.label}{meta.namedVectors ? " · full" : " · dense"}</option>
          {/each}
        </select>
      </div>

      <div class="capability-row" aria-label="Provider capabilities">
        <span class="cap-chip {providerMeta.dense ? 'on' : 'off'}">Dense research</span>
        <span class="cap-chip {providerMeta.namedVectors ? 'on' : 'off'}">Named vectors</span>
        <span class="cap-chip {vectorProvider === 'qdrant' ? 'on' : 'off'}">Payload indexes</span>
      </div>
      <p class="field-hint">{providerMeta.hosting}</p>

      <div class="field">
        <label for="conn-qdrant-url">{providerMeta.urlLabel}</label>
        <input
          id="conn-qdrant-url"
          type="url"
          bind:value={qdrantUrl}
          placeholder={providerMeta.urlPlaceholder}
          autocomplete="off"
        />
      </div>
      <div class="field">
        <label for="conn-qdrant-key">API key {providerMeta.needsApiKey ? "(required)" : "(optional)"}</label>
        <input
          id="conn-qdrant-key"
          type="password"
          bind:value={apiKey}
          placeholder={config.api_key_set ? "•••• (keep existing)" : "API key"}
        />
      </div>
      {#if providerMeta.needsCollection || vectorProvider === "pinecone"}
        <div class="field">
          <label for="conn-collection">{providerMeta.collectionLabel}</label>
          <input
            id="conn-collection"
            type="text"
            bind:value={collection}
            placeholder="genomics_evidence"
            list="conn-collections"
          />
          <datalist id="conn-collections">
            {#each qdrantStatus?.collections ?? [] as name}
              <option value={name}>{name}</option>
            {/each}
          </datalist>
          <p class="field-hint">{providerMeta.collectionHint}</p>
        </div>
      {/if}
      {#if vectorProvider === "pinecone"}
        <div class="field">
          <label for="conn-namespace">Namespace</label>
          <input id="conn-namespace" type="text" bind:value={namespace} placeholder="__default__ or sample scope" />
          <p class="field-hint">Pinecone namespace for upserts/queries. Empty uses <code>__default__</code>.</p>
        </div>
      {/if}
      {#if vectorProvider === "qdrant"}
        <label class="check-row">
          <input type="checkbox" bind:checked={namedVectors} />
          <span>Enable named vectors (Qdrant-only multi-space indexing)</span>
        </label>
      {/if}
      {#if providerProbe}
        <p class="field-hint">{providerProbe}</p>
      {/if}
      {#if !researchReady}
        <p class="field-hint warn-text">This provider does not support research sweeps yet.</p>
      {/if}
    </div>
  </div>

  <div class="connections-card">
    <h4>Host posture &amp; service versions</h4>
    {#if host}
      <div class="host-meta">
        <span class="host-chip">{host.platform}/{host.arch}</span>
        <span class="host-chip">{postureLabel(host.posture)}</span>
        {#each host.accel_backends as b}
          <span class="host-chip">{b}</span>
        {/each}
        <span class="host-chip">accel {fmtGiB(host.accel_bytes)}</span>
        <span class="host-chip">VRAM in use {fmtGiB(host.observed_vram_in_use_bytes)}</span>
      </div>
      <ul class="host-notes">
        {#each host.notes.slice(0, 5) as note}
          <li>{note}</li>
        {/each}
      </ul>
    {:else}
      <p class="connections-lead">Probe pending…</p>
    {/if}

    <div class="connections-actions">
      <button type="button" class="btn btn-secondary btn-sm" onclick={handleCheckUpdates} disabled={busy}>
        Check for updates
      </button>
    </div>
    {#if ollamaUpdate}
      <div class="update-box">
        <strong>
          Ollama {ollamaUpdate.installed_version || "?"}
          {#if ollamaUpdate.latest_version}
            · latest {ollamaUpdate.latest_version}
            {#if ollamaUpdate.update_available} (update available){/if}
          {/if}
        </strong>
        <ul class="host-notes">
          {#each ollamaUpdate.notes as n}<li>{n}</li>{/each}
        </ul>
      </div>
    {/if}
    {#if qdrantUpdate}
      <div class="update-box">
        <strong>
          Qdrant {qdrantUpdate.installed_version || "?"}
          {#if qdrantUpdate.latest_version}
            · latest {qdrantUpdate.latest_version}
            {#if qdrantUpdate.update_available} (update available){/if}
          {/if}
        </strong>
        <ul class="host-notes">
          {#each qdrantUpdate.notes as n}<li>{n}</li>{/each}
        </ul>
      </div>
    {/if}
  </div>

  <div class="connections-card">
    <div class="models-header">
      <div>
        <h4>Ollama models</h4>
        <p class="field-hint">
          Find tags on the Ollama library, then install by exact name here. Embed models power research;
          chat models power the agent.
        </p>
      </div>
      <div class="connections-actions">
        <button
          type="button"
          class="btn btn-secondary btn-sm"
          disabled={busy}
          onclick={() => openExternal(OLLAMA_LIBRARY_URL)}
        >
          Browse library
        </button>
        <button
          type="button"
          class="btn btn-secondary btn-sm"
          disabled={busy}
          onclick={() => openExternal(OLLAMA_SEARCH_URL)}
        >
          Search models
        </button>
        <button
          type="button"
          class="btn btn-accent btn-sm"
          disabled={busy || models.length === 0 || !ollamaUrl.trim()}
          onclick={handleUpdateAll}
        >
          Update all
        </button>
      </div>
    </div>

    <div class="suggest-row">
      <span class="suggest-label">Suggested embed:</span>
      {#each SUGGESTED_EMBED as tag}
        <button
          type="button"
          class="suggest-chip"
          disabled={busy}
          onclick={() => {
            pullName = tag;
          }}>{tag}</button
        >
      {/each}
      <span class="suggest-label">Suggested chat:</span>
      {#each SUGGESTED_CHAT as tag}
        <button
          type="button"
          class="suggest-chip"
          disabled={busy}
          onclick={() => {
            pullName = tag;
          }}>{tag}</button
        >
      {/each}
    </div>

    <div class="pull-row">
      <input
        type="text"
        bind:value={pullName}
        placeholder="Exact Ollama tag (e.g. mxbai-embed-large, qwen2.5:14b)"
        aria-label="Model name to install"
      />
      <button
        type="button"
        class="btn btn-accent btn-sm"
        disabled={busy || !pullName.trim() || !ollamaUrl.trim()}
        onclick={() => handlePull(pullName, false)}
      >
        Install
      </button>
    </div>

    {#if updateAllProgress}
      <div class="update-all-bar" role="status" aria-live="polite">
        <div class="update-all-track">
          <div
            class="update-all-fill"
            style={`width: ${(updateAllProgress.current / updateAllProgress.total) * 100}%`}
          ></div>
        </div>
        <p class="field-hint">
          {updateAllProgress.current}/{updateAllProgress.total} · {updateAllProgress.model}
        </p>
      </div>
    {/if}

    <p class="field-hint">
      Install / Update use <code>POST /api/pull</code> on your Ollama host. Update all re-pulls every listed
      tag sequentially with live progress.
    </p>

    {#if models.length === 0}
      <div class="connections-empty">Connect Ollama and scan to list models.</div>
    {:else}
      <div class="model-table-wrap">
        <table class="model-table">
          <thead>
            <tr>
              <th>Model</th>
              <th>Role</th>
              <th>Hint</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {#each models as m (m.name)}
              <tr>
                <td class="font-mono">{m.name}{#if m.currently_loaded} · loaded{/if}</td>
                <td>{m.role}</td>
                <td class="hint-{m.load_hint}">{m.load_hint}</td>
                <td class="model-actions">
                  <button
                    type="button"
                    class="btn btn-link btn-xs"
                    disabled={busy}
                    onclick={() => handlePull(m.name, true)}>Update</button
                  >
                  <button
                    type="button"
                    class="btn btn-link btn-xs danger"
                    disabled={busy}
                    onclick={() => handleDelete(m.name)}>Remove</button
                  >
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
      {#if chatModels.length === 0}
        <p class="field-hint">No chat models detected yet — search the library and Install a chat tag.</p>
      {/if}
    {/if}
  </div>
</section>
