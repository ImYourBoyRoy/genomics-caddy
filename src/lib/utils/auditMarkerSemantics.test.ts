import { execFileSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../..');
const script = path.join(root, 'scripts', 'audit_marker_semantics.mjs');

describe('marker semantics audit', () => {
  it('emits collision diagnostics without genotype fields', () => {
    const output = execFileSync(process.execPath, [script, '--json'], {
      cwd: root,
      encoding: 'utf8',
    });
    const summary = JSON.parse(output);
    expect(summary.marker_snapshot).toHaveLength(summary.marker_count);
    expect(summary.marker_snapshot[0]).toEqual(expect.objectContaining({
      marker_id: expect.any(String),
      pack: expect.any(String),
      effect_class: expect.any(String),
      callability_class: expect.any(String),
      scoring_allowed: expect.any(Boolean),
    }));
    expect(summary.duplicate_conflicts.length).toBeGreaterThan(0);
    expect(summary.duplicate_conflicts[0]).toEqual(expect.objectContaining({
      rsid: expect.any(String),
      fields: expect.any(Array),
      field_values: expect.any(Object),
      row_refs: expect.any(Array),
    }));
    expect(summary.orientation_audit).toEqual(expect.objectContaining({
      matchable_snp_rows: expect.any(Number),
      strand_ambiguous_rows: expect.any(Number),
      ambiguous_rows_missing_verification: expect.any(Array),
      matchable_rows_missing_source_build: expect.any(Array),
      matchable_rows_missing_expected_plus_alleles: expect.any(Array),
    }));
    expect(JSON.stringify(summary)).not.toContain('user_genotype');
    expect(JSON.stringify(summary)).not.toContain('normalized_genotype');
  });
});
