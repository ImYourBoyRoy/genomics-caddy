<!-- ./src/lib/components/layout/AppShell.svelte -->
<script lang="ts">
  import type { Snippet } from 'svelte';

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

<div class="app-layout">
  {@render sidebar()}
  {@render children()}
</div>
