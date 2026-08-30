import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./ClinicalFindingsTable.svelte', import.meta.url), 'utf8');

describe('ClinicalFindingsTable copy', () => {
  it('keeps repeated next-step guidance concise', () => {
    expect(source).toContain("import { formatClinicalFollowUp } from '../../utils/clinicalPresentation';");
    expect(source).toContain('const followUp = formatClinicalFollowUp(marker.confirm_with);');
    expect(source).toContain('if (followUp) return followUp;');
    expect(source).toContain("return 'Review need for clinical confirmation.'");
    expect(source).toContain("return 'Interpret with history and current guidance.'");
    expect(source).toContain("return 'Confirmation needed';");
    expect(source).toContain("return 'Contextual result';");
    expect(source).toContain("return 'Review blocked';");
    expect(source).not.toContain('Discuss medical-grade confirmation before making health decisions.');
    expect(source).not.toContain("return 'Review the listed follow-up.'");
  });

  it('keeps the table caption structural instead of repeating the global reminder', () => {
    expect(source).toContain('<caption>Structured findings for clinical review.</caption>');
    expect(source).not.toContain('Consumer-array calls are not diagnostic.');
    expect(source).not.toContain('getClaimFrame(');
  });

  it('stacks the clinical table before the sidebar-constrained widths become cramped', () => {
    expect(source).toContain('@media (max-width: 1400px)');
    expect(source).toContain('.clinical-findings-table {\n      display: block;\n      width: 100%;\n      min-width: 0;');
    expect(source).toContain('.clinical-findings-table thead {');
  });

  it('lets expanded technical details fit inside narrow stacked rows', () => {
    expect(source).toContain('.clinical-details {\n      min-width: 0;\n      max-width: 100%;');
    expect(source).toContain('.clinical-details dl {\n      min-width: 0;\n      max-width: 100%;');
    expect(source).toContain('grid-template-columns: minmax(6rem, 0.45fr) minmax(0, 1fr);');
  });

  it('uses real mobile field labels instead of CSS-only pseudo content', () => {
    expect(source).toContain('<span class="clinical-mobile-label">Finding</span>');
    expect(source).toContain('<span class="clinical-mobile-label">DNA result</span>');
    expect(source).toContain('<span class="clinical-mobile-label">Next helpful step</span>');
    expect(source).toContain('.clinical-mobile-label {\n    display: none;');
    expect(source).toContain('.clinical-findings-table tbody td::before {\n      content: none;');
  });

  it('keeps repeated report-level provenance out of each finding table', () => {
    expect(source).not.toContain('Data sources in this report');
    expect(source).not.toContain('Consumer DNA array');
    expect(source).not.toContain('Self-reported context');
    expect(source).not.toContain('Clinician-entered information');
    expect(source).not.toContain('.clinical-provenance');
  });

  it('groups exact repeated clinical interpretations once without dropping the row reference', () => {
    expect(source).toContain('Shared clinical context ({sharedInterpretations.length})');
    expect(source).toContain('These explanations apply to more than one finding in this section and are shown once');
    expect(source).toContain('Shared clinical context shown above.');
    expect(source).toContain('sharedInterpretationKeys.has(normalizeSharedCopy(marker.interpretation))');
  });

  it('keeps the clinical table markup structurally balanced', () => {
    expect(source.match(/<tbody>/g)).toHaveLength(1);
    expect(source.match(/<\/tbody>/g)).toHaveLength(1);
  });
});
