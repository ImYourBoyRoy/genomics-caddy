<!-- ./src/lib/components/ai/evidence/EvidenceSourcesSidebar.svelte -->
<script lang="ts">
  interface Props {
    sources: string[];
  }

  let { sources }: Props = $props();
</script>

<div class="sources-sidebar-column">
  <div class="sidebar-widget">
    <h4>Loaded Guideline Packs</h4>
    <p class="widget-desc">Citations and references currently loaded in your local database:</p>
    <div class="sources-list">
      {#each sources as src}
        <div class="source-item">
          <span class="source-icon">📄</span>
          <span class="source-name" title={src}>{src}</span>
        </div>
      {/each}
    </div>
  </div>
</div>

<style>
  .sources-sidebar-column {
    display: flex;
    flex-direction: column;
    width: 100%;
  }

  .sidebar-widget {
    background: rgba(255, 255, 255, 0.015);
    border: 1px solid var(--border-color);
    padding: 16px;
    border-radius: 10px;
  }

  .sidebar-widget h4 {
    font-size: 0.85rem;
    font-weight: 600;
    margin: 0 0 8px 0;
    color: var(--text-primary);
  }

  .widget-desc {
    font-size: 0.74rem;
    color: var(--text-secondary);
    margin: 0 0 12px 0;
    line-height: 1.4;
  }

  .sources-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 400px;
    overflow-y: auto;
  }

  .source-item {
    display: flex;
    align-items: center;
    gap: 8px;
    background: rgba(10, 11, 20, 0.3);
    border: 1px solid rgba(255, 255, 255, 0.03);
    padding: 8px 10px;
    border-radius: 6px;
    font-size: 0.74rem;
  }

  .source-item .source-icon {
    font-size: 0.8rem;
    opacity: 0.7;
  }

  .source-item .source-name {
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
  }
</style>
