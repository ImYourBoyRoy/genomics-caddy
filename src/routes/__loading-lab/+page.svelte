<!-- ./src/routes/__loading-lab/+page.svelte -->
<script lang="ts">
  /*
    Dev-only visual lab for loader states (startup, import, reference sync).
    Open with: pnpm run loading-lab
    Or: pnpm run dev → http://127.0.0.1:5173/__loading-lab?s=res-download
    Production / Tauri release builds show a closed gate instead of fixtures.
  */
  import { page } from "$app/state";
  import BootstrapOverlay from "#lib/components/common/bootstrap/BootstrapOverlay.svelte";
  import {
    LOADING_LAB_DEFAULT_SCENARIO,
    LOADING_LAB_SCENARIO_IDS,
    loadingLabScenario,
  } from "#lib/dev/loadingLabScenarios.js";

  const isDev = import.meta.env.DEV;
  const scenarioId = $derived(page.url.searchParams.get("s") ?? LOADING_LAB_DEFAULT_SCENARIO);
  const scenario = $derived(loadingLabScenario(scenarioId));
</script>

{#if !isDev}
  <main class="lab-gate">
    <h1>Loading lab unavailable</h1>
    <p>This route is a Vite-dev fixture only. Use <code>pnpm run loading-lab</code> during development.</p>
    <a href="/">Return home</a>
  </main>
{:else}
  <div class="lab-chrome" aria-label="Loading lab controls">
    <strong>Loading lab</strong>
    <nav>
      {#each LOADING_LAB_SCENARIO_IDS as id (id)}
        <a
          href={`?s=${id}`}
          class:active={scenario.id === id}
          aria-current={scenario.id === id ? "page" : undefined}
        >{loadingLabScenario(id).label}</a>
      {/each}
    </nav>
  </div>

  <BootstrapOverlay
    phase={scenario.phase}
    message={scenario.message}
    mode={scenario.mode}
    importPhase={scenario.importPhase}
    importProgress={scenario.importProgress}
    importProfileName={scenario.importProfileName}
    reportProgress={scenario.reportProgress}
    resourceSync={scenario.resourceSync}
    showWorkspaceAction={scenario.showWorkspaceAction}
  />
{/if}

<style>
  .lab-gate {
    min-height: 100dvh;
    display: grid;
    place-content: center;
    gap: 0.75rem;
    padding: 2rem;
    text-align: center;
    background: #0b1220;
    color: #e2e8f0;
  }

  .lab-gate a {
    color: #93c5fd;
  }

  .lab-chrome {
    position: fixed;
    z-index: 40;
    inset: 0.75rem auto auto 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
    max-width: min(22rem, calc(100vw - 1.5rem));
    padding: 0.65rem 0.75rem;
    border: 1px solid rgba(148, 163, 184, 0.35);
    border-radius: 12px;
    background: rgba(15, 23, 42, 0.92);
    color: #e2e8f0;
    font-size: 0.78rem;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.35);
  }

  .lab-chrome nav {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .lab-chrome a {
    color: #cbd5e1;
    text-decoration: none;
    padding: 0.2rem 0.35rem;
    border-radius: 6px;
  }

  .lab-chrome a:hover,
  .lab-chrome a.active {
    background: rgba(56, 189, 248, 0.16);
    color: #f8fafc;
  }
</style>
