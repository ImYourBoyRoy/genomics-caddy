<!-- ./src/lib/components/mcp/McpToolCatalog.svelte -->
<script lang="ts">
  /*
  Module Docstring:
  Purpose: Expandable catalog of active MCP tools and their parameter schemas.
  Responsibilities:
  - Render a header with the tool count badge.
  - Display an expandable accordion of all registered MCP tools.
  - Show parameter tables (name, type, required, description) for each tool.
  Key Inputs: mcpTools array from the Rust backend.
  Key Outputs: Visual tool catalog with expand/collapse per tool.
  Operational Notes: expandedTools state is managed locally. Uses fadeIn animation.
  */

  interface Props {
    mcpTools: any[];
  }

  let { mcpTools }: Props = $props();

  // Local expand/collapse state
  let expandedTools = $state<Record<string, boolean>>({});

  function toggleTool(name: string) {
    expandedTools[name] = !expandedTools[name];
  }
</script>

<section class="mcp-tools-section">
  <div class="tools-header-area">
    <h4>📋 Active Tool Catalog</h4>
    <span class="tools-count font-mono">{mcpTools.length} Tools Available</span>
  </div>

  <p class="section-hint">These functions are dynamically exposed by the local Rust binary in MCP mode:</p>

  <div class="tools-list">
    {#each mcpTools as tool}
      <div class="tool-item" class:expanded={expandedTools[tool.name]}>
        <button class="tool-summary-btn" onclick={() => toggleTool(tool.name)}>
          <span class="tool-arrow">▶</span>
          <span class="tool-name font-mono">{tool.name}</span>
        </button>

        {#if expandedTools[tool.name]}
          <div class="tool-details">
            <p class="tool-desc">{tool.description}</p>

            {#if tool.params && tool.params.length > 0}
              <div class="params-table-container">
                <h5>Arguments:</h5>
                <table class="params-table">
                  <thead>
                    <tr>
                      <th>Name</th>
                      <th>Type</th>
                      <th>Required</th>
                      <th>Description</th>
                    </tr>
                  </thead>
                  <tbody>
                    {#each tool.params as param}
                      <tr>
                        <td class="font-mono">{param.name}</td>
                        <td class="font-mono text-secondary">{param.type}</td>
                        <td class="font-mono text-center">
                          <span class="req-badge" class:required={param.required}>
                            {param.required ? "Yes" : "No"}
                          </span>
                        </td>
                        <td>{param.description}</td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
            {:else}
              <p class="no-params font-italic">Accepts no arguments.</p>
            {/if}
          </div>
        {/if}
      </div>
    {/each}
  </div>
</section>

<style>
  .mcp-tools-section {
    background: rgba(255, 255, 255, 0.01);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 20px;
    box-sizing: border-box;
  }

  .tools-header-area {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--border-color);
    padding-bottom: 10px;
    margin-bottom: 10px;
  }

  .tools-header-area h4 {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 600;
  }

  .tools-count {
    font-size: 0.72rem;
    background: rgba(16, 185, 129, 0.1);
    color: #34d399;
    padding: 2px 8px;
    border-radius: 4px;
    border: 1px solid rgba(16, 185, 129, 0.2);
  }

  .section-hint {
    font-size: 0.78rem;
    color: var(--text-secondary);
    margin: 0 0 14px 0;
    line-height: 1.3;
  }

  .tools-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 480px;
    overflow-y: auto;
  }

  .tool-item {
    border: 1px solid var(--border-color);
    background: rgba(0, 0, 0, 0.15);
    border-radius: 6px;
    overflow: hidden;
    transition: all 0.2s;
  }

  .tool-item.expanded {
    border-color: var(--accent);
  }

  .tool-summary-btn {
    width: 100%;
    background: none;
    border: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    color: var(--text-primary);
    cursor: pointer;
    text-align: left;
  }

  .tool-arrow {
    font-size: 0.6rem;
    transition: transform 0.2s;
    opacity: 0.5;
  }

  .tool-item.expanded .tool-arrow {
    transform: rotate(90deg);
  }

  .tool-name {
    font-size: 0.8rem;
    font-weight: 600;
    color: #ffffff;
  }

  .tool-details {
    padding: 0 12px 12px 12px;
    border-top: 1px dashed var(--border-color);
    background: rgba(0, 0, 0, 0.1);
    animation: fadeIn 0.2s ease-out;
  }

  @keyframes fadeIn {
    from { opacity: 0; transform: translateY(-4px); }
    to { opacity: 1; transform: translateY(0); }
  }

  .tool-desc {
    font-size: 0.78rem;
    line-height: 1.4;
    color: var(--text-secondary);
    margin: 8px 0;
  }

  .no-params {
    font-size: 0.72rem;
    color: var(--text-secondary);
    margin: 0;
  }

  .params-table-container h5 {
    margin: 8px 0 4px 0;
    font-size: 0.75rem;
    font-weight: 600;
  }

  .params-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.7rem;
    margin-top: 6px;
  }

  .params-table th, .params-table td {
    padding: 5px 8px;
    border: 1px solid var(--border-color);
    text-align: left;
  }

  .params-table th {
    background: rgba(0, 0, 0, 0.2);
    color: var(--text-primary);
  }

  .req-badge {
    font-size: 0.65rem;
    padding: 1px 4px;
    border-radius: 3px;
    background: rgba(255, 255, 255, 0.05);
    color: var(--text-secondary);
  }

  .req-badge.required {
    background: rgba(239, 68, 68, 0.1);
    color: #fca5a5;
    border: 1px solid rgba(239, 68, 68, 0.2);
  }

  .text-secondary {
    color: #a5b4fc;
  }

  .text-center {
    text-align: center;
  }

  .font-italic {
    font-style: italic;
  }

  .font-mono {
    font-family: monospace;
  }
</style>
