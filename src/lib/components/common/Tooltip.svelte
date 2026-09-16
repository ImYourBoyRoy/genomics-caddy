<!-- ./src/lib/components/common/Tooltip.svelte -->
<script lang="ts">
  import { onMount, tick } from 'svelte';
  import type { Snippet } from 'svelte';
  import { calculateTooltipPosition, type TooltipPlacement } from '../../utils/tooltipPosition';

  type InteractiveClickBehavior = 'toggle' | 'dismiss';

  interface Props {
    label: string;
    description: string;
    children: Snippet;
    learnMoreHref?: string;
    placement?: TooltipPlacement;
    triggerClass?: string;
    interactiveChildren?: boolean;
    interactiveClickBehavior?: InteractiveClickBehavior;
  }

  let {
    label,
    description,
    children,
    learnMoreHref,
    placement = 'top',
    triggerClass = '',
    interactiveChildren = false,
    interactiveClickBehavior = 'toggle',
  }: Props = $props();

  let isOpen = $state(false);
  let isHovered = $state(false);
  let isFocused = $state(false);
  let isClicked = $state(false);
  let hostElement: HTMLSpanElement;
  let triggerElement = $state<HTMLElement | undefined>(undefined);
  let panelElement = $state<HTMLSpanElement | undefined>(undefined);
  let panelResizeObserver: ResizeObserver | undefined;
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
    if (event.key !== 'Escape' || !isOpen) return;
    event.preventDefault();
    event.stopImmediatePropagation();
    close();
  }

  function handleTriggerClick(event: MouseEvent) {
    event.stopPropagation();
    if (interactiveChildren && interactiveClickBehavior === 'dismiss') {
      // The child is an action control. Its click should complete the action
      // without pinning the explanatory tooltip over the next UI state.
      close();
      return;
    }
    if (isClicked) {
      close();
      return;
    }
    isClicked = true;
    refreshOpenState();
  }

  function syncInteractiveTrigger() {
    if (!interactiveChildren || !hostElement || !tooltipId) return;
    const nextTrigger = hostElement.querySelector<HTMLElement>(
      'button, a, input, select, textarea, [tabindex]:not([tabindex="-1"])',
    );
    if (!nextTrigger) return;
    triggerElement = nextTrigger;
    if (!nextTrigger.getAttribute('aria-label')) nextTrigger.setAttribute('aria-label', label);
    if (isOpen && panelDescriptionId) {
      nextTrigger.setAttribute('aria-describedby', panelDescriptionId);
    } else {
      nextTrigger.removeAttribute('aria-describedby');
    }
    nextTrigger.setAttribute('aria-expanded', String(isOpen));
    if (learnMoreHref) {
      nextTrigger.setAttribute('aria-haspopup', 'dialog');
      if (isOpen) nextTrigger.setAttribute('aria-controls', tooltipId);
      else nextTrigger.removeAttribute('aria-controls');
    }
  }

  function updatePosition() {
    if (!isOpen || !triggerElement) return;
    const rect = triggerElement.getBoundingClientRect();
    const panelRect = panelElement?.getBoundingClientRect();
    const position = calculateTooltipPosition({
      trigger: rect,
      panel: panelRect,
      viewport: { width: window.innerWidth, height: window.innerHeight },
      placement,
    });
    panelStyle = `left: ${position.left}px; top: ${position.top}px; width: ${position.width}px;`;
  }

  function observePanelSize() {
    panelResizeObserver?.disconnect();
    if (!panelElement) return;
    panelResizeObserver = new ResizeObserver(() => {
      if (isOpen) requestAnimationFrame(updatePosition);
    });
    panelResizeObserver.observe(panelElement);
  }

  function handleHostFocusOut(event: FocusEvent) {
    const nextTarget = event.relatedTarget;
    if (nextTarget instanceof Node && hostElement?.contains(nextTarget)) return;
    isFocused = false;
    refreshOpenState();
  }

  $effect(() => {
    if (isOpen) {
      void tick().then(() => {
        if (!isOpen) return;
        if (interactiveChildren && tooltipId) syncInteractiveTrigger();
        observePanelSize();
        requestAnimationFrame(updatePosition);
      });
    } else {
      panelResizeObserver?.disconnect();
      panelResizeObserver = undefined;
    }
  });

  onMount(() => {
    tooltipId = `genomics-tooltip-${Math.random().toString(36).slice(2, 10)}`;
    document.addEventListener('pointerdown', handleDocumentPointerDown);
    // Capture Escape before parent drawers/modals can consume it.
    document.addEventListener('keydown', handleKeydown, true);
    window.addEventListener('resize', updatePosition);
    window.addEventListener('scroll', updatePosition, true);
    return () => {
      document.removeEventListener('pointerdown', handleDocumentPointerDown);
      document.removeEventListener('keydown', handleKeydown, true);
      window.removeEventListener('resize', updatePosition);
      window.removeEventListener('scroll', updatePosition, true);
      panelResizeObserver?.disconnect();
      clearHoverCloseTimer();
    };
  });

</script>

<div
  class="tooltip-host"
  class:interactive-children={interactiveChildren}
  bind:this={hostElement}
  data-tooltip-id={tooltipId}
  role="presentation"
  onmouseenter={handleHoverEnter}
  onmouseleave={handleHoverLeave}
  onfocusin={() => { isFocused = true; refreshOpenState(); }}
  onfocusout={handleHostFocusOut}
  onclick={interactiveChildren ? handleTriggerClick : undefined}
>
  {#if interactiveChildren}
    {@render children()}
  {:else}
    <button
      type="button"
      class={`tooltip-trigger ${triggerClass}`}
      bind:this={triggerElement}
      aria-label={label}
      aria-describedby={isOpen ? panelDescriptionId : undefined}
      aria-controls={isOpen && learnMoreHref ? tooltipId : undefined}
      aria-haspopup={learnMoreHref ? 'dialog' : undefined}
      aria-expanded={isOpen}
      onclick={handleTriggerClick}
    >
      {@render children()}
    </button>
  {/if}
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

  .tooltip-host.interactive-children {
    position: static;
    display: contents;
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
    box-shadow: 0 0.75rem 2rem var(--shadow-tooltip);
    font-size: 0.75rem;
    line-height: 1.4;
    max-height: min(24rem, calc(100vh - 1rem));
    overflow-y: auto;
    overscroll-behavior: contain;
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
