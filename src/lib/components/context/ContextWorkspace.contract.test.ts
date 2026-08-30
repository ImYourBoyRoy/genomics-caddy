import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const contextSource = readFileSync(new URL('./ContextPanel.svelte', import.meta.url), 'utf8');
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
});
