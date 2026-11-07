# Changelog

## 0.2.12 — 2026-11-07

### Fixed

- Keep the startup helix smooth: each strand node now runs one sampled transform animation, the glow is a gradient instead of a blurred shadow, and the strand is layout/paint contained so loading updates no longer make it stutter or freeze near 100%.
- Render the main workspace only after the startup overlay closes, so the report no longer paints behind the loading screen and stalls its animation.
- Resolve a Rust clippy error in offline catalog maintenance so the CI lint gate passes.

### Changed

- Offline reference-catalog sync, registry, and maintenance updates; marker-pack and discovery-catalog updates (pgx, discovery catalog) mirrored into the runtime resources; AI chat and research panel refinements; dependency and tooling updates.
- Added `CLAUDE.md` (defers to `AGENTS.md` and keeps it current) and an expanded `TASKS.md` backlog, including bring-your-own-AI, MCP hardening, and a proposed two-profile comparison with strict carrier/risk limits.

## 0.2.11 — 2026-11-04

### Added

- Expanded the digestive and skin marker packs with carefully qualified research context and clearer limits on what DNA can and cannot say.
- Added plain-language explanations for startup, report coverage, and download progress percentages.
- Added a first-run setup sequence that requires a valid liftover chain before DNA import or analysis and calls out the reference catalogs that still need downloading.
- Added Windows and macOS downloader contract checks to CI.

### Fixed

- Verify catalog checksums before replacing an existing file, preserving the last known-good copy when a new download fails validation.
- Ignore Cursor machine-local MCP and settings files that may contain access tokens.
- Stabilize the loading helix by synchronizing both strands on one animation timeline and removing forced per-dot compositor layers.
