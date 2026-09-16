# Project Memory

## Snapshot

- Genomics Caddy is a local-first Tauri 2 application with a Rust backend and
  Svelte frontend.
- Curated marker-pack sources live in `src/lib/marker-packs/`; keep required
  runtime mirrors under `src-tauri/App/Data/marker-packs/` synchronized.
- Raw user DNA, profile databases, generated reports, and release builds are
  local-only. Test fixtures committed to the repository must be synthetic and
  must not contain user identifiers or calls.
- GRCh37 imports use the documented 1-based-inclusive contract; mapping to
  GRCh38 is local and unmapped coordinates remain explicit.

## Validation workflow

- Resolve dependencies with `node ./scripts/pnpm_unlocked.mjs install`.
- Resource checks: `pnpm run validate:packs`, `pnpm run audit:markers`,
  `pnpm run audit:resources`, and `pnpm run audit:dna-fixtures`.
- Application checks: `pnpm test`, `pnpm run check`, `pnpm run build`, and
  `pnpm run cargo:test`.
- `node ./scripts/audit_reproducibility.mjs` verifies the intentional lock-free
  dependency workflow.
- The native UI fixture audit requires a running Tauri development window and
  `GENOMICS_AGENT_UI_TOKEN`; browser previews are supplemental only.

## Privacy and Git history

- Keep generated `builds/`, portable `App/` output, local databases, profile
  exports, dated tasklists, strategy notes, and detailed local memory out of
  tracked history.
- A follow-up privacy audit found one tracked personal DNA report, a user-specific
  planning document, private build-network defaults, and identity strings in
  earlier commits. These are being removed from publishable `master` history;
  do not push until the full reachable-history scan passes.
- Preserve local report and planning files on disk; ignore them rather than
  deleting user data. Internal Codex checkpoint refs are local recovery state;
  do not mirror-push them.

## Latest checkpoint

- Local checkpoint commits: `Checkpoint import, reporting, and privacy improvements`
  and `Checkpoint public history privacy scrub`; a further privacy checkpoint
  is pending after source and history remediation.
- No remote is configured; no push was performed.
- Recent validation passed: resource audits, 587 frontend tests, Svelte check,
  frontend build, 121 Rust tests, reproducibility audit, and Git whitespace
  checks. The native Tauri UI audit was not run because its UI token was absent.
