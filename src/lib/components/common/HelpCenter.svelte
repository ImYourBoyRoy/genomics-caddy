<script lang="ts">
  import { isTauri } from '@tauri-apps/api/core';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { appUpdateState } from '../../utils/appUpdateState';
  import { formatVersionLabel } from '../../utils/updater';

  async function openExternal(event: MouseEvent, url: string): Promise<void> {
    if (!isTauri()) return;
    event.preventDefault();
    try {
      await openUrl(url);
    } catch {
      window.open(url, '_blank', 'noopener,noreferrer');
    }
  }

  function updateStatusLabel(): string {
    switch ($appUpdateState.status) {
      case 'checking': return 'Checking for an app update…';
      case 'available': return `Version ${formatVersionLabel($appUpdateState.availableVersion ?? '')} is available.`;
      case 'current': return 'You are using the latest signed app release.';
      case 'installing': return 'Applying the signed update…';
      case 'error': return $appUpdateState.message || 'Could not check right now. Your installed version is unchanged.';
      default: return $appUpdateState.message || 'App updates are checked separately from DNA reference data.';
    }
  }

  function formatCheckedAt(timestamp: number): string {
    return new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' }).format(timestamp);
  }
</script>

<section class="help-center" aria-labelledby="help-center-title">
  <header class="help-center-header">
    <span class="help-center-kicker">Genomics Caddy</span>
    <h2 id="help-center-title">Help &amp; app info</h2>
    <p>Plain-language answers for getting started, understanding your results, and finding the right next step.</p>
  </header>

  <div class="help-center-grid">
    <div class="help-topics" aria-label="Help topics">
      <details class="help-topic" open>
        <summary><span class="help-topic-number">01</span><span><strong>Getting started</strong><small>Import first; reference downloads can follow.</small></span></summary>
        <div class="help-topic-body">
          <p>Import a supported raw DNA file and let the app finish indexing your profile. You can then browse the calls already in that file while reference catalogs download in the background.</p>
          <p>The Raw Browser reads calls from your local profile. Curated marker packs ship with the app; enriched database links and full-genome disease discovery depend on the public reference catalogs being installed and indexed.</p>
          <p>A missing call is <strong>unknown</strong>, and a lookup in an unfinished catalog is <strong>not yet available</strong>—neither means “no risk” or “no association.”</p>
        </div>
      </details>

      <details class="help-topic">
        <summary><span class="help-topic-number">02</span><span><strong>Understanding a finding</strong><small>What a DNA match can—and cannot—tell you.</small></span></summary>
        <div class="help-topic-body">
          <p>A finding is a research-backed connection between a DNA marker and a health topic. It is a clue to discuss or investigate, not a diagnosis or a measurement of your current health.</p>
          <ul>
            <li><strong>Higher concern / priority:</strong> a result the report suggests reviewing first. It is not a personal disease probability.</li>
            <li><strong>Evidence strength:</strong> how well the association is supported in research; it is separate from how strongly a result applies to you.</li>
            <li><strong>Uncalled / no data:</strong> the file did not provide a usable call for that marker. The result is unknown.</li>
            <li><strong>Clinical confirmation:</strong> some findings need a separate clinical test before they can guide care.</li>
          </ul>
        </div>
      </details>

      <details class="help-topic">
        <summary><span class="help-topic-number">03</span><span><strong>Finding what matters to you</strong><small>Search, filter, and open health areas.</small></span></summary>
        <div class="help-topic-body">
          <p>On Trait Report, search by gene, marker, or condition. Use the quick filters to focus on higher-concern or priority-review entries, then open a health area for its findings.</p>
          <p>Simple view is the short, plain-language starting point. Clinical view keeps the source rows and confirmation details. Compare view groups related context. None of these views changes the underlying DNA calls.</p>
        </div>
      </details>

      <details class="help-topic">
        <summary><span class="help-topic-number">04</span><span><strong>Reference catalogs &amp; updates</strong><small>Two separate kinds of updates.</small></span></summary>
        <div class="help-topic-body">
          <p><strong>Data &amp; updates</strong> in the left panel checks and downloads public research catalogs used for local annotations. Large catalog downloads can take time; the app reports download and indexing progress.</p>
          <p><strong>Check app updates</strong> below checks signed Genomics Caddy releases. Updating the app does not update the catalogs, and updating catalogs does not replace the app.</p>
        </div>
      </details>

      <details class="help-topic">
        <summary><span class="help-topic-number">05</span><span><strong>Privacy &amp; using results safely</strong><small>Keep DNA information in context.</small></span></summary>
        <div class="help-topic-body">
          <p>Imported DNA and saved profiles are handled by the desktop app on this computer. Checking signed app releases contacts the configured release service. Any separately configured online research or AI connection has its own destination and controls.</p>
          <p>Do not start, stop, or change medication, supplements, or treatment based only on a DNA report. For a finding that could affect care, discuss it with a qualified health professional and ask whether clinical confirmation is appropriate.</p>
        </div>
      </details>
    </div>

    <aside class="help-side-column">
      <section class="app-update-card" aria-labelledby="app-update-title">
        <div class="help-card-heading">
          <div>
            <span class="help-card-kicker">Signed app release</span>
            <h3 id="app-update-title">Version &amp; updates</h3>
          </div>
          <span class="app-version">{$appUpdateState.currentVersion || '—'}</span>
        </div>
        <p class="app-update-description">Current app version</p>
        <p class="app-update-status" role="status" aria-live="polite">{updateStatusLabel()}</p>
        {#if $appUpdateState.checkedAt}
          <p class="app-update-time">Last checked {formatCheckedAt($appUpdateState.checkedAt)}</p>
        {/if}
        <div class="app-update-actions">
          {#if $appUpdateState.status === 'available'}
            <button type="button" class="btn btn-primary" onclick={() => $appUpdateState.installAvailable?.()} disabled={!$appUpdateState.installAvailable}>
              Review &amp; install update
            </button>
          {/if}
          <button
            type="button"
            class="btn btn-secondary"
            onclick={() => $appUpdateState.checkNow?.()}
            disabled={!$appUpdateState.isDesktop || $appUpdateState.status === 'checking' || $appUpdateState.status === 'installing'}
          >
            {$appUpdateState.status === 'checking' ? 'Checking…' : 'Check for app updates'}
          </button>
        </div>
        {#if !$appUpdateState.isDesktop}
          <p class="app-update-note">Version and signed update checks are available in the installed desktop app.</p>
        {/if}
        <p class="app-update-note">This does not check research catalogs. Manage those separately under <strong>Data &amp; updates</strong>.</p>
      </section>

      <section class="about-card" aria-labelledby="about-title">
        <span class="help-card-kicker">About</span>
        <h3 id="about-title">Made to make DNA research easier to explore.</h3>
        <p>Genomics Caddy is created by Roy Dawson IV.</p>
        <nav class="about-links" aria-label="About and support links">
          <a href="https://imyourboyroy.com" target="_blank" rel="noopener noreferrer" onclick={(event) => openExternal(event, 'https://imyourboyroy.com')}>Personal site <span aria-hidden="true">↗</span></a>
          <a href="https://github.com/imyourboyroy" target="_blank" rel="noopener noreferrer" onclick={(event) => openExternal(event, 'https://github.com/imyourboyroy')}>GitHub <span aria-hidden="true">↗</span></a>
          <a href="https://venmo.com/itsyourboyroy" target="_blank" rel="noopener noreferrer" onclick={(event) => openExternal(event, 'https://venmo.com/itsyourboyroy')}>Support via Venmo <span aria-hidden="true">↗</span></a>
        </nav>
      </section>
    </aside>
  </div>
</section>

<style>
  .help-center { display: grid; gap: 1rem; max-width: 1100px; margin: 0 auto; padding: 0.2rem 0 1.5rem; color: var(--text-primary); }
  .help-center-header { padding: 1rem 1.15rem; border: 1px solid var(--border-color); border-radius: 0.85rem; background: var(--surface-raised); }
  .help-center-kicker, .help-card-kicker { color: var(--accent); font-size: 0.72rem; font-weight: 700; letter-spacing: 0.07em; text-transform: uppercase; }
  .help-center-header h2 { margin: 0.25rem 0 0; font-size: 1.45rem; line-height: 1.2; }
  .help-center-header p { margin: 0.45rem 0 0; max-width: 62ch; color: var(--text-secondary); line-height: 1.5; }
  .help-center-grid { display: grid; grid-template-columns: minmax(0, 1.45fr) minmax(260px, 0.8fr); gap: 0.85rem; align-items: start; }
  .help-topics, .help-side-column { display: grid; gap: 0.65rem; min-width: 0; }
  .help-topic, .app-update-card, .about-card { min-width: 0; border: 1px solid var(--border-color); border-radius: 0.75rem; background: var(--surface-raised); }
  .help-topic > summary { display: flex; min-height: 3.5rem; gap: 0.75rem; align-items: center; padding: 0.65rem 0.8rem; cursor: pointer; list-style: none; }
  .help-topic > summary::-webkit-details-marker { display: none; }
  .help-topic > summary::after { content: '+'; margin-left: auto; color: var(--text-secondary); font-size: 1.1rem; }
  .help-topic[open] > summary::after { content: '−'; }
  .help-topic > summary:focus-visible, .about-links a:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 2px; }
  .help-topic-number { display: grid; width: 1.8rem; height: 1.8rem; flex: 0 0 auto; place-items: center; border-radius: 50%; background: var(--accent-soft); color: var(--accent); font-size: 0.7rem; font-weight: 700; }
  .help-topic > summary strong, .help-topic > summary small { display: block; }
  .help-topic > summary strong { font-size: 0.9rem; }
  .help-topic > summary small { margin-top: 0.12rem; color: var(--text-secondary); font-size: 0.76rem; line-height: 1.35; }
  .help-topic-body { display: grid; gap: 0.6rem; padding: 0 0.9rem 0.85rem 3.35rem; color: var(--text-secondary); font-size: 0.84rem; line-height: 1.55; }
  .help-topic-body p, .help-topic-body ul { margin: 0; }
  .help-topic-body ul { display: grid; gap: 0.35rem; padding-left: 1.1rem; }
  .help-topic-body strong { color: var(--text-primary); }
  .app-update-card, .about-card { padding: 0.9rem; }
  .help-card-heading { display: flex; align-items: start; justify-content: space-between; gap: 0.65rem; }
  .help-card-heading h3, .about-card h3 { margin: 0.22rem 0 0; font-size: 1rem; line-height: 1.3; }
  .app-version { flex: 0 0 auto; padding: 0.25rem 0.5rem; border-radius: 999px; background: var(--surface-subtle); color: var(--text-primary); font-family: var(--font-mono), monospace; font-size: 0.8rem; font-weight: 700; }
  .app-update-description, .app-update-status, .app-update-time, .app-update-note, .about-card p { margin: 0.4rem 0 0; color: var(--text-secondary); font-size: 0.8rem; line-height: 1.45; }
  .app-update-status { color: var(--text-primary); }
  .app-update-time, .app-update-note { font-size: 0.74rem; }
  .app-update-actions { display: flex; flex-wrap: wrap; gap: 0.45rem; margin-top: 0.75rem; }
  .app-update-actions :global(.btn) { min-height: 2.5rem; }
  .about-links { display: grid; gap: 0.35rem; margin-top: 0.7rem; }
  .about-links a { display: flex; justify-content: space-between; gap: 0.5rem; min-height: 2.5rem; align-items: center; padding: 0.35rem 0.55rem; border-radius: 0.45rem; color: var(--accent); font-size: 0.84rem; font-weight: 650; text-decoration: none; }
  .about-links a:hover { background: var(--accent-soft); }
  @media (max-width: 850px) { .help-center-grid { grid-template-columns: minmax(0, 1fr); } .help-side-column { grid-template-columns: repeat(2, minmax(0, 1fr)); } }
  @media (max-width: 560px) { .help-side-column { grid-template-columns: minmax(0, 1fr); } .help-topic-body { padding-left: 0.9rem; } .help-center-header h2 { font-size: 1.25rem; } }
</style>
