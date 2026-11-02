<!-- ./src/lib/components/ai/ChatMessages.svelte -->
<script lang="ts">
  import { parseThinking, formatMarkdownSafe } from '../../utils/chatParser';
  import '$lib/styles/components/chat-messages.css';

  interface Props {
    messages: { role: 'user' | 'assistant' | 'system'; content: string; fullContent?: string; images?: string[]; safetyReview?: string }[];
    isChatting: boolean;
    copiedMsgId: number | null;
    showThinkingProcess: boolean;
    autoCollapseThinking: boolean;
    userCollapsedThinkingMap: Map<any, boolean>;
    copyToClipboard: (text: string, index: number) => void;
    editMessage: (index: number) => void;
    deleteMessage: (index: number) => void;
    selectedModel: string;
  }

  let {
    messages,
    isChatting,
    copiedMsgId,
    showThinkingProcess,
    autoCollapseThinking,
    userCollapsedThinkingMap,
    copyToClipboard,
    editMessage,
    deleteMessage,
    selectedModel,
  }: Props = $props();
</script>

<div class="messages-list">
  {#each messages as msg, idx}
    {#if msg.role === "system"}
      <div class="message-system-notification">
        <span class="system-icon">⚙️</span>
        <span class="system-text">{msg.content}</span>
      </div>
    {:else}
      <div class="message {msg.role}">
        <div class="message-meta-row">
          <span class="message-meta">
            {msg.role === "user" ? "You" : "Genomics Caddy AI"}
          </span>
          <div class="message-actions-row">
            {#if msg.role === "user" && !isChatting}
              <button
                type="button"
                class="btn-message-action-icon"
                onclick={() => editMessage(idx)}
                aria-label="Edit question and branch chat"
              >
                ✏️
              </button>
            {/if}
            {#if !isChatting}
              <button
                type="button"
                class="btn-message-action-icon delete-msg-btn"
                onclick={() => deleteMessage(idx)}
                aria-label="Delete message"
              >
                🗑
              </button>
            {/if}
            {#if msg.role === "assistant" && msg.content}
              <button
                type="button"
                class="btn-copy-msg"
                onclick={() => copyToClipboard(msg.content, idx)}
                aria-label={copiedMsgId === idx ? "Copied response text" : "Copy response text"}
              >
                {copiedMsgId === idx ? "✓ Copied!" : "📋 Copy"}
              </button>
            {/if}
          </div>
        </div>

        {#if msg.role === "user"}
          <div class="message-body">
            <p>{msg.content}</p>
            {#if msg.images && msg.images.length > 0}
              <div class="message-images-grid mt-2">
                {#each msg.images as base64Img}
                  <img class="message-thumbnail" src="data:image/jpeg;base64,{base64Img}" alt="User attachment" />
                {/each}
              </div>
            {/if}
          </div>
        {:else}
          <!-- Assistant Message: parse out thought process -->
          {@const parsed = parseThinking(msg.content)}

          {#if !msg.content && isChatting && idx === messages.length - 1}
            <div class="model-loading-indicator">
              <span class="loading-pulse-dot"></span>
              <span class="loading-text font-mono">Loading {selectedModel} into VRAM...</span>
            </div>
          {/if}

          {#if parsed.thought}
            {@const isCollapsed = userCollapsedThinkingMap.get(msg) ?? (autoCollapseThinking && parsed.thoughtCompleted)}
            {#if showThinkingProcess}
              <details
                class="thought-details-block"
                open={!isCollapsed}
                ontoggle={(e) => {
                  userCollapsedThinkingMap.set(msg, !e.currentTarget.open);
                }}
              >
                <summary class="thought-summary">
                  <span>🧠 {parsed.thoughtCompleted ? "Reasoning stream" : "Reasoning stream (Thinking...)"}</span>
                </summary>
                <div class="thought-body font-mono">
                  {parsed.thought}
                </div>
              </details>
            {/if}
          {/if}

          {#if parsed.response || (!parsed.thought && !isChatting)}
            <div class="message-body">
              {@html formatMarkdownSafe(parsed.response)}
              {#if !parsed.response && isChatting}
                <span class="typing-indicator">Streaming response...</span>
              {/if}
            </div>
          {/if}

          {#if msg.safetyReview}
            {@const parsedSafety = parseThinking(msg.safetyReview)}
            <div class="safety-review-warning-box">
              <div class="safety-review-header">
                <span>🛡️ Secondary Safety Review Panel</span>
              </div>
              {#if parsedSafety.thought}
                <details class="safety-thought-details-block" open={true}>
                  <summary class="safety-thought-summary">
                    <span>🧠 Safety Auditor Reasoning</span>
                  </summary>
                  <div class="safety-thought-body font-mono">{parsedSafety.thought}</div>
                </details>
              {/if}
              {#if parsedSafety.response}
                <div class="safety-review-body font-mono">
                  {@html formatMarkdownSafe(parsedSafety.response)}
                </div>
              {/if}
            </div>
          {/if}
        {/if}
      </div>
    {/if}
  {/each}
</div>
