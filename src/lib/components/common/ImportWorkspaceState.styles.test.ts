import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const workspaceSource = readFileSync(
  resolve(process.cwd(), 'src/lib/components/common/ImportWorkspaceState.svelte'),
  'utf8',
);
const pageSource = readFileSync(resolve(process.cwd(), 'src/routes/+page.svelte'), 'utf8');
const importSource = readFileSync(resolve(process.cwd(), 'src/lib/utils/pageSampleHandlers.ts'), 'utf8');
const themeSource = readFileSync(resolve(process.cwd(), 'src/lib/styles/theme.css'), 'utf8');

describe('DNA import review workspace', () => {
  it('keeps explicit import and cancel actions in the review screen', () => {
    expect(workspaceSource).toContain('onConfirmImport');
    expect(workspaceSource).toContain('onCancelImport');
    expect(workspaceSource).toContain('Import profile');
    expect(workspaceSource).toContain('Replace profile');
    expect(importSource).toContain('onConfirmationRequired(() => startImport(existing?.id), existing !== undefined)');
    expect(pageSource).toContain('onConfirmationRequired: (onConfirm, replacingExisting) =>');
    expect(pageSource).toContain('onConfirmImport={confirmImportReview}');
    expect(pageSource).toContain('onCancelImport={cancelImportReview}');
  });

  it('offers a report handoff when import and report preparation have completed', () => {
    const handoff = pageSource.match(/function continueToImportedReport\(\) \{([\s\S]*?)\n  \}/)?.[1] ?? '';
    expect(workspaceSource).toContain('{:else if phase === "ready"}');
    expect(workspaceSource).toContain('Continue to report');
    expect(workspaceSource).toContain('onclick={onContinueToReport}');
    expect(handoff).toContain('isImporting = false;');
    expect(handoff).toContain('importPhase = "idle";');
    expect(pageSource).toContain('onContinueToReport={continueToImportedReport}');
  });

  it('explains accepted and skipped DNA rows before confirmation', () => {
    expect(workspaceSource).toContain('Accepted records');
    expect(workspaceSource).toContain('Not imported');
    expect(workspaceSource).toContain('malformed');
    expect(workspaceSource).toContain('duplicates');
    expect(workspaceSource).toContain('coordinate_system');
    expect(workspaceSource).toContain('preview.diagnostics.warnings');
  });

  it('keeps the review screen and its progress design in the native static stylesheet', () => {
    expect(themeSource).toContain('.main-content .import-workspace-state');
    expect(themeSource).toContain('.main-content .import-workspace-actions');
    expect(themeSource).toContain('.main-content .import-step-timeline');
    expect(themeSource).toContain('.main-content .import-workspace-record-summary');
    expect(pageSource).toContain('{#if isImportPreparing || isImporting || importPhase === "awaiting-confirmation"}');
  });
});
