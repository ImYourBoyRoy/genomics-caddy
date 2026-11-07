<!-- ./src/lib/components/ai/ChatSidebar.svelte -->
<script lang="ts">
  import type { ChatSession } from "../../utils/chatSession";
  import type { ChatMessage } from "../../types/agent";
  import "#lib/styles/components/chat-sidebar.css";

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

  const dateFormat = new Intl.DateTimeFormat(undefined, { month: 'short', day: 'numeric', hour: 'numeric', minute: '2-digit' });
  const formatWhen = (ts: number) => (Number.isFinite(ts) && ts > 0 ? dateFormat.format(new Date(ts)) : '');

  function selectAllAndFocus(el: HTMLInputElement) {
    el.focus();
    el.select();
  }
</script>

<aside class="history-sidebar" class:collapsed={!showHistorySidebar}>
  <div class="sidebar-inner">
    <div class="sidebar-tabs" role="group" aria-label="Assistant workspace">
      <button type="button" class="sidebar-tab-btn" class:active={activeView === "chat"} aria-pressed={activeView === "chat"} onclick={() => activeView = "chat"}>
        Consults
      </button>
      <button type="button" class="sidebar-tab-btn" class:active={activeView === "evidence"} aria-pressed={activeView === "evidence"} onclick={() => activeView = "evidence"}>
        Evidence
      </button>
    </div>

    {#if activeView === "chat"}
      <div class="sidebar-header">
        <button type="button" class="btn btn-new-chat" onclick={startNewSession}>
          <span aria-hidden="true">＋</span> New consultation
        </button>
      </div>

      <div class="sessions-list scrollable">
        {#if filteredSessions.length === 0}
          <div class="empty-sessions">
            No previous consultations.
          </div>
        {:else}
          {#each filteredSessions as session (session.id)}
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
                  type="button"
                  class="session-item-btn"
                  onclick={() => loadSession(session.id)}
                  ondblclick={() => startEditingSession(session)}
                >
                  <span class="session-title">{session.title}</span>
                  <span class="session-when">{formatWhen(session.timestamp)}</span>
                </button>
                <div class="session-actions">
                  <button
                    type="button" class="btn-session-action edit-btn"
                    onclick={() => startEditingSession(session)}
                    aria-label="Rename consultation"
                  >
                    ✏️
                  </button>
                  <button
                    type="button" class="btn-session-action delete-btn"
                    onclick={() => deleteSession(session.id)}
                    aria-label="Delete consultation"
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
        <div class="sidebar-kicker">
          Guideline Index
        </div>
      </div>
      <div class="sessions-list scrollable">
        <div class="empty-sessions">
          Search and browse local guideline and literature references in the panel.
        </div>
      </div>
    {/if}
  </div>
</aside>
