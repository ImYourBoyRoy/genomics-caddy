<!-- ./src/lib/components/report/PlanView.svelte -->
<script lang="ts">
  /*
  Purpose: "Your plan" tab. Gathers every DNA-linked next step (tests,
  medicines, conditions, diet, exercise, supplements) in one scannable list so
  people can act without re-reading the whole report.
  Inputs: generatedReport (or null while loading), onViewProof, onOpenReport.
  Outputs: Domain filter chips, per-domain action rows, "View proof" links, and
  a copyable lab request list. No genotype values are rendered.
  Notes: Items come from deriveActionablePlan via planActions; evidence labels
  and avoid/discuss framing are preserved. Safety copy lives in the app footer.
  */
  import '$lib/styles/components/finding-rows.css';
  import '$lib/styles/components/plan-view.css';
  import type { GeneratedReport } from '../../types/genomics';
  import { buildLabRequestListText, deriveActionablePlan } from '../../utils/actionabilityEngine';
  import {
    ACTION_DOMAINS,
    ACTION_DOMAIN_LABELS,
    buildPlanActionItems,
    groupActionsByDomain,
    type ActionDomain,
    type PlanActionItem,
  } from '../../utils/planActions';

  interface Props {
    generatedReport: GeneratedReport | null;
    isGeneratingReport?: boolean;
    /** Reveal the proof (DNA finding) behind an action, by report link id. */
    onViewProof?: (linkId: string) => void;
    onOpenReport?: () => void;
  }

  let { generatedReport, isGeneratingReport = false, onViewProof, onOpenReport }: Props = $props();

  const DOMAIN_BLURBS: Record<ActionDomain, string> = {
    tests: 'Labs and checks worth asking about.',
    medicines: 'Prescription pathways to raise with a prescriber or pharmacist.',
    conditions: 'Patterns your DNA is associated with, to bring to a clinician.',
    diet: 'Foods to lean toward or steer away from.',
    exercise: 'Training and recovery starting points.',
    supplements: 'Options to consider, and ones to avoid.',
  };

  let plan = $derived(generatedReport ? deriveActionablePlan(generatedReport) : null);
  let items = $derived(plan ? buildPlanActionItems(plan) : []);
  let byDomain = $derived(groupActionsByDomain(items));
  let activeDomain = $state<ActionDomain | 'all'>('all');
  let visibleDomains = $derived(
    ACTION_DOMAINS.filter((domain) => byDomain[domain].length > 0 && (activeDomain === 'all' || activeDomain === domain)),
  );
  // Each area shows its first rows; the rest are one click away so 200+ items never become a wall.
  const INITIAL_ROWS = 6;
  let expandedDomains = $state<Partial<Record<ActionDomain, boolean>>>({});
  let labRequestCopied = $state(false);
  let labRequestFailed = $state(false);

  function proofLinkFor(item: PlanActionItem): string | undefined {
    if (!generatedReport) return undefined;
    const ids = new Set(item.markerIds);
    for (const section of generatedReport.sections) {
      const marker = section.markers.find((candidate) => ids.has(candidate.link_id) || ids.has(candidate.rsid));
      if (marker?.link_id) return marker.link_id;
    }
    return undefined;
  }

  function rowsFor(domain: ActionDomain): PlanActionItem[] {
    return expandedDomains[domain] ? byDomain[domain] : byDomain[domain].slice(0, INITIAL_ROWS);
  }

  function toggleDomain(domain: ActionDomain): void {
    expandedDomains = { ...expandedDomains, [domain]: !expandedDomains[domain] };
  }

  /** Hide placeholder purpose text that only restates that the test depends on context. */
  function usefulDetail(item: PlanActionItem): string {
    return /^purpose depends on/i.test(item.detail.trim()) ? '' : item.detail;
  }

  function polarityLabel(item: PlanActionItem): string {
    if (item.polarity === 'avoid') return 'Avoid';
    if (item.polarity === 'discuss') return 'Discuss';
    return 'Lean in';
  }

  async function copyLabRequestList(): Promise<void> {
    if (!plan) return;
    try {
      await navigator.clipboard.writeText(buildLabRequestListText(plan.labTests));
      labRequestCopied = true;
      labRequestFailed = false;
      window.setTimeout(() => { labRequestCopied = false; }, 1800);
    } catch {
      labRequestCopied = false;
      labRequestFailed = true;
    }
  }
</script>

<div class="plan-view">
  <header class="plan-header">
    <div>
      <span class="review-first-kicker">Your plan</span>
      <h2>What your DNA points you to</h2>
      <p>Every suggested next step from your report, grouped by what you would do with it.</p>
    </div>
    {#if onOpenReport}
      <button type="button" class="btn btn-secondary btn-sm" onclick={() => onOpenReport?.()}>Back to report</button>
    {/if}
  </header>

  {#if isGeneratingReport && !generatedReport}
    <div class="plan-empty" role="status">Building your plan…</div>
  {:else if !generatedReport || items.length === 0}
    <div class="plan-empty" role="status">
      <strong>No plan items yet.</strong>
      <span>Import a DNA file and open the report; matching tests, foods, and options appear here.</span>
    </div>
  {:else}
    <div class="plan-filters" role="group" aria-label="Filter plan by area">
      <button type="button" class="plan-filter" aria-pressed={activeDomain === 'all'} onclick={() => (activeDomain = 'all')}>
        All <span class="plan-filter-count">{items.length}</span>
      </button>
      {#each ACTION_DOMAINS as domain (domain)}
        {#if byDomain[domain].length > 0}
          <button
            type="button"
            class="plan-filter"
            data-domain={domain}
            aria-pressed={activeDomain === domain}
            onclick={() => (activeDomain = domain)}
          >
            {ACTION_DOMAIN_LABELS[domain]} <span class="plan-filter-count">{byDomain[domain].length}</span>
          </button>
        {/if}
      {/each}
    </div>

    <div class="plan-sections">
      {#each visibleDomains as domain (domain)}
        <section class="plan-section card" aria-labelledby="plan-{domain}-title">
          <header class="plan-section-head">
            <div>
              <h3 id="plan-{domain}-title"><span class="domain-tag" data-domain={domain}>{ACTION_DOMAIN_LABELS[domain]}</span></h3>
              <p>{DOMAIN_BLURBS[domain]}</p>
            </div>
            {#if domain === 'tests' && plan && plan.labTests.length > 0}
              <button type="button" class="btn btn-secondary btn-sm" onclick={copyLabRequestList}>
                {labRequestCopied ? 'Copied' : labRequestFailed ? 'Copy failed' : 'Copy lab request list'}
              </button>
            {/if}
          </header>
          <ul class="plan-items">
            {#each rowsFor(domain) as item (item.id)}
              {@const proofLink = proofLinkFor(item)}
              <li class="plan-item" data-polarity={item.polarity}>
                <span class="polarity-mark">{polarityLabel(item)}</span>
                <div class="plan-item-body">
                  <span class="plan-item-text">{item.text}</span>
                  {#if usefulDetail(item)}
                    <span class="plan-item-detail">{usefulDetail(item)}</span>
                  {/if}
                  <span class="plan-item-meta">
                    {#if item.genes.length > 0}<span>{item.genes.slice(0, 4).join(' · ')}</span>{/if}
                    {#if item.evidence}<span>{item.evidence}</span>{/if}
                  </span>
                </div>
                {#if proofLink && onViewProof}
                  <button type="button" class="plan-proof-link" onclick={() => onViewProof?.(proofLink)}>
                    View proof →
                  </button>
                {/if}
              </li>
            {/each}
          </ul>
          {#if byDomain[domain].length > INITIAL_ROWS}
            <button type="button" class="plan-more" aria-expanded={!!expandedDomains[domain]} onclick={() => toggleDomain(domain)}>
              {expandedDomains[domain] ? 'Show fewer' : `Show all ${byDomain[domain].length}`}
            </button>
          {/if}
        </section>
      {/each}
    </div>
  {/if}
</div>
