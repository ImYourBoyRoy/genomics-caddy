<!-- ./src/lib/components/ai/WelcomeChat.svelte -->
<script lang="ts">
  import '#lib/styles/components/chat-welcome.css';
  import type { OllamaEndpointScope } from '../../utils/ollamaSettings';

  interface Props {
    questions: { label: string; text: string }[];
    scope: OllamaEndpointScope;
    host: string;
    sendPrompt: (customPrompt?: string) => void;
    onOpenConnection: () => void;
  }

  let { questions, scope, host, sendPrompt, onOpenConnection }: Props = $props();

  const LEADING_GLYPH = /^(\p{Extended_Pictographic}\uFE0F?)\s*(.*)$/u;

  function splitLabel(label: string): { glyph: string; title: string } {
    const match = LEADING_GLYPH.exec(label);
    return match ? { glyph: match[1], title: match[2] } : { glyph: '✦', title: label };
  }

  let privacyLine = $derived(
    scope === 'local'
      ? 'Runs on a model on this device. Your findings and raw calls stay here.'
      : scope === 'remote'
        ? `Connected to ${host || 'a remote host'}. Selected findings, including raw genotype calls, are sent there and leave this device.`
        : 'Connect a local or private model to begin. Nothing is sent anywhere until you do.'
  );
</script>

<div class="welcome-chat">
  <div class="welcome-mark" aria-hidden="true">
    <svg viewBox="0 0 48 48" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
      <path d="M16 6c0 8 16 10 16 18S16 34 16 42" />
      <path d="M32 6c0 8-16 10-16 18s16 10 16 18" />
      <path d="M19 14h10M17 24h14M19 34h10" opacity="0.55" />
    </svg>
  </div>

  <h3>Ask your DNA</h3>
  <p class="welcome-lede">
    Turn your report into a conversation: what a result may mean, which tests to raise,
    and where diet, training, supplements or medicines are worth a closer look.
  </p>

  <p class="welcome-privacy" data-scope={scope}>
    <span class="welcome-privacy-dot" aria-hidden="true"></span>
    <span>
      {privacyLine}
      {#if scope === 'unconfigured'}
        <button type="button" class="welcome-link" onclick={onOpenConnection}>Set up connection</button>
      {/if}
    </span>
  </p>

  {#if questions.length > 0}
    <div class="welcome-prompt-grid" role="group" aria-label="Suggested questions">
      {#each questions as q, i (q.label)}
        {@const parts = splitLabel(q.label)}
        <button type="button" class="welcome-prompt-card" style:--i={Math.min(i, 8)} onclick={() => sendPrompt(q.text)}>
          <span class="welcome-prompt-glyph" aria-hidden="true">{parts.glyph}</span>
          <span class="welcome-prompt-title">{parts.title}</span>
          <span class="welcome-prompt-arrow" aria-hidden="true">→</span>
        </button>
      {/each}
    </div>
  {/if}

  <p class="welcome-hint">Or type your own question below.</p>
</div>
