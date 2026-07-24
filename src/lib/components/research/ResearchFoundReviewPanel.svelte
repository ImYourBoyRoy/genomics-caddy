<!-- ./src/lib/components/research/ResearchFoundReviewPanel.svelte -->
<script lang="ts">
  /*
  Purpose: Curate research_found markers — fill alleles, edit, drop, opt-in to reports.
  */

  import {
    deleteResearchFoundMarkers,
    fillResearchFoundAlleles,
    getResearchFoundPack,
    setResearchFoundEnabled,
    updateResearchFoundMarker,
    reloadMarkerPacks,
  } from "../../api/tauri";
  import { markerPacksStore } from "../../utils/markerPacksState.svelte";
  import ActivityPulse from "../common/loading/ActivityPulse.svelte";
  import "$lib/styles/components/vector-workbench.css";

  interface Props {
    actionsEnabled?: boolean;
    onLog?: (msg: string) => void;
    onOpenRsid?: (rsid: string) => void;
  }

  let { actionsEnabled = true, onLog, onOpenRsid }: Props = $props();

  const DIRECTIONS = [
    "risk",
    "protective",
    "context_dependent",
    "trait",
    "unknown",
    "not_applicable",
    "no_claim",
  ] as const;

  let name = $state("Research Found (Vector Discoveries)");
  let markers = $state<Array<Record<string, unknown>>>([]);
  let enabled = $state(false);
  let path = $state("");
  let busy = $state(false);
  let error = $state("");
  let message = $state("");
  let filter = $state("");
  let selectedRsid = $state("");
  let progressMessage = $state("");
  let locked = $derived(!actionsEnabled || busy);

  let selected = $derived(
    markers.find((m) => String(m.rsid || "").toLowerCase() === selectedRsid.toLowerCase()) ?? null
  );

  let filtered = $derived(
    markers.filter((m) => {
      const q = filter.trim().toLowerCase();
      if (!q) return true;
      return (
        String(m.rsid || "").toLowerCase().includes(q) ||
        String(m.gene || "").toLowerCase().includes(q) ||
        String(m.impact || "").toLowerCase().includes(q)
      );
    })
  );

  let incompleteCount = $derived(
    markers.filter(
      (m) =>
        !String(m.effect_allele || "") ||
        String(m.effect_allele) === "?" ||
        !String(m.gene || "") ||
        String(m.gene).toUpperCase() === "UNKNOWN"
    ).length
  );

  async function refresh() {
    if (!actionsEnabled) return;
    busy = true;
    error = "";
    progressMessage = "Loading research_found pack…";
    try {
      const view = await getResearchFoundPack();
      name = view.name;
      markers = view.markers;
      enabled = view.enabled_in_report;
      path = view.path;
      if (selectedRsid && !markers.some((m) => String(m.rsid) === selectedRsid)) {
        selectedRsid = "";
      }
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
      progressMessage = "";
    }
  }

  async function fillAlleles() {
    if (locked) return;
    busy = true;
    error = "";
    message = "";
    progressMessage = "Filling alleles from local catalogs…";
    try {
      const result = await fillResearchFoundAlleles();
      message = result.message;
      onLog?.(result.message);
      await refresh();
      await markerPacksStore.reload();
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
      progressMessage = "";
    }
  }

  async function toggleEnabled() {
    if (locked) return;
    busy = true;
    error = "";
    progressMessage = "Updating report opt-in…";
    try {
      enabled = await setResearchFoundEnabled(!enabled);
      await reloadMarkerPacks();
      await markerPacksStore.reload();
      message = enabled
        ? "research_found will appear in the Trait Report (opt-in on)."
        : "research_found excluded from Trait Report (opt-in off).";
      onLog?.(message);
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
      progressMessage = "";
    }
  }

  async function saveSelected() {
    if (!selected) return;
    busy = true;
    error = "";
    try {
      const view = await updateResearchFoundMarker({
        rsid: String(selected.rsid),
        gene: String(selected.gene ?? ""),
        effect_allele: String(selected.effect_allele ?? ""),
        effect_direction: String(selected.effect_direction ?? "context_dependent"),
        evidence_tier: String(selected.evidence_tier ?? ""),
        impact: String(selected.impact ?? ""),
        interpretation: String(selected.interpretation ?? ""),
        variant_name: String(selected.variant_name ?? ""),
        clinical_confirmation_required: Boolean(selected.clinical_confirmation_required ?? true),
      });
      markers = view.markers;
      message = `Saved ${selected.rsid}`;
      await markerPacksStore.reload();
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  async function dropSelected() {
    if (!selected) return;
    if (!confirm(`Remove ${selected.rsid} from research_found?`)) return;
    busy = true;
    error = "";
    try {
      const rsid = String(selected.rsid);
      const view = await deleteResearchFoundMarkers([rsid]);
      markers = view.markers;
      selectedRsid = "";
      message = `Dropped ${rsid}`;
      await markerPacksStore.reload();
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  function updateField(key: string, value: string | boolean) {
    if (!selectedRsid) return;
    markers = markers.map((m) =>
      String(m.rsid || "").toLowerCase() === selectedRsid.toLowerCase()
        ? { ...m, [key]: value }
        : m
    );
  }

  function provenanceNote(m: Record<string, unknown>): string {
    const sources = m.sources as Array<Record<string, unknown>> | undefined;
    const notes = sources?.[0]?.notes;
    return typeof notes === "string" ? notes : "";
  }

  $effect(() => {
    if (actionsEnabled) void refresh();
  });
</script>

<section class="vector-workbench-card" aria-labelledby="research-found-title">
  <header class="vector-workbench-header">
    <div>
      <h3 id="research-found-title">{name}</h3>
      <p class="vector-workbench-lead">
        Review hypothesis markers before enabling them in the Trait Report. Curated packs stay
        untouched. {markers.length} marker(s)
        {#if incompleteCount}
          · <span class="warn-chip">{incompleteCount} need allele/gene review</span>
        {/if}
      </p>
    </div>
    <div class="vector-workbench-actions">
      <button type="button" class="btn btn-secondary btn-sm" disabled={locked} onclick={refresh}>
        Refresh
      </button>
      <button type="button" class="btn btn-secondary btn-sm" disabled={locked} onclick={fillAlleles}>
        Fill alleles from catalogs
      </button>
      <button
        type="button"
        class="btn {enabled ? 'btn-accent' : 'btn-primary'} btn-sm"
        disabled={locked}
        onclick={toggleEnabled}
      >
        {enabled ? "Included in report ✓" : "Enable in Trait Report"}
      </button>
    </div>
  </header>

  {#if !actionsEnabled}
    <div class="workbench-boot-banner" role="status">
      <ActivityPulse message="Waiting for connections…" accent="#5eead4" />
    </div>
  {/if}

  {#if busy}
    <div class="workbench-boot-banner" role="status">
      <ActivityPulse message={progressMessage || "Working…"} accent="#38bdf8" />
    </div>
  {/if}

  {#if path}
    <p class="vector-workbench-path font-mono">{path}</p>
  {/if}
  {#if message}
    <p class="vector-workbench-note">{message}</p>
  {/if}
  {#if error}
    <p class="vector-workbench-error" role="alert">{error}</p>
  {/if}

  <div class="viewer-layout">
    <div class="viewer-table-wrap">
      <input
        type="search"
        class="viewer-search"
        bind:value={filter}
        placeholder="Filter rsID / gene / impact"
        aria-label="Filter markers"
      />
      {#if filtered.length === 0}
        <p class="vector-workbench-empty">
          No markers yet. Export + merge drafts from the Pack drafts tab, then fill alleles here.
        </p>
      {:else}
        <table class="viewer-table">
          <thead>
            <tr>
              <th>rsID</th>
              <th>Gene</th>
              <th>Allele</th>
              <th>Direction</th>
            </tr>
          </thead>
          <tbody>
            {#each filtered as m (String(m.rsid))}
              <tr
                class:selected={String(m.rsid) === selectedRsid}
                onclick={() => {
                  selectedRsid = String(m.rsid);
                  onOpenRsid?.(String(m.rsid));
                }}
              >
                <td class="font-mono">{String(m.rsid)}</td>
                <td>{String(m.gene || "")}</td>
                <td class="font-mono">{String(m.effect_allele || "?")}</td>
                <td>{String(m.effect_direction || "")}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>

    <aside class="viewer-detail" aria-label="Marker editor">
      {#if selected}
        <h4 class="subhead font-mono">{String(selected.rsid)}</h4>
        <label class="field">
          <span>Gene</span>
          <input
            value={String(selected.gene || "")}
            oninput={(e) => updateField("gene", e.currentTarget.value)}
          />
        </label>
        <label class="field">
          <span>Effect allele</span>
          <input
            value={String(selected.effect_allele || "")}
            oninput={(e) => updateField("effect_allele", e.currentTarget.value)}
          />
        </label>
        <label class="field">
          <span>Direction</span>
          <select
            value={String(selected.effect_direction || "context_dependent")}
            onchange={(e) => updateField("effect_direction", e.currentTarget.value)}
          >
            {#each DIRECTIONS as d (d)}
              <option value={d}>{d}</option>
            {/each}
          </select>
        </label>
        <label class="field">
          <span>Evidence tier</span>
          <input
            value={String(selected.evidence_tier || "")}
            oninput={(e) => updateField("evidence_tier", e.currentTarget.value)}
          />
        </label>
        <label class="field">
          <span>Impact</span>
          <input
            value={String(selected.impact || "")}
            oninput={(e) => updateField("impact", e.currentTarget.value)}
          />
        </label>
        <label class="field">
          <span>Interpretation</span>
          <textarea
            rows="4"
            value={String(selected.interpretation || "")}
            oninput={(e) => updateField("interpretation", e.currentTarget.value)}
          ></textarea>
        </label>
        {#if provenanceNote(selected)}
          <p class="vector-workbench-note"><strong>Why here:</strong> {provenanceNote(selected)}</p>
        {/if}
        <div class="vector-workbench-actions">
          <button type="button" class="btn btn-primary btn-sm" disabled={locked} onclick={saveSelected}>
            Save marker
          </button>
          <button type="button" class="btn btn-secondary btn-sm" disabled={locked} onclick={dropSelected}>
            Drop from pack
          </button>
        </div>
      {:else}
        <p class="vector-workbench-empty">Select a marker to edit alleles, direction, and tier.</p>
      {/if}
    </aside>
  </div>
</section>
