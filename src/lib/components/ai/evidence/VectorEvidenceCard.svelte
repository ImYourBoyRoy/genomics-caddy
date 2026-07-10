<!-- ./src/lib/components/ai/evidence/VectorEvidenceCard.svelte -->
<script lang="ts">
  import type { EvidenceCard } from "../../../types/research";
  import type { VariantNavTarget } from "../../../constants/traitCategories";
  import { userFindingFromEvidenceCard } from "../../../utils/userFinding";

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

  function formatP(p: number | undefined = undefined) {
    if (p == null) return "unknown";
    if (p < 0.001) return p.toExponential(2);
    return p.toFixed(6);
  }

  let userFinding = $derived(userFindingFromEvidenceCard(card));
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

  <section class="ev-user-finding impact-{userFinding.impact}">
    <div class="ev-user-heading">
      <div>
        <span class="finding-label">Scanned finding</span>
        <h4>{userFinding.title}</h4>
      </div>
      <div class="finding-status-stack" aria-label="Finding status">
        <span class="impact-chip">{userFinding.impactLabel}</span>
        <span class="confidence-chip">{userFinding.confidenceLabel}</span>
      </div>
    </div>
    <p class="impact-description">{userFinding.impactDescription}</p>
    <p>{userFinding.plainEnglishMeaning}</p>
    <div class="finding-badges">
      {#each userFinding.evidenceBadges as badge}
        <span>{badge}</span>
      {/each}
    </div>
    <p class="safety-boundary">{userFinding.safetyBoundary}</p>
    <div class="next-action-callout">
      <strong>Suggested next action</strong>
      <span>{userFinding.primaryNextStep}</span>
    </div>

    {#if expanded}
      <div class="finding-detail-grid">
        <div>
          <h5>Why surfaced</h5>
          <ul>
            {#each userFinding.whySurfaced as reason}
              <li>{reason}</li>
            {/each}
          </ul>
        </div>
        <div>
          <h5>Useful next steps</h5>
          <ul>
            {#each userFinding.suggestedNextSteps as step}
              <li>{step}</li>
            {/each}
          </ul>
        </div>
      </div>
    {/if}
  </section>

  <section class="ev-section summary technical-note">
    <h5>Indexed evidence note</h5>
    <p>{card.primary_trait || "Trait unknown"}</p>
    {#if card.trait_category}
      <span class="trait-cat">{card.trait_category.replace(/_/g, " ")}</span>
    {/if}
    <p class="synthesis">
      {#if expanded || card.synthesis.length <= 260}
        {card.synthesis}
      {:else}
        {card.synthesis.slice(0, 260)}… Expand details for the full indexed source text.
      {/if}
    </p>
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
      {showRaw ? "Hide raw" : "Raw payload"}
    </button>
    {#if onShowSimilar}
      <button type="button" class="btn btn-secondary btn-xs" onclick={() => onShowSimilar?.(card.rsid)}>
        Find similar
      </button>
    {/if}
    {#if onExportPacket}
      <button type="button" class="btn btn-secondary btn-xs" onclick={() => onExportPacket?.(card.rsid)}>
        Export evidence
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
  .ev-user-finding {
    border: 1px solid rgba(56, 189, 248, 0.18);
    border-radius: 10px;
    padding: 12px;
    margin: 10px 0 12px;
    background: linear-gradient(135deg, rgba(56, 189, 248, 0.08), rgba(0, 0, 0, 0.16));
  }
  .ev-user-finding.impact-clinical_review {
    border-color: rgba(248, 113, 113, 0.32);
    background: linear-gradient(135deg, rgba(127, 29, 29, 0.22), rgba(0, 0, 0, 0.16));
  }
  .ev-user-finding.impact-wellness_relevant {
    border-color: rgba(52, 211, 153, 0.32);
    background: linear-gradient(135deg, rgba(6, 78, 59, 0.22), rgba(0, 0, 0, 0.16));
  }
  .ev-user-finding.impact-low_confidence {
    border-color: rgba(251, 191, 36, 0.3);
    background: linear-gradient(135deg, rgba(113, 63, 18, 0.18), rgba(0, 0, 0, 0.16));
  }
  .ev-user-heading {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    align-items: flex-start;
  }
  .finding-label {
    display: inline-block;
    font-size: 0.62rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: #7dd3fc;
    margin-bottom: 4px;
  }
  .ev-user-heading h4 {
    margin: 0;
    font-size: 0.95rem;
    line-height: 1.3;
  }
  .confidence-chip {
    flex-shrink: 0;
    border-radius: 999px;
    padding: 3px 8px;
    font-size: 0.68rem;
    background: rgba(255, 255, 255, 0.08);
  }
  .finding-status-stack {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 5px;
    flex-shrink: 0;
    max-width: 44%;
  }
  .impact-chip {
    border-radius: 999px;
    padding: 3px 8px;
    font-size: 0.68rem;
    font-weight: 700;
    color: #dbeafe;
    background: rgba(59, 130, 246, 0.14);
  }
  .ev-user-finding p {
    margin: 8px 0;
    font-size: 0.86rem;
    line-height: 1.45;
  }
  .impact-description {
    color: #bfdbfe;
    font-size: 0.78rem !important;
  }
  .finding-badges {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .finding-badges span {
    font-size: 0.68rem;
    border-radius: 999px;
    padding: 2px 7px;
    background: rgba(255, 255, 255, 0.08);
  }
  .safety-boundary {
    color: #fcd34d;
    font-size: 0.74rem !important;
  }
  .next-action-callout {
    display: grid;
    gap: 3px;
    margin-top: 9px;
    padding: 9px 10px;
    border-radius: 9px;
    border: 1px solid rgba(125, 211, 252, 0.22);
    background: rgba(14, 165, 233, 0.08);
  }
  .next-action-callout strong {
    font-size: 0.68rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: #7dd3fc;
  }
  .next-action-callout span {
    font-size: 0.8rem;
    line-height: 1.4;
  }
  .finding-detail-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 10px;
    margin-top: 8px;
  }
  .finding-detail-grid h5 {
    margin: 0 0 5px;
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    opacity: 0.8;
  }
  .finding-detail-grid ul {
    margin: 0;
    padding-left: 18px;
    font-size: 0.78rem;
    line-height: 1.4;
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
  .technical-note {
    padding-top: 2px;
    opacity: 0.9;
  }
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
