<!-- AI settings entry point for the canonical profile context. -->
<script lang="ts">
  import type { UserBiohackingProfile } from '../../utils/aiPrompt';
  import { profileContextHasContent, type ProfileContext } from '../../utils/profileContext';

  interface Props {
    userProfile: UserBiohackingProfile;
    profileContext: ProfileContext;
    onOpenContext?: () => void;
  }

  let {
    userProfile = $bindable(),
    profileContext,
    onOpenContext,
  }: Props = $props();

  let hasContext = $derived(profileContextHasContent(profileContext));
  let contextItems = $derived([
    profileContext.notes.goals ? 'goals' : '',
    profileContext.notes.challenges ? 'symptoms' : '',
    profileContext.safety.medications.length > 0 ? 'medications' : '',
    profileContext.safety.supplements.length > 0 ? 'supplements' : '',
    profileContext.safety.labObservations.length > 0 ? 'labs' : '',
    profileContext.safety.cycleDiary?.length ? 'diary' : '',
  ].filter(Boolean));
</script>

<div class="ai-profile-context">
  <label class="extended-thinking-toggle">
    <input type="checkbox" bind:checked={userProfile.injectProfile} />
    <span class="highlight-text font-bold">Include profile Context in Connected Chat</span>
  </label>

  <div class="ai-profile-context-card">
    <div>
      <strong>{hasContext ? 'Profile Context is available' : 'No profile Context added'}</strong>
      <span>{hasContext ? `${contextItems.length} context area${contextItems.length === 1 ? '' : 's'} saved for the active profile.` : 'Add optional goals, symptoms, medications, labs, diet, or hormone details in Context.'}</span>
    </div>
    {#if onOpenContext}
      <button type="button" class="btn btn-secondary btn-sm" onclick={() => onOpenContext?.()}>Open Context</button>
    {/if}
  </div>

  <small class="ai-profile-context-hint">Model settings stay here; profile facts are maintained once in the profile-scoped Context tab.</small>
</div>

<style>
  .ai-profile-context { display: flex; flex-direction: column; gap: 0.75rem; }
  .ai-profile-context-card { display: flex; align-items: center; justify-content: space-between; gap: 0.75rem; border: 1px solid var(--border-color); border-radius: 0.5rem; background: var(--surface-subtle); padding: 0.75rem; }
  .ai-profile-context-card > div { display: flex; flex-direction: column; gap: 0.25rem; min-width: 0; }
  .ai-profile-context-card strong { color: var(--text-primary); font-size: 0.8rem; }
  .ai-profile-context-card span, .ai-profile-context-hint { color: var(--text-secondary); font-size: 0.7rem; line-height: 1.4; }
  .ai-profile-context-card .btn { flex: 0 0 auto; }
</style>
