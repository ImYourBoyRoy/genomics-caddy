<!-- ./src/lib/components/ai/settings/ChatGptConnectionCard.svelte -->
<script lang="ts">
  import { onMount } from "svelte";
  import {
    chatgptCancelSignIn,
    chatgptOpenUsage,
    chatgptSignIn,
    chatgptSignOut,
    chatgptStatus,
    type ChatGptStatus,
  } from "../../../api/chatgpt";
  import { CHATGPT_RAW_DATA_DISCLOSURE } from "../../../utils/chatProvider";
  import "#lib/styles/components/chatgpt-connection.css";

  interface Props {
    /** Called after a successful sign-in so the parent can refresh the model list. */
    onConnected?: () => void;
    onDisconnected?: () => void;
  }

  let { onConnected, onDisconnected }: Props = $props();

  let status = $state<ChatGptStatus | null>(null);
  let waiting = $state(false);
  let error = $state("");

  let connected = $derived(status?.connected === true);
  let sharing = $derived(status?.sharing === true);
  let account = $derived(status?.email || status?.name || "ChatGPT account");

  onMount(() => {
    void refresh();
  });

  async function refresh() {
    try {
      status = await chatgptStatus();
    } catch (err: unknown) {
      error = err instanceof Error ? err.message : String(err);
    }
  }

  async function signIn() {
    error = "";
    waiting = true;
    try {
      status = await chatgptSignIn();
      if (status.sharing) onConnected?.();
    } catch (err: unknown) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      waiting = false;
    }
  }

  async function cancel() {
    await chatgptCancelSignIn();
  }

  async function disconnect() {
    error = "";
    try {
      status = await chatgptSignOut();
      onDisconnected?.();
    } catch (err: unknown) {
      error = err instanceof Error ? err.message : String(err);
    }
  }
</script>

<div class="gpt-card" data-state={waiting ? "waiting" : connected ? (sharing ? "connected" : "limited") : "signed-out"}>
  {#if waiting}
    <div class="gpt-head">
      <span class="gpt-pulse" aria-hidden="true"></span>
      <div class="gpt-copy">
        <strong>Finish in your browser</strong>
        <span>Choose your ChatGPT account and approve plan usage. This window waits up to five minutes.</span>
      </div>
    </div>
    <button type="button" class="btn btn-secondary w-full" onclick={cancel}>Cancel sign-in</button>
  {:else if connected}
    <div class="gpt-head">
      <span class="gpt-dot" aria-hidden="true"></span>
      <div class="gpt-copy">
        <strong>{account}</strong>
        <span>{sharing ? "Using your ChatGPT plan" : "Signed in, but plan usage is not enabled"}</span>
      </div>
    </div>
    <div class="gpt-actions">
      {#if sharing}
        <button type="button" class="btn btn-secondary" onclick={() => void chatgptOpenUsage()}>Manage usage</button>
      {:else}
        <button type="button" class="btn btn-primary" onclick={signIn}>Enable plan usage</button>
      {/if}
      <button type="button" class="btn btn-secondary" onclick={disconnect}>Disconnect</button>
    </div>
  {:else}
    <div class="gpt-copy">
      <strong>Use your ChatGPT plan</strong>
      <span>Answer consultations with a higher-capability cloud model. No API key; your ChatGPT conversations are never accessed.</span>
    </div>
    <button type="button" class="btn btn-primary w-full" onclick={signIn}>Continue with ChatGPT</button>
  {/if}

  <p class="gpt-disclosure" role="note">🔒 {CHATGPT_RAW_DATA_DISCLOSURE}</p>

  {#if error}
    <div class="gpt-error" role="alert">{error}</div>
  {/if}
</div>
