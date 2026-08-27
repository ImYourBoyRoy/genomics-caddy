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
    expect(appShell).toContain('aria-label="Close data controls"');
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
});
