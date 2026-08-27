import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./components/sidebar.css', import.meta.url), 'utf8');
const theme = readFileSync(new URL('./theme.css', import.meta.url), 'utf8');
const sidebarComponent = readFileSync(new URL('../components/sidebar/Sidebar.svelte', import.meta.url), 'utf8');
const progressTrack = readFileSync(new URL('../components/sidebar/ProgressTrack.svelte', import.meta.url), 'utf8');

describe('sidebar status theme tokens', () => {
  it('keeps sidebar status presentation on semantic theme tokens', () => {
    expect(source).not.toMatch(/(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i);

    for (const token of [
      'var(--status-accent-text)',
      'var(--status-danger-bg)',
      'var(--status-danger-border)',
      'var(--status-success-bg)',
      'var(--status-warning-bg)',
      'var(--status-warning-text)',
      'var(--control-group-bg)',
      'var(--report-result-bg)',
      'var(--border-strong)',
    ]) {
      expect(source).toContain(token);
    }
  });

  it('keeps sidebar activity accents theme-aware', () => {
    expect(sidebarComponent).toContain('accent="var(--status-success-text)"');
    expect(sidebarComponent).not.toMatch(/accent="#[0-9a-f]{3,8}"/i);
  });

  it('keeps the concise sex badge readable across themes', () => {
    const sexPill = theme.match(/\.sex-pill \{[\s\S]*?\n\}/)?.[0] ?? '';
    expect(sexPill).toContain('background: var(--status-accent-bg);');
    expect(sexPill).toContain('border: 1px solid var(--status-accent-border);');
    expect(sexPill).toContain('color: var(--status-accent-text);');
    expect(sexPill).not.toMatch(/(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i);
  });

  it('centralizes sidebar progress semantics and presentation', () => {
    expect(sidebarComponent).toContain("import ProgressTrack from './ProgressTrack.svelte';");
    expect(sidebarComponent).not.toMatch(/style="width:/);
    expect(progressTrack).toContain('role="progressbar"');
    expect(progressTrack).toContain('aria-valuetext={valueText}');
    expect(progressTrack).toContain('--progress-width:');
  });

  it('keeps profile actions keyboard-visible and comfortably targetable', () => {
    expect(source).toContain('.sidebar .sample-name:focus-visible');
    expect(source).toContain('.sidebar .btn-delete:focus-visible');
    expect(source).toContain('.reset-dir-btn:focus-visible');
    expect(source).toContain('min-width: 2.25rem;');
    expect(source).toContain('min-height: 2.25rem;');
    expect(source).toContain('var(--focus-ring)');
  });
});
