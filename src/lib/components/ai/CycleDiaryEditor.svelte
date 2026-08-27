<!-- Resource-authored, local daily cycle and symptom diary. -->
<script lang="ts">
  import {
    CYCLE_DIARY_SCHEMA,
    cycleDiaryAppliesToContext,
    normalizeCycleDiaryEntry,
    populatedCycleDiaryFields,
    type CycleDiaryField,
    type CycleDiaryValues,
  } from '../../utils/cycleDiary';
  import {
    savePersonalSafetyContext,
    type PersonalSafetyContext,
  } from '../../utils/personalSafetyContext';

  interface Props {
    personalSafetyContext: PersonalSafetyContext;
    sampleId?: number;
    reproductiveContext?: string;
  }

  let {
    personalSafetyContext = $bindable(),
    sampleId,
    reproductiveContext,
  }: Props = $props();

  let expanded = $state(false);
  let error = $state('');
  let draft = $state<CycleDiaryValues>({});
  let applicable = $derived(cycleDiaryAppliesToContext(reproductiveContext));
  let entries = $derived(personalSafetyContext.cycleDiary || []);
  let visibleEntries = $derived(entries.slice(0, 14));

  function fieldMin(field: CycleDiaryField): number | undefined {
    return 'min' in field ? field.min : undefined;
  }

  function fieldMax(field: CycleDiaryField): number | undefined {
    return 'max' in field ? field.max : undefined;
  }

  function updateField(fieldId: string, event: Event): void {
    const target = event.currentTarget as HTMLInputElement | HTMLTextAreaElement;
    draft = { ...draft, [fieldId]: target.value };
    error = '';
  }

  function addEntry(): void {
    const entry = normalizeCycleDiaryEntry({
      id: `diary-${draft.entry_date || 'undated'}-${Date.now()}`,
      values: draft,
    });
    if (!entry) {
      error = 'Add the observation date before saving this entry.';
      return;
    }

    const nextEntries = [
      entry,
      ...entries.filter((existing) => existing.values.entry_date !== entry.values.entry_date),
    ];
    personalSafetyContext = {
      ...personalSafetyContext,
      cycleDiary: nextEntries,
    };
    savePersonalSafetyContext(sampleId, personalSafetyContext);
    draft = {};
    error = '';
  }

  function removeEntry(entryId: string): void {
    const nextEntries = entries.filter((entry) => entry.id !== entryId);
    personalSafetyContext = {
      ...personalSafetyContext,
      ...(nextEntries.length > 0 ? { cycleDiary: nextEntries } : { cycleDiary: undefined }),
    };
    savePersonalSafetyContext(sampleId, personalSafetyContext);
  }
</script>

{#if applicable}
  <section class="cycle-diary" aria-labelledby="cycle-diary-title">
    <button
      type="button"
      class="cycle-diary-toggle"
      aria-expanded={expanded ? 'true' : 'false'}
      onclick={() => expanded = !expanded}
    >
      <span>
        <strong id="cycle-diary-title">{CYCLE_DIARY_SCHEMA.title}</strong>
        <small>{entries.length > 0 ? `${entries.length} local entr${entries.length === 1 ? 'y' : 'ies'}` : 'Track patterns across cycles'}</small>
      </span>
      <span aria-hidden="true">{expanded ? '▾' : '▸'}</span>
    </button>

    {#if expanded}
      <div class="cycle-diary-body">
        <p class="cycle-diary-description">{CYCLE_DIARY_SCHEMA.description}</p>
        <p class="cycle-diary-privacy">🔒 {CYCLE_DIARY_SCHEMA.privacy_note}</p>

        <div class="cycle-diary-form">
          {#each CYCLE_DIARY_SCHEMA.fields as field (field.id)}
            <div class="cycle-diary-field">
              <label for="cycle-diary-{field.id}">{field.label}</label>
              {#if field.input_type === 'textarea'}
                <textarea
                  id="cycle-diary-{field.id}"
                  value={draft[field.id] || ''}
                  placeholder={field.placeholder}
                  maxlength="500"
                  oninput={(event) => updateField(field.id, event)}
                ></textarea>
              {:else}
                <input
                  id="cycle-diary-{field.id}"
                  type={field.input_type}
                  value={draft[field.id] || ''}
                  placeholder={field.placeholder}
                  min={fieldMin(field)}
                  max={fieldMax(field)}
                  step={'step' in field ? field.step : undefined}
                  oninput={(event) => updateField(field.id, event)}
                />
              {/if}
              <small>{field.help}</small>
            </div>
          {/each}
        </div>

        <div class="cycle-diary-actions">
          <button type="button" class="btn btn-secondary btn-sm" onclick={addEntry}>Save diary entry</button>
          {#if error}<span class="cycle-diary-error" role="alert">{error}</span>{/if}
        </div>

        {#if entries.length > 0}
          <div class="cycle-diary-entries">
            <strong>Saved observations</strong>
            {#if entries.length > visibleEntries.length}<small class="cycle-diary-entry-count">Showing the 14 most recent of {entries.length} saved observations.</small>{/if}
            {#each visibleEntries as entry (entry.id)}
              <article class="cycle-diary-entry">
                <div class="cycle-diary-entry-header">
                  <strong>{entry.values.entry_date}</strong>
                  <button type="button" class="btn btn-xs btn-link" onclick={() => removeEntry(entry.id)}>Remove</button>
                </div>
                <div class="cycle-diary-entry-values">
                  {#each populatedCycleDiaryFields(entry.values) as item (item.field.id)}
                    {#if item.field.id !== 'entry_date'}<span><b>{item.field.label}:</b> {item.value}</span>{/if}
                  {/each}
                </div>
              </article>
            {/each}
          </div>
        {/if}

        <ul class="cycle-diary-boundaries">
          {#each CYCLE_DIARY_SCHEMA.do_not_infer as boundary (boundary)}<li>{boundary}</li>{/each}
        </ul>
      </div>
    {/if}
  </section>
{/if}

<style>
  .cycle-diary {
    border: 1px solid var(--status-success-border);
    border-radius: 8px;
    background: var(--status-success-bg);
    overflow: hidden;
  }

  .cycle-diary-toggle {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    border: 0;
    background: transparent;
    color: var(--text-primary);
    padding: 0.7rem 0.8rem;
    text-align: left;
    cursor: pointer;
  }

  .cycle-diary-toggle span:first-child {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .cycle-diary-toggle small,
  .cycle-diary-field small,
  .cycle-diary-description,
  .cycle-diary-privacy {
    color: var(--text-secondary);
    font-size: 0.7rem;
    line-height: 1.4;
  }

  .cycle-diary-body {
    display: flex;
    flex-direction: column;
    gap: 0.7rem;
    padding: 0 0.8rem 0.8rem;
  }

  .cycle-diary-description,
  .cycle-diary-privacy {
    margin: 0;
  }

  .cycle-diary-privacy {
    color: var(--status-success-text);
  }

  .cycle-diary-form {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(13rem, 1fr));
    gap: 0.65rem;
  }

  .cycle-diary-field {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .cycle-diary-field label {
    color: var(--text-secondary);
    font-size: 0.72rem;
  }

  .cycle-diary-field input,
  .cycle-diary-field textarea {
    width: 100%;
    box-sizing: border-box;
    border: 1px solid var(--border-color);
    border-radius: 5px;
    background: var(--surface-control);
    color: var(--text-primary);
    font: inherit;
    font-size: 0.76rem;
    padding: 0.45rem;
  }

  .cycle-diary-field textarea {
    min-height: 3.6rem;
    resize: vertical;
  }

  .cycle-diary-actions {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    flex-wrap: wrap;
  }

  .cycle-diary-error {
    color: var(--status-danger-strong-text);
    font-size: 0.72rem;
  }

  .cycle-diary-entries {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .cycle-diary-entry {
    border: 1px solid var(--border-color);
    border-radius: 6px;
    padding: 0.55rem;
  }

  .cycle-diary-entry-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }

  .cycle-diary-entry-values {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem 0.75rem;
    color: var(--text-secondary);
    font-size: 0.7rem;
    line-height: 1.4;
    margin-top: 0.35rem;
  }

  .cycle-diary-entry-count {
    color: var(--text-secondary);
    font-size: 0.7rem;
  }

  .cycle-diary-boundaries {
    margin: 0;
    padding-left: 1.1rem;
    color: var(--text-secondary);
    font-size: 0.7rem;
    line-height: 1.4;
  }
</style>
