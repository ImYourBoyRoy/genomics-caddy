import { describe, expect, it } from 'vitest';
import { calculateTooltipPosition, type TooltipRect } from './tooltipPosition';

const viewport = { width: 1000, height: 800 };

function trigger(left: number, top: number, width = 40, height = 32): TooltipRect {
  return {
    left,
    top,
    right: left + width,
    bottom: top + height,
    width,
    height,
  };
}

describe('calculateTooltipPosition', () => {
  it('flips a top placement below a trigger near the top edge', () => {
    expect(calculateTooltipPosition({
      trigger: trigger(480, 4),
      panel: { width: 280, height: 120 },
      viewport,
      placement: 'top',
    })).toEqual({ left: 360, top: 44, width: 280 });
  });

  it('flips a bottom placement above a trigger near the bottom edge', () => {
    expect(calculateTooltipPosition({
      trigger: trigger(480, 760),
      panel: { width: 280, height: 120 },
      viewport,
      placement: 'bottom',
    })).toEqual({ left: 360, top: 632, width: 280 });
  });

  it('clamps side placements on both horizontal and vertical edges', () => {
    expect(calculateTooltipPosition({
      trigger: trigger(2, 2),
      panel: { width: 280, height: 160 },
      viewport,
      placement: 'left',
    })).toEqual({ left: 8, top: 8, width: 280 });
    expect(calculateTooltipPosition({
      trigger: trigger(960, 680),
      panel: { width: 280, height: 160 },
      viewport,
      placement: 'right',
    })).toEqual({ left: 712, top: 616, width: 280 });
  });

  it('shrinks the panel to fit a narrow viewport with equal edge padding', () => {
    expect(calculateTooltipPosition({
      trigger: trigger(170, 60),
      panel: { width: 420, height: 120 },
      viewport: { width: 390, height: 844 },
      placement: 'top',
    })).toEqual({ left: 8, top: 100, width: 374 });
  });
});
