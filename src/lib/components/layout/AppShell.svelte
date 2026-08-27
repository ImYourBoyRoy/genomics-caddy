<!-- ./src/lib/components/layout/AppShell.svelte -->
<script lang="ts">
  import { onMount, tick } from 'svelte';
  import type { Snippet } from 'svelte';
  import ThemeToggle from '../common/ThemeToggle.svelte';
  let focusMode = $state(false);
  let mobileSidebarOpen = $state(false);
  let isNarrowViewport = $state(false);
  let mobileToggleButton = $state<HTMLButtonElement | undefined>(undefined);
  let sidebarSlot = $state<HTMLDivElement | undefined>(undefined);

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

  function focusSidebarContent() {
    void tick().then(() => {
      const firstFocusable = sidebarSlot?.querySelector<HTMLElement>(
        'button:not([disabled]), input:not([disabled]), select:not([disabled]), a[href], [tabindex]:not([tabindex="-1"])'
      );
      firstFocusable?.focus();
    });
  }

  function openMobileSidebar() {
    mobileSidebarOpen = true;
    focusSidebarContent();
  }

  function closeMobileSidebar() {
    mobileSidebarOpen = false;
    void tick().then(() => mobileToggleButton?.focus());
  }

  function getMobileFocusables(): HTMLElement[] {
    const candidates = [
      ...Array.from(sidebarSlot?.querySelectorAll<HTMLElement>(
        'button:not([disabled]), input:not([disabled]), select:not([disabled]), a[href], [tabindex]:not([tabindex="-1"])'
      ) ?? []),
      mobileToggleButton,
    ];
    return candidates.filter((element): element is HTMLElement => {
      if (!element) return false;
      const style = getComputedStyle(element);
      return style.display !== 'none' && style.visibility !== 'hidden';
    });
  }

  function handleDocumentKeydown(event: KeyboardEvent) {
    if (!mobileSidebarOpen) return;
    if (event.key === 'Escape') {
      event.preventDefault();
      closeMobileSidebar();
      return;
    }
    if (event.key !== 'Tab') return;

    const focusable = getMobileFocusables();
    if (focusable.length === 0) return;
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  }

  onMount(() => {
    const mediaQuery = window.matchMedia('(max-width: 900px)');
    const updateViewportMode = () => {
      isNarrowViewport = mediaQuery.matches;
      if (!isNarrowViewport) mobileSidebarOpen = false;
    };

    updateViewportMode();
    mediaQuery.addEventListener('change', updateViewportMode);
    document.addEventListener('keydown', handleDocumentKeydown);
    return () => {
      mediaQuery.removeEventListener('change', updateViewportMode);
      document.removeEventListener('keydown', handleDocumentKeydown);
    };
  });
</script>

<div class="app-layout" class:focus-mode={focusMode} class:mobile-sidebar-open={mobileSidebarOpen}>
  <div
    class="sidebar-slot"
    bind:this={sidebarSlot}
    aria-hidden={focusMode || (isNarrowViewport && !mobileSidebarOpen) ? 'true' : undefined}
    inert={focusMode || (isNarrowViewport && !mobileSidebarOpen) ? true : undefined}
  >
    {@render sidebar()}
  </div>
  <div class="main-slot">
    <div class="focus-toolbar no-print">
      <div class="focus-toolbar-inner">
        <button
          type="button"
          class="focus-toggle"
          aria-label={focusMode ? 'Show data sidebar' : 'Hide data sidebar'}
          aria-controls="data-sidebar"
          aria-pressed={focusMode}
          onclick={() => focusMode = !focusMode}
        >
          {focusMode ? '☰ Show data' : '⤢ Focus report'}
        </button>
        <ThemeToggle />
      </div>
    </div>
    {@render children()}
  </div>
  {#if isNarrowViewport && mobileSidebarOpen}
    <button
      type="button"
      class="mobile-sidebar-backdrop no-print"
      aria-hidden="true"
      tabindex="-1"
      onclick={closeMobileSidebar}
    ></button>
  {/if}
  {#if isNarrowViewport}
    <button
      type="button"
      class="mobile-sidebar-toggle no-print"
      bind:this={mobileToggleButton}
      aria-label={mobileSidebarOpen ? 'Close data controls' : 'Open data controls'}
      aria-controls="data-sidebar"
      aria-expanded={mobileSidebarOpen}
      onclick={mobileSidebarOpen ? closeMobileSidebar : openMobileSidebar}
    >
      {mobileSidebarOpen ? '× Close data' : '☰ Data controls'}
    </button>
  {/if}
</div>
