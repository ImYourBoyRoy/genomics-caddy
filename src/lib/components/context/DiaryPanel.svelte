<script lang="ts">
  import type { GenomeSample } from '../../types/genomics';
  import cycleSupport from '../../marker-packs/cycle_support_guidance.json';
  import CycleDiaryEditor from '../ai/CycleDiaryEditor.svelte';
  import {
    saveProfileContext,
    type ProfileContext,
  } from '../../utils/profileContext';
  import { cycleDiaryAppliesToContext } from '../../utils/cycleDiary';
  import { reproductiveContextOptionIsSuggestedForGeneticSex } from '../../utils/reproductiveContext';

  interface Props {
    selectedSample: GenomeSample;
    profileContext: ProfileContext;
    onOpenContext?: () => void;
  }

  let {
    selectedSample,
    profileContext = $bindable(),
    onOpenContext,
  }: Props = $props();

  let contextOptions = $derived(cycleSupport.context_options.filter((option) => option.id !== 'none_or_unknown'));
  let suggestedContextOptions = $derived(contextOptions.filter((option) =>
    reproductiveContextOptionIsSuggestedForGeneticSex(option.id, selectedSample.genetic_sex),
  ));
  let otherContextOptions = $derived(contextOptions.filter((option) =>
    !reproductiveContextOptionIsSuggestedForGeneticSex(option.id, selectedSample.genetic_sex),
  ));
  let diaryAvailable = $derived(cycleDiaryAppliesToContext(profileContext.selectedReproductiveContext));

  $effect(() => {
    profileContext.safety;
    profileContext.selectedReproductiveContext;
    saveProfileContext(selectedSample.id, profileContext);
  });

  function setContext(event: Event): void {
    profileContext = {
      ...profileContext,
      selectedReproductiveContext: (event.currentTarget as HTMLSelectElement).value,
    };
    saveProfileContext(selectedSample.id, profileContext);
  }
</script>

<div class="diary-page">
  <header class="diary-hero">
    <div>
      <span class="diary-kicker">Optional local tracker</span>
      <h2>Diary</h2>
      <p>Record observations over time when a pattern is worth tracking. Entries stay with this profile and can be included in exports.</p>
    </div>
    <div class="diary-profile"><strong>{selectedSample.name}</strong><span>{profileContext.selectedReproductiveContext ? 'Context selected' : 'No diary context selected'}</span></div>
  </header>

  <section class="diary-context card" aria-labelledby="diary-context-title">
    <div>
      <span class="diary-section-kicker">Diary type</span>
      <h3 id="diary-context-title">Choose a context to track</h3>
      <p>The diary currently supports cycle and symptom observations. Other context types remain available in Context for Chat and exports.</p>
    </div>
    <div class="diary-context-controls">
      <select aria-labelledby="diary-context-title" value={profileContext.selectedReproductiveContext} onchange={setContext}>
        <option value="">No diary selected</option>
        <optgroup label="Suggested for this profile">
          {#each suggestedContextOptions as option (option.id)}
            <option value={option.id}>{option.label}</option>
          {/each}
        </optgroup>
        {#if otherContextOptions.length > 0}
          <optgroup label="Other contexts — choose if relevant">
            {#each otherContextOptions as option (option.id)}
              <option value={option.id}>{option.label}</option>
            {/each}
          </optgroup>
        {/if}
      </select>
      {#if onOpenContext}
        <button type="button" class="btn btn-secondary" onclick={() => onOpenContext?.()}>Open Context</button>
      {/if}
    </div>
  </section>

  {#if diaryAvailable}
    <section class="diary-editor card" aria-labelledby="diary-editor-title">
      <div class="diary-editor-heading">
        <div>
          <span class="diary-section-kicker">{cycleSupport.diary_schema.title}</span>
          <h3 id="diary-editor-title">Track observations for {selectedSample.name}</h3>
        </div>
        <span class="diary-local-label">Saved locally</span>
      </div>
      <CycleDiaryEditor
        bind:personalSafetyContext={profileContext.safety}
        sampleId={selectedSample.id}
        reproductiveContext={profileContext.selectedReproductiveContext}
        expandedByDefault
      />
    </section>
  {:else}
    <section class="diary-empty card" aria-labelledby="diary-empty-title">
      <div class="diary-empty-mark" aria-hidden="true">✦</div>
      <div>
        <h3 id="diary-empty-title">Your diary is optional</h3>
        <p>Select a cycle or symptom context above when you want to record observations. Nothing is required for the DNA report.</p>
      </div>
    </section>
  {/if}
</div>

<style>
  .diary-page { display: flex; flex-direction: column; gap: 1rem; max-width: 1500px; margin: 0 auto; padding-bottom: 2rem; }
  .diary-hero, .diary-context, .diary-editor-heading, .diary-empty { display: flex; align-items: center; justify-content: space-between; gap: 1.25rem; }
  .diary-hero { padding: 0.25rem 0 0.5rem; }
  .diary-kicker, .diary-section-kicker { color: var(--accent); font-size: 0.68rem; font-weight: 800; letter-spacing: 0.08em; text-transform: uppercase; }
  .diary-hero h2, .diary-context h3, .diary-editor h3, .diary-empty h3 { margin: 0.2rem 0 0; color: var(--text-primary); }
  .diary-hero p, .diary-context p, .diary-empty p { margin: 0.35rem 0 0; color: var(--text-secondary); font-size: 0.78rem; line-height: 1.45; }
  .diary-profile { display: flex; flex-direction: column; align-items: flex-end; gap: 0.15rem; color: var(--text-secondary); font-size: 0.76rem; }
  .diary-profile strong { color: var(--text-primary); font-size: 0.9rem; }
  .diary-context, .diary-editor, .diary-empty { padding: 1rem 1.1rem; }
  .diary-context-controls { display: flex; align-items: center; gap: 0.65rem; min-width: min(34rem, 45%); }
  .diary-context-controls select { flex: 1; min-width: 0; }
  .diary-editor-heading { margin-bottom: 0.8rem; }
  .diary-local-label { color: var(--status-success-text); font-size: 0.74rem; font-weight: 700; }
  .diary-empty { justify-content: flex-start; border-color: var(--status-info-border); background: var(--status-info-soft-bg); }
  .diary-empty-mark { display: grid; place-items: center; width: 2.5rem; height: 2.5rem; flex: 0 0 auto; border-radius: 0.7rem; background: var(--status-info-bg); color: var(--status-info-text); font-size: 1.3rem; }
  .diary-context-controls select:focus { outline: 2px solid var(--accent); outline-offset: 1px; }
  @media (max-width: 760px) {
    .diary-hero, .diary-context { align-items: flex-start; flex-direction: column; }
    .diary-profile { align-items: flex-start; }
    .diary-context-controls { width: 100%; min-width: 0; }
  }
</style>
