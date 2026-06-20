<!-- ./src/lib/components/ai/evidence/VectorEvidenceCard.svelte -->
<script lang="ts">
  import type { EvidenceCard } from "../../../types/research";
  import type { VariantNavTarget } from "../../../constants/traitCategories";

  interface Props {
    card: EvidenceCard;
    expanded?: boolean;
    showRaw?: boolean;
    onNavigateToVariant?: (rsid: string, target: VariantNavTarget) => void;
    onExportPacket?: (rsid: string) => void;
    onShowSimilar?: (rsid: string) => void;
  }

  let {
    card,
    expanded = $bindable(false),
    showRaw = $bindable(false),
    onNavigateToVariant,
    onExportPacket,
    onShowSimilar,
  }: Props = $props();

  function scorePct(v: number) {
    return `${Math.round(v * 100)}%`;
  }

  function formatP(p?: number) {
    if (p == null) return "unknown";
    if (p < 0.001) return p.toExponential(2);
    return p.toFixed(6);
  }
</script>

<article class="evidence-card" class:stale={card.stale}>
  <header class="ev-card-header">
    <div class="ev-card-title">
      {#if card.gene_symbol}
        <span class="gene-badge">{card.gene_symbol}</span>
      {/if}
      <span class="rsid-badge">{card.rsid}</span>
      {#if card.genotype}
        <span class="genotype-badge">{card.genotype}</span>
      {/if}
    </div>
    <div class="ev-card-scores">
      {#if card.semantic_score > 0}
        <span class="score-chip semantic" title="Vector similarity">{scorePct(card.semantic_score)} match</span>
      {/if}
      <span class="score-chip dq" title="Data quality">DQ {scorePct(card.data_quality_score)}</span>
      {#if card.has_direction}
        <span class="score-chip direction" title="Structured direction available">Direction known</span>
      {:else}
        <span class="score-chip unknown-dir" title="Do not infer direction from p-value alone">Direction unknown</span>
      {/if}
    </div>
  </header>

  <section class="ev-section summary">
    <h5>Summary</h5>
    <p>{card.primary_trait || "Trait unknown"}</p>
    {#if card.trait_category}
      <span class="trait-cat">{card.trait_category.replace(/_/g, " ")}</span>
    {/if}
    <p class="synthesis">{card.synthesis}</p>
  </section>

  <section class="ev-section">
    <h5>Directionality</h5>
    <p class="direction-label">{card.directionality_label}</p>
    <p class="direction-code"><code>{card.personal_direction}</code></p>
  </section>

  {#if card.evidence_ledger.length > 0}
    <section class="ev-section">
      <h5>Evidence ledger</h5>
      <div class="ledger-table-wrap">
        <table class="ledger-table">
          <thead>
            <tr>
              <th>Source</th>
              <th>Trait</th>
              <th>Gene</th>
              <th>p-value</th>
              <th>Direction</th>
            </tr>
          </thead>
          <tbody>
            {#each card.evidence_ledger.slice(0, expanded ? 12 : 4) as row}
              <tr>
                <td>{row.source_name}</td>
                <td>{row.trait_name || "—"}</td>
                <td>{row.mapped_gene || "—"}</td>
                <td>{formatP(row.p_value)}</td>
                <td>{row.personal_direction.replace(/_/g, " ")}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </section>
  {/if}

  <section class="ev-section">
    <h5>Source provenance</h5>
    <div class="tag-row">
      {#if card.has_gwas}<span class="source-tag">GWAS</span>{/if}
      {#if card.has_clinvar}<span class="source-tag">ClinVar</span>{/if}
      {#if card.has_pgs}<span class="source-tag">PGS</span>{/if}
      {#if card.has_pharmgkb}<span class="source-tag">PharmGKB</span>{/if}
      {#if card.has_reactome}<span class="source-tag">Reactome</span>{/if}
      {#each card.source_names as src}
        <span class="source-tag">{src}</span>
      {/each}
    </div>
    {#if !card.has_gwas && !card.has_clinvar && !card.has_pgs && !card.has_pharmgkb && !card.has_reactome && card.source_names.length === 0}
      <p class="muted">No source names recorded</p>
    {/if}
  </section>

  {#if card.missing_fields.length > 0 || card.quality_flags.length > 0}
    <section class="ev-section warnings">
      <h5>Missing metadata &amp; quality flags</h5>
      {#if card.missing_fields.length > 0}
        <ul class="flag-list missing">
          {#each card.missing_fields as f}
            <li>Missing: {f}</li>
          {/each}
        </ul>
      {/if}
      {#if card.quality_flags.length > 0}
        <ul class="flag-list">
          {#each card.quality_flags as f}
            <li>{f.replace(/_/g, " ")}</li>
          {/each}
        </ul>
      {/if}
    </section>
  {/if}

  {#if card.match_explanation}
    <section class="ev-section">
      <h5>Why this matched</h5>
      {#if card.match_explanation.boost_reasons.length > 0}
        <ul class="flag-list positive">
          {#each card.match_explanation.boost_reasons as r}
            <li>{r}</li>
          {/each}
        </ul>
      {/if}
      {#if card.match_explanation.missing_metadata_warnings.length > 0}
        <ul class="flag-list missing">
          {#each card.match_explanation.missing_metadata_warnings as w}
            <li>{w}</li>
          {/each}
        </ul>
      {/if}
    </section>
  {/if}

  {#if expanded && card.verification_ideas.length > 0}
    <section class="ev-section">
      <h5>Verification ideas (low risk)</h5>
      <ul class="flag-list">
        {#each card.verification_ideas as idea}
          <li>{idea}</li>
        {/each}
      </ul>
    </section>
  {/if}

  <footer class="ev-card-actions">
    <button type="button" class="btn btn-secondary btn-xs" onclick={() => (expanded = !expanded)}>
      {expanded ? "Collapse" : "Expand details"}
    </button>
    <button type="button" class="btn btn-secondary btn-xs" onclick={() => (showRaw = !showRaw)}>
      {showRaw ? "Hide raw" : "View raw payload"}
    </button>
    {#if onShowSimilar}
      <button type="button" class="btn btn-secondary btn-xs" onclick={() => onShowSimilar?.(card.rsid)}>
        Similar
      </button>
    {/if}
    {#if onExportPacket}
      <button type="button" class="btn btn-secondary btn-xs" onclick={() => onExportPacket?.(card.rsid)}>
        Export packet
      </button>
    {/if}
    {#if onNavigateToVariant}
      <button type="button" class="btn btn-secondary btn-xs" onclick={() => onNavigateToVariant?.(card.rsid, "report")}>Report</button>
      <button type="button" class="btn btn-secondary btn-xs" onclick={() => onNavigateToVariant?.(card.rsid, "map")}>Map</button>
    {/if}
  </footer>

  {#if showRaw}
    <pre class="raw-payload">{JSON.stringify(card, null, 2)}</pre>
  {/if}
</article>

<style>
  .evidence-card {
    border: 1px solid var(--border-subtle, rgba(255, 255, 255, 0.08));
    border-radius: 10px;
    padding: 14px;
    background: var(--surface-elevated, rgba(0, 0, 0, 0.2));
    margin-bottom: 12px;
  }
  .evidence-card.stale {
    border-color: rgba(255, 180, 80, 0.35);
  }
  .ev-card-header {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
    margin-bottom: 10px;
  }
  .ev-card-title {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
  }
  .gene-badge, .rsid-badge, .genotype-badge, .score-chip, .source-tag, .trait-cat {
    font-size: 0.75rem;
    padding: 2px 8px;
    border-radius: 999px;
    background: rgba(120, 160, 255, 0.12);
  }
  .genotype-badge { background: rgba(100, 200, 140, 0.15); }
  .score-chip.unknown-dir { background: rgba(255, 160, 80, 0.15); }
  .score-chip.direction { background: rgba(100, 200, 140, 0.15); }
  .ev-card-scores { display: flex; flex-wrap: wrap; gap: 6px; }
  .ev-section { margin-top: 10px; }
  .ev-section h5 {
    margin: 0 0 6px;
    font-size: 0.78rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    opacity: 0.75;
  }
  .synthesis, .direction-label { margin: 4px 0; font-size: 0.9rem; }
  .direction-code { font-size: 0.75rem; opacity: 0.8; }
  .ledger-table-wrap { overflow-x: auto; }
  .ledger-table { width: 100%; border-collapse: collapse; font-size: 0.78rem; }
  .ledger-table th, .ledger-table td {
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    padding: 4px 6px;
    text-align: left;
  }
  .tag-row { display: flex; flex-wrap: wrap; gap: 6px; }
  .flag-list { margin: 0; padding-left: 18px; font-size: 0.82rem; }
  .flag-list.missing { color: #e8a85a; }
  .flag-list.positive { color: #7fd4a0; }
  .muted { opacity: 0.65; font-size: 0.85rem; }
  .ev-card-actions { display: flex; flex-wrap: wrap; gap: 6px; margin-top: 12px; }
  .raw-payload {
    margin-top: 10px;
    max-height: 240px;
    overflow: auto;
    font-size: 0.7rem;
    background: rgba(0, 0, 0, 0.35);
    padding: 8px;
    border-radius: 6px;
  }
</style>
