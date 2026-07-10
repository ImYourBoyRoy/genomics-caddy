<!-- ./src/lib/components/mcp/McpInstructions.svelte -->
<script lang="ts">
  /*
  Module Docstring:
  Purpose: Tabbed setup-instructions panel for MCP client integrations.
  Responsibilities:
  - Render 5 tab buttons (Claude Desktop, Cursor, Cline/Roo, Claude Code, Raw CLI).
  - Display the corresponding configuration snippets / step-by-step instructions.
  - Provide copy-to-clipboard buttons wired to the parent's copyText callback.
  Key Inputs: activeTab, config strings, copyText callback, isDevMode, projectDir.
  Key Outputs: Rendered instruction content with copy functionality.
  Operational Notes: Uses Svelte 5 runes. activeTab is $bindable so the parent can read it.
  */

  interface Props {
    activeTab: string;
    claudeConfig: string;
    cursorConfig: { name: string; type: string; command: string };
    clineConfig: string;
    claudeCodeCommand: string;
    rawCommand: string;
    isDevMode: boolean;
    projectDir: string;
    copyStatus: string;
    copyText: (text: string, id: string) => void;
  }

  let {
    activeTab = $bindable(),
    claudeConfig,
    cursorConfig,
    clineConfig,
    claudeCodeCommand,
    rawCommand,
    isDevMode,
    projectDir,
    copyStatus,
    copyText,
  }: Props = $props();
</script>

<section class="mcp-instructions-section">
  <div class="tabs-header">
    <button class="tab-item" class:active={activeTab === "claude"} onclick={() => activeTab = "claude"}>
      💬 Claude Desktop
    </button>
    <button class="tab-item" class:active={activeTab === "cursor"} onclick={() => activeTab = "cursor"}>
      🛰️ Cursor
    </button>
    <button class="tab-item" class:active={activeTab === "cline"} onclick={() => activeTab = "cline"}>
      💻 Cline / Roo
    </button>
    <button class="tab-item" class:active={activeTab === "claude-code"} onclick={() => activeTab = "claude-code"}>
      🐚 Claude Code
    </button>
    <button class="tab-item" class:active={activeTab === "raw"} onclick={() => activeTab = "raw"}>
      ⚙️ Raw CLI
    </button>
  </div>

  <div class="tab-body">
    {#if activeTab === "claude"}
      <div class="instruction-content">
        <h4>Claude Desktop Setup</h4>
        <p>Add the following server configuration to your global config file:</p>
        <div class="path-badge">
          📄 Path: <code class="font-mono">%APPDATA%\Claude\claude_desktop_config.json</code>
        </div>

        <div class="code-container">
          <button
            class="copy-btn"
            onclick={() => copyText(claudeConfig, "claude")}
          >
            {copyStatus === "claude" ? "✅ Copied!" : "📋 Copy Config"}
          </button>
          <pre><code class="language-json">{claudeConfig}</code></pre>
        </div>

        {#if isDevMode}
          <div class="info-alert mt-3">
            <strong>💡 Dev Note:</strong> This spawns the server via <code>npm run mcp</code> which launches inside your project workspace folder <code>{projectDir}</code>.
          </div>
        {/if}
      </div>
    {:else if activeTab === "cursor"}
      <div class="instruction-content">
        <h4>Cursor Setup</h4>
        <p>Cursor is configured directly in the application settings interface:</p>
        <ol class="step-list">
          <li>Open Cursor and navigate to <strong>Settings &gt; Features &gt; MCP</strong>.</li>
          <li>Click <strong>+ Add New MCP Server</strong>.</li>
          <li>Fill out the form with these details:</li>
        </ol>

        <table class="config-table">
          <tbody>
            <tr>
              <th>Name</th>
              <td class="font-mono">{cursorConfig.name}</td>
            </tr>
            <tr>
              <th>Type</th>
              <td class="font-mono">{cursorConfig.type}</td>
            </tr>
            <tr>
              <th>Command</th>
              <td>
                <div class="inline-code-container">
                  <code class="font-mono">{cursorConfig.command}</code>
                  <button
                    class="copy-btn-inline"
                    onclick={() => copyText(cursorConfig.command, "cursor-cmd")}
                  >
                    {copyStatus === "cursor-cmd" ? "✅" : "📋"}
                  </button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>

        {#if isDevMode}
          <div class="info-alert mt-3">
            <strong>💡 Dev Note:</strong> For Cursor to locate <code>npm</code>, make sure Cursor is launched from a terminal environment that has Node.js in its <code>PATH</code>, or specify the absolute path to your global <code>npm.cmd</code>.
          </div>
        {/if}
      </div>
    {:else if activeTab === "cline"}
      <div class="instruction-content">
        <h4>Cline / Roo Code Setup</h4>
        <p>Cline and Roo Code store their MCP settings inside VS Code's global storage directory:</p>
        <div class="path-badge">
          📄 Path: <code class="font-mono">%APPDATA%\Code\User\globalStorage\saoudrizwan.claude-dev\settings\cline_mcp_settings.json</code>
        </div>

        <div class="code-container">
          <button
            class="copy-btn"
            onclick={() => copyText(clineConfig, "cline")}
          >
            {copyStatus === "cline" ? "✅ Copied!" : "📋 Copy Config"}
          </button>
          <pre><code class="language-json">{clineConfig}</code></pre>
        </div>
      </div>
    {:else if activeTab === "claude-code"}
      <div class="instruction-content">
        <h4>Claude Code CLI Setup</h4>
        <p>If you use Anthropic's interactive command-line tool (Claude Code), run this command to register Genomics Caddy globally:</p>

        <div class="code-container">
          <button
            class="copy-btn"
            onclick={() => copyText(claudeCodeCommand, "claudecode")}
          >
            {copyStatus === "claudecode" ? "✅ Copied!" : "📋 Copy Command"}
          </button>
          <pre><code class="language-bash">{claudeCodeCommand}</code></pre>
        </div>
        <p class="hint mt-2">After executing, Claude Code will automatically connect to this server when launched.</p>
      </div>
    {:else if activeTab === "raw"}
      <div class="instruction-content">
        <h4>Raw Terminal Execution</h4>
        <p>To run the headless MCP server loop manually over stdin/stdout, run this command in your terminal:</p>

        <div class="code-container">
          <button
            class="copy-btn"
            onclick={() => copyText(rawCommand, "raw")}
          >
            {copyStatus === "raw" ? "✅ Copied!" : "📋 Copy Command"}
          </button>
          <pre><code class="language-bash">{rawCommand}</code></pre>
        </div>
      </div>
    {/if}
  </div>
</section>

<style>
  .mcp-instructions-section {
    background: rgba(0, 0, 0, 0.15);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    overflow: hidden;
  }

  .tabs-header {
    display: flex;
    background: rgba(0, 0, 0, 0.2);
    border-bottom: 1px solid var(--border-color);
    overflow-x: auto;
  }

  .tab-item {
    background: none;
    border: none;
    color: var(--text-secondary);
    padding: 12px 16px;
    font-size: 0.82rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
    border-bottom: 2px solid transparent;
    white-space: nowrap;
  }

  .tab-item:hover {
    color: var(--text-primary);
    background: rgba(255, 255, 255, 0.02);
  }

  .tab-item.active {
    color: var(--accent);
    border-bottom-color: var(--accent);
    background: rgba(255, 255, 255, 0.04);
  }

  .tab-body {
    padding: 20px;
    min-height: 280px;
  }

  .instruction-content h4 {
    margin: 0 0 10px 0;
    font-size: 1rem;
    font-weight: 600;
  }

  .instruction-content p {
    font-size: 0.82rem;
    line-height: 1.4;
    color: var(--text-secondary);
    margin: 0 0 12px 0;
  }

  .path-badge {
    background: rgba(255, 255, 255, 0.05);
    padding: 6px 12px;
    border-radius: 4px;
    font-size: 0.75rem;
    color: #e5e7eb;
    margin-bottom: 12px;
    border: 1px solid rgba(255, 255, 255, 0.05);
    display: inline-block;
  }

  .code-container {
    position: relative;
    margin: 10px 0;
    border-radius: 6px;
    overflow: hidden;
    border: 1px solid var(--border-color);
  }

  .code-container pre {
    margin: 0;
    padding: 14px;
    background: rgba(0, 0, 0, 0.35);
    overflow-x: auto;
  }

  .code-container code {
    font-family: monospace;
    font-size: 0.8rem;
    color: #c7d2fe;
    display: block;
    line-height: 1.4;
  }

  .copy-btn {
    position: absolute;
    top: 8px;
    right: 8px;
    background: rgba(88, 80, 236, 0.2);
    border: 1px solid rgba(88, 80, 236, 0.4);
    color: #ffffff;
    padding: 4px 8px;
    border-radius: 4px;
    font-size: 0.7rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .copy-btn:hover {
    background: var(--accent);
    border-color: var(--accent);
  }

  .inline-code-container {
    display: flex;
    align-items: center;
    gap: 8px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--border-color);
    padding: 4px 8px;
    border-radius: 4px;
  }

  .inline-code-container code {
    flex: 1;
    word-break: break-all;
    font-size: 0.75rem;
  }

  .copy-btn-inline {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 0.8rem;
    padding: 2px;
  }

  .copy-btn-inline:hover {
    color: var(--text-primary);
  }

  .step-list {
    font-size: 0.82rem;
    color: var(--text-secondary);
    padding-left: 20px;
    margin-bottom: 14px;
  }

  .step-list li {
    margin-bottom: 6px;
  }

  .config-table {
    width: 100%;
    border-collapse: collapse;
    margin-top: 10px;
    font-size: 0.8rem;
  }

  .config-table th, .config-table td {
    padding: 8px 12px;
    border: 1px solid var(--border-color);
    text-align: left;
  }

  .config-table th {
    background: rgba(0, 0, 0, 0.2);
    width: 100px;
    color: var(--text-primary);
  }

  .info-alert {
    background: rgba(88, 80, 236, 0.1);
    border: 1px solid rgba(88, 80, 236, 0.2);
    padding: 10px 14px;
    border-radius: 6px;
    font-size: 0.78rem;
    color: #e0e7ff;
    line-height: 1.4;
  }

  .info-alert code {
    background: rgba(0, 0, 0, 0.2);
    padding: 1px 4px;
    border-radius: 3px;
    font-family: monospace;
  }

  .hint {
    font-size: 0.78rem;
    color: var(--text-secondary);
  }

  .mt-2 { margin-top: 8px; }
  .mt-3 { margin-top: 12px; }
  .font-mono { font-family: monospace; }
</style>
