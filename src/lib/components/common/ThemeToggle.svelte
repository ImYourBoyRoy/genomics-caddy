<!-- ./src/lib/components/common/ThemeToggle.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';

  type ThemeMode = 'system' | 'light' | 'dark';
  const STORAGE_KEY = 'genomics_theme_mode';
  const THEME_TRANSITION_MS = 2500;
  let mode = $state<ThemeMode>('system');
  let transitionTimer: ReturnType<typeof setTimeout> | undefined;
  let transitionId = 0;
  let transitionFrame: number | undefined;

  const themeOptions: Array<{ id: ThemeMode; label: string; shortLabel: string; icon: string }> = [
    { id: 'system', label: 'System default', shortLabel: 'Auto', icon: '◐' },
    { id: 'light', label: 'Light mode', shortLabel: 'Light', icon: '☀️' },
    { id: 'dark', label: 'Dark mode', shortLabel: 'Dark', icon: '🌙' },
  ];

  function readStoredTheme(): ThemeMode {
    try {
      const stored = localStorage.getItem(STORAGE_KEY);
      return stored === 'system' || stored === 'light' || stored === 'dark' ? stored : 'system';
    } catch {
      return 'system';
    }
  }

  function persistTheme(nextMode: ThemeMode) {
    try {
      localStorage.setItem(STORAGE_KEY, nextMode);
    } catch {
      // A restricted storage area must not prevent changing the active theme.
    }
  }

  function applyTheme(nextMode: ThemeMode, animate = false) {
    const root = document.documentElement;
    const currentTransitionId = ++transitionId;
    mode = nextMode;
    if (transitionFrame !== undefined) {
      window.cancelAnimationFrame(transitionFrame);
      transitionFrame = undefined;
    }
    if (transitionTimer) {
      clearTimeout(transitionTimer);
      transitionTimer = undefined;
    }

    if (!animate || root.dataset.theme === nextMode) {
      delete root.dataset.themeTransition;
      root.dataset.theme = nextMode;
      persistTheme(nextMode);
      return;
    }

    // Always establish a fully opaque cover before swapping the theme. This
    // keeps repeated rapid switches from inheriting a partially revealed veil.
    root.dataset.themeTransition = 'cover';
    void root.offsetWidth;
    transitionFrame = window.requestAnimationFrame(() => {
      if (currentTransitionId !== transitionId) return;
      root.dataset.theme = nextMode;
      persistTheme(nextMode);
      transitionFrame = window.requestAnimationFrame(() => {
        if (currentTransitionId === transitionId) {
          root.dataset.themeTransition = 'reveal';
          transitionFrame = undefined;
        }
      });
    });
    transitionTimer = setTimeout(() => {
      if (currentTransitionId === transitionId) {
        delete root.dataset.themeTransition;
        transitionTimer = undefined;
      }
    }, THEME_TRANSITION_MS + 60);
  }

  function selectTheme(nextMode: ThemeMode) {
    applyTheme(nextMode, true);
  }

  onMount(() => {
    mode = readStoredTheme();
    document.documentElement.dataset.theme = mode;
  });
</script>

<div class="theme-toggle no-print" data-theme-toggle aria-label="Color theme">
  <span class="theme-toggle-label">Theme</span>
  <div class="theme-toggle-options" role="group" aria-label="Color theme options">
    {#each themeOptions as option}
      <button
        type="button"
        class:active={mode === option.id}
        data-theme-mode={option.id}
        aria-pressed={mode === option.id}
        aria-label={option.label}
        onclick={() => selectTheme(option.id)}
      >
        <span aria-hidden="true">{option.icon}</span>
        <span>{option.shortLabel}</span>
      </button>
    {/each}
  </div>
</div>

<style>
  .theme-toggle {
    display: grid;
    gap: 0.35rem;
    width: 100%;
    color: var(--text-secondary);
  }

  .theme-toggle-label {
    padding-inline: 0.15rem;
    font-size: 0.72rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .theme-toggle-options {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.25rem;
    padding: 0.2rem;
    border: 1px solid var(--border-color);
    border-radius: 0.65rem;
    background: var(--control-group-bg);
  }

  :global(.theme-toggle-options button) {
    display: inline-flex;
    appearance: none;
    -webkit-appearance: none;
    min-width: 0;
    min-height: 44px;
    align-items: center;
    justify-content: center;
    gap: 0.25rem;
    padding: 0.3rem 0.2rem;
    border: 1px solid transparent;
    border-radius: 0.45rem;
    background: var(--surface-raised);
    color: var(--text-secondary);
    font: inherit;
    font-size: 0.72rem;
    font-weight: 700;
    cursor: pointer;
  }

  :global(.theme-toggle-options button:hover),
  :global(.theme-toggle-options button:focus-visible),
  :global(.theme-toggle-options button.active) {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--text-primary);
  }

  @media (prefers-reduced-motion: reduce) {
    :global(.theme-toggle-options button) {
      transition: none;
    }
  }
</style>
