<!-- ./src/lib/components/mcp/McpPanel.svelte -->
<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentExe, getMcpTools } from "../../api/tauri";
  import type { AppPaths } from "../../types/genomics";
  import McpInstructions from "./McpInstructions.svelte";
  import McpToolCatalog from "./McpToolCatalog.svelte";

  /*
  Module Docstring:
  Purpose: Model Context Protocol (MCP) dynamic integration center (container shell).
  Responsibilities:
  - Dynamically query running executable path and available MCP tool definitions from Rust backend.
  - Automatically detect and toggle configuration settings for Development mode vs Production mode.
  - Provide copy-pasteable configuration payloads for Claude Desktop, Cursor, Cline/Roo Code, and Claude Code (CLI).
  - List and explain all dynamically registered MCP tools and their schemas.
  Key Inputs: appPaths (AppPaths | null).
  Key Outputs: Visual MCP configuration generator and tool catalog.
  Operational Notes: Uses Svelte 5 runes. Delegates instructions and tool catalog to subcomponents.
  */

  interface Props {
    appPaths: AppPaths | null;
  }

  let { appPaths }: Props = $props();

  // Core state
  let exePath = $state("");
  let mcpTools = $state<any[]>([]);
  let activeTab = $state("claude"); // "claude", "cursor", "cline", "claude-code", "raw"
  let isDevMode = $state(import.meta.env.DEV); // Default to current env, user can toggle
  let copyStatus = $state("");

  const TARGET_SUFFIXES = [
    "/src-tauri/target/debug/DNA-Tools.exe",
    "/src-tauri/target/release/DNA-Tools.exe",
    "/src-tauri/target/debug/DNA-Tools",
    "/src-tauri/target/release/DNA-Tools",
    "/src-tauri/target/debug/tauri-app.exe",
    "/src-tauri/target/release/tauri-app.exe",
    "/src-tauri/target/debug/tauri-app",
    "/src-tauri/target/release/tauri-app",
    "/App/DNA-Tools.exe",
    "/App/DNA-Tools",
  ];

  function deriveProjectDirFromExe(path: string): string {
    const normalized = path.replace(/\\/g, "/");
    for (const suffix of TARGET_SUFFIXES) {
      if (normalized.endsWith(suffix)) {
        return normalized.slice(0, -suffix.length);
      }
    }
    const appIdx = normalized.lastIndexOf("/App/");
    if (appIdx >= 0) return normalized.slice(0, appIdx);
    return normalized;
  }

  function fallbackExeFromAppPaths(paths: AppPaths): string {
    const db = paths.db_path.replace(/\\/g, "/");
    if (db.includes("/App/Data/")) {
      return db.replace("/App/Data/user_genome.db", "/App/DNA-Tools");
    }
    if (db.includes("/data/")) {
      return db.replace("/data/user_genome.db", "/src-tauri/target/debug/DNA-Tools");
    }
    return "";
  }

  // Derived paths
  let projectDir = $derived.by(() => {
    if (appPaths?.project_root) return appPaths.project_root.replace(/\\/g, "/");
    if (exePath) return deriveProjectDirFromExe(exePath);
    return "";
  });

  onMount(async () => {
    try {
      exePath = await getCurrentExe();
    } catch (e) {
      console.error("Failed to fetch current executable path:", e);
      if (appPaths) {
        exePath = fallbackExeFromAppPaths(appPaths);
      }
    }

    try {
      mcpTools = await getMcpTools();
    } catch (e) {
      console.error("Failed to fetch MCP tools:", e);
    }
  });

  // Copy helper
  function copyText(text: string, id: string) {
    navigator.clipboard.writeText(text);
    copyStatus = id;
    setTimeout(() => {
      if (copyStatus === id) copyStatus = "";
    }, 2000);
  }

  // Configuration strings based on current state
  let claudeConfig = $derived.by(() => {
    if (isDevMode) {
      return JSON.stringify({
        mcpServers: {
          "genomics-caddy-dev": {
            command: "npm",
            args: ["run", "mcp"],
            options: {
              cwd: projectDir
            }
          }
        }
      }, null, 2);
    } else {
      return JSON.stringify({
        mcpServers: {
          "genomics-caddy": {
            command: exePath,
            args: ["--mcp"]
          }
        }
      }, null, 2);
    }
  });

  let cursorConfig = $derived.by(() => {
    if (isDevMode) {
      return {
        name: "genomics-caddy-dev",
        type: "command",
        command: `npm run mcp`
      };
    } else {
      return {
        name: "genomics-caddy",
        type: "command",
        command: `"${exePath}" --mcp`
      };
    }
  });

  let clineConfig = $derived.by(() => {
    if (isDevMode) {
      return JSON.stringify({
        mcpServers: {
          "genomics-caddy-dev": {
            command: "npm",
            args: ["run", "mcp"],
            options: {
              cwd: projectDir
            },
            disabled: false
          }
        }
      }, null, 2);
    } else {
      return JSON.stringify({
        mcpServers: {
          "genomics-caddy": {
            command: exePath,
            args: ["--mcp"],
            disabled: false
          }
        }
      }, null, 2);
    }
  });

  let claudeCodeCommand = $derived.by(() => {
    if (isDevMode) {
      return `claude mcp add genomics-caddy-dev npm -- run mcp`;
    } else {
      return `claude mcp add genomics-caddy "${exePath}" -- --mcp`;
    }
  });

  let rawCommand = $derived.by(() => {
    if (isDevMode) {
      return `npm run mcp`;
    } else {
      return `"${exePath}" --mcp`;
    }
  });
</script>

<div class="mcp-container card">
  <!-- Header Banner -->
  <div class="mcp-header">
    <div class="mcp-title-area">
      <span class="mcp-icon">🤖</span>
      <div>
        <h3>Model Context Protocol (MCP) Integration</h3>
        <p class="subtitle">Securely expose your genomic database to local AI agents and coding tools.</p>
      </div>
    </div>

    <!-- Environment Toggle -->
    <div class="env-toggle">
      <span class="toggle-label" class:active={isDevMode}>Developer Mode</span>
      <button
        class="switch-btn"
        onclick={() => isDevMode = !isDevMode}
        aria-label="Toggle Environment Mode"
      >
        <span class="switch-slider" class:checked={!isDevMode}></span>
      </button>
      <span class="toggle-label" class:active={!isDevMode}>Production Mode</span>
    </div>
  </div>

  <div class="mcp-description">
    <p>
      Model Context Protocol (MCP) is an open standard that allows Large Language Models (like Claude, Cursor, Cline, or local Antigravity subagents) to safely query external tools.
      By enabling MCP, you grant your AI assistants read-only access to query DNA variants, analyze allele severity, and run trait pack reports strictly on your local machine.
    </p>
  </div>

  <!-- Main Grid -->
  <div class="mcp-grid">
    <McpInstructions
      bind:activeTab
      {claudeConfig}
      {cursorConfig}
      {clineConfig}
      {claudeCodeCommand}
      {rawCommand}
      {isDevMode}
      {projectDir}
      {copyStatus}
      {copyText}
    />
    <McpToolCatalog {mcpTools} />
  </div>
</div>

<style>
  .mcp-container {
    display: flex;
    flex-direction: column;
    gap: 20px;
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 24px;
    box-sizing: border-box;
    max-height: calc(100vh - 140px);
    overflow-y: auto;
  }

  .mcp-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--border-color);
    padding-bottom: 16px;
    flex-wrap: wrap;
    gap: 16px;
  }

  .mcp-title-area {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .mcp-icon {
    font-size: 2.5rem;
  }

  .mcp-header h3 {
    margin: 0 0 4px 0;
    font-size: 1.25rem;
    font-weight: 600;
  }

  .subtitle {
    margin: 0;
    font-size: 0.85rem;
    color: var(--text-secondary);
  }

  /* Environment Slider Toggle */
  .env-toggle {
    display: flex;
    align-items: center;
    gap: 10px;
    background: rgba(0, 0, 0, 0.2);
    padding: 6px 12px;
    border-radius: 9999px;
    border: 1px solid var(--border-color);
  }

  .toggle-label {
    font-size: 0.75rem;
    color: var(--text-secondary);
    font-weight: 500;
    transition: color 0.2s;
  }

  .toggle-label.active {
    color: var(--text-primary);
    font-weight: 600;
  }

  .switch-btn {
    position: relative;
    width: 36px;
    height: 20px;
    background: var(--accent);
    border-radius: 9999px;
    border: none;
    cursor: pointer;
    padding: 0;
    transition: background 0.2s;
  }

  .switch-slider {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    background: #ffffff;
    border-radius: 50%;
    transition: transform 0.2s ease;
  }

  .switch-slider.checked {
    transform: translateX(16px);
  }

  .mcp-description {
    font-size: 0.88rem;
    line-height: 1.5;
    color: var(--text-secondary);
    margin: 0;
  }

  .mcp-grid {
    display: grid;
    grid-template-columns: 1.2fr 0.8fr;
    gap: 24px;
    align-items: start;
  }

  @media (max-width: 1024px) {
    .mcp-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
