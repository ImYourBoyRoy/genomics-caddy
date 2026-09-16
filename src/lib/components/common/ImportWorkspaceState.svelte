<!-- ./src/lib/components/common/ImportWorkspaceState.svelte -->
<script lang="ts">
  import type { GenomeImportPreview } from "$lib/types/genomics";
  import ImportStepTimeline from "./bootstrap/ImportStepTimeline.svelte";
  import type { ImportPhase, ImportStepId } from "$lib/utils/importProgress";

  interface Props {
    phase: ImportPhase;
    percentage: number;
    status: string;
    profileName: string;
    preview?: GenomeImportPreview | null;
    failedStep?: ImportStepId | null;
    replacingExisting?: boolean;
    onConfirmImport: () => void;
    onCancelImport: () => void;
  }

  let {
    phase,
    percentage,
    status,
    profileName,
    preview = null,
    failedStep = null,
    replacingExisting = false,
    onConfirmImport,
    onCancelImport,
  }: Props = $props();

  let isCheckingPreview = $derived(phase === "preview");
  let isAwaitingConfirmation = $derived(phase === "awaiting-confirmation");
  let clampedPercentage = $derived(Math.max(0, Math.min(100, Math.round(percentage))));
  let skippedRows = $derived(
    preview ? preview.diagnostics.malformed_rows + preview.diagnostics.duplicate_rows : 0,
  );

  let title = $derived(
    isAwaitingConfirmation
      ? "Review DNA import"
      : isCheckingPreview
        ? "Checking DNA export"
      : phase === "ready"
        ? "DNA profile ready"
        : "Importing DNA profile",
  );

  let lead = $derived(
    isAwaitingConfirmation
      ? "The local check is finished. Review the counts below, then choose an action here. Nothing is written until you confirm."
      : isCheckingPreview
        ? "Checking the selected export on this device before writing any profile data."
      : phase === "ready"
        ? "The imported profile and report are ready."
        : "The main workspace will open the imported report after the profile is loaded.",
  );
</script>

<section
  class="import-workspace-state card"
  aria-busy={!isAwaitingConfirmation && phase !== "ready"}
  aria-label="DNA profile import status"
>
  <div class="import-workspace-heading">
    <div>
      <p class="import-workspace-kicker">DNA import</p>
      <h1>{title}</h1>
      <p class="import-workspace-lead">{lead}</p>
    </div>
    <span class="import-workspace-local">On-device</span>
  </div>

  <div class="import-workspace-status" class:import-workspace-status-ready={isAwaitingConfirmation} role="status" aria-live="polite">
    <span class="import-workspace-status-dot" aria-hidden="true"></span>
    <span>{status || (isAwaitingConfirmation ? "Preview ready — no import is running." : "Preparing the imported profile…")}</span>
  </div>

  {#if isAwaitingConfirmation}
    <div class="import-workspace-confirmation-note" role="note">
      <strong>{replacingExisting ? "Profile replacement needs your confirmation" : "Ready to create an active profile"}</strong>
      {#if replacingExisting}
        <span>This replaces the existing profile’s genotype data and clears its derived report, research, and chat state. Entered Context and Diary data are preserved.</span>
      {:else}
        <span>The file has only been checked. Import it as “{profileName}” to create the profile and open its report, or cancel to leave your data unchanged.</span>
      {/if}
    </div>
  {:else}
    <div class="import-workspace-progress" aria-label="DNA import progress">
      <div class="import-workspace-progress-track">
        <div class="import-workspace-progress-fill" style="width: {clampedPercentage}%"></div>
      </div>
      <div class="import-workspace-progress-meta">
        <span>{`${clampedPercentage}% complete`}</span>
        <span>{profileName || "New profile"}</span>
      </div>
    </div>
  {/if}

  <ImportStepTimeline phase={phase} failedStep={failedStep} />

  {#if preview}
    <div class="import-workspace-details">
      <div class="import-workspace-details-heading">
        <span>Import details</span>
        <span class="import-workspace-local">Checked locally</span>
      </div>
      <div class="import-workspace-record-summary" aria-label="DNA row summary">
        <div class="import-workspace-record-count import-workspace-record-accepted">
          <span>Accepted records</span>
          <strong>{preview.diagnostics.accepted_rows.toLocaleString()}</strong>
          <small>of {preview.diagnostics.total_rows.toLocaleString()} checked</small>
        </div>
        <div class="import-workspace-record-count" class:import-workspace-record-skipped={skippedRows > 0}>
          <span>Not imported</span>
          <strong>{skippedRows.toLocaleString()}</strong>
          {#if skippedRows > 0}
            <small>
              {preview.diagnostics.malformed_rows.toLocaleString()} malformed ·
              {preview.diagnostics.duplicate_rows.toLocaleString()} duplicates
            </small>
          {:else}
            <small>All checked records accepted</small>
          {/if}
        </div>
      </div>
      <dl>
        <div>
          <dt>Profile name</dt>
          <dd>{profileName || "New profile"}</dd>
        </div>
        <div>
          <dt>Selected file</dt>
          <dd>{preview.source_file_name}</dd>
        </div>
        <div>
          <dt>Format</dt>
          <dd>{preview.diagnostics.vendor} · {preview.diagnostics.delimiter}</dd>
        </div>
        <div>
          <dt>Source build</dt>
          <dd>{preview.diagnostics.source_build}</dd>
        </div>
        <div>
          <dt>Allele orientation</dt>
          <dd>{preview.diagnostics.allele_orientation}</dd>
        </div>
        <div>
          <dt>Coordinate system</dt>
          <dd>{preview.diagnostics.coordinate_system}</dd>
        </div>
        <div>
          <dt>Liftover</dt>
          <dd>{preview.liftover_available ? "Available" : "Not installed"}</dd>
        </div>
      </dl>
      {#if preview.diagnostics.warnings.length > 0}
        <ul class="import-workspace-warnings" aria-label="Import preview warnings">
          {#each preview.diagnostics.warnings as warning}
            <li>{warning}</li>
          {/each}
        </ul>
      {/if}
      <p class="import-workspace-count-note">Blank lines and recognized vendor headers are excluded from the checked-record count.</p>
    </div>
  {/if}

  {#if isAwaitingConfirmation}
    <div class="import-workspace-actions">
      <button type="button" class="btn btn-secondary" onclick={onCancelImport}>Cancel</button>
      <button type="button" class="btn btn-accent" onclick={onConfirmImport}>
        {replacingExisting ? "Replace profile" : "Import profile"}
      </button>
    </div>
  {/if}

  <p class="import-workspace-note">
    Your DNA file stays on this device. No report content is shown until the imported profile is ready.
  </p>
</section>

<style>
  .import-workspace-state {
    width: min(100%, 60rem);
    box-sizing: border-box;
    margin-inline: auto;
    padding: clamp(1.2rem, 3vw, 2rem);
    background: var(--surface-raised);
    border: 1px solid var(--border-strong);
    box-shadow: 0 18px 48px var(--shadow-card-hover);
  }

  .import-workspace-heading {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
  }

  .import-workspace-kicker {
    margin: 0 0 0.35rem;
    color: var(--accent);
    font-size: 0.68rem;
    font-weight: 800;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .import-workspace-heading h1 {
    margin: 0;
    color: var(--text-primary);
    font-size: clamp(1.25rem, 2vw, 1.75rem);
    line-height: 1.2;
  }

  .import-workspace-lead {
    max-width: 44rem;
    margin: 0.55rem 0 0;
    color: var(--text-secondary);
    font-size: 0.9rem;
    line-height: 1.5;
  }

  .import-workspace-local {
    flex: 0 0 auto;
    padding: 0.22rem 0.5rem;
    border: 1px solid var(--status-success-border);
    border-radius: 999px;
    color: var(--status-success-text);
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 0.62rem;
    font-weight: 700;
    white-space: nowrap;
  }

  .import-workspace-status {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    margin-top: 1.15rem;
    padding: 0.7rem 0.8rem;
    border: 1px solid var(--status-info-soft-border);
    border-radius: 0.6rem;
    background: var(--status-info-soft-bg);
    color: var(--text-primary);
    font-size: 0.82rem;
    font-weight: 600;
  }

  .import-workspace-status-ready {
    border-color: var(--status-success-border);
    background: var(--status-success-bg);
  }

  .import-workspace-status-ready .import-workspace-status-dot {
    background: var(--status-success-text);
    box-shadow: 0 0 0 0.2rem var(--status-success-bg);
  }

  .import-workspace-confirmation-note {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    margin-top: 1rem;
    padding: 0.75rem 0.85rem;
    border: 1px solid var(--status-warning-border);
    border-radius: 0.6rem;
    background: var(--status-warning-bg);
    color: var(--text-secondary);
    font-size: 0.78rem;
    line-height: 1.45;
  }

  .import-workspace-confirmation-note strong {
    color: var(--status-warning-text);
  }

  .import-workspace-actions {
    display: flex;
    justify-content: flex-end;
    flex-wrap: wrap;
    gap: 0.65rem;
    margin: 0.75rem 0 1rem;
  }

  .import-workspace-actions .btn {
    min-width: 8.5rem;
    min-height: 2.65rem;
    border-radius: 0.6rem;
    font-weight: 700;
  }

  .import-workspace-status-dot {
    width: 0.55rem;
    height: 0.55rem;
    flex: 0 0 auto;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 0 0.2rem var(--status-info-soft-bg);
  }

  .import-workspace-progress {
    margin-top: 1rem;
  }

  .import-workspace-progress-track {
    height: 0.35rem;
    overflow: hidden;
    border-radius: 999px;
    background: var(--surface-subtle);
  }

  .import-workspace-progress-fill {
    height: 100%;
    min-width: 0.35rem;
    border-radius: inherit;
    background: linear-gradient(90deg, var(--accent), var(--success));
    transition: width 0.35s ease;
  }

  .import-workspace-progress-meta {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    margin-top: 0.35rem;
    color: var(--text-secondary);
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 0.68rem;
  }

  .import-workspace-details {
    margin-top: 0.25rem;
    padding: 0.8rem 0.9rem;
    border: 1px solid var(--border-color);
    border-radius: 0.7rem;
    background: var(--surface-card);
  }

  .import-workspace-details-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    margin-bottom: 0.65rem;
    color: var(--text-primary);
    font-size: 0.74rem;
    font-weight: 700;
  }

  .import-workspace-details dl {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.65rem 1rem;
    margin: 0;
  }

  .import-workspace-record-summary {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.65rem;
    margin-bottom: 0.75rem;
  }

  .import-workspace-record-count {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: baseline;
    gap: 0.12rem 0.75rem;
    min-width: 0;
    padding: 0.7rem 0.8rem;
    border: 1px solid var(--border-color);
    border-radius: 0.6rem;
    background: var(--surface-subtle);
  }

  .import-workspace-record-count > span,
  .import-workspace-record-count > small {
    min-width: 0;
    color: var(--text-secondary);
    font-size: 0.68rem;
  }

  .import-workspace-record-count > strong {
    grid-column: 2;
    grid-row: 1 / span 2;
    color: var(--text-primary);
    font-size: 1.15rem;
    font-variant-numeric: tabular-nums;
  }

  .import-workspace-record-skipped {
    border-color: var(--status-warning-border);
    background: var(--status-warning-bg);
  }

  .import-workspace-record-skipped > strong {
    color: var(--status-warning-text);
  }

  .import-workspace-count-note {
    margin: 0.65rem 0 0;
    color: var(--text-secondary);
    font-size: 0.66rem;
    line-height: 1.4;
  }

  .import-workspace-warnings {
    display: grid;
    gap: 0.35rem;
    margin: 0.75rem 0 0;
    padding: 0.7rem 0.8rem 0.7rem 2rem;
    border: 1px solid var(--status-warning-border);
    border-radius: 0.6rem;
    background: var(--status-warning-bg);
    color: var(--status-warning-text);
    font-size: 0.72rem;
    line-height: 1.4;
  }

  .import-workspace-details dl > div {
    min-width: 0;
  }

  .import-workspace-details dt {
    margin-bottom: 0.15rem;
    color: var(--text-secondary);
    font-size: 0.62rem;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .import-workspace-details dd {
    margin: 0;
    color: var(--text-primary);
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 0.7rem;
    line-height: 1.35;
    overflow-wrap: anywhere;
  }

  .import-workspace-note {
    margin: 1rem 0 0;
    color: var(--text-secondary);
    font-size: 0.72rem;
    line-height: 1.45;
  }

  @media (max-width: 620px) {
    .import-workspace-heading {
      flex-direction: column;
    }

    .import-workspace-details dl {
      grid-template-columns: minmax(0, 1fr);
    }

    .import-workspace-record-summary {
      grid-template-columns: minmax(0, 1fr);
    }

    .import-workspace-actions {
      flex-direction: column-reverse;
    }

    .import-workspace-actions .btn {
      width: 100%;
    }
  }
</style>
