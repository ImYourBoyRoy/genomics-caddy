<script lang="ts">
  import type { ReportExportAudience } from '../../utils/reportAudienceExport';

  interface Props {
    audienceExportBusy: ReportExportAudience | '';
    aiJsonBusy: boolean;
    discoveryExportBusy: boolean;
    isPreparingPrint: boolean;
    audienceExportHint: string;
    discoveryExportHint: string;
    onExportAiJson: () => void | Promise<void>;
    onExportAudience: (audience: ReportExportAudience) => void | Promise<void>;
    onExportCuratedJson: () => void | Promise<void>;
    onExportFullCatalogJson: () => void | Promise<void>;
    onPrintReport: () => void | Promise<void>;
  }

  let {
    audienceExportBusy,
    aiJsonBusy,
    discoveryExportBusy,
    isPreparingPrint,
    audienceExportHint,
    discoveryExportHint,
    onExportAiJson,
    onExportAudience,
    onExportCuratedJson,
    onExportFullCatalogJson,
    onPrintReport,
  }: Props = $props();

  let hasStatus = $derived(Boolean(audienceExportHint || discoveryExportHint));
</script>

<section class="report-export-strip card no-print" aria-labelledby="report-export-title">
  <div class="report-export-intro">
    <span class="quality-kicker">Share this report</span>
    <h3 id="report-export-title">Save a useful copy</h3>
    <p>Local-only. AI and clinician handoffs include raw genotype calls; choose Personal or PDF for a summary-oriented copy.</p>
  </div>

  <div class="report-export-primary" aria-label="Common report exports">
    <button type="button" class="btn btn-primary btn-sm" onclick={onExportAiJson} disabled={aiJsonBusy}>
      {aiJsonBusy ? 'Saving…' : 'AI-ready JSON'}
    </button>
    <button type="button" class="btn btn-secondary btn-sm" onclick={() => onExportAudience('clinician')} disabled={Boolean(audienceExportBusy)}>
      {audienceExportBusy === 'clinician' ? 'Saving…' : 'Clinician handoff'}
    </button>
    <button type="button" class="btn btn-secondary btn-sm" onclick={() => onExportAudience('personal')} disabled={Boolean(audienceExportBusy)}>
      {audienceExportBusy === 'personal' ? 'Saving…' : 'Personal report'}
    </button>
    <button type="button" class="btn btn-secondary btn-sm" onclick={onPrintReport} disabled={isPreparingPrint}>
      {isPreparingPrint ? 'Preparing PDF…' : 'PDF'}
    </button>
    <button type="button" class="btn btn-secondary btn-sm" onclick={() => onExportAudience('ai')} disabled={Boolean(audienceExportBusy)}>
      {audienceExportBusy === 'ai' ? 'Saving…' : 'AI review bundle'}
    </button>

    <details class="report-export-more">
      <summary>More</summary>
      <div class="report-export-more-menu">
        <button type="button" class="btn btn-secondary btn-sm" onclick={onExportCuratedJson}>
          Curated pack JSON
        </button>
        <button type="button" class="btn btn-secondary btn-sm" onclick={onExportFullCatalogJson} disabled={discoveryExportBusy}>
          {discoveryExportBusy ? 'Exporting…' : 'Full catalog associations'}
        </button>
      </div>
    </details>
  </div>

  {#if hasStatus}
    <div class="report-export-status" role="status">
      {#if audienceExportHint}<span>{audienceExportHint}</span>{/if}
      {#if discoveryExportHint}<span>{discoveryExportHint}</span>{/if}
    </div>
  {/if}
</section>

<style>
  .report-export-strip {
    display: grid;
    grid-template-columns: minmax(15rem, 1fr) minmax(0, auto);
    align-items: center;
    gap: 1rem 1.5rem;
    width: min(100%, var(--report-dashboard-surface-width));
    margin: 0.7rem auto 0.9rem;
    padding: 0.7rem 0.85rem;
    box-sizing: border-box;
    border-color: var(--border-strong);
    background: var(--surface-subtle);
  }

  .report-export-intro {
    min-width: 0;
  }

  .report-export-intro h3 {
    margin: 0.15rem 0 0;
    color: var(--text-primary);
    font-size: 0.95rem;
  }

  .report-export-intro p {
    margin: 0.2rem 0 0;
    color: var(--text-secondary);
    font-size: 0.72rem;
    line-height: 1.35;
  }

  .report-export-primary {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 0.4rem;
    flex-wrap: wrap;
  }

  .report-export-more {
    position: relative;
  }

  .report-export-more > summary {
    display: inline-flex;
    min-height: 2.1rem;
    align-items: center;
    padding: 0 0.7rem;
    border: 1px solid var(--border-color);
    border-radius: 0.4rem;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 0.76rem;
    font-weight: 700;
    list-style: none;
  }

  .report-export-more > summary::-webkit-details-marker {
    display: none;
  }

  .report-export-more > summary::after {
    content: '⌄';
    margin-left: 0.35rem;
  }

  .report-export-more[open] > summary {
    border-color: var(--border-strong);
    color: var(--text-primary);
  }

  .report-export-more[open] > summary::after {
    content: '⌃';
  }

  .report-export-more > summary:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }

  .report-export-more-menu {
    position: absolute;
    z-index: 4;
    top: calc(100% + 0.35rem);
    right: 0;
    display: grid;
    min-width: 13rem;
    gap: 0.35rem;
    padding: 0.45rem;
    border: 1px solid var(--border-strong);
    border-radius: 0.55rem;
    background: var(--surface-raised);
    box-shadow: var(--shadow-card);
  }

  .report-export-more-menu .btn {
    justify-content: flex-start;
    text-align: left;
  }

  .report-export-status {
    display: grid;
    grid-column: 1 / -1;
    gap: 0.2rem;
    margin-top: -0.2rem;
    color: var(--text-secondary);
    font-size: 0.7rem;
  }

  @media (max-width: 900px) {
    .report-export-strip {
      grid-template-columns: 1fr;
    }

    .report-export-primary {
      justify-content: flex-start;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .report-export-more-menu {
      transition: none;
    }
  }
</style>
