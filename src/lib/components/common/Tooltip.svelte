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
  let hostElement: HTMLSpanElement;
  let triggerElement: HTMLButtonElement;
  let panelElement = $state<HTMLSpanElement | undefined>(undefined);
  let panelStyle = $state('');
  let tooltipId = $state('');
  let hoverCloseTimer: ReturnType<typeof setTimeout> | undefined;
  let panelLabelId = $derived(tooltipId ? `${tooltipId}-label` : undefined);
  let panelDescriptionId = $derived(tooltipId ? `${tooltipId}-description` : undefined);

  function refreshOpenState() {
    isOpen = isHovered || isFocused || isClicked;
  }

  function clearHoverCloseTimer() {
    if (hoverCloseTimer !== undefined) {
      clearTimeout(hoverCloseTimer);
      hoverCloseTimer = undefined;
    }
  }

  function close() {
    clearHoverCloseTimer();
    isClicked = false;
    isHovered = false;
    isFocused = false;
    refreshOpenState();
  }

  function handleHoverEnter() {
    clearHoverCloseTimer();
    isHovered = true;
    refreshOpenState();
  }

  function handleHoverLeave() {
    clearHoverCloseTimer();
    hoverCloseTimer = setTimeout(() => {
      isHovered = false;
      hoverCloseTimer = undefined;
      refreshOpenState();
    }, 120);
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
    const panelRect = panelElement?.getBoundingClientRect();
    const width = Math.min(panelRect?.width || 288, window.innerWidth - 16);
    const height = panelRect?.height || 120;
    const gap = 8;
    let left = rect.left + (rect.width / 2) - (width / 2);
    let top = placement === 'bottom'
      ? rect.bottom + gap
      : placement === 'left' || placement === 'right'
        ? rect.top + (rect.height / 2) - (height / 2)
        : rect.top - height - gap;

    if (placement === 'left') left = rect.left - width - gap;
    if (placement === 'right') left = rect.right + gap;
    if (placement === 'top' && top < 8) top = rect.bottom + gap;
    if (placement === 'bottom' && top + height > window.innerHeight - 8) top = rect.top - height - gap;
    left = Math.max(8, Math.min(left, window.innerWidth - width - 8));
    top = Math.max(8, Math.min(top, window.innerHeight - height - 8));
    panelStyle = `left: ${Math.round(left)}px; top: ${Math.round(top)}px; width: ${Math.round(width)}px;`;
  }

  function handleHostFocusOut(event: FocusEvent) {
    const nextTarget = event.relatedTarget;
    if (nextTarget instanceof Node && hostElement?.contains(nextTarget)) return;
    isFocused = false;
    refreshOpenState();
  }

  $effect(() => {
    if (isOpen) requestAnimationFrame(updatePosition);
  });

  onMount(() => {
    tooltipId = `genomics-tooltip-${Math.random().toString(36).slice(2, 10)}`;
    document.addEventListener('pointerdown', handleDocumentPointerDown);
    document.addEventListener('keydown', handleKeydown);
    window.addEventListener('resize', updatePosition);
    window.addEventListener('scroll', updatePosition, true);
    return () => {
      document.removeEventListener('pointerdown', handleDocumentPointerDown);
      document.removeEventListener('keydown', handleKeydown);
      window.removeEventListener('resize', updatePosition);
      window.removeEventListener('scroll', updatePosition, true);
      clearHoverCloseTimer();
    };
  });

</script>

<div
  class="tooltip-host"
  bind:this={hostElement}
  data-tooltip-id={tooltipId}
  role="group"
  aria-label={label}
  onmouseenter={handleHoverEnter}
  onmouseleave={handleHoverLeave}
  onfocusin={() => { isFocused = true; refreshOpenState(); }}
  onfocusout={handleHostFocusOut}
>
  <button
    type="button"
    class="tooltip-trigger"
    bind:this={triggerElement}
    aria-label={label}
    aria-describedby={isOpen ? panelDescriptionId : undefined}
    aria-controls={isOpen && learnMoreHref ? tooltipId : undefined}
    aria-haspopup={learnMoreHref ? 'dialog' : undefined}
    aria-expanded={isOpen}
    onclick={(event) => {
      event.stopPropagation();
      isClicked = !isClicked;
      refreshOpenState();
    }}
  >
    {@render children()}
  </button>
  {#if isOpen}
    <span
      bind:this={panelElement}
      class="tooltip-panel tooltip-{placement}"
      id={tooltipId}
      role={learnMoreHref ? 'dialog' : 'tooltip'}
      aria-labelledby={panelLabelId}
      aria-describedby={panelDescriptionId}
      aria-modal={learnMoreHref ? 'false' : undefined}
      style={panelStyle}
      onmouseenter={handleHoverEnter}
      onmouseleave={handleHoverLeave}
      onclick={(event) => event.stopPropagation()}
    >
      <strong id={panelLabelId}>{label}</strong>
      <span id={panelDescriptionId}>{description}</span>
      {#if learnMoreHref}
        <a href={learnMoreHref} target="_blank" rel="noopener noreferrer">Learn more</a>
      {/if}
    </span>
  {/if}
</div>

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
    box-sizing: border-box;
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

  .tooltip-panel strong,
  .tooltip-panel span,
  .tooltip-panel a {
    overflow-wrap: anywhere;
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
