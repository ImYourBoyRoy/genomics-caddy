<!-- ./src/lib/components/common/EmptyState.svelte -->
<script lang="ts">
  import type { AppPaths } from '../../types/genomics';

  /*
  Module Docstring:
  Purpose: Welcome screen displayed when no active profile is loaded.
  Responsibilities:
  - Present introductory branding message.
  - Expose application database and assembly file paths for troubleshooting.
  Key Inputs: appPaths (AppPaths | null).
  Key Outputs: Visual welcome layout.
  Operational Notes: Maps storage paths in a clean table format.
  */

  interface Props {
    appPaths: AppPaths | null;
  }

  let { appPaths }: Props = $props();
</script>

<div class="welcome-screen">
  <img src="/logo.png" alt="Genomics Caddy Logo" class="welcome-logo" />
  <h1>Genomics & Personal Biohacking Dashboard</h1>
  <p>Ingest your AncestryDNA or 23andMe exports to map your chromosomes, track hormone & neurotransmitter traits, and connect securely with AI model assistants.</p>
  {#if appPaths}
    <div class="app-paths-info card">
      <h4>Application Folder Storage</h4>
      <p><strong>Database Path</strong>: <code>{appPaths.db_path}</code></p>
      <p><strong>Liftover Chain</strong>: <code>{appPaths.chain_path}</code></p>
    </div>
  {/if}
</div>
