import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const css = readFileSync(new URL('./sequencing-helix.css', import.meta.url), 'utf8');

function keyframes(name: string): string {
  const start = css.indexOf(`@keyframes ${name}`);
  expect(start).toBeGreaterThan(-1);
  const end = css.indexOf('\n}\n', start);
  return css.slice(start, end);
}

describe('sequencing helix motion', () => {
  it('animates only compositor-friendly properties', () => {
    for (const name of ['gc-helix-sway', 'gc-helix-depth', 'gc-helix-rung', 'gc-helix-scan']) {
      const body = keyframes(name);
      expect(body).not.toMatch(/box-shadow|background|filter|width|height|left|top/);
    }
  });

  it('uses sine easing with alternating half-period cycles instead of piecewise-linear steps', () => {
    expect(css).toContain('cubic-bezier(0.37, 0, 0.63, 1)');
    expect(css).toContain('infinite alternate');
    expect(css).toContain('animation-direction: alternate-reverse');
    expect(css).not.toContain('12.5%');
  });

  it('keeps a reduced-motion fallback', () => {
    expect(css).toContain('prefers-reduced-motion: reduce');
  });
});
