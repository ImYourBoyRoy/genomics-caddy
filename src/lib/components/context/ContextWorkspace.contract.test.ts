import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const contextSource = readFileSync(new URL('./ContextPanel.svelte', import.meta.url), 'utf8');
const contextStyles = readFileSync(new URL('../../styles/components/context-panel.css', import.meta.url), 'utf8');
const diarySource = readFileSync(new URL('./DiaryPanel.svelte', import.meta.url), 'utf8');
const pageSource = readFileSync(new URL('../../../routes/+page.svelte', import.meta.url), 'utf8');

describe('profile context workspaces', () => {
  it('exposes profile-scoped context categories without putting them in the report', () => {
    for (const label of [
      'Goals',
      'Challenges and symptoms',
      'Medications',
      'Supplements and OTC products',
      'Allergies and intolerances',
      'Diet and exclusions',
      'Supportive tests or clinician findings',
      'Reproductive, hormone, and care context',
    ]) {
      expect(contextSource).toContain(label);
    }
    expect(contextSource).toContain('saveProfileContext(selectedSample.id, profileContext)');
    expect(contextSource).toContain('selectedSample.id');
  });

  it('keeps Diary optional and uses the resource-defined cycle editor', () => {
    expect(diarySource).toContain('Optional local tracker');
    expect(diarySource).toContain('cycleDiaryAppliesToContext');
    expect(diarySource).toContain('CycleDiaryEditor');
    expect(diarySource).toContain('bind:personalSafetyContext={profileContext.safety}');
    expect(diarySource).toContain('saveProfileContext(selectedSample.id, profileContext)');
  });

  it('registers Context and Diary as primary tabs and renders both workspaces', () => {
    expect(pageSource).toContain('{ id: "context", label: "Context" }');
    expect(pageSource).toContain('{ id: "diary", label: "Diary" }');
    expect(pageSource).toContain('activeTab === "context"');
    expect(pageSource).toContain('activeTab === "diary"');
  });

  it('keeps context field chrome in an external stylesheet that cannot overlap', () => {
    expect(contextSource).toContain("import '$lib/styles/components/context-panel.css'");
    expect(pageSource).toContain('import "$lib/styles/components/context-panel.css"');
    expect(contextSource).not.toContain('<style>');
    expect(contextStyles).toContain('grid-template-columns: repeat(2, minmax(0, 1fr));');
    expect(contextStyles).toContain('min-width: 0;');
    expect(contextStyles).toContain('background: var(--surface-inset);');
    expect(contextStyles).toContain('field-sizing: fixed;');
    expect(contextStyles).not.toContain('repeat(3,');
  });
});
