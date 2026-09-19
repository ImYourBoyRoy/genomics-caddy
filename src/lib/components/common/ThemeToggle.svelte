<!-- ./src/lib/components/common/ThemeToggle.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import '$lib/styles/components/theme-toggle.css';

  type ThemeMode = 'system' | 'light' | 'dark';
  const STORAGE_KEY = 'genomics_theme_mode';
  const THEME_TRANSITION_MS = 2500;
  let mode = $state<ThemeMode>('system');
  let transitionTimer: ReturnType<typeof setTimeout> | undefined;
  let transitionId = 0;
  let transitionFrame: number | undefined;

  const themeOptions: Array<{ id: ThemeMode; label: string; shortLabel: string }> = [
    { id: 'system', label: 'System default', shortLabel: 'Auto' },
    { id: 'light', label: 'Light mode', shortLabel: 'Light' },
    { id: 'dark', label: 'Dark mode', shortLabel: 'Dark' },
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
  <div class="theme-toggle-options" role="group" aria-label="Color theme options">
    {#each themeOptions as option (option.id)}
      <button
        type="button"
        class:active={mode === option.id}
        data-theme-mode={option.id}
        aria-pressed={mode === option.id}
        aria-label={option.label}
        onclick={() => selectTheme(option.id)}
      >
        {option.shortLabel}
      </button>
    {/each}
  </div>
</div>
