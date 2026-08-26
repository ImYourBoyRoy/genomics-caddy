<!-- ./src/lib/components/layout/AppShell.svelte -->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import ThemeToggle from '../common/ThemeToggle.svelte';
  let focusMode = $state(false);

  /*
  Module Docstring:
  Purpose: Main layout grid wrapper for the Genomics Caddy application.
  Responsibilities:
  - Establish the outer grid container conforming to the .app-layout stylesheet rules.
  - Render a sidebar slot and a main content slot via Svelte 5 snippets.
  Key Inputs: sidebar snippet, children snippet.
  Key Outputs: Structured layout markup.
  Operational Notes: Relies on theme.css for positioning and styling.
  */

  interface Props {
    sidebar: Snippet;
    children: Snippet;
  }

  let { sidebar, children }: Props = $props();
</script>

<div class="app-layout" class:focus-mode={focusMode}>
  {@render sidebar()}
  {@render children()}
  <button
    type="button"
    class="focus-toggle no-print"
    aria-label={focusMode ? 'Show data sidebar' : 'Hide data sidebar'}
    aria-pressed={focusMode}
    onclick={() => focusMode = !focusMode}
  >
    {focusMode ? '☰ Show data' : '⤢ Focus report'}
  </button>
  <ThemeToggle />
</div>
