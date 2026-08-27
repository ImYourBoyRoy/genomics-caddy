<!-- ./src/lib/components/ai/ChatMessages.svelte -->
<script lang="ts">
  import { parseThinking, formatMarkdownSafe } from '../../utils/chatParser';

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

<style>
  .messages-list { display: flex; flex-direction: column; gap: 20px; }
  .message { display: flex; flex-direction: column; max-width: 85%; gap: 4px; animation: fadeIn 0.25s ease-out; }
  @keyframes fadeIn { from { opacity: 0; transform: translateY(8px); } to { opacity: 1; transform: translateY(0); } }
  .message.user { align-self: flex-end; }
  .message.assistant { align-self: flex-start; }
  .message-meta-row { display: flex; justify-content: space-between; align-items: center; width: 100%; margin-bottom: 2px; gap: 20px; }
  .message-meta { font-size: 0.72rem; color: var(--text-secondary); font-weight: 600; }
  .message.user .message-meta-row { justify-content: flex-end; }
  .message-actions-row { display: flex; gap: 6px; align-items: center; }
  .btn-message-action-icon { background: none; border: none; cursor: pointer; font-size: 0.72rem; padding: 2px 4px; min-width: 44px; min-height: 44px; border-radius: 4px; transition: all 0.2s; opacity: 0.45; }
  .btn-message-action-icon:hover { opacity: 1; background: rgba(255, 255, 255, 0.08); }
  .btn-message-action-icon:focus-visible, .btn-copy-msg:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 2px; opacity: 1; }
  .btn-message-action-icon.delete-msg-btn:hover { color: var(--danger); background: rgba(239, 68, 68, 0.15); }
  .btn-copy-msg { background: none; border: none; color: var(--text-secondary); font-size: 0.7rem; cursor: pointer; padding: 2px 6px; border-radius: 4px; transition: all 0.2s; background: rgba(255, 255, 255, 0.03); border: 1px solid rgba(255, 255, 255, 0.05); }
  .btn-copy-msg:hover { color: var(--accent); background: rgba(88, 80, 236, 0.1); border-color: rgba(88, 80, 236, 0.2); }
  .message-body { padding: 12px 16px; border-radius: 12px; font-size: 0.9rem; line-height: 1.5; box-sizing: border-box; }
  .message.user .message-body { background: rgba(88, 80, 236, 0.22); border: 1px solid rgba(88, 80, 236, 0.35); color: var(--text-primary); border-top-right-radius: 2px; }
  .message.assistant .message-body { background: rgba(255, 255, 255, 0.03); border: 1px solid var(--border-color); color: #e5e7eb; border-top-left-radius: 2px; }
  .typing-indicator { font-size: 0.78rem; color: var(--accent); font-style: italic; display: block; margin-top: 6px; }
  .thought-details-block { margin-bottom: 12px; border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 8px; background: rgba(255, 255, 255, 0.015); overflow: hidden; transition: border-color 0.2s; max-width: 85%; align-self: flex-start; margin-left: 20px; }
  .thought-details-block[open] { border-color: rgba(88, 80, 236, 0.3); background: rgba(88, 80, 236, 0.02); }
  .thought-summary { padding: 8px 12px; font-size: 0.76rem; color: #9ca3af; cursor: pointer; user-select: none; list-style: none; display: flex; align-items: center; font-weight: 500; }
  .thought-summary::-webkit-details-marker { display: none; }
  .thought-body { padding: 10px 12px; font-size: 0.78rem; line-height: 1.4; color: #818cf8; border-top: 1px dashed rgba(255, 255, 255, 0.06); white-space: pre-wrap; max-height: 300px; overflow-y: auto; }
  .message-system-notification { display: flex; align-items: center; gap: 8px; padding: 8px 12px; background: rgba(255, 255, 255, 0.02); border: 1px solid rgba(255, 255, 255, 0.05); border-radius: 6px; font-size: 0.76rem; color: var(--text-secondary); align-self: center; max-width: 90%; }
  .system-icon { font-size: 0.85rem; }
  .message-images-grid { display: flex; gap: 8px; flex-wrap: wrap; }
  .message-thumbnail { max-width: 160px; max-height: 120px; border-radius: 6px; border: 1px solid rgba(255, 255, 255, 0.1); object-fit: contain; cursor: pointer; background: rgba(0, 0, 0, 0.2); transition: transform 0.2s, border-color 0.2s; }
  .message-thumbnail:hover { transform: scale(1.02); border-color: var(--accent); }
  .mt-2 { margin-top: 0.5rem; }
  :global(.message-body p) { margin: 0 0 10px 0; }
  :global(.message-body p:last-child) { margin-bottom: 0; }
  :global(.message-body strong) { color: #ffffff; font-weight: 600; }
  :global(.message-body .md-h1, .message-body .md-h2, .message-body .md-h3) { color: #ffffff; font-weight: 700; margin: 14px 0 6px 0; }
  :global(.message-body .md-h1) { font-size: 1.15rem; }
  :global(.message-body .md-h2) { font-size: 1.05rem; }
  :global(.message-body .md-h3) { font-size: 0.95rem; }
  :global(.message-body .md-quote) { border-left: 3px solid var(--accent); background: rgba(88, 80, 236, 0.05); margin: 8px 0; padding: 6px 12px; font-style: italic; color: #9ca3af; border-radius: 2px; }
  :global(.message-body .md-list) { margin: 8px 0; padding-left: 20px; list-style-type: disc; }
  :global(.message-body .md-list li) { margin-bottom: 4px; }
  :global(.message-body .code-inline) { background: rgba(255, 255, 255, 0.08); color: #c7d2fe; padding: 2px 4px; border-radius: 4px; font-family: monospace; font-size: 0.8rem; }
  :global(.message-body .code-block) { background: rgba(0, 0, 0, 0.4); border: 1px solid var(--border-color); padding: 12px; border-radius: 6px; overflow-x: auto; margin: 10px 0; }
  :global(.message-body .code-block code) { font-family: monospace; font-size: 0.8rem; color: #e5e7eb; display: block; line-height: 1.4; }
  .safety-review-warning-box { margin-top: 12px; border: 1px solid rgba(245, 158, 11, 0.35); border-radius: 8px; background: rgba(245, 158, 11, 0.04); overflow: hidden; max-width: 85%; align-self: flex-start; margin-left: 20px; animation: fadeIn 0.25s ease-out; }
  .safety-review-header { padding: 6px 12px; font-size: 0.72rem; font-weight: 600; color: #f59e0b; background: rgba(245, 158, 11, 0.08); border-bottom: 1px solid rgba(245, 158, 11, 0.15); display: flex; align-items: center; gap: 6px; }
  .safety-review-body { padding: 10px 12px; font-size: 0.78rem; line-height: 1.4; color: #fbbf24; }
  :global(.safety-review-body p) { margin: 0 0 6px 0; }
  :global(.safety-review-body p:last-child) { margin-bottom: 0; }
  .safety-thought-details-block { margin: 8px 12px; border: 1px solid rgba(245, 158, 11, 0.2); border-radius: 6px; background: rgba(0, 0, 0, 0.15); overflow: hidden; }
  .safety-thought-details-block[open] { border-color: rgba(245, 158, 11, 0.35); }
  .safety-thought-summary { padding: 6px 10px; font-size: 0.72rem; color: #fbbf24; cursor: pointer; user-select: none; list-style: none; display: flex; align-items: center; font-weight: 500; background: rgba(245, 158, 11, 0.04); }
  .safety-thought-summary::-webkit-details-marker { display: none; }
  .safety-thought-body { padding: 8px 10px; font-size: 0.74rem; line-height: 1.35; color: #fbbf24; opacity: 0.8; border-top: 1px dashed rgba(245, 158, 11, 0.15); white-space: pre-wrap; max-height: 200px; overflow-y: auto; }
  .model-loading-indicator { display: flex; align-items: center; gap: 8px; padding: 10px 14px; background: rgba(255, 255, 255, 0.02); border: 1px dashed rgba(255, 255, 255, 0.1); border-radius: 8px; margin-left: 20px; align-self: flex-start; animation: fadeIn 0.25s ease-out; margin-bottom: 12px; }
  .loading-pulse-dot { width: 6px; height: 6px; border-radius: 50%; background: var(--accent); box-shadow: 0 0 6px var(--accent); animation: loading-pulse 1.5s infinite ease-in-out; }
  @keyframes loading-pulse { 0% { transform: scale(0.9); opacity: 0.6; } 50% { transform: scale(1.15); opacity: 1; box-shadow: 0 0 10px var(--accent); } 100% { transform: scale(0.9); opacity: 0.6; } }
  .loading-text { font-size: 0.72rem; color: var(--text-secondary); }
</style>
