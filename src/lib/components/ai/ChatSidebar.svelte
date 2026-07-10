<!-- ./src/lib/components/ai/ChatSidebar.svelte -->
<script lang="ts">
  import type { ChatSession } from "../../utils/chatSession";
  import type { ChatMessage } from "../../types/agent";

  interface SidebarSession extends ChatSession {
    messages: ChatMessage[];
  }

  interface Props {
    filteredSessions: ChatSession[];
    currentSessionId: string | null;
    startNewSession: () => void;
    loadSession: (id: string) => void;
    deleteSession: (id: string) => void;
    saveSessionTitle: (session: ChatSession) => void;
    showHistorySidebar: boolean;
    activeView: "chat" | "evidence";
  }

  let {
    filteredSessions,
    currentSessionId = $bindable(null),
    startNewSession,
    loadSession,
    deleteSession,
    saveSessionTitle,
    showHistorySidebar,
    activeView = $bindable("chat")
  }: Props = $props();

  let editingSessionId = $state<string | null>(null);
  let editingSessionTitle = $state("");

  function startEditingSession(session: ChatSession) {
    editingSessionId = session.id;
    editingSessionTitle = session.title;
  }

  function handleSave(session: ChatSession) {
    if (editingSessionTitle.trim()) {
      session.title = editingSessionTitle.trim();
      saveSessionTitle(session);
    }
    editingSessionId = null;
  }

  function selectAllAndFocus(el: HTMLInputElement) {
    el.focus();
    el.select();
  }
</script>

<aside class="history-sidebar" class:collapsed={!showHistorySidebar}>
  <div class="sidebar-inner">
    <div class="sidebar-tabs">
      <button class="sidebar-tab-btn" class:active={activeView === "chat"} onclick={() => activeView = "chat"}>
        💬 Consults
      </button>
      <button class="sidebar-tab-btn" class:active={activeView === "evidence"} onclick={() => activeView = "evidence"}>
        📚 Evidence
      </button>
    </div>

    {#if activeView === "chat"}
      <div class="sidebar-header">
        <button class="btn btn-accent btn-new-chat w-full" onclick={startNewSession}>
          ➕ New Consultation
        </button>
      </div>
      
      <div class="sessions-list scrollable">
        {#if filteredSessions.length === 0}
          <div class="empty-sessions">
            No previous consultations.
          </div>
        {:else}
          {#each filteredSessions as session}
            <div class="session-item-wrapper" class:active={currentSessionId === session.id}>
              {#if editingSessionId === session.id}
                <input
                  type="text"
                  class="session-title-input"
                  bind:value={editingSessionTitle}
                  onkeydown={(e) => {
                    if (e.key === "Enter") handleSave(session);
                    if (e.key === "Escape") editingSessionId = null;
                  }}
                  onblur={() => handleSave(session)}
                  use:selectAllAndFocus
                />
              {:else}
                <button 
                  class="session-item-btn" 
                  onclick={() => loadSession(session.id)}
                  ondblclick={() => startEditingSession(session)}
                >
                  <span class="session-icon">💬</span>
                  <span class="session-title" title={session.title}>{session.title}</span>
                </button>
                <div class="session-actions">
                  <button 
                    class="btn-session-action edit-btn" 
                    onclick={() => startEditingSession(session)}
                    title="Rename consultation"
                  >
                    ✏️
                  </button>
                  <button 
                    class="btn-session-action delete-btn" 
                    onclick={() => deleteSession(session.id)}
                    title="Delete consultation"
                  >
                    🗑
                  </button>
                </div>
              {/if}
            </div>
          {/each}
        {/if}
      </div>
    {:else}
      <div class="sidebar-header">
        <div style="font-size: 0.8rem; font-weight: 600; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.05em; padding: 4px 0;">
          Guideline Index
        </div>
      </div>
      <div class="sessions-list scrollable">
        <div style="font-size: 0.76rem; color: var(--text-secondary); line-height: 1.4; padding: 8px 4px; font-style: italic;">
          Use the Evidence panel to search the local guideline and literature references.
        </div>
      </div>
    {/if}
  </div>
</aside>

<style>
  .history-sidebar {
    flex: 0 0 250px;
    width: 250px;
    display: flex;
    flex-direction: column;
    background: rgba(10, 11, 20, 0.45);
    border-right: 1px solid var(--border-color);
    box-sizing: border-box;
    height: 100%;
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    overflow: hidden;
  }

  .history-sidebar.collapsed {
    width: 0;
    flex: 0 0 0px;
    border-right-color: transparent;
  }

  .sidebar-inner {
    width: 250px;
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .sidebar-header {
    padding: 16px;
    border-bottom: 1px solid var(--border-color);
    background: rgba(0, 0, 0, 0.1);
  }

  .btn-new-chat {
    font-size: 0.85rem;
    font-weight: 600;
  }

  .sessions-list {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .empty-sessions {
    font-size: 0.8rem;
    color: var(--text-secondary);
    text-align: center;
    padding: 20px 10px;
    font-style: italic;
  }

  .session-item-wrapper {
    display: flex;
    align-items: center;
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.01);
    border: 1px solid rgba(255, 255, 255, 0.03);
    transition: all 0.25s;
    position: relative;
    overflow: hidden;
  }

  .session-item-wrapper:hover {
    background: rgba(255, 255, 255, 0.04);
    border-color: rgba(255, 255, 255, 0.08);
  }

  .session-item-wrapper.active {
    background: rgba(88, 80, 236, 0.12);
    border-color: rgba(88, 80, 236, 0.35);
  }

  .session-item-btn {
    flex: 1;
    background: none;
    border: none;
    text-align: left;
    padding: 10px 12px;
    color: var(--text-secondary);
    font-size: 0.82rem;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 8px;
    overflow: hidden;
    min-width: 0;
  }

  .session-item-wrapper.active .session-item-btn {
    color: var(--text-primary);
  }

  .session-icon {
    font-size: 0.9rem;
    opacity: 0.7;
    flex-shrink: 0;
  }

  .session-title {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
  }

  .session-title-input {
    flex: 1;
    background: rgba(0, 0, 0, 0.4);
    border: 1px solid var(--accent);
    color: var(--text-primary);
    padding: 8px 10px;
    border-radius: 6px;
    font-size: 0.8rem;
    margin: 4px;
    outline: none;
  }

  .session-actions {
    display: flex;
    gap: 4px;
    padding-right: 8px;
    opacity: 0;
    transition: opacity 0.2s;
    background: linear-gradient(to left, rgba(10, 11, 20, 0.95) 75%, transparent);
    height: 100%;
    align-items: center;
    position: absolute;
    right: 0;
    top: 0;
  }

  .session-item-wrapper:hover .session-actions,
  .session-item-wrapper.active .session-actions {
    opacity: 1;
  }

  .btn-session-action {
    background: none;
    border: none;
    cursor: pointer;
    padding: 4px;
    font-size: 0.75rem;
    border-radius: 4px;
    transition: transform 0.15s, background 0.15s;
    color: var(--text-secondary);
  }

  .btn-session-action:hover {
    transform: scale(1.15);
    background: rgba(255, 255, 255, 0.1);
  }

  .delete-btn:hover {
    color: var(--danger);
    background: rgba(239, 68, 68, 0.15);
  }

  .w-full {
    width: 100%;
  }

  .scrollable {
    overflow-y: auto;
  }

  .sidebar-tabs {
    display: flex;
    padding: 12px 12px 0 12px;
    gap: 6px;
    background: rgba(0, 0, 0, 0.1);
    border-bottom: 1px solid var(--border-color);
  }

  .sidebar-tab-btn {
    flex: 1;
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid var(--border-color);
    border-bottom: none;
    color: var(--text-secondary);
    padding: 8px;
    border-radius: 6px 6px 0 0;
    font-size: 0.76rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }

  .sidebar-tab-btn:hover {
    background: rgba(255, 255, 255, 0.05);
    color: var(--text-primary);
  }

  .sidebar-tab-btn.active {
    background: rgba(88, 80, 236, 0.15);
    border-color: var(--accent);
    color: var(--text-primary);
    box-shadow: inset 0 -2px 0 var(--accent);
  }
</style>
