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
    for (const name of ['gc-helix-orbit', 'gc-helix-rung', 'gc-helix-scan']) {
      const body = keyframes(name);
      expect(body).not.toMatch(/box-shadow|background|filter|width|height|left|top|translate:|scale:/);
    }
  });

  it('runs one sampled transform animation per node and isolates the strand', () => {
    expect(css).toContain('animation: gc-helix-orbit var(--helix-period) linear infinite;');
    expect(css).toContain('animation-delay: calc(var(--helix-row-phase) - var(--helix-period) / 2);');
    expect(css).toContain('contain: layout paint style;');
    expect(css).not.toContain('alternate');
    expect(css).not.toMatch(/box-shadow:/);
    expect(css).not.toContain('will-change: transform');
  });

  it('keeps a reduced-motion fallback', () => {
    expect(css).toContain('prefers-reduced-motion: reduce');
  });
});
