import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./components/sidebar.css', import.meta.url), 'utf8');
const theme = readFileSync(new URL('./theme.css', import.meta.url), 'utf8');
const sidebarComponent = readFileSync(new URL('../components/sidebar/Sidebar.svelte', import.meta.url), 'utf8');
const sampleList = readFileSync(new URL('../components/samples/SampleList.svelte', import.meta.url), 'utf8');
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
    expect(source).toContain('.samples-card .sample-name:focus-visible');
    expect(source).toContain('.samples-card .sample-name-text');
    expect(source).toContain('.samples-card .sample-sex-symbol');
    expect(source).toContain('.samples-card .btn-delete:focus-visible');
    expect(source).toContain('.samples-card .btn-delete svg');
    expect(source).toContain('.samples-card .samples-actions-note');
    expect(source).toContain('.reset-dir-btn:focus-visible');
    expect(source).toContain('min-width: 2.25rem;');
    expect(source).toContain('min-height: 2.25rem;');
    expect(source).toContain('var(--focus-ring)');
  });

  it('keeps profile-list presentation in the sidebar stylesheet', () => {
    for (const selector of [
      '.samples-card .sample-list',
      '.samples-card .sample-item',
      '.samples-card .sample-name-wrap',
      '.samples-card .sample-name',
      '.samples-card .sample-name-text',
    ]) {
      expect(source).toContain(selector);
    }
    expect(sampleList).toContain('class="sample-name-text"');
    expect(sampleList).toContain('class="sample-sex-symbol"');
    expect(sampleList).toContain('aria-label={`Delete profile ${s.name}`}');
    expect(sampleList).toContain('title={disabled ? disabledReason');
    expect(sampleList).toContain('<svg viewBox="0 0 20 20"');
    expect(sampleList).not.toContain('🗑️');
    expect(sampleList).toContain("label === 'Female' ? '♀' : label === 'Male' ? '♂' : '•'");
    expect(sampleList).not.toContain('Tooltip');
    expect(sampleList).not.toContain('sample-scope-help');

    expect(theme).not.toMatch(/\.sample-list\s*\{/);
    expect(theme).not.toMatch(/\.sample-item(?:\.active)?\s*\{/);
    expect(theme).not.toMatch(/\.sample-name(?:-wrap)?\s*\{/);
    expect(theme).not.toMatch(/\.sample-scope-help\s*\{/);
    expect(theme).not.toMatch(/\.btn-delete\s*\{/);
  });

  it('gives profile names a stable readable track beside the sex symbol and actions', () => {
    expect(source).toContain('grid-template-columns: minmax(0, 1fr) auto;');
    expect(source).toContain('grid-template-columns: 1.4rem minmax(0, 1fr);');
    expect(source).toContain('display: block;');
    expect(source).toContain('min-width: 0;');
  });

  it('uses semantic tokens for visible shell surfaces and scrollbars', () => {
    expect(theme).toContain('--app-gradient-highlight:');
    expect(theme).toContain('--sidebar-surface:');
    expect(theme).toContain('--scrollbar-track:');
    expect(theme).toContain('--scrollbar-thumb:');
    expect(theme).toContain('--scrollbar-thumb-hover:');
    expect(theme).toContain('background: var(--scrollbar-track);');
    expect(theme).toContain('background: var(--scrollbar-thumb);');
    expect(theme).toContain('background: var(--scrollbar-thumb-hover);');
    expect(theme).toContain('background: var(--bg-primary);');
    expect(theme).toContain('background-color: var(--sidebar-surface);');
  });

  it('keeps the advanced Connections launch surface compact', () => {
    expect(source).toContain('.connections-launch-card.card');
    expect(source).toContain('.connections-launch-copy');
    expect(source).toContain('.connections-launch-actions');
    expect(sidebarComponent).toContain('class="connections-launch-card card"');
    expect(sidebarComponent).toContain('Open Connections');
  });

  it('gives the desktop sidebar enough width for its action groups', () => {
    expect(theme).toContain('--sidebar-width: 320px;');
    expect(theme).toContain('--sidebar-width: 280px;');
    expect(source).toContain('.connections-launch-actions');
    expect(sidebarComponent).toContain('catalogs are processed in order while SQLite is occupied');
    expect(sidebarComponent).not.toContain('downloads can still run in parallel');
  });

  it('keeps profile work ahead of technical administration', () => {
    const profileWorkspace = sidebarComponent.indexOf('class="sidebar-profile-workspace"');
    const profileList = sidebarComponent.indexOf('<SampleList');
    const importPanel = sidebarComponent.indexOf('<GenomeImportPanel');
    const toolsBoundary = sidebarComponent.indexOf('>App tools</span>');

    expect(profileWorkspace).toBeGreaterThan(-1);
    expect(profileList).toBeGreaterThan(profileWorkspace);
    expect(importPanel).toBeGreaterThan(profileList);
    expect(toolsBoundary).toBeGreaterThan(importPanel);
    expect(sidebarComponent).not.toContain('exportDiscoveryFindings');
  });

  it('keeps a healthy Liftover status compact while preserving attention actions', () => {
    expect(source).toContain('.liftover-status-card.card');
    expect(source).toContain('.liftover-status-copy');
    expect(source).toContain('.liftover-status-actions');
    expect(sidebarComponent).toContain('class="chain-status-card card liftover-status-card"');
    expect(sidebarComponent).toContain('Download the chain to map imported coordinates to GRCh38.');
    expect(sidebarComponent).toContain('Update chain');
  });
});
