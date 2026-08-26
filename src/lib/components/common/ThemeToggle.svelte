<!-- ./src/lib/components/common/ThemeToggle.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';

  type ThemeMode = 'system' | 'light' | 'dark';
  const STORAGE_KEY = 'genomics_theme_mode';
  let mode = $state<ThemeMode>('system');
  let isOpen = $state(false);
  let rootElement: HTMLDivElement;

  function readStoredTheme(): ThemeMode {
    try {
      const stored = localStorage.getItem(STORAGE_KEY);
      return stored === 'system' || stored === 'light' || stored === 'dark' ? stored : 'system';
    } catch {
      return 'system';
    }
  }

  function applyTheme(nextMode: ThemeMode) {
    mode = nextMode;
    document.documentElement.dataset.theme = nextMode;
    try {
      localStorage.setItem(STORAGE_KEY, nextMode);
    } catch {
      // A restricted storage area must not prevent changing the active theme.
    }
  }

  function handleDocumentPointerDown(event: PointerEvent) {
    const target = event.target;
    if (target instanceof Node && !rootElement?.contains(target)) isOpen = false;
  }

  function handleDocumentKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') isOpen = false;
  }

  onMount(() => {
    mode = readStoredTheme();
    document.documentElement.dataset.theme = mode;
    document.addEventListener('pointerdown', handleDocumentPointerDown);
    document.addEventListener('keydown', handleDocumentKeydown);
    return () => {
      document.removeEventListener('pointerdown', handleDocumentPointerDown);
      document.removeEventListener('keydown', handleDocumentKeydown);
    };
  });
</script>

<div class="theme-toggle no-print" data-theme-toggle bind:this={rootElement}>
  <button
    type="button"
    class="theme-toggle-button"
    aria-expanded={isOpen}
    aria-controls="theme-options"
    aria-haspopup="menu"
    aria-label={`Theme: ${mode}`}
    onclick={() => isOpen = !isOpen}
  >
    <span aria-hidden="true">{mode === 'dark' ? '🌙' : mode === 'light' ? '☀️' : '◐'}</span>
    <span>Theme</span>
  </button>
  {#if isOpen}
    <div id="theme-options" class="theme-options" role="menu" aria-label="Theme preference">
      {#each [
        { id: 'system', label: 'System', icon: '◐' },
        { id: 'light', label: 'Light', icon: '☀️' },
        { id: 'dark', label: 'Dark', icon: '🌙' },
      ] as option}
        <button
          type="button"
          class:active={mode === option.id}
          role="menuitemradio"
          aria-checked={mode === option.id}
          onclick={() => { applyTheme(option.id as ThemeMode); isOpen = false; }}
        >
          <span aria-hidden="true">{option.icon}</span> {option.label}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .theme-toggle {
    position: fixed;
    z-index: 120;
    left: calc(var(--sidebar-width) + 0.75rem);
    bottom: 0.75rem;
  }

  .theme-toggle-button,
  .theme-options button {
    min-height: 44px;
    border: 1px solid var(--border-color);
    border-radius: 0.6rem;
    background: var(--surface-raised);
    color: var(--text-primary);
    font: inherit;
    cursor: pointer;
  }

  .theme-toggle-button {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    padding: 0.45rem 0.7rem;
    font-size: 0.75rem;
    font-weight: 700;
    box-shadow: 0 0.5rem 1.25rem rgba(0, 0, 0, 0.18);
  }

  .theme-options {
    position: absolute;
    bottom: calc(100% + 0.4rem);
    left: 0;
    display: grid;
    min-width: 9rem;
    gap: 0.3rem;
    padding: 0.4rem;
    border: 1px solid var(--border-color);
    border-radius: 0.7rem;
    background: var(--surface-raised);
    box-shadow: 0 0.75rem 2rem rgba(0, 0, 0, 0.25);
  }

  .theme-options button {
    padding: 0.4rem 0.6rem;
    text-align: left;
  }

  .theme-options button:hover,
  .theme-options button.active {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  @media (max-width: 900px) {
    .theme-toggle {
      left: 0.75rem;
    }
  }
</style>
