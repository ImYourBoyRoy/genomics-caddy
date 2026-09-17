# Genomics Caddy project instructions

## Boundary and purpose

This repository contains the local Tauri/Rust + Svelte DNA analysis tool. Stay
inside this repository unless a task identifies a concrete external dependency.
Human and agent operator docs: root `README.md` (quick start), `docs/`, and
`ARCHITECTURE.md` (file map). Local session notes live in `MEMORY.md` and must
stay untracked.

Raw DNA fixtures and local app data are private; do not print genotype values,
copy them into logs, or commit secrets.

Generated release staging under repository-root `builds/` and `App/` is local
output and must remain untracked. Ignore app databases and SQLite sidecars in
any `App/Data/` tree. The curated runtime mirror at
`src-tauri/App/Data/marker-packs/` is source-managed and must remain tracked.

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
`ARCHITECTURE.md`. Read local `MEMORY.md` when it exists; do not commit it.
Then read only the files relevant to the task. Preserve unrelated dirty
worktree changes.

For resource changes, run at minimum:

```text
pnpm run validate:packs
pnpm run audit:markers
pnpm run audit:resources
pnpm run audit:dna-fixtures
pnpm test
pnpm run check
pnpm run build
pnpm run cargo:test
git diff --check
```

The dependency workflow is intentionally lock-free for local build testing:
direct package versions are recorded in `package.json`, but npm/pnpm and Cargo
lockfiles are not retained, and Rust validation/release commands must not use
`--locked`. Use `node ./scripts/pnpm_unlocked.mjs install` for frontend dependency resolution;
pnpm 12's resolver lock is kept in a temporary directory outside the repository
and removed on exit. Run `node ./scripts/audit_reproducibility.mjs` when changing
manifests or release workflow; it verifies the lock-free state and unlocked
Cargo metadata.

AI and clinician technical handoffs always include the exact raw calls used by
their findings; the deprecated profile export toggles must not be reintroduced
as suppression controls. Connected AI Chat includes raw calls for the findings
selected by its persisted context mode and must disclose the destination when
the Ollama endpoint is remote. Session context mode is persisted with chat
history so reopening a session cannot silently change its genotype scope.

GRCh37 and explicit GRCh38 imports are supported under the parser's
1-based-inclusive contract. GRCh37 sources map forward when the local chain is
available; GRCh38 sources retain their source coordinates and use validated
inverse mapping for GRCh37 when available, leaving unmapped values explicit.

For report/sidebar or other desktop UI changes, also run `pnpm run audit:tauri-ui`
while the Tauri development window is running with `GENOMICS_AGENT_UI_TOKEN` set;
pass the same token to the audit command. This is the application-level fixture
check; the Svelte renderer URL and browser preview are supplemental only. The
debug bridge requires that token for `/ui/*`; use
`GENOMICS_AGENT_UI_ALLOW_UNAUTHENTICATED=1` only for disposable debug QA.

The fixture audit is read-only and must not emit or persist raw genotype values.
Report source/runtime drift, unverified external behavior, warnings, and
remaining coverage limits explicitly. Update `README.md` for externally
visible behavior. After file-layout changes, regenerate `ARCHITECTURE.md`.
Write a concise handoff to local `MEMORY.md` after meaningful work.
GitHub pull requests and `master` pushes must pass `.github/workflows/ci.yml`.
Tagged `v*` publishes reuse that gate, then draft signed desktop installers
and `latest.json` for in-app updates. Never commit the updater private key;
GitHub secrets `TAURI_SIGNING_PRIVATE_KEY` and
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` sign release artifacts.
