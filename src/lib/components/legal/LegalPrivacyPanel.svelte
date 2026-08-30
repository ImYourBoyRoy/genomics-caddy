<script lang="ts">
  import type { GenomeSample } from '../../types/genomics';

  interface Props {
    selectedSample: GenomeSample;
  }

  let { selectedSample }: Props = $props();
</script>

<section class="legal-page" aria-labelledby="legal-page-title">
  <div class="legal-page-header">
    <div>
      <p class="eyebrow">APPLICATION INFORMATION</p>
      <h2 id="legal-page-title">Legal &amp; Privacy</h2>
      <p class="legal-page-intro">How this local report handles sensitive genetic and profile information.</p>
    </div>
    <span class="local-badge">Local application</span>
  </div>

  <div class="legal-grid">
    <article class="legal-card">
      <span class="legal-icon" aria-hidden="true">⌂</span>
      <div>
        <h3>Local by design</h3>
        <p>Reports, profile context, diary entries, and exports are handled on this computer unless you deliberately use an external connection or share an export.</p>
      </div>
    </article>

    <article class="legal-card">
      <span class="legal-icon" aria-hidden="true">◈</span>
      <div>
        <h3>What the report shows</h3>
        <p>Trait Report presents DNA-associated signals and their evidence. Context, diary entries, measured labs, and medication history are kept separate and are included only when you choose.</p>
      </div>
    </article>

    <article class="legal-card">
      <span class="legal-icon" aria-hidden="true">⇧</span>
      <div>
        <h3>Sharing an export</h3>
        <p>Exports can contain sensitive genetic and health information. Check the audience and raw-genotype setting before sharing a file with a clinician or AI service.</p>
      </div>
    </article>

    <article class="legal-card">
      <span class="legal-icon" aria-hidden="true">?</span>
      <div>
        <h3>Use and responsibility</h3>
        <p>Use the report as an organized research reference. Important findings and health decisions should be reviewed with a qualified healthcare professional.</p>
      </div>
    </article>
  </div>

  <footer class="legal-page-footer">
    <span>Current profile: <strong>{selectedSample.name}</strong></span>
    <span>Profile data stays scoped to this sample.</span>
  </footer>
</section>

<style>
  .legal-page {
    max-width: 1180px;
    margin: 0 auto;
    padding: 2rem 1.25rem 3rem;
    color: var(--text-primary);
  }

  .legal-page-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1.5rem;
    padding-bottom: 1.5rem;
    border-bottom: 1px solid var(--border-color);
  }

  .eyebrow {
    margin: 0 0 0.35rem;
    color: var(--accent);
    font-size: 0.7rem;
    font-weight: 800;
    letter-spacing: 0.12em;
  }

  h2, h3, p { margin-top: 0; }
  h2 { margin-bottom: 0.35rem; font-size: clamp(1.45rem, 2vw, 2rem); }
  h3 { margin-bottom: 0.45rem; font-size: 1rem; }
  .legal-page-intro { margin-bottom: 0; color: var(--text-secondary); }

  .local-badge {
    flex: 0 0 auto;
    padding: 0.45rem 0.7rem;
    border: 1px solid var(--status-success-border);
    border-radius: 999px;
    background: var(--status-success-bg);
    color: var(--status-success-text);
    font-size: 0.75rem;
    font-weight: 700;
  }

  .legal-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 1rem;
    margin-top: 1.5rem;
  }

  .legal-card {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 0.8rem;
    padding: 1.1rem;
    border: 1px solid var(--border-color);
    border-radius: 0.75rem;
    background: var(--surface-card);
  }

  .legal-icon {
    display: grid;
    width: 2rem;
    height: 2rem;
    place-items: center;
    border-radius: 0.55rem;
    background: var(--status-info-bg);
    color: var(--status-info-text);
    font-weight: 800;
  }

  .legal-card p { margin-bottom: 0; color: var(--text-secondary); line-height: 1.5; }

  .legal-page-footer {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    margin-top: 1.25rem;
    padding-top: 1rem;
    border-top: 1px solid var(--border-color);
    color: var(--text-muted);
    font-size: 0.8rem;
  }

  @media (max-width: 760px) {
    .legal-grid { grid-template-columns: 1fr; }
    .legal-page-footer { flex-direction: column; }
  }
</style>
