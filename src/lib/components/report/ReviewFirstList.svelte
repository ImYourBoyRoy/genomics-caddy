<!-- ./src/lib/components/report/ReviewFirstList.svelte -->
<script lang="ts">
  /*
  Purpose: Report "Review first" list. One compact row per finding, grouped by
  health area; a row expands inline to show what the finding points you to
  (tests, medicines, conditions, diet, exercise, supplements).
  Inputs: report, plan (deriveActionablePlan), jump/open callbacks.
  Outputs: Accessible disclosure rows; no genotype values are rendered.
  Notes: Wording comes from the authored plan and layperson copy. The single
  app-wide safety line lives in the app footer, not in these rows.
  */
  import '#lib/styles/components/finding-rows.css';
  import type { GeneratedReport } from '../../types/genomics';
  import type { ActionablePlan, TopFinding } from '../../utils/actionabilityEngine';
  import { getCompactSimpleReviewAction, getLaypersonTranslation, getSimpleFindingCopy } from '../../utils/layperson';
  import {
    ACTION_DOMAINS,
    ACTION_DOMAIN_LABELS,
    actionsForFinding,
    buildPlanActionItems,
    evidenceMeterLevel,
    groupFindingsByArea,
    type PlanActionItem,
  } from '../../utils/planActions';

  interface Props {
    report: GeneratedReport;
    plan: ActionablePlan;
    onJumpToMarker?: (linkId: string) => void;
    onOpenPlan?: () => void;
  }

  let { report, plan, onJumpToMarker, onOpenPlan }: Props = $props();

  const MAX_ROWS = 9;
  const MAX_ACTIONS_PER_DOMAIN = 3;
  const METER_SEGMENTS = [1, 2, 3, 4] as const;

  let actionItems = $derived(buildPlanActionItems(plan));
  let groups = $derived(groupFindingsByArea(plan.topFindings, MAX_ROWS));
  let shownCount = $derived(groups.reduce((total, group) => total + group.findings.length, 0));
  // Rows are collapsed by default except the top-ranked finding, so the list stays calm.
  let openRows = $state<Record<string, boolean>>({});
  let firstFindingId = $derived(plan.topFindings[0]?.finding_id ?? '');

  function isOpen(findingId: string): boolean {
    return openRows[findingId] ?? findingId === firstFindingId;
  }

  function toggle(findingId: string): void {
    openRows = { ...openRows, [findingId]: !isOpen(findingId) };
  }

  function copyFor(finding: TopFinding) {
    for (const section of report.sections || []) {
      const marker = section.markers.find((candidate) => candidate.link_id === finding.link_id);
      if (marker) return getSimpleFindingCopy(marker, getLaypersonTranslation(marker));
    }
    return undefined;
  }

  function titleFor(finding: TopFinding): string {
    return finding.plain_title || copyFor(finding)?.plain_title || 'Research finding';
  }

  function directionFor(finding: TopFinding): string {
    return finding.direction_label || copyFor(finding)?.direction_label || 'Direction incomplete';
  }

  function meaningFor(finding: TopFinding): string {
    const copy = copyFor(finding);
    if (!copy) return 'A DNA-linked finding is available to review in the right personal context.';
    return [copy.signal, copy.why_it_matters].filter(Boolean).join(' ');
  }

  function nextStepFor(finding: TopFinding): string {
    const action = copyFor(finding)?.review_action ?? 'Review the detailed finding for personal relevance.';
    return getCompactSimpleReviewAction(action);
  }

  function concernFor(finding: TopFinding): 'high' | 'moderate' | 'low' {
    if (finding.priority_tone === 'danger') return 'high';
    if (finding.priority_tone === 'warning') return 'moderate';
    return 'low';
  }

  function actionsByDomain(finding: TopFinding) {
    const linked = actionsForFinding(actionItems, finding);
    return ACTION_DOMAINS.map((domain) => ({
      domain,
      items: linked.filter((item) => item.domain === domain),
    })).filter((entry) => entry.items.length > 0);
  }

  function polarityMark(item: PlanActionItem): string {
    if (item.polarity === 'avoid') return 'Avoid';
    if (item.polarity === 'discuss') return 'Discuss';
    return 'Lean in';
  }
</script>

<section class="action-queue review-first card" aria-labelledby="review-first-title">
  <header class="review-first-header">
    <div>
      <span class="review-first-kicker">Start here</span>
      <h3 id="review-first-title">Review first</h3>
      <p>The strongest DNA-linked signals in this report. Open a row to see what it points you to.</p>
    </div>
    <span class="review-first-count">{shownCount} shown</span>
  </header>

  {#if shownCount > 0}
    <div class="review-list">
      {#each groups as group (group.id)}
        <section class="review-group" aria-labelledby="review-group-{group.id}">
          <h4 id="review-group-{group.id}" class="review-group-title">
            {group.label}
            <span class="review-group-count">{group.findings.length}</span>
          </h4>
          <ul class="review-group-rows">
            {#each group.findings as { finding, rank } (finding.finding_id)}
              {@const open = isOpen(finding.finding_id)}
              {@const domainGroups = actionsByDomain(finding)}
              <li class="review-row" data-priority={rank} data-concern={concernFor(finding)} data-open={open}>
                <button
                  type="button"
                  class="review-row-head"
                  aria-expanded={open}
                  aria-controls={open ? `review-detail-${finding.finding_id}` : undefined}
                  onclick={() => toggle(finding.finding_id)}
                >
                  <span class="review-rank" aria-label="Priority {rank}">{rank}</span>
                  <span class="review-main">
                    <span class="review-title">{titleFor(finding)}</span>
                    <span class="review-direction">{directionFor(finding)}</span>
                  </span>
                  <span class="review-tags">
                    {#each domainGroups as entry (entry.domain)}
                      <span class="domain-tag" data-domain={entry.domain}>{ACTION_DOMAIN_LABELS[entry.domain]}</span>
                    {/each}
                  </span>
                  <span
                    class="evidence-meter"
                    role="img"
                    aria-label="Evidence strength {evidenceMeterLevel(finding.evidence_tiers)} of 4"
                  >
                    {#each METER_SEGMENTS as segment (segment)}
                      <span class="evidence-meter-segment" data-on={segment <= evidenceMeterLevel(finding.evidence_tiers)}></span>
                    {/each}
                  </span>
                  <span class="review-chevron" aria-hidden="true"></span>
                </button>

                {#if open}
                  <div class="review-detail" id="review-detail-{finding.finding_id}">
                    <p class="review-meaning">{meaningFor(finding)}</p>
                    <p class="review-next action-queue-next">
                      <strong>Next</strong> {nextStepFor(finding)}
                    </p>

                    {#if domainGroups.length > 0}
                      <div class="review-points">
                        <h5>This points you to</h5>
                        <dl class="review-points-list">
                          {#each domainGroups as entry (entry.domain)}
                            <div class="review-points-row">
                              <dt><span class="domain-tag" data-domain={entry.domain}>{ACTION_DOMAIN_LABELS[entry.domain]}</span></dt>
                              <dd>
                                <ul>
                                  {#each entry.items.slice(0, MAX_ACTIONS_PER_DOMAIN) as item (item.id)}
                                    <li data-polarity={item.polarity}>
                                      <span class="polarity-mark">{polarityMark(item)}</span>
                                      <span class="point-text">{item.text}</span>
                                    </li>
                                  {/each}
                                </ul>
                                {#if entry.items.length > MAX_ACTIONS_PER_DOMAIN}
                                  <span class="review-more">+{entry.items.length - MAX_ACTIONS_PER_DOMAIN} more in Plan</span>
                                {/if}
                              </dd>
                            </div>
                          {/each}
                        </dl>
                      </div>
                    {/if}

                    <div class="review-row-actions">
                      {#if onJumpToMarker}
                        <button class="btn btn-xs btn-link jump-btn" type="button" onclick={() => onJumpToMarker?.(finding.link_id)}>
                          View proof →
                        </button>
                      {/if}
                      {#if onOpenPlan && domainGroups.length > 0}
                        <button class="btn btn-xs btn-link jump-btn" type="button" onclick={() => onOpenPlan?.()}>
                          Open in Plan →
                        </button>
                      {/if}
                    </div>
                  </div>
                {/if}
              </li>
            {/each}
          </ul>
        </section>
      {/each}
    </div>
  {:else}
    <div class="review-empty">
      <strong>No immediate genetic follow-up prompts were generated.</strong>
      <span>Use routine care and selected personal context to guide follow-up.</span>
    </div>
  {/if}
</section>
