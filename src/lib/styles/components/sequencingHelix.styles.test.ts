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
    for (const name of ['gc-helix-path', 'gc-helix-rung', 'gc-helix-scan']) {
      const body = keyframes(name);
      expect(body).not.toMatch(/box-shadow|background|filter|width|height|left|top/);
    }
  });

  it('keeps both strands on one sine-eased timeline with a half-cycle offset', () => {
    expect(css).toContain('cubic-bezier(0.37, 0, 0.63, 1)');
    expect(css).toContain('animation-timing-function: var(--helix-sine)');
    expect(css).toContain('animation: gc-helix-path var(--helix-period) linear infinite;');
    expect(css).toContain('animation-delay: calc(var(--helix-row-phase) - var(--helix-period) / 2);');
    expect(css).not.toContain('alternate-reverse');
    expect(css).not.toContain('will-change: translate');
    expect(css).not.toContain('will-change: scale');
  });

  it('keeps a reduced-motion fallback', () => {
    expect(css).toContain('prefers-reduced-motion: reduce');
  });
});
