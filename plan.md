# Plan: Genomics Caddy Enhancements

Tracking checklist for the Genomics Caddy enhancements (File Pickers, ZIP files, Progress Bars, and expanded AuDHD/Health profiling).

- [x] **Phase 1: Dependency Setup**
  - [x] Add `rfd` and `zip` to `Cargo.toml`
  - [x] Run `cargo check` to verify compilation
- [x] **Phase 2: Ingestion & Backend Enhancements**
  - [x] Update `parser.rs` to support zip parsing in memory
  - [x] Update `db.rs` to support chromosome Y density checks for sex determination
  - [x] Update `lib.rs` to add `select_file` command, progress events, and path exposure commands
- [x] **Phase 3: Frontend Ingestion & Progress UI**
  - [x] Add "Browse..." file button linking to native dialog
  - [x] Add a progress bar component with status updates (Unpacking, Parsing, Liftover, SQL Insertion)
  - [x] Display DB and Chain storage path details
- [x] **Phase 4: Curation & Diagnostics Summary**
  - [x] Expand `markers.json` to cover AuDHD, cognitive performance, MTHFR, APOE, and BRCA1 cancer markers
  - [x] Build a Profile Summary block showing Genetic Sex, key metrics, and clinical disclaimers
  - [x] Verify execution and compile warning-free (0 errors, 0 warnings compile checks passed)
