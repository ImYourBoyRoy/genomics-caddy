<!-- ./src/lib/components/common/ThemeToggle.svelte -->
<script lang="ts">
  import { onMount, tick } from 'svelte';

  type ThemeMode = 'system' | 'light' | 'dark';
  const STORAGE_KEY = 'genomics_theme_mode';
  let mode = $state<ThemeMode>('system');
  let isOpen = $state(false);
  let rootElement: HTMLDivElement;
  let toggleButton = $state<HTMLButtonElement | undefined>(undefined);
  let optionsElement = $state<HTMLDivElement | undefined>(undefined);

  const themeOptions: Array<{ id: ThemeMode; label: string; icon: string }> = [
    { id: 'system', label: 'System default', icon: '◐' },
    { id: 'light', label: 'Light mode', icon: '☀️' },
    { id: 'dark', label: 'Dark mode', icon: '🌙' },
  ];

  function themeModeLabel(value: ThemeMode): string {
    return themeOptions.find((option) => option.id === value)?.label ?? 'System default';
  }

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
    if (event.key === 'Escape' && isOpen) {
      event.preventDefault();
      closeMenu(true);
    }
  }

  function openMenu() {
    isOpen = true;
    void tick().then(() => {
      optionsElement?.querySelector<HTMLButtonElement>('[aria-checked="true"]')?.focus();
    });
  }

  function closeMenu(returnFocus = false) {
    isOpen = false;
    if (returnFocus) toggleButton?.focus();
  }

  function toggleMenu() {
    if (isOpen) {
      closeMenu();
    } else {
      openMenu();
    }
  }

  function selectTheme(nextMode: ThemeMode) {
    applyTheme(nextMode);
    closeMenu(true);
  }

  function handleMenuKeydown(event: KeyboardEvent) {
    const buttons = Array.from(optionsElement?.querySelectorAll<HTMLButtonElement>('[role="menuitemradio"]') ?? []);
    const currentIndex = buttons.findIndex((button) => button === document.activeElement);
    if (event.key === 'Escape') {
      event.preventDefault();
      closeMenu(true);
      return;
    }

    let nextIndex: number | undefined;
    if (event.key === 'ArrowDown') nextIndex = currentIndex < buttons.length - 1 ? currentIndex + 1 : 0;
    if (event.key === 'ArrowUp') nextIndex = currentIndex > 0 ? currentIndex - 1 : buttons.length - 1;
    if (event.key === 'Home') nextIndex = 0;
    if (event.key === 'End') nextIndex = buttons.length - 1;
    if (nextIndex !== undefined && buttons.length > 0) {
      event.preventDefault();
      buttons[nextIndex]?.focus();
    }
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
    bind:this={toggleButton}
    aria-expanded={isOpen}
    aria-controls="theme-options"
    aria-haspopup="menu"
    aria-label={`Appearance: ${themeModeLabel(mode)}`}
    onclick={toggleMenu}
  >
    <span aria-hidden="true">{mode === 'dark' ? '🌙' : mode === 'light' ? '☀️' : '◐'}</span>
    <span>{themeModeLabel(mode)}</span>
  </button>
  {#if isOpen}
    <div id="theme-options" class="theme-options" role="menu" tabindex="-1" aria-label="Appearance preference" aria-orientation="vertical" bind:this={optionsElement} onkeydown={handleMenuKeydown}>
      {#each themeOptions as option}
        <button
          type="button"
          class:active={mode === option.id}
          role="menuitemradio"
          aria-checked={mode === option.id}
          onclick={() => selectTheme(option.id)}
        >
          <span aria-hidden="true">{option.icon}</span> {option.label}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .theme-toggle {
    position: relative;
    z-index: 120;
    flex: 0 0 auto;
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
    box-shadow: 0 0.5rem 1.25rem var(--shadow-floating);
  }

  .theme-options {
    position: absolute;
    top: calc(100% + 0.4rem);
    right: 0;
    display: grid;
    min-width: 9rem;
    gap: 0.3rem;
    padding: 0.4rem;
    border: 1px solid var(--border-color);
    border-radius: 0.7rem;
    background: var(--surface-raised);
    box-shadow: 0 0.75rem 2rem var(--shadow-floating);
  }

  .theme-options button {
    padding: 0.4rem 0.6rem;
    text-align: left;
  }

  .theme-options button:hover,
  .theme-options button.active,
  .theme-options button:focus-visible {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

</style>
