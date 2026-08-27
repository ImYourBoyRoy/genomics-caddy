/**
 * Calculate a fixed tooltip panel position without allowing it outside the
 * visible viewport. The DOM component supplies measured trigger/panel boxes;
 * keeping the math here makes viewport-edge behavior deterministic and easy
 * to test without rendering private report content.
 */

export type TooltipPlacement = 'top' | 'bottom' | 'left' | 'right';

export interface TooltipRect {
  left: number;
  top: number;
  right: number;
  bottom: number;
  width: number;
  height: number;
}

export interface TooltipPositionInput {
  trigger: TooltipRect;
  panel?: Pick<TooltipRect, 'width' | 'height'>;
  viewport: { width: number; height: number };
  placement: TooltipPlacement;
  fallbackWidth?: number;
  fallbackHeight?: number;
  gap?: number;
  viewportPadding?: number;
}

export interface TooltipPosition {
  left: number;
  top: number;
  width: number;
}

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.min(Math.max(value, minimum), maximum);
}

export function calculateTooltipPosition({
  trigger,
  panel,
  viewport,
  placement,
  fallbackWidth = 288,
  fallbackHeight = 120,
  gap = 8,
  viewportPadding = 8,
}: TooltipPositionInput): TooltipPosition {
  const horizontalPadding = Math.max(0, Math.min(viewportPadding, viewport.width / 2));
  const availableWidth = Math.max(0, viewport.width - horizontalPadding * 2);
  const width = Math.min(Math.max(panel?.width || fallbackWidth, 0), availableWidth);
  const height = Math.max(panel?.height || fallbackHeight, 0);
  let left = trigger.left + trigger.width / 2 - width / 2;
  let top = placement === 'bottom'
    ? trigger.bottom + gap
    : placement === 'left' || placement === 'right'
      ? trigger.top + trigger.height / 2 - height / 2
      : trigger.top - height - gap;

  if (placement === 'left') {
    const leftSpace = trigger.left - horizontalPadding;
    const rightSpace = viewport.width - horizontalPadding - trigger.right;
    left = leftSpace >= width + gap || rightSpace < width + gap
      ? trigger.left - width - gap
      : trigger.right + gap;
  }
  if (placement === 'right') {
    const rightSpace = viewport.width - horizontalPadding - trigger.right;
    const leftSpace = trigger.left - horizontalPadding;
    left = rightSpace >= width + gap || leftSpace < width + gap
      ? trigger.right + gap
      : trigger.left - width - gap;
  }
  if (placement === 'top' && top < horizontalPadding) top = trigger.bottom + gap;
  if (placement === 'bottom' && top + height > viewport.height - horizontalPadding) {
    top = trigger.top - height - gap;
  }

  const maximumLeft = Math.max(horizontalPadding, viewport.width - width - horizontalPadding);
  const maximumTop = Math.max(horizontalPadding, viewport.height - height - horizontalPadding);

  return {
    left: Math.round(clamp(left, horizontalPadding, maximumLeft)),
    top: Math.round(clamp(top, horizontalPadding, maximumTop)),
    width: Math.round(width),
  };
}
