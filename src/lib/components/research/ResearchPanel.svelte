<!-- ./src/lib/components/research/ResearchPanel.svelte -->
<script lang="ts">
  import { onMount } from "svelte";
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
  } from "../../api/tauri";
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
    researchEventContext,
    resetLiveProgress,
    resetLiveProgressForResume,
  } from "../../research/liveProgress.svelte";
  import ResearchConnectionCard from "./ResearchConnectionCard.svelte";
  import ResearchJobControls from "./ResearchJobControls.svelte";
  import ResearchLiveLog from "./ResearchLiveLog.svelte";
  import ResearchScopeSection from "./ResearchScopeSection.svelte";

  interface Props {
    selectedSample: { id: number; name: string } | null;
    ollamaUrl: string;
    ollamaToken?: string;
    job?: ResearchJob | null;
  }

  let {
    selectedSample = null,
    ollamaUrl = $bindable("http://localhost:11434"),
    ollamaToken = $bindable(""),
    job = $bindable(null)
  }: Props = $props();

  let config = $state<QdrantConfigPublic>({ ...DEFAULT_QDRANT_CONFIG });
  let researchScope = $state({ ...DEFAULT_RESEARCH_SCOPE });
  let isConfigLoaded = $state(false);
  let connectionStatus = $state<QdrantConnectionStatus | null>(null);
  let connectionActivity = $state<ConnectionActivity>({ ...DEFAULT_CONNECTION_ACTIVITY });
  let isStarting = $state(false);
  let isCreatingCollection = $state(false);
  let liveLog = $state<string[]>([]);
  let researchDebugLog = $state(false);
  let scopePreview = $state<ResearchScopePreview | null>(null);
  let lastFeedPulseAt = $state(0);
  let lastFeedPhaseKey = $state("");
  let lastLoggedActivity = $state("");
  let gnomadSweepReadyState = $state(true);
  const gnomadSweepReady = $derived(
    researchScope.enrichment_sources?.gnomad ? gnomadSweepReadyState : true
  );

  const FEED_PULSE_INTERVAL_MS = 15_000;
  const JOB_POLL_INTERVAL_MS = 8_000;

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

      const loopStillActive = job?.loop_active === true;
      job = {
        ...fresh,
        current_rsid: liveProgress.lastActivityMessage
          ? fresh.current_rsid ?? job?.current_rsid
          : fresh.current_rsid,
        loop_active:
          fresh.status === "running"
            ? true
            : loopStillActive && fresh.status === "paused"
              ? true
              : fresh.loop_active,
      };
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

  async function handleStart(options?: { forceReenrich?: boolean }) {
    if (!selectedSample) return;
    isStarting = true;
    syncScopeSourcesBeforeRun();
    try {
      await saveResearchScope(researchScope);
    } catch (e: any) {
      pushLog(`Failed to save scope before start: ${e.message || String(e)}`);
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
    } catch (e: any) {
      pushLog(`Error: ${e.message || String(e)}`);
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
    if (!selectedSample) return;
    try {
      const updated = await cancelResearchJob(selectedSample.id);
      job = updated;
      resetLiveProgress();
      liveProgress.progressSpeed = null;
      liveProgress.recentProgressSpeed = null;
      pushLog("Sweep cancelled — ready to start fresh.");
    } catch (e: any) {
      pushLog(`Failed to cancel: ${e.message || String(e)}`);
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
      (job?.status === "running" ||
        (job?.status === "paused" && job?.loop_active === true));

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
  <div class="panel-header">
    <div class="header-icon">🔬</div>
    <div class="header-info">
      <h1>Autonomous Marker Research</h1>
      <p>
        Crawls GWAS, ClinVar, gnomAD, and PubMed to build a vector knowledge library. Choose sweep
        scopes below — curated packs, agent discoveries, GWAS hits, and non-reference genotypes —
        all stored under <strong>./data</strong>, not AppData.
      </p>
    </div>
  </div>

  <div class="panel-grid">
    <div class="grid-col">
      <ResearchConnectionCard
        bind:config
        bind:ollamaUrl
        bind:ollamaToken
        bind:connectionStatus
        bind:connectionActivity
        selectedSampleId={selectedSample?.id ?? null}
        {isConfigLoaded}
        onLog={pushLog}
        onConfigUpdated={loadConfig}
        onJobReset={handleJobReset}
      />
    </div>

    <div class="grid-col">
      {#if !selectedSample}
        <div class="glass-card empty-state">
          <div class="empty-icon">🧬</div>
          <h3>No Genome Loaded</h3>
          <p>Load a genomic sample to run autonomous variant enrichment.</p>
        </div>
      {:else}
        <ResearchScopeSection
          {selectedSample}
          sweepRunning={job?.status === "running"}
          bind:scope={researchScope}
          bind:preview={scopePreview}
          onLog={pushLog}
          onGnomadReadyChange={(ready) => {
            gnomadSweepReadyState = ready;
          }}
        />
        <ResearchJobControls
          {selectedSample}
          {job}
          {config}
          {connectionStatus}
          {connectionActivity}
          {scopePreview}
          {ollamaUrl}
          researchScope={researchScope}
          {gnomadSweepReady}
          {isStarting}
          {isCreatingCollection}
          onStart={handleStart}
          onPause={handlePause}
          onResume={handleResume}
          onCancel={handleCancel}
          onCreateCollection={handleCreateCollection}
        />
        <ResearchLiveLog
          logs={liveLog}
          sweepRunning={job?.status === "running"}
          bind:debugLogEnabled={researchDebugLog}
        />
      {/if}
    </div>
  </div>
</div>

<style>
  .research-panel {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 4px;
  }

  .panel-header {
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 12px;
    padding: 16px;
    display: flex;
    gap: 16px;
    align-items: center;
  }

  .header-icon {
    font-size: 2.2rem;
    background: rgba(139, 92, 246, 0.15);
    padding: 8px 14px;
    border-radius: 12px;
    border: 1px solid rgba(139, 92, 246, 0.3);
    color: var(--accent);
  }

  .header-info h1 {
    font-size: 1.15rem;
    font-weight: 700;
    color: var(--text-primary);
    margin: 0 0 4px 0;
  }

  .header-info p {
    font-size: 0.76rem;
    color: var(--text-secondary);
    margin: 0;
    line-height: 1.4;
  }

  .panel-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
  }

  @media (max-width: 768px) {
    .panel-grid {
      grid-template-columns: 1fr;
    }
  }

  .grid-col {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .glass-card.empty-state {
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 12px;
    padding: 32px 18px;
    text-align: center;
  }

  .empty-icon {
    font-size: 2rem;
    margin-bottom: 8px;
  }

  .empty-state h3 {
    margin: 0 0 8px 0;
    color: var(--text-primary);
  }

  .empty-state p {
    margin: 0;
    font-size: 0.75rem;
    color: var(--text-secondary);
  }
</style>
