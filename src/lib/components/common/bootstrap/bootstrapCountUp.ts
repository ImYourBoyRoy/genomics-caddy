// ./src/lib/components/common/bootstrap/bootstrapCountUp.ts
/*
Purpose: Eased count-up helper for bootstrap stat cards.
*/

export function easeOutCubic(t: number): number {
  return 1 - Math.pow(1 - t, 3);
}

export function animateCountUp(
  target: number,
  durationMs: number,
  onUpdate: (value: number) => void
): Promise<void> {
  if (durationMs <= 0 || target === 0) {
    onUpdate(target);
    return Promise.resolve();
  }

  const prefersReduced =
    typeof window !== "undefined" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  if (prefersReduced) {
    onUpdate(target);
    return Promise.resolve();
  }

  return new Promise((resolve) => {
    const start = performance.now();
    const tick = (now: number) => {
      const t = Math.min(1, (now - start) / durationMs);
      onUpdate(Math.round(target * easeOutCubic(t)));
      if (t < 1) {
        requestAnimationFrame(tick);
      } else {
        onUpdate(target);
        resolve();
      }
    };
    requestAnimationFrame(tick);
  });
}
