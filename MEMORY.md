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
- `.gitignore` also covers VCF/BAM/FASTQ/PDF exports, root SQLite files, and
  runtime dirs under `src-tauri/App/Data/` except the tracked marker-pack mirror.
  Synthetic parser fixtures in `src-tauri/testdata/` are explicitly un-ignored.
- Preserve local report and planning files on disk; ignore them rather than
  deleting user data. Internal Codex checkpoint refs are local recovery state;
  never `git push --mirror` or push `refs/codex/*`.
- Author identity in README and Cargo.toml is intentional and public.

## Latest checkpoint

- Source privacy pass: completed `.gitignore` coverage, removed local machine
  paths from tracked docs/UI, tracked synthetic testdata, and replaced the
  README layout DNA-file entry with synthetic fixtures.
- `master` history was rewritten with git-filter-repo: `builds/`, personal
  report JSON, planning notes, and local usernames/paths are gone from
  reachable `master` objects. A full identifier scan of 453 `master` commits
  found no `v1x0r`, `Users/Roy`, or personal DNA filenames.
- Local Codex refs (`refs/codex/*`) still point at pre-rewrite recovery
  commits and must not be pushed. Push only `master` (`git push -u origin master`).
- Author identity in README and Cargo.toml is intentional and public.
- No remote is configured; no push was performed.
