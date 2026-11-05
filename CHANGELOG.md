# Changelog

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
- Add the Windows Common Controls manifest to Rust test binaries so native downloader tests start correctly.
