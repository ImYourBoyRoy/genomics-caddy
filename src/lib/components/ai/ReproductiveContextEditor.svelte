<!-- Resource-authored, self-reported cycle and hormone context editor. -->
<script lang="ts">
  import { REPRODUCTIVE_INTAKE_SCHEMA, hasReproductiveIntake, normalizeReproductiveIntake, reproductiveIntakeValue } from '../../utils/reproductiveIntake';
  import {
    savePersonalSafetyContext,
    type PersonalSafetyContext,
  } from '../../utils/personalSafetyContext';

  interface Props {
    personalSafetyContext: PersonalSafetyContext;
    sampleId?: number;
    compact?: boolean;
  }

  let {
    personalSafetyContext = $bindable(),
    sampleId,
    compact = false,
  }: Props = $props();

  let expanded = $state(false);
  let hasValues = $derived(hasReproductiveIntake(personalSafetyContext.reproductiveIntake));
  let idPrefix = $derived(`reproductive-intake-${sampleId ?? 'profile'}`);

  function updateField(fieldId: string, event: Event): void {
    const target = event.currentTarget as HTMLInputElement | HTMLTextAreaElement;
    const nextValues = normalizeReproductiveIntake({
      ...(personalSafetyContext.reproductiveIntake || {}),
      [fieldId]: target.value,
    });
    personalSafetyContext = {
      ...personalSafetyContext,
      ...(hasReproductiveIntake(nextValues)
        ? { reproductiveIntake: nextValues }
        : { reproductiveIntake: undefined }),
    };
    savePersonalSafetyContext(sampleId, personalSafetyContext);
  }
</script>

<section class="reproductive-intake" class:reproductive-intake-compact={compact} aria-labelledby="{idPrefix}-title">
  <button
    type="button"
    class="reproductive-intake-toggle"
    aria-expanded={expanded ? 'true' : 'false'}
    onclick={() => expanded = !expanded}
  >
    <span>
      <strong id="{idPrefix}-title">{REPRODUCTIVE_INTAKE_SCHEMA.title}</strong>
      {#if hasValues}<small>Details recorded for this DNA profile</small>{:else}<small>Optional — leave blank when not relevant</small>{/if}
    </span>
    <span aria-hidden="true">{expanded ? '▾' : '▸'}</span>
  </button>

  {#if expanded}
    <div class="reproductive-intake-body">
      <p class="reproductive-intake-description">{REPRODUCTIVE_INTAKE_SCHEMA.description}</p>
      <p class="reproductive-intake-privacy">🔒 {REPRODUCTIVE_INTAKE_SCHEMA.privacy_note}</p>

      {#each REPRODUCTIVE_INTAKE_SCHEMA.groups as group (group.id)}
        <fieldset class="reproductive-intake-group">
          <legend>{group.title}</legend>
          <p>{group.description}</p>
          {#each group.field_ids as fieldId (fieldId)}
            {@const field = REPRODUCTIVE_INTAKE_SCHEMA.fields.find((candidate) => candidate.id === fieldId)}
            {#if field}
              <div class="reproductive-intake-field">
                <label for="{idPrefix}-{field.id}">{field.label}</label>
                {#if field.input_type === 'textarea'}
                  <textarea
                    id="{idPrefix}-{field.id}"
                    value={reproductiveIntakeValue(personalSafetyContext.reproductiveIntake, field.id)}
                    placeholder={field.placeholder}
                    oninput={(event) => updateField(field.id, event)}
                  ></textarea>
                {:else}
                  <input
                    id="{idPrefix}-{field.id}"
                    type={field.input_type}
                    value={reproductiveIntakeValue(personalSafetyContext.reproductiveIntake, field.id)}
                    placeholder={field.placeholder}
                    oninput={(event) => updateField(field.id, event)}
                  />
                {/if}
                <small>{field.help}</small>
              </div>
            {/if}
          {/each}
        </fieldset>
      {/each}

      <ul class="reproductive-intake-boundaries">
        {#each REPRODUCTIVE_INTAKE_SCHEMA.do_not_infer as boundary (boundary)}<li>{boundary}</li>{/each}
      </ul>
    </div>
  {/if}
</section>

<style>
  .reproductive-intake {
    border: 1px solid rgba(96, 165, 250, 0.28);
    border-radius: 8px;
    background: rgba(59, 130, 246, 0.06);
    overflow: hidden;
  }

  .reproductive-intake-toggle {
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

  .reproductive-intake-toggle span:first-child {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .reproductive-intake-toggle small,
  .reproductive-intake-field small,
  .reproductive-intake-group p,
  .reproductive-intake-description,
  .reproductive-intake-privacy {
    color: var(--text-secondary);
    font-size: 0.7rem;
    line-height: 1.4;
  }

  .reproductive-intake-body {
    display: flex;
    flex-direction: column;
    gap: 0.7rem;
    padding: 0 0.8rem 0.8rem;
  }

  .reproductive-intake-description,
  .reproductive-intake-privacy {
    margin: 0;
  }

  .reproductive-intake-privacy {
    color: #93c5fd;
  }

  .reproductive-intake-group {
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
    min-width: 0;
    border: 1px solid var(--border-color);
    border-radius: 6px;
    padding: 0.65rem;
  }

  .reproductive-intake-group legend {
    color: var(--text-primary);
    font-size: 0.78rem;
    font-weight: 600;
    padding: 0 0.25rem;
  }

  .reproductive-intake-group p {
    margin: -0.15rem 0 0;
  }

  .reproductive-intake-field {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .reproductive-intake-field label {
    color: var(--text-secondary);
    font-size: 0.72rem;
  }

  .reproductive-intake-field input,
  .reproductive-intake-field textarea {
    width: 100%;
    box-sizing: border-box;
    border: 1px solid var(--border-color);
    border-radius: 5px;
    background: rgba(0, 0, 0, 0.22);
    color: var(--text-primary);
    font: inherit;
    font-size: 0.76rem;
    padding: 0.45rem;
  }

  .reproductive-intake-field textarea {
    min-height: 3.6rem;
    resize: vertical;
  }

  .reproductive-intake-field input:focus,
  .reproductive-intake-field textarea:focus {
    outline: none;
    border-color: var(--accent);
  }

  .reproductive-intake-boundaries {
    margin: 0;
    padding-left: 1.1rem;
    color: var(--text-secondary);
    font-size: 0.7rem;
    line-height: 1.4;
  }
</style>
