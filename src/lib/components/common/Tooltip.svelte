<!-- ./src/lib/components/common/Tooltip.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import type { Snippet } from 'svelte';

  interface Props {
    label: string;
    description: string;
    children: Snippet;
    learnMoreHref?: string;
    placement?: 'top' | 'bottom' | 'left' | 'right';
  }

  let {
    label,
    description,
    children,
    learnMoreHref,
    placement = 'top',
  }: Props = $props();

  let isOpen = $state(false);
  let isHovered = $state(false);
  let isFocused = $state(false);
  let isClicked = $state(false);
  let triggerElement: HTMLButtonElement;
  let panelStyle = $state('');
  const tooltipId = `genomics-tooltip-${Math.random().toString(36).slice(2, 10)}`;

  function refreshOpenState() {
    isOpen = isHovered || isFocused || isClicked;
  }

  function close() {
    isClicked = false;
    isHovered = false;
    refreshOpenState();
  }

  function handleDocumentPointerDown(event: PointerEvent) {
    const target = event.target;
    if (target instanceof Element && !target.closest(`[data-tooltip-id="${tooltipId}"]`)) {
      close();
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      close();
    }
  }

  function updatePosition() {
    if (!isOpen || !triggerElement) return;
    const rect = triggerElement.getBoundingClientRect();
    const width = Math.min(288, window.innerWidth - 32);
    const estimatedHeight = 120;
    const gap = 8;
    let left = rect.left + (rect.width / 2) - (width / 2);
    let top = placement === 'bottom'
      ? rect.bottom + gap
      : placement === 'left' || placement === 'right'
        ? rect.top + (rect.height / 2) - (estimatedHeight / 2)
        : rect.top - estimatedHeight - gap;

    if (placement === 'left') left = rect.left - width - gap;
    if (placement === 'right') left = rect.right + gap;
    if (placement === 'top' && top < 8) top = rect.bottom + gap;
    if (placement === 'bottom' && top + estimatedHeight > window.innerHeight - 8) top = rect.top - estimatedHeight - gap;
    left = Math.max(8, Math.min(left, window.innerWidth - width - 8));
    top = Math.max(8, Math.min(top, window.innerHeight - estimatedHeight - 8));
    panelStyle = `left: ${Math.round(left)}px; top: ${Math.round(top)}px; width: ${Math.round(width)}px;`;
  }

  $effect(() => {
    if (isOpen) requestAnimationFrame(updatePosition);
  });

  onMount(() => {
    document.addEventListener('pointerdown', handleDocumentPointerDown);
    document.addEventListener('keydown', handleKeydown);
    window.addEventListener('resize', updatePosition);
    window.addEventListener('scroll', updatePosition, true);
    return () => {
      document.removeEventListener('pointerdown', handleDocumentPointerDown);
      document.removeEventListener('keydown', handleKeydown);
      window.removeEventListener('resize', updatePosition);
      window.removeEventListener('scroll', updatePosition, true);
    };
  });

</script>

<span class="tooltip-host" data-tooltip-id={tooltipId}>
  <button
    type="button"
    class="tooltip-trigger"
    bind:this={triggerElement}
    aria-label={label}
    aria-describedby={isOpen ? tooltipId : undefined}
    aria-expanded={isOpen}
    onclick={() => {
      isClicked = !isClicked;
      refreshOpenState();
    }}
    onmouseenter={() => { isHovered = true; refreshOpenState(); }}
    onmouseleave={() => { isHovered = false; refreshOpenState(); }}
    onfocus={() => { isFocused = true; refreshOpenState(); }}
    onblur={() => { isFocused = false; refreshOpenState(); }}
  >
    {@render children()}
  </button>
  {#if isOpen}
    <span class="tooltip-panel tooltip-{placement}" id={tooltipId} role="tooltip" style={panelStyle}>
      <strong>{label}</strong>
      <span>{description}</span>
      {#if learnMoreHref}
        <a href={learnMoreHref} target="_blank" rel="noopener noreferrer">Learn more</a>
      {/if}
    </span>
  {/if}
</span>

<style>
  .tooltip-host {
    position: relative;
    display: inline-flex;
    max-width: 100%;
  }

  .tooltip-trigger {
    min-width: 44px;
    min-height: 44px;
    padding: 0;
    border: 0;
    border-radius: 0.4rem;
    background: transparent;
    color: inherit;
    font: inherit;
    cursor: help;
  }

  .tooltip-trigger:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }

  .tooltip-panel {
    position: fixed;
    z-index: 1000;
    display: flex;
    width: min(18rem, calc(100vw - 2rem));
    flex-direction: column;
    gap: 0.35rem;
    padding: 0.75rem 0.85rem;
    border: 1px solid var(--tooltip-border);
    border-radius: 0.65rem;
    background: var(--tooltip-bg);
    color: var(--tooltip-text);
    box-shadow: 0 0.75rem 2rem rgba(0, 0, 0, 0.26);
    font-size: 0.75rem;
    line-height: 1.4;
    pointer-events: auto;
  }

  .tooltip-panel strong {
    color: var(--tooltip-heading);
    font-size: 0.78rem;
  }

  .tooltip-panel a {
    color: var(--accent);
    font-weight: 700;
  }

  @media (prefers-reduced-motion: no-preference) {
    .tooltip-panel { animation: tooltip-in 120ms ease-out; }
  }

  @keyframes tooltip-in {
    from { opacity: 0; }
    to { opacity: 1; }
  }
</style>
