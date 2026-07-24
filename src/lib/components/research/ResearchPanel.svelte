<!-- ./src/lib/components/research/ResearchPanel.svelte -->
<script lang="ts">
  import { onMount } from "svelte";
  import "$lib/styles/components/research-panel.css";
  import "$lib/styles/components/research-connection-card.css";
  import "$lib/styles/components/research-scope-section.css";
  import "$lib/styles/components/research-job-controls.css";
  import "$lib/styles/components/research-live-log.css";
  import {
    startResearchEventListeners,
  } from "../../research/researchEvents";
  import {
    getQdrantConfig,
    getResearchJobStatus,
    startResearchJob,
    pauseResearchJob,
    cancelResearchJob,
    resumeResearchJob,
    createQdrantCollection,
    saveResearchScope,
    testQdrantConnection,
    getRecentFindingPreviews,
    scanOllamaModels,
  } from "../../api/tauri";
  import {
    shouldAutoCheckConnections,
    withInvokeTimeout,
    connectionCheckTimeoutMs,
  } from "../../utils/researchConnection";
  import type { OllamaConnectionState } from "../../utils/researchRunReadiness";
  import type {
    QdrantConfigPublic,
    QdrantConnectionStatus,
    ResearchJob,
    ConnectionActivity,
    ResearchScopePreview,
  } from "../../types/research";
  import { DEFAULT_QDRANT_CONFIG, DEFAULT_RESEARCH_SCOPE, DEFAULT_CONNECTION_ACTIVITY, DEFAULT_ENRICHMENT_SOURCES, FULL_ENRICHMENT_SOURCES } from "../../types/research";
  import {
    liveProgress,
    liveDebugLines,
    liveFindingPreviews,
    researchEventContext,
    resetLiveProgress,
    resetLiveProgressForResume,
    applyLiveFieldsFromJob,
  } from "../../research/liveProgress.svelte";
  import ResearchConnectionCard from "./ResearchConnectionCard.svelte";
  import ResearchJobControls from "./ResearchJobControls.svelte";
  import ResearchLiveFindings from "./ResearchLiveFindings.svelte";
  import ResearchLiveLog from "./ResearchLiveLog.svelte";
  import ResearchScopeSection from "./ResearchScopeSection.svelte";
  import VectorViewerPanel from "./VectorViewerPanel.svelte";
  import PackDraftPanel from "./PackDraftPanel.svelte";
  import ResearchFoundReviewPanel from "./ResearchFoundReviewPanel.svelte";
  import VectorAtlasPanel from "../ai/evidence/VectorAtlasPanel.svelte";
  import ActivityPulse from "../common/loading/ActivityPulse.svelte";
  import "$lib/styles/components/vector-workbench.css";

  function formatSweepStartError(raw: string): string {
    if (/database is locked|database table is locked/i.test(raw)) {
      return "Database busy — another operation holds a SQLite lock (sync/import or a previous sweep). Wait a few seconds and start again.";
    }
    return raw;
  }

  type WorkbenchTab = "sweep" | "viewer" | "map" | "packs" | "found";

  interface Props {
    selectedSample: { id: number; name: string } | null;
    ollamaUrl: string;
    ollamaToken?: string;
    job?: ResearchJob | null;
  }

  let {
    selectedSample = null,
    ollamaUrl = $bindable(""),
    ollamaToken = $bindable(""),
    job = $bindable(null)
  }: Props = $props();

  let config = $state<QdrantConfigPublic>({ ...DEFAULT_QDRANT_CONFIG });
  let researchScope = $state({ ...DEFAULT_RESEARCH_SCOPE });
  let isConfigLoaded = $state(false);
  let connectionStatus = $state<QdrantConnectionStatus | null>(null);
  let connectionActivity = $state<ConnectionActivity>({ ...DEFAULT_CONNECTION_ACTIVITY });
  let isStarting = $state(false);
  let isCancelling = $state(false);
  let isCreatingCollection = $state(false);
  let liveLog = $state<string[]>([]);
  let researchDebugLog = $state(false);
  let scopePreview = $state<ResearchScopePreview | null>(null);
  let scopePreviewLoading = $state(false);
  let ollamaStatus = $state<OllamaConnectionState>("untested");
  let lastFeedPulseAt = $state(0);
  let lastFeedPhaseKey = $state("");
  let lastLoggedActivity = $state("");
  let gnomadSweepReadyState = $state(true);
  let workbenchTab = $state<WorkbenchTab>("sweep");
  let inspectRsid = $state("");

  /** Block workbench actions until config + connection probes settle. */
  let workbenchReady = $derived(
    isConfigLoaded &&
      connectionActivity.phase !== "checking-qdrant" &&
      connectionActivity.phase !== "checking-ollama" &&
      connectionActivity.phase !== "loading-scope" &&
      ollamaStatus !== "testing"
  );

  let workbenchBootMessage = $derived.by(() => {
    if (!isConfigLoaded) return "Loading saved connection settings…";
    if (connectionActivity.phase === "checking-qdrant") return "Checking vector database…";
    if (connectionActivity.phase === "checking-ollama" || ollamaStatus === "testing") {
      return "Checking Ollama embedding models…";
    }
    if (connectionActivity.phase === "loading-scope") return "Preparing research scope…";
    return "Finishing startup checks…";
  });

  function openRsidInViewer(rsid: string) {
    inspectRsid = rsid;
    workbenchTab = "viewer";
  }
  const sweepLoopActive = $derived(
    job?.status === "running" && job?.loop_active !== false
  );

  const gnomadSweepReady = $derived(
    researchScope.enrichment_sources?.gnomad ? gnomadSweepReadyState : true
  );

  const FEED_PULSE_INTERVAL_MS = 3_000;
  const JOB_POLL_INTERVAL_MS = 2_000;
  let lastPolledEnriched = $state<number | null>(null);

  function isHeartbeatMessage(msg: string): boolean {
    return (
      msg.startsWith("gnomAD prefetch") ||
      msg.startsWith("API prefetch") ||
      msg.startsWith("prepare ") ||
      msg.startsWith("prepare") ||
      msg.startsWith("embedding")
    );
  }

  function isMilestoneMessage(msg: string): boolean {
    const lower = msg.toLowerCase();
    return (
      msg.includes("error") ||
      msg.includes("skipped") ||
      lower.includes("enriched") ||
      lower.includes("resumed") ||
      lower.includes("started") ||
      lower.includes("paused") ||
      lower.includes("cancelled") ||
      lower.includes("complete") ||
      lower.includes("fatal")
    );
  }

  function maybeLogActivity(
    msg: string,
    phase: string | null | undefined,
    rsid: string | null | undefined
  ) {
    if (msg === lastLoggedActivity) return;

    // Always surface verbose debug lines in the feed.
    if (msg.startsWith("[sweep]") || msg.startsWith("[enrich]") || msg.startsWith("[prefetch]")) {
      lastLoggedActivity = msg;
      pushLog(msg);
      return;
    }

    if (isMilestoneMessage(msg) || !isHeartbeatMessage(msg)) {
      lastLoggedActivity = msg;
      pushLog(msg);
      return;
    }

    const phaseKey = `${phase ?? ""}|${rsid ?? ""}|${msg.split(" ")[0] ?? ""}`;
    const now = Date.now();
    if (phaseKey !== lastFeedPhaseKey || now - lastFeedPulseAt >= FEED_PULSE_INTERVAL_MS) {
      lastFeedPulseAt = now;
      lastFeedPhaseKey = phaseKey;
      lastLoggedActivity = msg;
      pushLog(`⟳ ${msg}`);
    }
  }

  $effect(() => {
    const msg = liveProgress.lastActivityMessage;
    void liveProgress.tick;
    if (!msg) return;
    maybeLogActivity(msg, liveProgress.activityPhase, job?.current_rsid ?? null);
  });

  $effect(() => {
    researchEventContext.debugEnabled = researchDebugLog;
  });

  let mergedDebugCount = $state(0);

  $effect(() => {
    if (!researchDebugLog) {
      mergedDebugCount = 0;
      return;
    }
    const n = liveDebugLines.length;
    if (n <= mergedDebugCount) return;
    const added = liveDebugLines.slice(0, n - mergedDebugCount);
    mergedDebugCount = n;
    for (let i = added.length - 1; i >= 0; i--) {
      const body = added[i].replace(/^\[[^\]]+\]\s*/, "");
      pushLog(body);
    }
  });

  async function pollJobStatus(sampleId: number) {
    try {
      const fresh = await getResearchJobStatus(sampleId);
      if (!fresh) return;

      // While cancel is in flight, never let a racing poll resurrect running/paused.
      if (
        isCancelling &&
        (fresh.status === "running" || fresh.status === "paused")
      ) {
        job = {
          ...fresh,
          status: "idle",
          loop_active: fresh.loop_active === true,
          error_message: fresh.error_message ?? "Sweep cancelled by user.",
        };
        return;
      }

      const loopStillActive = job?.loop_active === true;
      const prevEnriched = job?.enriched_count ?? lastPolledEnriched;
      applyLiveFieldsFromJob(fresh);

      job = {
        ...fresh,
        loop_active:
          fresh.loop_active === true
            ? true
            : fresh.status === "running"
              ? true
              : fresh.status === "idle" ||
                  fresh.status === "complete" ||
                  fresh.status === "error"
                ? false
                : loopStillActive && fresh.status === "paused"
                  ? true
                  : fresh.loop_active,
      };

      if (fresh.status === "running") {
        const msg = fresh.live_message ?? liveProgress.lastActivityMessage;
        if (msg && fresh.enriched_count !== prevEnriched) {
          pushLog(
            `Indexed ${fresh.enriched_count.toLocaleString()} / ${fresh.total_markers.toLocaleString()} — ${msg}`,
          );
          lastPolledEnriched = fresh.enriched_count;
        } else if (msg) {
          maybeLogActivity(msg, fresh.activity_phase ?? liveProgress.activityPhase, fresh.current_rsid ?? null);
        }
      }
    } catch (e) {
      console.warn("Research job poll failed:", e);
    }
  }

  $effect(() => {
    if (!researchScope.enrichment_sources?.gnomad) {
      gnomadSweepReadyState = true;
    }
  });

  async function loadConfig() {
    try {
      config = await getQdrantConfig();
      researchScope = { ...(config.research_scope ?? DEFAULT_RESEARCH_SCOPE) };
    } catch (e) {
      console.warn("Could not load Qdrant config, using default:", e);
      config = { ...DEFAULT_QDRANT_CONFIG };
    } finally {
      isConfigLoaded = true;
    }
  }

  async function loadJob(sampleId: number) {
    try {
      const loaded = await getResearchJobStatus(sampleId);
      job = loaded;
      if (loaded) {
        applyLiveFieldsFromJob(loaded);
      }
      if (loaded?.status === "running") {
        pushLog("Monitoring active research sweep.");
      }
    } catch (e) {
      console.error("Failed to load research job status:", e);
      job = null;
    }
  }

  async function reloadJob(sampleId: number) {
    await loadJob(sampleId);
  }

  function handleJobReset() {
    if (selectedSample) {
      void reloadJob(selectedSample.id);
    } else {
      job = null;
    }
    pushLog("Enrichment progress reset after collection purge — start a fresh sweep.");
  }

  function pushLog(msg: string) {
    const timestamp = new Date().toLocaleTimeString();
    liveLog = [`[${timestamp}] ${msg}`, ...liveLog].slice(0, 50);
  }

  function syncScopeSourcesBeforeRun() {
    if (!researchScope.enrichment_sources) {
      researchScope.enrichment_sources = researchScope.sweep_fast
        ? { ...DEFAULT_ENRICHMENT_SOURCES }
        : { ...FULL_ENRICHMENT_SOURCES };
    }
    const s = researchScope.enrichment_sources;
    researchScope.sweep_fast =
      !s.gnomad &&
      !s.clinvar_live &&
      !s.pubmed &&
      !s.gtex &&
      !s.vep_dbsnp &&
      !s.secondary;
  }

  async function ensureConnectionsBeforeRun(): Promise<boolean> {
    const qdrantOk =
      connectionStatus?.success && connectionStatus.collection_exists;
    const ollamaOk = ollamaStatus === "live";
    if (qdrantOk && ollamaOk) {
      return true;
    }

    pushLog("Verifying Qdrant and Ollama before sweep…");
    connectionActivity = { phase: "checking-qdrant", message: "Pinging Qdrant server…" };

    try {
      const status = await withInvokeTimeout(
        testQdrantConnection(config.url, undefined, config.collection),
        connectionCheckTimeoutMs(config.url, true),
        "Qdrant connection check"
      );
      connectionStatus = status;
      if (!status.success) {
        pushLog(`Qdrant check failed: ${status.error || "unknown error"}`);
        connectionActivity = { phase: "error", message: status.error || "Qdrant failed" };
        return false;
      }
      if (!status.collection_exists) {
        pushLog(
          `Collection '${config.collection}' not found on Qdrant. Create it or check the collection name in settings.`
        );
        connectionActivity = { phase: "ready", message: "Qdrant reachable — collection missing" };
        return false;
      }
      pushLog(
        `Qdrant ready — ${status.vectors_count?.toLocaleString() ?? 0} vectors in '${config.collection}'.`
      );

      connectionActivity = { phase: "checking-ollama", message: "Scanning Ollama embedding models…" };
      ollamaStatus = "testing";
      const models = await withInvokeTimeout(
        scanOllamaModels(ollamaUrl, ollamaToken || undefined),
        connectionCheckTimeoutMs(ollamaUrl, true),
        "Ollama model scan"
      );
      if (!models?.length) {
        ollamaStatus = "dead";
        pushLog("Ollama check failed: no models returned.");
        connectionActivity = { phase: "error", message: "Ollama returned no models" };
        return false;
      }
      ollamaStatus = "live";
      pushLog(`Ollama ready — ${models.length} models available.`);
      connectionActivity = { phase: "ready", message: "Connections verified" };
      return true;
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      ollamaStatus = "dead";
      pushLog(`Connection check failed: ${msg}`);
      connectionActivity = { phase: "error", message: msg };
      return false;
    }
  }

  async function handleStart(options: { forceReenrich?: boolean } = {}) {
    if (!selectedSample) return;
    isStarting = true;
    syncScopeSourcesBeforeRun();
    try {
      await saveResearchScope(researchScope);
    } catch (e: any) {
      pushLog(`Failed to save scope before start: ${e.message || String(e)}`);
    }

    if (!(await ensureConnectionsBeforeRun())) {
      isStarting = false;
      return;
    }

    liveLog = [];
    resetLiveProgress();
    lastFeedPulseAt = 0;
    lastFeedPhaseKey = "";
    lastLoggedActivity = "";
    pushLog(
      options?.forceReenrich
        ? "Force re-enrich: rebuilding all queued variants..."
        : "Initializing research loop..."
    );
    try {
      await startResearchJob(selectedSample.id, ollamaUrl, researchScope, options?.forceReenrich);
      // Refresh immediately so Run Controller flips to Running (don't wait for poll).
      await loadJob(selectedSample.id);
      if (job?.status !== "running") {
        await pollJobStatus(selectedSample.id);
      }
    } catch (e: any) {
      const raw = e?.message || String(e);
      pushLog(`Error: ${formatSweepStartError(raw)}`);
    } finally {
      isStarting = false;
    }
  }

  async function handlePause() {
    if (!selectedSample) return;
    try {
      const updated = await pauseResearchJob(selectedSample.id);
      if (updated) job = updated;
      pushLog("Sweep paused.");
    } catch (e: any) {
      pushLog(`Failed to pause: ${e.message || String(e)}`);
    }
  }

  async function handleCancel() {
    if (!selectedSample || isCancelling) return;
    isCancelling = true;
    pushLog("Cancelling sweep…");
    try {
      const updated = await cancelResearchJob(selectedSample.id);
      // Keep backend loop_active when the worker is still winding down so Start stays blocked.
      job = updated
        ? {
            ...updated,
            status:
              updated.status === "complete" || updated.status === "error"
                ? updated.status
                : "idle",
            loop_active: updated.loop_active === true,
            error_message:
              updated.status === "complete" || updated.status === "error"
                ? updated.error_message
                : updated.error_message ?? "Sweep cancelled by user.",
          }
        : updated;
      // Wait for the cooperative loop to land on idle with loop_active false.
      for (let i = 0; i < 40; i++) {
        if (
          !job ||
          ((job.status === "idle" || job.status === "complete" || job.status === "error") &&
            job.loop_active !== true)
        ) {
          break;
        }
        await new Promise((r) => setTimeout(r, 150));
        await loadJob(selectedSample.id);
        if (job && job.status !== "complete" && job.status !== "error") {
          job = {
            ...job,
            status: "idle",
            // Preserve live loop_active from backend during wind-down.
            loop_active: job.loop_active === true,
            error_message: job.error_message ?? "Sweep cancelled by user.",
          };
        }
      }
      resetLiveProgress();
      liveProgress.progressSpeed = null;
      liveProgress.recentProgressSpeed = null;
      if (job?.status === "idle" || !job) {
        pushLog("Sweep cancelled — ready to start fresh.");
      } else {
        pushLog(
          `Cancel signaled — job status is still '${job.status}'. If it sticks, restart the app and try again.`,
        );
      }
    } catch (e: any) {
      pushLog(`Failed to cancel: ${formatSweepStartError(e?.message || String(e))}`);
    } finally {
      isCancelling = false;
    }
  }

  async function handleResume() {
    if (!selectedSample) return;
    isStarting = true;
    resetLiveProgressForResume();
    lastFeedPulseAt = 0;
    lastFeedPhaseKey = "";
    lastLoggedActivity = "";
    pushLog("Resuming research loop...");
    try {
      if (!(await ensureConnectionsBeforeRun())) {
        pushLog("Resume blocked — fix connection/scope readiness, then try again.");
        return;
      }
      await resumeResearchJob(selectedSample.id, ollamaUrl);
      await loadJob(selectedSample.id);
    } catch (e: any) {
      pushLog(`Failed to resume: ${e.message || String(e)}`);
    } finally {
      isStarting = false;
    }
  }

  async function handleCreateCollection() {
    isCreatingCollection = true;
    try {
      pushLog(`Creating Qdrant collection '${config.collection}'...`);
      const status = await createQdrantCollection(ollamaUrl);
      connectionStatus = status;
      if (status.success && status.collection_exists) {
        pushLog(`Collection '${config.collection}' ready — ${status.vectors_count?.toLocaleString() ?? 0} vectors`);
      } else if (status.success) {
        pushLog(`Collection '${config.collection}' created successfully (empty).`);
      } else {
        pushLog(`Failed to create collection: ${status.error || "unknown error"}`);
      }
    } catch (e: any) {
      pushLog(`Failed to create collection: ${e.message || String(e)}`);
    } finally {
      isCreatingCollection = false;
    }
  }

  $effect(() => {
    const sampleId = selectedSample?.id;
    const running = job?.status === "running";
    if (!sampleId || liveFindingPreviews.length > 0) return;

    void getRecentFindingPreviews(sampleId, 12)
      .then((rows) => {
        if (rows.length === 0) return;
        for (const row of rows.reverse()) {
          if (!liveFindingPreviews.some((f) => f.rsid === row.rsid)) {
            liveFindingPreviews.unshift(row);
          }
        }
        if (liveFindingPreviews.length > 24) {
          liveFindingPreviews.length = 24;
        }
        liveProgress.tick += 1;
        if (!running) {
          pushLog(`Loaded ${rows.length.toLocaleString()} indexed variants from Qdrant for preview.`);
        }
      })
      .catch((e) => {
        console.warn("Could not load recent finding previews:", e);
      });
  });

  $effect(() => {
    if (selectedSample) {
      void loadJob(selectedSample.id);
    } else {
      job = null;
    }
  });

  $effect(() => {
    const sampleId = selectedSample?.id;
    const shouldPoll =
      sampleId != null &&
      (job?.status === "running" || job?.status === "paused");

    if (!shouldPoll || sampleId == null) {
      return;
    }

    void pollJobStatus(sampleId);
    const interval = setInterval(() => {
      void pollJobStatus(sampleId);
    }, JOB_POLL_INTERVAL_MS);

    return () => clearInterval(interval);
  });

  onMount(() => {
    void loadConfig();
    void startResearchEventListeners();
  });
</script>

<div class="research-panel">
  <header class="research-hero">
    <div class="research-hero-icon">🔬</div>
    <div>
      <h1>Vector Research</h1>
      <p>
        Enrich your genome into a vector knowledge library, inspect what was indexed, explore
        association space, and draft stronger curated marker packs. Vector DB is whatever you set in
        Connections (Qdrant / Pinecone / Chroma / Weaviate).
      </p>
    </div>
  </header>

  <div class="research-stack">
    <ResearchConnectionCard
      bind:config
      bind:ollamaUrl
      bind:ollamaToken
      bind:connectionStatus
      bind:connectionActivity
      bind:ollamaStatus
      selectedSampleId={selectedSample?.id ?? null}
      {isConfigLoaded}
      disabled={sweepLoopActive}
      onLog={pushLog}
      onConfigUpdated={loadConfig}
      onJobReset={handleJobReset}
    />

    {#if !selectedSample}
      <div class="glass-card empty-state">
        <div class="empty-icon">🧬</div>
        <h3>No Genome Loaded</h3>
        <p>Load a genomic sample in the sidebar to configure sweep scopes and run enrichment.</p>
      </div>
    {:else}
      <nav class="research-workbench-tabs" aria-label="Vector research workbench">
        <button
          type="button"
          class="wb-tab"
          class:active={workbenchTab === "sweep"}
          disabled={!workbenchReady && workbenchTab !== "sweep"}
          onclick={() => (workbenchTab = "sweep")}>Sweep</button
        >
        <button
          type="button"
          class="wb-tab"
          class:active={workbenchTab === "viewer"}
          disabled={!workbenchReady}
          onclick={() => (workbenchTab = "viewer")}>Viewer</button
        >
        <button
          type="button"
          class="wb-tab"
          class:active={workbenchTab === "map"}
          disabled={!workbenchReady}
          onclick={() => (workbenchTab = "map")}>Vector map</button
        >
        <button
          type="button"
          class="wb-tab"
          class:active={workbenchTab === "packs"}
          disabled={!workbenchReady}
          onclick={() => (workbenchTab = "packs")}>Pack drafts</button
        >
        <button
          type="button"
          class="wb-tab"
          class:active={workbenchTab === "found"}
          disabled={!workbenchReady}
          onclick={() => (workbenchTab = "found")}>Research found</button
        >
      </nav>

      {#if !workbenchReady}
        <div class="workbench-boot-banner" role="status" aria-live="polite">
          <ActivityPulse message={workbenchBootMessage} accent="#5eead4" />
          <p>Viewer, map, packs, and review unlock after connections finish loading.</p>
        </div>
      {/if}

      {#if workbenchTab === "sweep"}
        <ResearchScopeSection
          {selectedSample}
          sweepRunning={sweepLoopActive}
          bind:scope={researchScope}
          bind:preview={scopePreview}
          bind:previewLoading={scopePreviewLoading}
          onLog={pushLog}
          onGnomadReadyChange={(ready) => {
            gnomadSweepReadyState = ready;
          }}
        />
        <div class="research-row-controls">
          <ResearchJobControls
            {selectedSample}
            {job}
            {config}
            {connectionStatus}
            {connectionActivity}
            {scopePreview}
            {scopePreviewLoading}
            {ollamaStatus}
            {ollamaUrl}
            researchScope={researchScope}
            {gnomadSweepReady}
            {isStarting}
            {isCancelling}
            {isCreatingCollection}
            onStart={handleStart}
            onPause={handlePause}
            onResume={handleResume}
            onCancel={handleCancel}
            onCreateCollection={handleCreateCollection}
          />
          <ResearchLiveLog
            logs={liveLog}
            sweepRunning={sweepLoopActive}
            bind:debugLogEnabled={researchDebugLog}
          />
        </div>
        <ResearchLiveFindings
          findings={liveFindingPreviews}
          sweepRunning={sweepLoopActive}
          qdrantSampleCount={liveProgress.qdrantSampleCount ?? job?.qdrant_sample_count ?? null}
        />
      {:else if workbenchTab === "viewer"}
        <VectorViewerPanel
          sampleId={selectedSample.id}
          {ollamaUrl}
          actionsEnabled={workbenchReady}
          initialRsid={inspectRsid}
          onSelectRsid={(rsid) => (inspectRsid = rsid)}
          onLog={pushLog}
        />
      {:else if workbenchTab === "map"}
        <VectorAtlasPanel
          sampleId={selectedSample.id}
          {ollamaUrl}
          actionsEnabled={workbenchReady}
          onSelectRsid={openRsidInViewer}
        />
      {:else if workbenchTab === "packs"}
        <PackDraftPanel
          sampleId={selectedSample.id}
          sampleName={selectedSample.name}
          {ollamaUrl}
          actionsEnabled={workbenchReady}
          onLog={pushLog}
        />
      {:else}
        <ResearchFoundReviewPanel
          actionsEnabled={workbenchReady}
          onLog={pushLog}
          onOpenRsid={openRsidInViewer}
        />
      {/if}
    {/if}
  </div>
</div>
