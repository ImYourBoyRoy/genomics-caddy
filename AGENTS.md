# Genomics Caddy project instructions

## Boundary and purpose

This repository contains the local Tauri/Rust + Svelte DNA analysis tool. Stay
inside this repository unless a task identifies a concrete external dependency.
Raw DNA fixtures and local app data are private; do not print genotype values,
copy them into logs, or commit secrets.

## Resource workflow

- `src/lib/marker-packs/` is the curated source of marker packs and support
  resources. Never remove a marker pack for cleanup or synchronization work.
- Keep every curated pack synchronized with its mirror under
  `src-tauri/App/Data/marker-packs/`. Keep `discovery_catalog.json` mirrored as
  well because the research UI uses it directly.
- Treat common SNPs as probabilistic context, not diagnoses or certainties.
  Preserve evidence tiers, `do_not_claim`, `confirm_with`, raw-DNA limitations,
  actionability classes, and clinical-confirmation requirements.
- Keep `src/lib/marker-packs/research_taxonomy.json` as the shared source of
  truth for discovery category labels, keyword routing, pack links, and
  consultation-mode hints. The Svelte UI and Rust crossmap must consume this
  resource rather than maintaining parallel biomedical keyword lists.
- Reproductive and hormone resources must not infer anatomy, identity, current
  hormone levels, pregnancy, contraception, or adenomyosis from raw DNA.
  Route cycle symptoms, medication composition, and suspected pelvic disease to
  user-supplied context, labs, clinical history, and appropriate care.
- Research-found drafts may target only the protected `research_found` pack;
  curated packs are not valid draft-merge targets.

## Required verification

Before broad exploration, inspect Git status, this file, `README.md`, and
`MEMORY.md`, then read only the files relevant to the task. Preserve unrelated
dirty worktree changes.

For resource changes, run at minimum:

```text
npm run validate:packs
npm run audit:resources
npm run audit:dna-fixtures
npm test
npm run check
npm run build
npm run cargo:test
git diff --check
```

For report/sidebar or other desktop UI changes, also run `npm run audit:tauri-ui`
while the Tauri development window is running. This is the application-level
fixture check; the Svelte renderer URL and browser preview are supplemental only.

The fixture audit is read-only and must not emit or persist raw genotype values.
Report source/runtime drift, unverified external behavior, warnings, and
remaining coverage limits explicitly. Update `README.md` for externally
visible behavior and `MEMORY.md` with a concise handoff after meaningful work.
