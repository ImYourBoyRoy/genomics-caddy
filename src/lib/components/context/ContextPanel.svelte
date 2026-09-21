<script lang="ts">
  import type { GenomeSample } from '../../types/genomics';
  import cycleSupport from '../../marker-packs/cycle_support_guidance.json';
  import ReproductiveContextEditor from '../ai/ReproductiveContextEditor.svelte';
  import {
    clearUnassignedLegacyAiProfile,
    loadUnassignedLegacyAiProfile,
    profileContextHasContent,
    saveProfileContext,
    type ProfileContext,
    type ProfileContextNotes,
  } from '../../utils/profileContext';
  import {
    parseContextList,
    EMPTY_PERSONAL_DIETARY_PROFILE,
    type PersonalSafetyContext,
  } from '../../utils/personalSafetyContext';
  import { reproductiveContextOptionIsSuggestedForGeneticSex } from '../../utils/reproductiveContext';
  import { formatGeneticSexLabel } from '../../utils/uiLabels';
  import '$lib/styles/components/context-panel.css';

  interface Props {
    selectedSample: GenomeSample;
    profileContext: ProfileContext;
    onOpenDiary?: () => void;
  }

  let {
    selectedSample,
    profileContext = $bindable(),
    onOpenDiary,
  }: Props = $props();

  type NoteKey = keyof ProfileContextNotes;
  type SafetyKey = 'medications' | 'supplements' | 'allergies' | 'symptoms' | 'labObservations';
  type DietaryKey = keyof typeof EMPTY_PERSONAL_DIETARY_PROFILE;

  const noteFields: Array<{ key: NoteKey; label: string; placeholder: string }> = [
    { key: 'goals', label: 'Goals', placeholder: 'What would you like to improve or understand?' },
    { key: 'challenges', label: 'Challenges and symptoms', placeholder: 'Symptoms, patterns, or day-to-day challenges.' },
    { key: 'relevantBodySystems', label: 'Relevant body systems or life context', placeholder: 'For example: thyroid, cardiovascular, reproductive, sleep.' },
    { key: 'diagnoses', label: 'Diagnoses or working diagnoses', placeholder: 'Diagnoses you choose to record for context.' },
    { key: 'supportiveTests', label: 'Supportive tests or clinician findings', placeholder: 'Test names, dates, or concise clinician notes.' },
    { key: 'reproductiveHormoneContext', label: 'Additional hormone or reproductive notes', placeholder: 'Optional notes not covered by the structured fields below.' },
  ];

  const safetyFields: Array<{ key: SafetyKey; label: string; placeholder: string }> = [
    { key: 'medications', label: 'Medications', placeholder: 'Exact product names, active ingredients, dose, or schedule.' },
    { key: 'supplements', label: 'Supplements and OTC products', placeholder: 'Product names and approximate use.' },
    { key: 'allergies', label: 'Allergies and intolerances', placeholder: 'Food, medication, or environmental reactions.' },
    { key: 'symptoms', label: 'Symptoms and timing', placeholder: 'What happens, when it happens, and what changes it.' },
    { key: 'labObservations', label: 'Recent labs or clinician findings', placeholder: 'Test, value, unit, date, and reference range when known.' },
  ];

  const dietaryFields: Array<{ key: DietaryKey; label: string; placeholder: string }> = [
    { key: 'hard_exclusions', label: 'Foods or ingredients to exclude', placeholder: 'One item per line.' },
    { key: 'allergies_confirmed', label: 'Confirmed food allergies', placeholder: 'One item per line.' },
    { key: 'allergies_suspected', label: 'Suspected food reactions', placeholder: 'One item per line.' },
    { key: 'religious_cultural_profiles', label: 'Religious or cultural pattern', placeholder: 'For example: halal, kosher, vegetarian.' },
    { key: 'ethical_preference_profiles', label: 'Ethical preferences', placeholder: 'For example: vegan, local, low-waste.' },
    { key: 'medical_diet_profiles', label: 'Medical diet pattern', placeholder: 'For example: low-FODMAP, gluten-free, renal diet.' },
    { key: 'goals', label: 'Diet goals', placeholder: 'For example: high protein, weight change, performance.' },
  ];

  let legacyContext = $state(loadUnassignedLegacyAiProfile());
  let contextOptions = $derived(cycleSupport.context_options.filter((option) => option.id !== 'none_or_unknown'));
  let suggestedContextOptions = $derived(contextOptions.filter((option) =>
    reproductiveContextOptionIsSuggestedForGeneticSex(option.id, selectedSample.genetic_sex),
  ));
  let otherContextOptions = $derived(contextOptions.filter((option) =>
    !reproductiveContextOptionIsSuggestedForGeneticSex(option.id, selectedSample.genetic_sex),
  ));
  let hasContent = $derived(profileContextHasContent(profileContext));

  $effect(() => {
    profileContext.safety;
    profileContext.selectedReproductiveContext;
    profileContext.exportPreferences;
    saveProfileContext(selectedSample.id, profileContext);
  });

  function persist(): void {
    saveProfileContext(selectedSample.id, profileContext);
  }

  function updateNote(key: NoteKey, event: Event): void {
    profileContext = {
      ...profileContext,
      notes: {
        ...profileContext.notes,
        [key]: (event.currentTarget as HTMLTextAreaElement).value,
      },
    };
    persist();
  }

  function updateSafety(key: SafetyKey, event: Event): void {
    profileContext = {
      ...profileContext,
      safety: {
        ...profileContext.safety,
        [key]: parseContextList((event.currentTarget as HTMLTextAreaElement).value),
      },
    };
    persist();
  }

  function updateDietary(key: DietaryKey, event: Event): void {
    const nextProfile = {
      ...EMPTY_PERSONAL_DIETARY_PROFILE,
      ...(profileContext.safety.dietaryProfile || {}),
      [key]: parseContextList((event.currentTarget as HTMLTextAreaElement).value),
    };
    const hasDietaryValues = Object.values(nextProfile).some((items) => items.length > 0);
    const nextSafety: PersonalSafetyContext = {
      ...profileContext.safety,
      ...(hasDietaryValues ? { dietaryProfile: nextProfile } : { dietaryProfile: undefined }),
    };
    profileContext = { ...profileContext, safety: nextSafety };
    persist();
  }

  function setReproductiveContext(event: Event): void {
    profileContext = {
      ...profileContext,
      selectedReproductiveContext: (event.currentTarget as HTMLSelectElement).value,
    };
    persist();
  }

  function importLegacyContext(): void {
    if (!legacyContext) return;
    const mergedSafety: PersonalSafetyContext = {
      ...profileContext.safety,
      medications: parseContextList([
        ...profileContext.safety.medications,
        ...legacyContext.safety.medications,
      ].join('\n')),
      supplements: parseContextList([
        ...profileContext.safety.supplements,
        ...legacyContext.safety.supplements,
      ].join('\n')),
      labObservations: parseContextList([
        ...profileContext.safety.labObservations,
        ...legacyContext.safety.labObservations,
      ].join('\n')),
    };
    profileContext = {
      ...profileContext,
      notes: {
        ...profileContext.notes,
        goals: legacyContext.goals || profileContext.notes.goals,
        challenges: legacyContext.challenges || profileContext.notes.challenges,
        relevantBodySystems: legacyContext.relevantBodySystems || profileContext.notes.relevantBodySystems,
        reproductiveHormoneContext: legacyContext.reproductiveHormoneContext || profileContext.notes.reproductiveHormoneContext,
        diet: legacyContext.diet || profileContext.notes.diet,
        diagnoses: legacyContext.diagnoses || profileContext.notes.diagnoses,
        supportiveTests: legacyContext.supportiveTests || profileContext.notes.supportiveTests,
      },
      safety: mergedSafety,
    };
    persist();
    clearUnassignedLegacyAiProfile();
    legacyContext = null;
  }

  function dismissLegacyContext(): void {
    clearUnassignedLegacyAiProfile();
    legacyContext = null;
  }
</script>

<div class="profile-context-page">
  <header class="context-hero">
    <div>
      <span class="context-kicker">Optional profile workspace</span>
      <h2>Context for Chat and handoffs</h2>
      <p>Add only what helps an AI assistant or clinician understand this profile. The Trait Report remains DNA-focused.</p>
    </div>
    <div class="context-profile-badge">
      <strong>{selectedSample.name}</strong>
      <span>{formatGeneticSexLabel(selectedSample.genetic_sex)}</span>
    </div>
  </header>

  <section class="context-toolbar card" aria-labelledby="context-focus-title">
    <div>
      <span class="context-section-kicker">Optional focus</span>
      <h3 id="context-focus-title">What should this profile be reviewed around?</h3>
      <p>Suggestions are organized from the chromosome-call context; choose only what is relevant.</p>
    </div>
    <div class="context-toolbar-controls">
      <select aria-labelledby="context-focus-title" value={profileContext.selectedReproductiveContext} onchange={setReproductiveContext}>
        <option value="">No focus selected</option>
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
      {#if onOpenDiary}
        <button type="button" class="btn btn-secondary" onclick={() => onOpenDiary?.()}>Open Diary</button>
      {/if}
    </div>
  </section>

  {#if legacyContext}
    <section class="legacy-context card" aria-labelledby="legacy-context-title">
      <div>
        <strong id="legacy-context-title">Older AI profile found</strong>
        <p>This saved profile is not attached to a genome. Import it into {selectedSample.name} only if it belongs to this profile.</p>
      </div>
      <div class="legacy-context-actions">
        <button type="button" class="btn btn-secondary" onclick={importLegacyContext}>Import into this profile</button>
        <button type="button" class="btn btn-ghost" onclick={dismissLegacyContext}>Dismiss and delete</button>
      </div>
    </section>
  {/if}

  <div class="context-grid">
    <section class="context-card card" aria-labelledby="context-notes-title">
      <div class="context-card-heading">
        <div>
          <span class="context-section-kicker">Personal context</span>
          <h3 id="context-notes-title">Goals and lived context</h3>
        </div>
      </div>
      {#each noteFields as field (field.key)}
        <label class="context-field">
          <span>{field.label}</span>
          <textarea value={profileContext.notes[field.key]} placeholder={field.placeholder} oninput={(event) => updateNote(field.key, event)}></textarea>
        </label>
      {/each}
    </section>

    <section class="context-card card" aria-labelledby="context-safety-title">
      <div class="context-card-heading">
        <div>
          <span class="context-section-kicker">Safety context</span>
          <h3 id="context-safety-title">Medications and health details</h3>
        </div>
      </div>
      {#each safetyFields as field (field.key)}
        <label class="context-field">
          <span>{field.label}</span>
          <textarea value={profileContext.safety[field.key].join('\n')} placeholder={field.placeholder} oninput={(event) => updateSafety(field.key, event)}></textarea>
        </label>
      {/each}
    </section>

    <section class="context-card card" aria-labelledby="context-diet-title">
      <div class="context-card-heading">
        <div>
          <span class="context-section-kicker">Food context</span>
          <h3 id="context-diet-title">Diet and exclusions</h3>
        </div>
      </div>
      <label class="context-field">
        <span>Diet pattern</span>
        <textarea value={profileContext.notes.diet} placeholder="For example: Mediterranean, vegetarian, high protein." oninput={(event) => updateNote('diet', event)}></textarea>
      </label>
      {#each dietaryFields as field (field.key)}
        <label class="context-field context-field-compact">
          <span>{field.label}</span>
          <textarea value={profileContext.safety.dietaryProfile?.[field.key]?.join('\n') || ''} placeholder={field.placeholder} oninput={(event) => updateDietary(field.key, event)}></textarea>
        </label>
      {/each}
    </section>
  </div>

  <section class="context-card context-reproductive card" aria-labelledby="context-reproductive-title">
    <div class="context-card-heading">
      <div>
        <span class="context-section-kicker">Optional structured details</span>
        <h3 id="context-reproductive-title">Reproductive, hormone, and care context</h3>
        <p>Use the structured fields when cycle timing, hormone products, fertility, pregnancy, pelvic symptoms, or androgen care is relevant.</p>
      </div>
    </div>
    <ReproductiveContextEditor
      bind:personalSafetyContext={profileContext.safety}
      sampleId={selectedSample.id}
      reproductiveContext={profileContext.selectedReproductiveContext}
      compact
    />
  </section>

  <section class="context-export card" aria-labelledby="context-export-title">
    <div>
      <span class="context-section-kicker">Export readiness</span>
      <h3 id="context-export-title">{hasContent ? 'Context is ready to share' : 'No context added yet'}</h3>
      <p>When you export or enable profile context for Connected Chat, these saved details travel with {selectedSample.name}'s DNA review as separate user-provided context. AI and clinician handoffs always include the raw genotype calls needed to trace findings.</p>
    </div>
    <div class="context-export-controls">
      <span class="context-raw-policy" role="note">Raw genotype calls: always included for AI and clinician handoffs</span>
      <span class="context-status" class:context-status-ready={hasContent}>{hasContent ? 'Saved locally' : 'Optional context'}</span>
    </div>
  </section>
</div>
