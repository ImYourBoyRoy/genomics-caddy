# 🔴 Red Team Audit — Genomics Caddy

Comprehensive audit of design, implementation, UX/UI, data quality, and feature completeness.

---

## 🚨 Category 1: Critical Bugs & Logic Errors

### C1.1 — `variant_type` field is silently dropped by Rust backend
**Severity: Medium** | Files: [report.rs](src-tauri/src/report.rs), all marker packs

The marker JSON packs include a `variant_type: "snp"` field on every marker, and [genomics.ts](src/lib/types/genomics.ts) defines a `VariantType` union. But `report.rs` **does not deserialize or serialize** `variant_type`. The Rust `MarkerDefinition` struct has no `variant_type` field, so serde silently ignores it. The frontend never receives it.

**Impact**: When you add non-SNP markers (star alleles, haplotypes, CNVs), the backend will not forward their type — blocking correct genotype matching logic.

**Fix**: Add `pub variant_type: Option<String>` to both `MarkerDefinition` and `EvaluatedMarker` in `report.rs`.

---

### C1.2 — Duplicate rsIDs across packs cause double-counting in scoring
**Severity: High** | Files: [metabolic.json](src/lib/marker-packs/metabolic.json), [cardiovascular.json](src/lib/marker-packs/cardiovascular.json)

5 rsIDs are duplicated across `metabolic` and `cardiovascular` packs:
- `rs708272` (CETP) — 2 sections
- `rs328` (LPL) — 2 sections
- `rs662799` (APOA5) — 2 sections
- `rs10455872` (LPA) — 2 sections
- `rs3798220` (LPA) — 2 sections

The same alleles are evaluated and counted **twice** — once in each section. This inflates the overall signal score (both `risk_possible` and `risk_effect_count` are double-counted).

**Fix**: Either deduplicate (put in one pack only), or add a `cross_ref: true` flag and exclude cross-refs from overall scoring.

---

### C1.3 — `any` types pervasive in +page.svelte state
**Severity: Medium** | File: [+page.svelte](src/routes/+page.svelte)

Multiple state variables use `any`:
```typescript
let samples = $state<any[]>([]);           // line 74
let selectedSample = $state<any>(null);     // line 75
let appPaths = $state<any>(null);           // line 78
let generatedReport = $state<any>(null);    // line 91
let browserResults = $state<any[]>([]);     // line 98
```

And the `PACKS_MAP` is typed as `Record<string, any>` (line 60).

**Impact**: TypeScript cannot catch property access errors at compile time. If the backend schema changes and drops a field, the frontend silently renders `undefined`.

**Fix**: Replace all `any` with proper types from `genomics.ts`. `PACKS_MAP` should be `Record<string, SectionDefinition>`.

---

### C1.4 — `delete_sample` relies on CASCADE but SQLite requires PRAGMA
**Severity: Low** | File: [db.rs](src-tauri/src/db.rs#L281-L284)

The `DELETE FROM samples` relies on `ON DELETE CASCADE` from the foreign key constraint. But SQLite does **not** enforce foreign keys by default — you must run `PRAGMA foreign_keys = ON` on every connection. Currently `init_user_db()` does not set this pragma.

**Impact**: Deleting a sample leaves orphaned genotype rows in the database, wasting storage.

**Fix**: Add `conn.execute_batch("PRAGMA foreign_keys = ON;")` after opening the connection.

---

## 🎨 Category 2: UX/UI Weaknesses

### U2.1 — GenomeMap is entirely fake
**Severity: High** | File: [GenomeMap.svelte](src/lib/components/genome/GenomeMap.svelte)

The chromosome map shows hardcoded fake bands and variant indicators only on chromosomes 1, 2, 11, 17, and X. It does not read any actual data. This is misleading — it looks like a data visualization but is pure decoration.

**Fix**: Either connect it to real imported data (query SNP density per chromosome from the database), or replace with a placeholder that clearly says "Coming Soon" rather than showing fake indicators.

---

### U2.2 — No error display when report generation fails
**Severity: High** | File: [+page.svelte](src/routes/+page.svelte#L227-L239)

```typescript
async function triggerReport() {
  ...
  } catch (e: any) {
    console.error(e);  // <-- silently logged to console
  }
```

If the Rust backend fails to generate a report (bad JSON, DB corruption, template mismatch), the user sees... nothing. No error message, no toast, no indication that something went wrong. The report area just stays empty.

**Fix**: Add a `reportError` state variable and display it prominently in the ReportView.

---

### U2.3 — No loading state for chain download
**Severity: Low** | File: [Sidebar.svelte](src/lib/components/sidebar/Sidebar.svelte#L72-L74)

The chain file download button disables and shows "Downloading..." but there is no progress indicator. The Ensembl chain file is ~800KB and downloads quickly, but on slow connections the user has no feedback.

---

### U2.4 — Delete confirmation uses `confirm()` instead of styled modal
**Severity: Low** | File: [+page.svelte](src/routes/+page.svelte#L242)

```typescript
if (!confirm("Are you sure you want to delete this sample...")) return;
```

Browser `confirm()` is ugly and blocks the event loop. Inconsistent with the premium dark design.

---

### U2.5 — No keyboard navigation for sample list
**Severity: Medium** | File: [SampleList.svelte](src/lib/components/samples/SampleList.svelte#L39-L46)

The sample items use `<span role="button" tabindex="0">` for click, but there are **3 a11y-ignore comments** suppressing warnings about missing keyboard handlers. Pressing Enter or Space on a focused sample does nothing.

**Fix**: Add `onkeydown` handler that triggers `onSelectSample` on Enter/Space.

---

### U2.6 — Sidebar has no collapsibility on smaller screens
**Severity: Medium** | File: [AppShell.svelte](src/lib/components/layout/AppShell.svelte)

The sidebar is a fixed 280px column with no responsive breakpoint. On screens < 900px (common for laptops in split-screen), the content area is cramped. There's no hamburger menu or collapse toggle.

---

### U2.7 — Tab navigation has no visual keyboard focus indicator
**Severity: Low** | File: [+page.svelte](src/routes/+page.svelte#L324-L329)

Tab buttons have no `:focus-visible` styling. Users navigating with keyboard can't see which tab is focused.

---

## 📊 Category 3: Data Quality Issues

### D3.1 — 5 cross-pack duplicate markers (see C1.2)
Already covered. 5 markers in both `metabolic` and `cardiovascular`.

### D3.2 — Evidence tier distribution is bottom-heavy
**Severity: Medium** | All marker packs

| Tier | Count | Percentage |
|---|---|---|
| A (Clinical Guideline) | 19 | 18% |
| B (Well-Studied) | 38 | 36% |
| C (Biohacker Hypothesis) | 40 | 38% |
| D (Research Only) | 8 | 8% |

38% of markers are Tier C ("Biohacker Hypothesis"). For a tool marketing itself as clinical-grade, this is a credibility risk. Users and AI models should be able to filter by tier.

**Fix**: Add a tier filter toggle in the report UI so users can hide Tier C/D markers.

---

### D3.3 — Only 4 protective markers out of 100
**Severity: Medium** | All marker packs

Only `PPARG`, `FAAH`, `LPL`, and `CETP` are marked `protective`. The protective category feels under-populated. Consider adding known protective variants like:
- `PCSK9 rs11591147` (protective against high LDL)
- `SLC30A8 rs13266634` (T2D protection)
- `IL28B/IFNL4 rs12979860` (hepatitis C clearance)

---

## ⚙️ Category 4: Feature Gaps

### F4.1 — No filtering / sorting in the report
**Severity: High**

Users cannot:
- Filter by severity class (show only risk findings)
- Filter by evidence tier (show only Tier A/B)
- Sort markers by severity
- Collapse/expand entire sections

For 100+ markers, scrolling through everything is overwhelming. The most important findings get lost.

**Fix**: Add filter bar above sections with toggle chips: "Show risk only", "Tier A/B only", "Hide not-detected".

---

### F4.2 — JSON export has no schema version or metadata
**Severity: Medium** | File: [ReportView.svelte](src/lib/components/report/ReportView.svelte#L37-L44)

The JSON export is just `JSON.stringify(generatedReport)`. It lacks:
- Schema version (for AI/MCP consumers to know what fields to expect)
- Export timestamp
- Sample metadata (name, sex, import date)
- Application version
- Marker pack versions

**Fix**: Wrap the report in a metadata envelope:
```json
{
  "schema_version": "1.0.0",
  "app_version": "0.1.0",
  "exported_at": "2026-06-07T20:00:00Z",
  "sample": { "name": "Roy", "genetic_sex": "XY (Male)" },
  "report": { ... }
}
```

---

### F4.3 — MCP server lacks a `generate_report` tool
**Severity: High** | File: [mcp.rs](src-tauri/src/mcp.rs)

The MCP server exposes only 3 tools: `list_samples`, `get_variants_by_rsid`, `get_variants_in_region`. It does **not** expose `generate_report`, meaning an AI model connected via MCP cannot generate a full trait report — it can only query raw genotypes.

**Fix**: Add a `generate_report` MCP tool that accepts `sample_id` and optional `marker_pack_ids`, runs the same evaluation logic, and returns the `GeneratedReport` JSON.

---

### F4.4 — MCP server lacks `notifications/initialized` response
**Severity: Low** | File: [mcp.rs](src-tauri/src/mcp.rs#L89-L104)

The MCP spec requires the server to handle `notifications/initialized` after the client sends it. Currently unmatched methods return a `-32601 Method not found` error, which some clients treat as a connection failure.

**Fix**: Add a no-op handler for `notifications/initialized`.

---

### F4.5 — No "About" or version info in the UI
**Severity: Low**

No version number, build date, or author attribution anywhere in the running application. The user has no way to know what version they're running.

---

## 🖨️ Category 5: Print / Export Issues

### P5.1 — Print CSS references stale class `.marker-signal-level`
**Severity: Low** | File: [print.css](src/lib/styles/print.css#L66)

```css
.marker-signal-level {
  color: black !important;
}
```

This class was removed in the scoring overhaul. It's dead CSS.

**Fix**: Replace with `.marker-severity-label`.

---

### P5.2 — Print CSS doesn't handle collapsed cards or legend
**Severity: Medium** | File: [print.css](src/lib/styles/print.css)

Missing print rules for:
- `.marker-card-collapsed` (should print at full opacity)
- `.severity-explainer` (needs dark-on-light colors)
- `.summary-pill` (needs visible borders on white background)
- `.score-explainer` (needs print-safe colors)
- `.report-legend` (currently hidden by `.no-print` — should it be included in PDF for context?)

---

### P5.3 — PDF export is just `window.print()` — not a real PDF generator
**Severity: Medium**

The "Export PDF" button calls `window.print()`, which opens the OS print dialog. The result depends on the user's printer settings, page size, and browser rendering. Content frequently cuts off or overlaps.

**Fix (future)**: Consider using a dedicated PDF library (e.g. `jspdf` + `html2canvas`, or Tauri's print API) for consistent output. For now, at minimum add a print-preview note explaining the limitation.

---

## 🧹 Category 6: Code Quality & Architecture

### Q6.1 — No error boundary in the Svelte app
**Severity: Medium**

If any component throws during rendering, the entire app crashes to a blank screen with no recovery path. Svelte 5 doesn't have built-in error boundaries.

**Fix**: Wrap the main content area in a try/catch rendering pattern or add an `{#if}` guard around the report view.

---

### Q6.2 — Database connection opened per-command (no pooling)
**Severity: Low** | File: [lib.rs](src-tauri/src/lib.rs)

Every Tauri command calls `db::init_user_db()` which opens a new SQLite connection, runs CREATE TABLE IF NOT EXISTS, and potentially runs ALTER TABLE migration. For a local app this works, but it's wasteful.

**Fix (future)**: Use `tauri::State<Mutex<Connection>>` to share a single connection.

---

### Q6.3 — No `#[serde(deny_unknown_fields)]` on Rust input structs
**Severity: Low** | File: [report.rs](src-tauri/src/report.rs)

Typos in marker JSON field names (e.g. `"efect_allele"`) are silently ignored by serde. This makes debugging data quality issues harder.

**Fix**: Add `#[serde(deny_unknown_fields)]` to `MarkerDefinition` and `SectionDefinition` to catch typos at parse time.

---

## 📋 Category 7: Summary Priority Matrix

| # | Finding | Severity | Effort | Priority |
|---|---|---|---|---|
| C1.2 | Duplicate rsIDs double-count scoring | 🔴 High | Low | **P0** |
| U2.1 | GenomeMap is entirely fake | 🔴 High | Medium | **P0** |
| U2.2 | No error display for report failures | 🔴 High | Low | **P0** |
| F4.1 | No filtering/sorting in report | 🔴 High | Medium | **P0** |
| F4.3 | MCP server lacks generate_report | 🔴 High | Medium | **P1** |
| C1.1 | variant_type silently dropped | 🟡 Medium | Low | **P1** |
| C1.3 | `any` types in +page.svelte | 🟡 Medium | Low | **P1** |
| F4.2 | JSON export lacks metadata | 🟡 Medium | Low | **P1** |
| D3.2 | 38% of markers are Tier C | 🟡 Medium | High | **P2** |
| U2.5 | No keyboard nav for samples | 🟡 Medium | Low | **P2** |
| U2.6 | Sidebar not responsive | 🟡 Medium | Medium | **P2** |
| P5.2 | Print CSS incomplete for new styles | 🟡 Medium | Low | **P2** |
| C1.4 | CASCADE requires PRAGMA | 🟢 Low | Trivial | **P2** |
| P5.1 | Stale class in print.css | 🟢 Low | Trivial | **P3** |
| U2.3 | No chain download progress | 🟢 Low | Low | **P3** |
| U2.4 | Browser confirm() vs modal | 🟢 Low | Medium | **P3** |
| U2.7 | No focus-visible on tabs | 🟢 Low | Trivial | **P3** |
| F4.4 | MCP missing initialized handler | 🟢 Low | Trivial | **P3** |
| F4.5 | No version info in UI | 🟢 Low | Trivial | **P3** |
| Q6.1 | No error boundary | 🟡 Medium | Medium | **P2** |
| Q6.2 | No DB connection pooling | 🟢 Low | Medium | **P3** |
| Q6.3 | No deny_unknown_fields | 🟢 Low | Trivial | **P3** |
| P5.3 | window.print() PDF | 🟡 Medium | High | **P3** |
| D3.3 | Only 4 protective markers | 🟡 Medium | High | **P3** |

---

## Recommended Next Sprint

**P0 fixes (immediate, high-impact):**
1. Fix duplicate rsID double-counting (deduplicate or add cross-ref exclusion)
2. Add report error display state
3. Replace fake GenomeMap with honest placeholder or real data
4. Add filter/sort controls to the report view

**P1 fixes (next):**
5. Add `generate_report` tool to MCP server
6. Add metadata envelope to JSON export
7. Fix `variant_type` passthrough in Rust
8. Type all `any` variables in +page.svelte
