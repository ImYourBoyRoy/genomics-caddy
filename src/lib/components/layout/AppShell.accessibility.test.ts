import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const appShell = readFileSync(new URL('./AppShell.svelte', import.meta.url), 'utf8');
const sidebar = readFileSync(new URL('../sidebar/Sidebar.svelte', import.meta.url), 'utf8');
const theme = readFileSync(new URL('../../styles/theme.css', import.meta.url), 'utf8');

describe('responsive data-sidebar access', () => {
  it('gives the narrow drawer a keyboard and assistive-technology control contract', () => {
    expect(appShell).toContain('aria-controls="data-sidebar"');
    expect(appShell).toContain('aria-expanded={mobileSidebarOpen}');
    expect(appShell).toContain('aria-label={mobileSidebarOpen ? \'Close data controls\' : \'Open data controls\'}');
    expect(appShell).toContain('aria-hidden={focusMode || (isNarrowViewport && !mobileSidebarOpen) ? \'true\' : undefined}');
    expect(appShell).toContain('inert={focusMode || (isNarrowViewport && !mobileSidebarOpen) ? true : undefined}');
    expect(appShell).toContain('class:mobile-sidebar-open={mobileSidebarOpen}');
    expect(appShell).toContain('if (event.key === \'Escape\')');
    expect(appShell).toContain('event.key !== \'Tab\'');
    expect(appShell).toContain('last.focus();');
    expect(appShell).toContain('first.focus();');
    expect(appShell).toContain('class="mobile-sidebar-backdrop no-print"');
    expect(appShell).toContain('aria-hidden="true"');
    expect(appShell).toContain('tabindex="-1"');
  });

  it('provides a stable sidebar target and narrow drawer styles', () => {
    expect(sidebar).toContain('id="data-sidebar"');
    expect(theme).toContain('.sidebar-slot {');
    expect(theme).toContain('.sidebar-slot[aria-hidden="true"] {');
    expect(theme).toContain('.app-layout.mobile-sidebar-open .sidebar {');
    expect(theme).toContain('.mobile-sidebar-toggle {');
    expect(theme).toContain('.mobile-sidebar-backdrop {');
    expect(theme).toContain('min-height: 44px;');
  });

  it('keeps the hide control at the sidebar edge and appearance controls in the footer', () => {
    expect(appShell).toContain('<div class="app-shell-stack">');
    expect(appShell).toContain('<AppUpdateHost />');
    expect(appShell).toContain('<div class="main-slot">');
    expect(appShell).toContain('<div class="sidebar-region">');
    expect(sidebar).toContain('<div class="sidebar-footer no-print" role="toolbar" aria-label="Library and appearance">');
    expect(appShell).toContain('class="sidebar-focus-toggle"');
    expect(appShell).toContain('aria-label="Hide data sidebar"');
    expect(appShell).toContain('class="focus-restore-toggle no-print"');
    expect(sidebar).toContain('<ThemeToggle />');
    expect(sidebar).toContain('<LibraryPanel />');
    expect(appShell).not.toContain('class="focus-toolbar');
    expect(theme).toContain('.sidebar-region {');
    expect(theme).toContain('.sidebar-footer {');
    expect(theme).toContain('right: 0.7rem;');
    expect(theme).toContain('box-shadow: none;');
    expect(theme).toContain('.focus-restore-toggle {');
    expect(theme).toContain('position: absolute;');
  });
});
