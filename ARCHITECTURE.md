# Architecture

Inventory of **non-gitignored** toolkit files (git-tracked plus local
untracked files that are not ignored). Local-only paths are omitted:
`MEMORY.md`, raw DNA, reports, `.env`, `App/`, `builds/`, and other gitignored data.

- Files: **652**
- Total size: **23.0 MB** (24,145,948 bytes)
- Text lines (non-binary): **273,368**
- Directories: **70**

Sizes are on-disk bytes. Line counts are newline-based. Binary icons show `—`.
Use `docs/` for how to run the app. Regenerate with `python3 ./scripts/generate_architecture.py`.

## Directories

- [`(repository root)`](#repository-root) — 15 files, 109.5 KB
- [`.cursor/skills/connections-vector-providers`](#.cursor-skills-connections-vector-providers) — 2 files, 7.2 KB
- [`.cursor/skills/linux-portable-ui-refresh`](#.cursor-skills-linux-portable-ui-refresh) — 1 files, 4.0 KB
- [`.cursor/skills/vector-research-sweep-qa`](#.cursor-skills-vector-research-sweep-qa) — 1 files, 5.3 KB
- [`.github/workflows`](#.github-workflows) — 2 files, 5.7 KB
- [`.vscode`](#.vscode) — 2 files, 168 B
- [`docker`](#docker) — 4 files, 5.9 KB
- [`docs`](#docs) — 1 files, 1.2 KB
- [`docs/ai-chat`](#docs-ai-chat) — 1 files, 2.5 KB
- [`docs/build`](#docs-build) — 1 files, 3.3 KB
- [`docs/development`](#docs-development) — 1 files, 4.3 KB
- [`docs/mcp`](#docs-mcp) — 1 files, 5.1 KB
- [`docs/usage`](#docs-usage) — 1 files, 3.5 KB
- [`reference_files`](#reference_files) — 3 files, 7.4 MB
- [`scripts`](#scripts) — 40 files, 400.6 KB
- [`scripts/remote_build`](#scripts-remote_build) — 10 files, 55.5 KB
- [`src`](#src) — 1 files, 817 B
- [`src-tauri`](#src-tauri) — 5 files, 3.7 KB
- [`src-tauri/App/Data/marker-packs`](#src-tauri-app-data-marker-packs) — 25 files, 2.3 MB
- [`src-tauri/capabilities`](#src-tauri-capabilities) — 1 files, 284 B
- [`src-tauri/dev-bins`](#src-tauri-dev-bins) — 1 files, 670 B
- [`src-tauri/dev-bins/src`](#src-tauri-dev-bins-src) — 2 files, 3.0 KB
- [`src-tauri/icons`](#src-tauri-icons) — 17 files, 3.0 MB
- [`src-tauri/icons/android/mipmap-anydpi-v26`](#src-tauri-icons-android-mipmap-anydpi-v26) — 1 files, 261 B
- [`src-tauri/icons/android/mipmap-hdpi`](#src-tauri-icons-android-mipmap-hdpi) — 3 files, 49.5 KB
- [`src-tauri/icons/android/mipmap-mdpi`](#src-tauri-icons-android-mipmap-mdpi) — 3 files, 29.0 KB
- [`src-tauri/icons/android/mipmap-xhdpi`](#src-tauri-icons-android-mipmap-xhdpi) — 3 files, 93.1 KB
- [`src-tauri/icons/android/mipmap-xxhdpi`](#src-tauri-icons-android-mipmap-xxhdpi) — 3 files, 189.6 KB
- [`src-tauri/icons/android/mipmap-xxxhdpi`](#src-tauri-icons-android-mipmap-xxxhdpi) — 3 files, 321.1 KB
- [`src-tauri/icons/android/values`](#src-tauri-icons-android-values) — 1 files, 115 B
- [`src-tauri/icons/ios`](#src-tauri-icons-ios) — 18 files, 1.4 MB
- [`src-tauri/linux`](#src-tauri-linux) — 2 files, 949 B
- [`src-tauri/src`](#src-tauri-src) — 19 files, 629.3 KB
- [`src-tauri/src/bin`](#src-tauri-src-bin) — 2 files, 0 B
- [`src-tauri/src/offline`](#src-tauri-src-offline) — 16 files, 249.2 KB
- [`src-tauri/src/research`](#src-tauri-src-research) — 25 files, 397.1 KB
- [`src-tauri/src/research/evidence`](#src-tauri-src-research-evidence) — 22 files, 231.9 KB
- [`src-tauri/src/research/gnomad`](#src-tauri-src-research-gnomad) — 16 files, 145.1 KB
- [`src-tauri/src/research/vector_store`](#src-tauri-src-research-vector_store) — 4 files, 68.6 KB
- [`src-tauri/testdata`](#src-tauri-testdata) — 4 files, 724 B
- [`src/lib/api`](#src-lib-api) — 1 files, 33.9 KB
- [`src/lib/components/agent`](#src-lib-components-agent) — 7 files, 66.2 KB
- [`src/lib/components/ai`](#src-lib-components-ai) — 16 files, 120.0 KB
- [`src/lib/components/ai/evidence`](#src-lib-components-ai-evidence) — 20 files, 104.2 KB
- [`src/lib/components/ai/settings`](#src-lib-components-ai-settings) — 8 files, 59.9 KB
- [`src/lib/components/common`](#src-lib-components-common) — 16 files, 83.2 KB
- [`src/lib/components/common/bootstrap`](#src-lib-components-common-bootstrap) — 9 files, 27.1 KB
- [`src/lib/components/common/loading`](#src-lib-components-common-loading) — 3 files, 5.4 KB
- [`src/lib/components/context`](#src-lib-components-context) — 3 files, 25.4 KB
- [`src/lib/components/discovery`](#src-lib-components-discovery) — 1 files, 12.5 KB
- [`src/lib/components/genome`](#src-lib-components-genome) — 2 files, 24.5 KB
- [`src/lib/components/import`](#src-lib-components-import) — 1 files, 3.6 KB
- [`src/lib/components/layout`](#src-lib-components-layout) — 2 files, 9.0 KB
- [`src/lib/components/legal`](#src-lib-components-legal) — 2 files, 5.2 KB
- [`src/lib/components/mcp`](#src-lib-components-mcp) — 3 files, 26.2 KB
- [`src/lib/components/report`](#src-lib-components-report) — 27 files, 309.5 KB
- [`src/lib/components/research`](#src-lib-components-research) — 21 files, 173.7 KB
- [`src/lib/components/samples`](#src-lib-components-samples) — 1 files, 3.1 KB
- [`src/lib/components/search`](#src-lib-components-search) — 2 files, 20.6 KB
- [`src/lib/components/settings`](#src-lib-components-settings) — 2 files, 30.2 KB
- [`src/lib/components/sidebar`](#src-lib-components-sidebar) — 3 files, 71.3 KB
- [`src/lib/constants`](#src-lib-constants) — 3 files, 2.4 KB
- [`src/lib/marker-packs`](#src-lib-marker-packs) — 53 files, 3.1 MB
- [`src/lib/research`](#src-lib-research) — 4 files, 13.8 KB
- [`src/lib/styles`](#src-lib-styles) — 8 files, 170.6 KB
- [`src/lib/styles/components`](#src-lib-styles-components) — 20 files, 148.8 KB
- [`src/lib/types`](#src-lib-types) — 4 files, 43.0 KB
- [`src/lib/utils`](#src-lib-utils) — 120 files, 862.2 KB
- [`src/routes`](#src-routes) — 2 files, 38.3 KB
- [`static`](#static) — 5 files, 353.0 KB

## `(repository root)`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `.env.example` | 1.9 KB | 78 | Documented env keys; copy to local `.env`, never commit secrets. |
| `.gitignore` | 1.9 KB | 102 | Ignore rules for DNA, reports, builds, env, and local memory. |
| `.npmrc` | 258 B | 5 | pnpm peer-dependency compatibility settings. |
| `AGENTS.md` | 5.0 KB | 98 | Repo-local agent rules: packs, privacy, and validation gates. |
| `ARCHITECTURE.md` | 75.1 KB | 1,088 | Non-gitignored file map with sizes, line counts, and one-line roles. |
| `package.json` | 4.6 KB | 97 | Frontend package manifest and npm/pnpm scripts. |
| `paths.md` | 9.7 KB | 168 | Internal research-agent pathway map. |
| `plan.md` | 1.2 KB | 19 | Legacy enhancement checklist kept in the tree. |
| `pnpm-workspace.yaml` | 41 B | 2 | pnpm workspace definition. |
| `README.md` | 3.2 KB | 89 | GitHub-facing product page, warning, and quick start. |
| `svelte.config.js` | 885 B | 24 | SvelteKit adapter and preprocessor config. |
| `tsconfig.json` | 694 B | 19 | TypeScript compiler options for the Svelte app. |
| `vite.config.js` | 1.6 KB | 58 | Vite bundler config for the Tauri webview. |
| `vitest.config.ts` | 164 B | 8 | Vitest test runner config. |
| `walkthrough.md` | 3.3 KB | 31 | Internal notes on config loading and sweep UI safeguards. |

## `.cursor/skills/connections-vector-providers`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `reference.md` | 2.6 KB | 59 | Vector-store adapter dispatch notes for Connections providers. |
| `SKILL.md` | 4.5 KB | 92 | Genomics Caddy Advanced → Connections and multi-provider vector DB conventions (Qdrant, Pinecone, Chroma, Weaviate), Ollama model install/update-all, and research dense-path…. |

## `.cursor/skills/linux-portable-ui-refresh`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `SKILL.md` | 4.0 KB | 100 | Genomics Caddy Linux portable UI refresh: refresh the matching FreeDesktop launcher and DNA icon, clear WebKit/CacheStorage/GPU caches, verify splash/report assets, and relaunch…. |

## `.cursor/skills/vector-research-sweep-qa`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `SKILL.md` | 5.3 KB | 118 | Genomics Caddy Vector Research sweep QA: start/pause/resume/cancel races, agent UI bridge (127.0.0.1:17321), SQLite research_jobs assertions, and known cancel wind-down pitfalls. |

## `.github/workflows`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `ci.yml` | 2.4 KB | 97 | PR and master Validate workflow, including updater-config audit. |
| `release.yml` | 3.4 KB | 106 | Tag publish: signed installers and latest.json for in-app updates. |

## `.vscode`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `extensions.json` | 127 B | 7 | Recommended VS Code / Cursor extensions. |
| `settings.json` | 41 B | 3 | Shared editor settings for this workspace. |

## `docker`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `docker-compose.yml` | 1.6 KB | 53 | Compose service for the sweep worker. |
| `Dockerfile` | 1.6 KB | 45 | Image for the headless sweep worker. |
| `nginx.conf` | 489 B | 19 | Optional nginx config for a static UI container. |
| `README.md` | 2.2 KB | 62 | Headless sweep worker and optional static UI container. |

## `docs`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `README.md` | 1.2 KB | 22 | Index of operator docs for humans and agents. |

## `docs/ai-chat`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `README.md` | 2.5 KB | 72 | Ollama, vector connections, chat, and evidence workbench. |

## `docs/build`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `README.md` | 3.3 KB | 113 | Release builds, Linux launcher, remote compile, Docker. |

## `docs/development`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `README.md` | 4.3 KB | 125 | Tests, audits, layout, and lock-free install. |

## `docs/mcp`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `README.md` | 5.1 KB | 168 | MCP server flags, client configs, and tool catalog. |

## `docs/usage`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `README.md` | 3.5 KB | 89 | How to import DNA, read reports, and share exports. |

## `reference_files`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `Logo.clean.png` | 1.7 MB | — | Source brand-logo master. |
| `Logo.png` | 1.6 MB | — | Source brand-logo master. |
| `Logo.svg` | 4.2 MB | 8,799 | Source brand-logo master. |

## `scripts`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `audit_content_quality.mjs` | 6.4 KB | 101 | Privacy-safe aggregate audit for report-content quality. |
| `audit_dna_fixtures.py` | 10.1 KB | 288 | Read-only audit of local DNA fixtures against curated marker coverage. |
| `audit_inflammation.mjs` | 8.1 KB | 187 | Audit the inflammation support layer. |
| `audit_marker_semantics.mjs` | 11.1 KB | 242 | Read-only audit of curated marker assertion semantics. |
| `audit_mcp_runtime.mjs` | 13.8 KB | 366 | Exercise the compiled Genomics Caddy MCP process without exposing private report or genotype content. |
| `audit_reproducibility.mjs` | 2.5 KB | 78 | Enforce the repository's lock-free dependency contract. |
| `audit_resource_quality.mjs` | 31.2 KB | 604 | Audit curated marker and support resources for evidence, claim-boundary, callability, actionability, and source-registry coverage. |
| `audit_tauri_report.mjs` | 36.1 KB | 784 | Exercise the running Tauri desktop report through its local UI bridge. |
| `audit_tauri_update_flow.mjs` | 9.7 KB | 287 | Exercise the desktop Tauri update path against a disposable fixture. |
| `audit_updater_config.mjs` | 3.6 KB | 85 | Static audit of signed updater pubkey, endpoints, and release wiring. |
| `benchmark_build.ps1` | 5.2 KB | 168 | Time Genomics Caddy build-cache purge and/or full production rebuild on Windows. |
| `benchmark_build.sh` | 5.3 KB | 210 | Time Genomics Caddy build-cache purge and/or full production rebuild on Linux/macOS. |
| `benchmark_sweep.ps1` | 2.2 KB | 60 | Prints resolved tuning knobs for the current machine (or GENOMICS_CPU_LIMIT) and optionally probes Qdrant/Ollama from .env. |
| `build.py` | 16.2 KB | 388 | DNA-Tools remote build engine — CLI entry point. |
| `clean_deep.mjs` | 4.8 KB | 125 | Remove project-local, rebuildable frontend and Tauri artifacts. |
| `content_quality_metrics.mjs` | 30.7 KB | 744 | Privacy-safe content-quality metrics for curated genomic resources. |
| `generate_architecture.py` | 19.1 KB | 463 | Regenerate ARCHITECTURE.md from the non-gitignored tree. |
| `Inspect-GenomicsRefs.ps1` | 20.2 KB | 664 | requires -Version 7.0. |
| `install_linux_desktop.sh` | 4.6 KB | 170 | Install FreeDesktop .desktop entry + hicolor icons so Genomics Caddy shows the DNA logo in the GNOME/Ubuntu dock (Wayland) instead of a generic gear. |
| `load_env.py` | 1.3 KB | 43 | Load key=value pairs from a `.env` file without third-party dependencies. |
| `migrate_data_to_app.ps1` | 1.4 KB | 42 | One-time copy from legacy ./data to ./App/Data (does not delete source unless -Move). |
| `pnpm_unlocked.mjs` | 2.9 KB | 90 | Resolve pnpm dependencies without retaining a project lockfile. |
| `preload-typescript6-for-svelte.cjs` | 1.9 KB | 47 | Compatibility shim so Svelte tooling can run while the project uses TypeScript 7. |
| `purge_and_build.ps1` | 8.7 KB | 281 | Purges frontend/Rust *build caches* only, then compiles a production Tauri release. |
| `purge_and_build.sh` | 5.6 KB | 199 | Purges frontend/Rust *build caches* only, then compiles a production Tauri release. |
| `purge_git_secrets.ps1` | 2.0 KB | 48 | Removes sensitive paths from entire git history (local repo rewrite). |
| `refresh_linux_portable_ui.sh` | 4.5 KB | 162 | Refresh the FreeDesktop launcher/icon to match the current portable binary, then clear WebKit UI caches. |
| `run_benchmark_build.mjs` | 1.8 KB | 51 | Cross-platform entry for build purge/rebuild timing benchmarks. |
| `run_release_build.mjs` | 2.1 KB | 59 | Cross-platform entry for production Tauri release builds. |
| `run_svelte_check.mjs` | 1.3 KB | 37 | Run svelte-kit sync + svelte-check against a TypeScript 7 project. |
| `run_tsc.mjs` | 2.0 KB | 59 | Invoke the project TypeScript 7 `tsc` even when npm's `.bin/tsc` was stolen by `@typescript/old` (pulled in by `@typescript/typescript6`). |
| `setup_linux_deps.sh` | 2.3 KB | 93 | One-time Linux/macOS prerequisite installer for Genomics Caddy (Tauri v2). |
| `smoke_test.py` | 8.6 KB | 240 | Genomics Caddy — local integration smoke tests. |
| `smoketest_macos.sh` | 11.1 KB | 271 | Smoke-test the macOS Universal .app bundle for Genomics Caddy. |
| `sync_dna_to_server.py` | 12.0 KB | 311 | DEPRECATED — use scripts/build.py instead. |
| `system_check.sh` | 2.1 KB | 66 | Quick environment diagnostic for Genomics Caddy builds (Linux/macOS). |
| `uninstall_linux_desktop.sh` | 1.5 KB | 44 | Remove FreeDesktop launcher + icons and clear WebKit asset caches so the portable DNA-Tools binary is not shadowed by a stale desktop registration. |
| `update_all.mjs` | 12.1 KB | 345 | One-shot “Update All” for Genomics Caddy toolchains and project dependencies. |
| `validate_marker_packs.mjs` | 70.0 KB | 1,347 | Validate the curated marker-pack contract and source/runtime parity. |
| `verify_vite_css_load.mjs` | 4.5 KB | 142 | Fail if Vite emits "failed to load virtual css module" for our Svelte components. |

## `scripts/remote_build`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `__init__.py` | 227 B | 7 | DNA_Tools remote build package. |
| `_toolkit.py` | 2.7 KB | 77 | Remote_Build toolkit path shim + re-exports. |
| `cache.py` | 6.8 KB | 228 | Remote build cache purge. |
| `collect.py` | 6.9 KB | 218 | Artifact download helpers. |
| `lint.py` | 9.3 KB | 270 | Lint runner for remote build pipelines. |
| `linux.py` | 3.6 KB | 101 | Linux x86_64 build phase. |
| `macos.py` | 5.9 KB | 163 | macOS Universal (arm64 + x86_64) build phase. |
| `sync.py` | 6.4 KB | 204 | Source packaging and upload helpers. |
| `toolchain.py` | 10.4 KB | 295 | Build environment upgrade & auto-healing runner — runs before every compile pass. |
| `windows.py` | 3.3 KB | 92 | Windows x86_64 cross-compile build phase. |

## `src`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `app.html` | 817 B | 33 | SvelteKit HTML shell: dark first paint and webview title. |

## `src-tauri`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `.gitignore` | 173 B | 7 | Ignore Cargo target/ and generated Tauri schemas. |
| `build.rs` | 43 B | 3 | Tauri build script hook. |
| `Cargo.toml` | 1.6 KB | 58 | Rust crate manifest, edition, and dependencies. |
| `desktop.template` | 180 B | 10 | Linux .desktop template for packaged builds. |
| `tauri.conf.json` | 1.7 KB | 60 | Tauri window, CSP, bundle, updater pubkey, and identifier config. |

## `src-tauri/App/Data/marker-packs`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `allergy_atopy_mast_cell.json` | 60.6 KB | 1,423 | Runtime mirror of curated pack `allergy_atopy_mast_cell`: Allergy, Atopy & Mast-Cell Context. |
| `bone_growth_mineral_density.json` | 45.5 KB | 1,110 | Runtime mirror of curated pack `bone_growth_mineral_density`: Bone Growth, Mineral Density & Skeletal Development. |
| `cancer_confirmation_only.json` | 222.5 KB | 4,767 | Runtime mirror of curated pack `cancer_confirmation_only`: High-Stakes Cancer Predisposition. |
| `cardiovascular.json` | 149.3 KB | 3,375 | Runtime mirror of curated pack `cardiovascular`: Cardiovascular & Lipid Transport. |
| `connective_tissue.json` | 134.5 KB | 2,925 | Runtime mirror of curated pack `connective_tissue`: Connective Tissue Laxity. |
| `core.json` | 58.7 KB | 1,372 | Runtime mirror of curated pack `core`: Core Physiology Traits. |
| `dental_oral_health.json` | 34.9 KB | 843 | Runtime mirror of curated pack `dental_oral_health`: Dental, Oral Health & Taste Context. |
| `digestive_gut_microbiome.json` | 61.4 KB | 1,462 | Runtime mirror of curated pack `digestive_gut_microbiome`: Digestive, Gut & Microbiome Context. |
| `discovery_catalog.json` | 376.2 KB | 7,774 | Runtime mirror: curated discovery targets for catalog scans and research UI. |
| `hormones_reproductive.json` | 116.6 KB | 2,391 | Runtime mirror of curated pack `hormones_reproductive`: Hormones, Reproductive Biology & PMDD Context. |
| `immune_autoimmune_general.json` | 68.1 KB | 1,574 | Runtime mirror of curated pack `immune_autoimmune_general`: Immune & Autoimmune General Susceptibility. |
| `kidney_fluid_electrolytes.json` | 46.8 KB | 1,140 | Runtime mirror of curated pack `kidney_fluid_electrolytes`: Kidney, Fluid Balance & Electrolytes. |
| `longevity_aging_resilience.json` | 62.7 KB | 1,434 | Runtime mirror of curated pack `longevity_aging_resilience`: Longevity, Aging Resilience & Healthspan Context. |
| `manifest.json` | 7.9 KB | 165 | Runtime mirror: pack catalog: ids, titles, and which JSON files load at runtime. |
| `metabolic.json` | 114.3 KB | 2,626 | Runtime mirror of curated pack `metabolic`: Metabolic Health & T2D. |
| `muscle_performance_recovery.json` | 66.1 KB | 1,555 | Runtime mirror of curated pack `muscle_performance_recovery`: Muscle Performance, Hypertrophy & Recovery. |
| `neuropsych.json` | 159.2 KB | 3,379 | Runtime mirror of curated pack `neuropsych`: Neurotransmitter & Mood Resiliency. |
| `nutrients.json` | 85.8 KB | 2,081 | Runtime mirror of curated pack `nutrients`: Nutrients & One-Carbon Methylation. |
| `pain_migraine_sensory.json` | 56.4 KB | 1,317 | Runtime mirror of curated pack `pain_migraine_sensory`: Pain, Migraine & Sensory Processing. |
| `pgx.json` | 172.7 KB | 3,680 | Runtime mirror of curated pack `pgx`: Pharmacogenomics (PGx). |
| `research_found.json` | 69 B | 4 | Runtime mirror: protected pack for research-found drafts only. |
| `respiratory_airway.json` | 53.3 KB | 1,215 | Runtime mirror of curated pack `respiratory_airway`: Respiratory, Airway & Oxygenation Context. |
| `skin_hair_dermatology.json` | 56.1 KB | 1,312 | Runtime mirror of curated pack `skin_hair_dermatology`: Skin, Hair & Dermatology Context. |
| `sleep.json` | 70.3 KB | 1,695 | Runtime mirror of curated pack `sleep`: Sleep & Circadian Rhythms. |
| `thyroid_autoimmune.json` | 108.2 KB | 2,466 | Runtime mirror of curated pack `thyroid_autoimmune`: Thyroid Hormone & Autoimmune Risk. |

## `src-tauri/capabilities`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `default.json` | 284 B | 12 | Tauri capability permissions including updater and relaunch. |

## `src-tauri/dev-bins`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `Cargo.toml` | 670 B | 23 | Tracked file `Cargo.toml`. |

## `src-tauri/dev-bins/src`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `inspect_db.rs` | 1.7 KB | 52 | CLI helper: print resolved data-dir and genome DB paths. |
| `seed_mcp_fixture.rs` | 1.3 KB | 34 | CLI helper: seed a disposable MCP test fixture database. |

## `src-tauri/icons`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `128x128.png` | 27.6 KB | — | Desktop/store icon asset (128x128.png). |
| `128x128@2x.png` | 90.2 KB | — | Desktop/store icon asset (128x128@2x.png). |
| `32x32.png` | 2.7 KB | — | Desktop/store icon asset (32x32.png). |
| `64x64.png` | 8.7 KB | — | Desktop/store icon asset (64x64.png). |
| `icon.icns` | 2.1 MB | — | Desktop/store icon asset (icon.icns). |
| `icon.ico` | 109.6 KB | — | Desktop/store icon asset (icon.ico). |
| `icon.png` | 327.9 KB | — | Desktop/store icon asset (icon.png). |
| `Square107x107Logo.png` | 20.5 KB | — | Desktop/store icon asset (Square107x107Logo.png). |
| `Square142x142Logo.png` | 32.8 KB | — | Desktop/store icon asset (Square142x142Logo.png). |
| `Square150x150Logo.png` | 35.9 KB | — | Desktop/store icon asset (Square150x150Logo.png). |
| `Square284x284Logo.png` | 108.3 KB | — | Desktop/store icon asset (Square284x284Logo.png). |
| `Square30x30Logo.png` | 2.4 KB | — | Desktop/store icon asset (Square30x30Logo.png). |
| `Square310x310Logo.png` | 127.1 KB | — | Desktop/store icon asset (Square310x310Logo.png). |
| `Square44x44Logo.png` | 4.7 KB | — | Desktop/store icon asset (Square44x44Logo.png). |
| `Square71x71Logo.png` | 10.3 KB | — | Desktop/store icon asset (Square71x71Logo.png). |
| `Square89x89Logo.png` | 15.1 KB | — | Desktop/store icon asset (Square89x89Logo.png). |
| `StoreLogo.png` | 5.7 KB | — | Desktop/store icon asset (StoreLogo.png). |

## `src-tauri/icons/android/mipmap-anydpi-v26`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `ic_launcher.xml` | 261 B | 5 | Android mipmap-anydpi-v26 launcher icon. |

## `src-tauri/icons/android/mipmap-hdpi`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `ic_launcher.png` | 4.3 KB | — | Android mipmap-hdpi launcher icon. |
| `ic_launcher_foreground.png` | 40.9 KB | — | Android mipmap-hdpi foreground icon. |
| `ic_launcher_round.png` | 4.2 KB | — | Android mipmap-hdpi round launcher icon. |

## `src-tauri/icons/android/mipmap-mdpi`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `ic_launcher.png` | 4.1 KB | — | Android mipmap-mdpi launcher icon. |
| `ic_launcher_foreground.png` | 20.9 KB | — | Android mipmap-mdpi foreground icon. |
| `ic_launcher_round.png` | 4.0 KB | — | Android mipmap-mdpi round launcher icon. |

## `src-tauri/icons/android/mipmap-xhdpi`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `ic_launcher.png` | 13.3 KB | — | Android mipmap-xhdpi launcher icon. |
| `ic_launcher_foreground.png` | 66.9 KB | — | Android mipmap-xhdpi foreground icon. |
| `ic_launcher_round.png` | 12.9 KB | — | Android mipmap-xhdpi round launcher icon. |

## `src-tauri/icons/android/mipmap-xxhdpi`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `ic_launcher.png` | 26.3 KB | — | Android mipmap-xxhdpi launcher icon. |
| `ic_launcher_foreground.png` | 137.8 KB | — | Android mipmap-xxhdpi foreground icon. |
| `ic_launcher_round.png` | 25.5 KB | — | Android mipmap-xxhdpi round launcher icon. |

## `src-tauri/icons/android/mipmap-xxxhdpi`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `ic_launcher.png` | 42.6 KB | — | Android mipmap-xxxhdpi launcher icon. |
| `ic_launcher_foreground.png` | 237.2 KB | — | Android mipmap-xxxhdpi foreground icon. |
| `ic_launcher_round.png` | 41.3 KB | — | Android mipmap-xxxhdpi round launcher icon. |

## `src-tauri/icons/android/values`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `ic_launcher_background.xml` | 115 B | 4 | Android values launcher icon. |

## `src-tauri/icons/ios`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `AppIcon-20x20@1x.png` | 1.2 KB | — | iOS app-icon raster (AppIcon-20x20@1x.png). |
| `AppIcon-20x20@2x-1.png` | 3.8 KB | — | iOS app-icon raster (AppIcon-20x20@2x-1.png). |
| `AppIcon-20x20@2x.png` | 3.8 KB | — | iOS app-icon raster (AppIcon-20x20@2x.png). |
| `AppIcon-20x20@3x.png` | 7.6 KB | — | iOS app-icon raster (AppIcon-20x20@3x.png). |
| `AppIcon-29x29@1x.png` | 2.2 KB | — | iOS app-icon raster (AppIcon-29x29@1x.png). |
| `AppIcon-29x29@2x-1.png` | 7.1 KB | — | iOS app-icon raster (AppIcon-29x29@2x-1.png). |
| `AppIcon-29x29@2x.png` | 7.1 KB | — | iOS app-icon raster (AppIcon-29x29@2x.png). |
| `AppIcon-29x29@3x.png` | 14.1 KB | — | iOS app-icon raster (AppIcon-29x29@3x.png). |
| `AppIcon-40x40@1x.png` | 3.8 KB | — | iOS app-icon raster (AppIcon-40x40@1x.png). |
| `AppIcon-40x40@2x-1.png` | 12.2 KB | — | iOS app-icon raster (AppIcon-40x40@2x-1.png). |
| `AppIcon-40x40@2x.png` | 12.2 KB | — | iOS app-icon raster (AppIcon-40x40@2x.png). |
| `AppIcon-40x40@3x.png` | 24.2 KB | — | iOS app-icon raster (AppIcon-40x40@3x.png). |
| `AppIcon-512@2x.png` | 1.2 MB | — | iOS app-icon raster (AppIcon-512@2x.png). |
| `AppIcon-60x60@2x.png` | 24.2 KB | — | iOS app-icon raster (AppIcon-60x60@2x.png). |
| `AppIcon-60x60@3x.png` | 47.9 KB | — | iOS app-icon raster (AppIcon-60x60@3x.png). |
| `AppIcon-76x76@1x.png` | 11.3 KB | — | iOS app-icon raster (AppIcon-76x76@1x.png). |
| `AppIcon-76x76@2x.png` | 36.0 KB | — | iOS app-icon raster (AppIcon-76x76@2x.png). |
| `AppIcon-83.5x83.5@2x.png` | 42.2 KB | — | iOS app-icon raster (AppIcon-83.5x83.5@2x.png). |

## `src-tauri/linux`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `post_install.sh` | 737 B | 19 | After Linux package install: canonical FreeDesktop launcher. |
| `post_remove.sh` | 212 B | 8 | After Linux package removal: drop the canonical launcher. |

## `src-tauri/src`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `agent.rs` | 44.9 KB | 1,253 | Evidence-Grade Variant Validation & Safety QA Audit engine for Genomics Caddy. |
| `agent_commands.rs` | 5.8 KB | 191 | Tauri command handlers for variant evidence and safety audit workflows. |
| `agent_ui.rs` | 12.4 KB | 375 | Agent/MCP bridge to inspect and drive the live Svelte UI. |
| `app_log.rs` | 1.9 KB | 63 | Provide robust file logging for the Genomics Caddy application. |
| `config.rs` | 34.0 KB | 1,030 | Application configuration, .env loading, and secure secret storage. |
| `db.rs` | 125.4 KB | 3,529 | Database management (SQLite) for storing standard genomes and clinical reference data. |
| `db_crypto.rs` | 3.9 KB | 110 | Legacy sealed-DB cleanup only (encryption removed). |
| `db_runtime.rs` | 1.0 KB | 33 | Run SQLite work on blocking threads so the Tauri webview stays responsive. |
| `file_utils.rs` | 2.9 KB | 83 | Rust backend `file_utils`. |
| `inference_host.rs` | 21.3 KB | 635 | Dynamic inference-host and Ollama model capability discovery. |
| `lib.rs` | 86.9 KB | 2,288 | Tauri library entry point and command handler declarations. |
| `liftover.rs` | 15.5 KB | 430 | Memory-efficient coordinate liftover (GRCh37 → GRCh38) using UCSC chain files. |
| `main.rs` | 2.8 KB | 78 | Entry point for the Tauri app. |
| `mcp.rs` | 82.9 KB | 1,972 | Model Context Protocol (MCP) server implementation for genomic database interaction. |
| `parser.rs` | 27.8 KB | 719 | Validated parser for raw genomic data files (AncestryDNA and 23andMe formats), including CSV/TSV files and ZIP archives. |
| `paths.rs` | 10.7 KB | 328 | Resolve portable application data paths for Genomics Caddy. |
| `report.rs` | 132.2 KB | 3,421 | Direction-aware, template-based report generator for genetic trait profiling. |
| `service_ops.rs` | 15.2 KB | 428 | Ollama / vector-DB service operations for Advanced → Connections. |
| `stream_control.rs` | 2.0 KB | 69 | Cooperative cancellation and namespaced events for Ollama streaming. |

## `src-tauri/src/bin`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `inspect_db.rs` | 0 B | 0 | Rust backend `inspect_db`. |
| `seed_mcp_fixture.rs` | 0 B | 0 | Rust backend `seed_mcp_fixture`. |

## `src-tauri/src/offline`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `commands.rs` | 11.7 KB | 326 | Rust offline catalog/update `commands`. |
| `compress.rs` | 5.0 KB | 138 | Rust offline catalog/update `compress`. |
| `discovery_export.rs` | 23.7 KB | 700 | Export marker-pack coverage vs genome-wide catalog hits for pack authoring. |
| `download.rs` | 33.0 KB | 923 | Rust offline catalog/update `download`. |
| `import_clingen.rs` | 7.9 KB | 215 | Import ClinGen gene–disease validity CSV into the clingen catalog DB. |
| `import_clinvar.rs` | 18.9 KB | 494 | Rust offline catalog/update `import_clinvar`. |
| `import_dbsnp.rs` | 22.7 KB | 675 | Import NCBI dbSNP merged/withdrawn RefSNP JSON into dbsnp.db. |
| `import_mane.rs` | 5.8 KB | 156 | Import MANE Select summary TSV into the mane catalog DB. |
| `import_pharmgkb.rs` | 15.1 KB | 409 | Rust offline catalog/update `import_pharmgkb`. |
| `lookup.rs` | 10.4 KB | 333 | Rust offline catalog/update `lookup`. |
| `manifest.rs` | 10.4 KB | 293 | Rust offline catalog/update `manifest`. |
| `mod.rs` | 936 B | 33 | Offline genomics reference layer (Tiers 0–2). |
| `registry.rs` | 11.2 KB | 320 | Rust offline catalog/update `registry`. |
| `schema.rs` | 6.0 KB | 183 | Offline/reference SQLite schema migrations that are safe when catalog DBs are absent. |
| `sync.rs` | 61.8 KB | 1,649 | Rust offline catalog/update `sync`. |
| `tier2.rs` | 4.7 KB | 139 | Rust offline catalog/update `tier2`. |

## `src-tauri/src/research`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `commands.rs` | 29.6 KB | 963 | Tauri command handlers for Qdrant research loop and related settings. |
| `crossmap.rs` | 8.2 KB | 266 | Trait taxonomy, marker-pack bridging, and cross-reference tags for vector enrichment. |
| `debug_log.rs` | 2.6 KB | 97 | Rust research/enrichment `debug_log`. |
| `embed.rs` | 5.9 KB | 213 | Rust research/enrichment `embed`. |
| `enrich.rs` | 30.3 KB | 929 | Rust research/enrichment `enrich`. |
| `headless.rs` | 3.6 KB | 117 | Headless sweep runner for Docker and server deployments. |
| `http.rs` | 7.0 KB | 202 | Shared HTTP clients and outbound rate limits for the research enrichment pipeline. |
| `job.rs` | 9.7 KB | 258 | Rust research/enrichment `job`. |
| `markers.rs` | 10.8 KB | 339 | Rust research/enrichment `markers`. |
| `mod.rs` | 2.1 KB | 70 | Autonomous marker research, enrichment sources, and Qdrant vector indexing. |
| `pack_draft.rs` | 26.2 KB | 771 | Export draft marker-pack JSON from vector research and merge into research_found only. |
| `prefetch.rs` | 9.7 KB | 286 | Rust research/enrichment `prefetch`. |
| `promote.rs` | 4.2 KB | 140 | Auto-promote high-confidence vector-enriched GWAS hits into SQLite for trait report display. |
| `qdrant.rs` | 50.3 KB | 1,769 | Rust research/enrichment `qdrant`. |
| `references.rs` | 19.9 KB | 573 | Download and load external reference catalogs into ./data/references. |
| `research_found_pack.rs` | 18.8 KB | 571 | Enrich, review, and manage the research_found marker pack. |
| `sources.rs` | 16.4 KB | 525 | Rust research/enrichment `sources`. |
| `sources_config.rs` | 3.6 KB | 131 | Rust research/enrichment `sources_config`. |
| `state.rs` | 4.2 KB | 112 | Rust research/enrichment `state`. |
| `sweep.rs` | 67.0 KB | 1,880 | Rust research/enrichment `sweep`. |
| `sweep_metrics.rs` | 13.1 KB | 456 | Rust research/enrichment `sweep_metrics`. |
| `sweep_runtime.rs` | 2.0 KB | 62 | Rust research/enrichment `sweep_runtime`. |
| `tuning.rs` | 15.8 KB | 523 | Runtime pipeline tuning for vector enrichment sweeps. |
| `types.rs` | 12.9 KB | 377 | Rust research/enrichment `types`. |
| `util.rs` | 23.1 KB | 700 | Rust research/enrichment `util`. |

## `src-tauri/src/research/evidence`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `adapters.rs` | 19.6 KB | 682 | Rust research/enrichment `adapters`. |
| `atlas.rs` | 9.0 KB | 300 | Rust research/enrichment `atlas`. |
| `backfill.rs` | 10.7 KB | 341 | Rust research/enrichment `backfill`. |
| `cache.rs` | 13.6 KB | 478 | Rust research/enrichment `cache`. |
| `card.rs` | 8.0 KB | 209 | Rust research/enrichment `card`. |
| `catalog.rs` | 7.0 KB | 210 | Rust research/enrichment `catalog`. |
| `commands.rs` | 10.0 KB | 342 | Rust research/enrichment `commands`. |
| `corpus.rs` | 12.8 KB | 349 | Rust research/enrichment `corpus`. |
| `dashboard.rs` | 19.2 KB | 533 | Rust research/enrichment `dashboard`. |
| `mod.rs` | 558 B | 27 | Rust research/enrichment `mod`. |
| `named_vectors.rs` | 9.1 KB | 301 | Rust research/enrichment `named_vectors`. |
| `ncbi_context.rs` | 10.3 KB | 354 | Rust research/enrichment `ncbi_context`. |
| `normalize.rs` | 12.8 KB | 355 | Rust research/enrichment `normalize`. |
| `ontology.rs` | 2.5 KB | 96 | Rust research/enrichment `ontology`. |
| `packet.rs` | 6.1 KB | 166 | Rust research/enrichment `packet`. |
| `pgs_match.rs` | 9.4 KB | 317 | Rust research/enrichment `pgs_match`. |
| `schema.rs` | 8.3 KB | 217 | Rust research/enrichment `schema`. |
| `scoring.rs` | 10.9 KB | 345 | Rust research/enrichment `scoring`. |
| `search.rs` | 12.5 KB | 398 | Rust research/enrichment `search`. |
| `source_records.rs` | 7.7 KB | 282 | Rust research/enrichment `source_records`. |
| `store.rs` | 21.9 KB | 632 | Rust research/enrichment `store`. |
| `types.rs` | 9.8 KB | 317 | Rust research/enrichment `types`. |

## `src-tauri/src/research/gnomad`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `allele_norm.rs` | 2.1 KB | 70 | Rust research/enrichment `allele_norm`. |
| `batch.rs` | 7.6 KB | 234 | Rust research/enrichment `batch`. |
| `cache.rs` | 17.8 KB | 557 | Rust research/enrichment `cache`. |
| `commands.rs` | 4.4 KB | 118 | Rust research/enrichment `commands`. |
| `config.rs` | 8.2 KB | 193 | Rust research/enrichment `config`. |
| `graphql.rs` | 9.5 KB | 269 | Rust research/enrichment `graphql`. |
| `lookup.rs` | 15.8 KB | 531 | Rust research/enrichment `lookup`. |
| `manifest.rs` | 13.5 KB | 382 | Rust research/enrichment `manifest`. |
| `mod.rs` | 652 B | 24 | Rust research/enrichment `mod`. |
| `readiness.rs` | 18.5 KB | 540 | Rust research/enrichment `readiness`. |
| `schema.rs` | 5.0 KB | 150 | Rust research/enrichment `schema`. |
| `types.rs` | 13.4 KB | 420 | Rust research/enrichment `types`. |
| `validate.rs` | 5.5 KB | 153 | Rust research/enrichment `validate`. |
| `vcf_local.rs` | 6.8 KB | 189 | Rust research/enrichment `vcf_local`. |
| `vcf_parse.rs` | 6.2 KB | 201 | Rust research/enrichment `vcf_parse`. |
| `vcf_remote.rs` | 10.3 KB | 321 | Rust research/enrichment `vcf_remote`. |

## `src-tauri/src/research/vector_store`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `chroma.rs` | 16.8 KB | 505 | Chroma HTTP dense vector adapter (self-host / cloud). |
| `mod.rs` | 19.3 KB | 580 | Provider-agnostic vector store facade for research dense indexing/search. |
| `pinecone.rs` | 15.6 KB | 453 | Pinecone dense vector adapter (index host + Api-Key). |
| `weaviate.rs` | 16.9 KB | 472 | Weaviate dense vector adapter (objects batch + GraphQL nearVector). |

## `src-tauri/testdata`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `synthetic_23andme_grch38.csv` | 180 B | 5 | Synthetic parser fixture; no user DNA. |
| `synthetic_23andme_grch38.txt` | 177 B | 5 | Synthetic parser fixture; no user DNA. |
| `synthetic_ancestry_grch37.csv` | 184 B | 5 | Synthetic parser fixture; no user DNA. |
| `synthetic_ancestry_grch37.tsv` | 183 B | 5 | Synthetic parser fixture; no user DNA. |

## `src/lib/api`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `tauri.ts` | 33.9 KB | 1,173 | Tauri command invocation wrapper API layer. |

## `src/lib/components/agent`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `AgentQuickLaunch.svelte` | 3.7 KB | 135 | Component to render shortcuts for high-impact genomic findings detected in the user's report. |
| `AgentReportView.svelte` | 7.3 KB | 268 | Svelte UI component `AgentReportView`. |
| `AgentResearchPanel.svelte` | 15.2 KB | 399 | Svelte UI component `AgentResearchPanel`. |
| `AgentRunner.svelte` | 5.1 KB | 198 | Svelte UI component `AgentRunner`. |
| `AgentSetupForm.svelte` | 10.6 KB | 361 | Advanced setup form for genomics research agent including dynamic target selectors. |
| `AutonomousScanSection.svelte` | 15.9 KB | 541 | Autonomous scanning controls and save path configurations for the Genomics Research Agent. |
| `DiscoveredVariantsList.svelte` | 8.4 KB | 302 | List view and checklist categorization for discovered genomic variants. |

## `src/lib/components/ai`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `AccessibleActions.test.ts` | 1.1 KB | 34 | Tests for `AccessibleActions`. |
| `AiAssistantPanel.svelte` | 21.9 KB | 508 | Svelte UI component `AiAssistantPanel`. |
| `BiohackingProfileSection.svelte` | 2.6 KB | 55 | Svelte UI component `BiohackingProfileSection`. |
| `ChatInput.svelte` | 5.0 KB | 194 | Svelte UI component `ChatInput`. |
| `ChatMessages.svelte` | 13.0 KB | 215 | Svelte UI component `ChatMessages`. |
| `ChatModals.svelte` | 7.5 KB | 286 | Svelte UI component `ChatModals`. |
| `ChatSettingsDrawer.svelte` | 6.9 KB | 245 | Svelte UI component `ChatSettingsDrawer`. |
| `ChatSidebar.svelte` | 9.0 KB | 347 | Svelte UI component `ChatSidebar`. |
| `ChatWindow.svelte` | 10.6 KB | 376 | Svelte UI component `ChatWindow`. |
| `CycleDiaryEditor.svelte` | 8.5 KB | 311 | Svelte UI component `CycleDiaryEditor`. |
| `EvidenceLibraryPanel.svelte` | 14.8 KB | 439 | Svelte UI component `EvidenceLibraryPanel`. |
| `ReproductiveContextEditor.svelte` | 6.4 KB | 219 | Svelte UI component `ReproductiveContextEditor`. |
| `ReproductiveEditors.styles.test.ts` | 1.2 KB | 27 | CSS/layout tests for `ReproductiveEditors`. |
| `VectorResearchCitations.svelte` | 10.1 KB | 343 | Svelte UI component `VectorResearchCitations`. |
| `VectorResearchCitations.tooltip.test.ts` | 675 B | 14 | Tests for `VectorResearchCitations.tooltip`. |
| `WelcomeChat.svelte` | 982 B | 43 | Svelte UI component `WelcomeChat`. |

## `src/lib/components/ai/evidence`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `ActionabilityMatrix.svelte` | 3.8 KB | 117 | Svelte UI component `ActionabilityMatrix`. |
| `ActionabilityMatrix.tooltip.test.ts` | 677 B | 14 | Tests for `ActionabilityMatrix.tooltip`. |
| `CandidateFindingsPanel.svelte` | 5.3 KB | 121 | Svelte UI component `CandidateFindingsPanel`. |
| `EvidenceCorpusOverview.svelte` | 3.5 KB | 100 | Svelte UI component `EvidenceCorpusOverview`. |
| `EvidenceQualityDashboard.svelte` | 9.1 KB | 320 | Svelte UI component `EvidenceQualityDashboard`. |
| `EvidenceResultsList.svelte` | 9.9 KB | 375 | Svelte UI component `EvidenceResultsList`. |
| `EvidenceResultsList.tooltip.test.ts` | 809 B | 16 | Tests for `EvidenceResultsList.tooltip`. |
| `EvidenceSearchForm.svelte` | 4.7 KB | 205 | Svelte UI component `EvidenceSearchForm`. |
| `EvidenceSearchToolbar.svelte` | 8.4 KB | 238 | Svelte UI component `EvidenceSearchToolbar`. |
| `EvidenceSearchToolbar.tooltip.test.ts` | 687 B | 13 | Tests for `EvidenceSearchToolbar.tooltip`. |
| `EvidenceSourcesSidebar.svelte` | 2.1 KB | 102 | Svelte UI component `EvidenceSourcesSidebar`. |
| `FindingsNavigator.svelte` | 11.4 KB | 356 | Svelte UI component `FindingsNavigator`. |
| `PathwayFlowPanel.svelte` | 3.3 KB | 111 | Svelte UI component `PathwayFlowPanel`. |
| `QdrantResultsList.svelte` | 4.8 KB | 125 | Svelte UI component `QdrantResultsList`. |
| `SimilarAssociationsPanel.svelte` | 2.8 KB | 95 | Svelte UI component `SimilarAssociationsPanel`. |
| `TraitClusterPanel.svelte` | 3.5 KB | 104 | Svelte UI component `TraitClusterPanel`. |
| `VectorAtlasPanel.svelte` | 12.6 KB | 376 | Svelte UI component `VectorAtlasPanel`. |
| `VectorAtlasPanel.tooltip.test.ts` | 1.3 KB | 24 | Tests for `VectorAtlasPanel.tooltip`. |
| `VectorEvidenceCard.svelte` | 14.3 KB | 457 | Svelte UI component `VectorEvidenceCard`. |
| `VectorEvidenceTooltip.test.ts` | 1.3 KB | 25 | Tests for `VectorEvidenceTooltip`. |

## `src/lib/components/ai/settings`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `AdvancedSettingsSection.svelte` | 15.6 KB | 386 | Svelte UI component `AdvancedSettingsSection`. |
| `AdvancedSettingsSection.tooltip.test.ts` | 1.3 KB | 27 | Tests for `AdvancedSettingsSection.tooltip`. |
| `ConnectionModelSection.svelte` | 14.7 KB | 495 | Svelte UI component `ConnectionModelSection`. |
| `GenomicContextSection.svelte` | 2.9 KB | 115 | Svelte UI component `GenomicContextSection`. |
| `InferenceSettingsSection.svelte` | 9.3 KB | 286 | Svelte UI component `InferenceSettingsSection`. |
| `QdrantResearchSettingsDetails.svelte` | 8.5 KB | 216 | Svelte UI component `QdrantResearchSettingsDetails`. |
| `SystemPromptSection.svelte` | 1.4 KB | 55 | Svelte UI component `SystemPromptSection`. |
| `VectorResearchSettingsSection.svelte` | 6.2 KB | 188 | Svelte UI component `VectorResearchSettingsSection`. |

## `src/lib/components/common`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `AppBootstrapScreen.styles.test.ts` | 7.4 KB | 129 | CSS/layout tests for `AppBootstrapScreen`. |
| `AppBootstrapScreen.svelte` | 20.4 KB | 661 | Svelte UI component `AppBootstrapScreen`. |
| `AppUpdateHost.svelte` | 3.5 KB | 104 | Launch-time updater host: banner, confirm, download, relaunch. |
| `EmptyState.copy.test.ts` | 794 B | 16 | Tests for `EmptyState.copy`. |
| `EmptyState.svelte` | 5.3 KB | 135 | Welcome / onboarding screen when no genome profile is selected. |
| `GlobalDialogs.styles.test.ts` | 1.5 KB | 36 | CSS/layout tests for `GlobalDialogs`. |
| `GlobalDialogs.svelte` | 4.8 KB | 183 | Global dialog backdrop rendering alerts and confirms. |
| `ImportWorkspaceState.styles.test.ts` | 2.2 KB | 41 | CSS/layout tests for `ImportWorkspaceState`. |
| `ImportWorkspaceState.svelte` | 13.2 KB | 468 | Svelte UI component `ImportWorkspaceState`. |
| `ThemeToggle.styles.test.ts` | 2.7 KB | 57 | CSS/layout tests for `ThemeToggle`. |
| `ThemeToggle.svelte` | 4.8 KB | 167 | Svelte UI component `ThemeToggle`. |
| `Tooltip.accessibility.test.ts` | 3.7 KB | 68 | Tests for `Tooltip.accessibility`. |
| `Tooltip.styles.test.ts` | 1.0 KB | 23 | CSS/layout tests for `Tooltip`. |
| `Tooltip.svelte` | 9.0 KB | 319 | Svelte UI component `Tooltip`. |
| `UpdateBanner.styles.test.ts` | 1.6 KB | 34 | CSS/layout tests for `UpdateBanner`. |
| `UpdateBanner.svelte` | 1.3 KB | 36 | Dismissible signed-update notification strip. |

## `src/lib/components/common/bootstrap`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `BootstrapActivityPulse.svelte` | 552 B | 21 | Svelte UI component `BootstrapActivityPulse`. |
| `bootstrapCountUp.ts` | 1.1 KB | 43 | Eased count-up helper for bootstrap stat cards. |
| `BootstrapHelixBackdrop.svelte` | 5.4 KB | 204 | Svelte UI component `BootstrapHelixBackdrop`. |
| `BootstrapOverlay.svelte` | 3.5 KB | 110 | Svelte UI component `BootstrapOverlay`. |
| `BootstrapPhaseOrb.svelte` | 2.3 KB | 93 | Svelte UI component `BootstrapPhaseOrb`. |
| `bootstrapPhases.ts` | 2.4 KB | 88 | Bootstrap phase metadata, step labels, and in-flight ticker copy. |
| `BootstrapStatReveal.svelte` | 3.3 KB | 123 | Svelte UI component `BootstrapStatReveal`. |
| `BootstrapStepTimeline.svelte` | 3.8 KB | 174 | Svelte UI component `BootstrapStepTimeline`. |
| `ImportStepTimeline.svelte` | 4.9 KB | 211 | Svelte UI component `ImportStepTimeline`. |

## `src/lib/components/common/loading`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `ActivityPulse.styles.test.ts` | 1.4 KB | 32 | CSS/layout tests for `ActivityPulse`. |
| `ActivityPulse.svelte` | 2.9 KB | 120 | Svelte UI component `ActivityPulse`. |
| `PanelLoadingState.svelte` | 1.1 KB | 51 | Svelte UI component `PanelLoadingState`. |

## `src/lib/components/context`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `ContextPanel.svelte` | 17.2 KB | 347 | Svelte UI component `ContextPanel`. |
| `ContextWorkspace.contract.test.ts` | 1.8 KB | 40 | Tests for `ContextWorkspace.contract`. |
| `DiaryPanel.svelte` | 6.4 KB | 135 | Svelte UI component `DiaryPanel`. |

## `src/lib/components/discovery`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `DiscoveryPanel.svelte` | 12.5 KB | 376 | In-app browser for genome × catalog associations beyond curated packs. |

## `src/lib/components/genome`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `GenomeMap.svelte` | 22.9 KB | 620 | Live database-backed chromosome visualization with interactive zoom, overlays, and comparison views. |
| `GenomeMap.tooltip.test.ts` | 1.6 KB | 31 | Tests for `GenomeMap.tooltip`. |

## `src/lib/components/import`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `GenomeImportPanel.svelte` | 3.6 KB | 92 | DNA ingestion panel for selecting and processing raw genomic data. |

## `src/lib/components/layout`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `AppShell.accessibility.test.ts` | 3.0 KB | 53 | Tests for `AppShell.accessibility`. |
| `AppShell.svelte` | 6.0 KB | 184 | Main layout grid wrapper for the Genomics Caddy application. |

## `src/lib/components/legal`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `LegalPrivacyPanel.svelte` | 4.2 KB | 147 | Svelte UI component `LegalPrivacyPanel`. |
| `LegalPrivacyPanel.test.ts` | 1.1 KB | 24 | Tests for `LegalPrivacyPanel`. |

## `src/lib/components/mcp`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `McpInstructions.svelte` | 10.7 KB | 376 | Tabbed setup-instructions panel for MCP client integrations. |
| `McpPanel.svelte` | 9.5 KB | 379 | Model Context Protocol (MCP) dynamic integration center (container shell). |
| `McpToolCatalog.svelte` | 6.0 KB | 254 | Expandable catalog of active MCP tools and their parameter schemas. |

## `src/lib/components/report`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `ClinicalFindingsTable.copy.test.ts` | 4.7 KB | 79 | Tests for `ClinicalFindingsTable.copy`. |
| `ClinicalFindingsTable.svelte` | 15.2 KB | 489 | Svelte UI component `ClinicalFindingsTable`. |
| `ConfirmWithList.svelte` | 821 B | 29 | Displays follow-up laboratory testing suggestions for a variant. |
| `DashboardSummaryPanel.styles.test.ts` | 22.7 KB | 399 | CSS/layout tests for `DashboardSummaryPanel`. |
| `DashboardSummaryPanel.svelte` | 98.0 KB | 3,144 | Svelte UI component `DashboardSummaryPanel`. |
| `DiscoveredFindingsBanner.styles.test.ts` | 2.0 KB | 41 | CSS/layout tests for `DiscoveredFindingsBanner`. |
| `DiscoveredFindingsBanner.svelte` | 7.1 KB | 265 | Svelte UI component `DiscoveredFindingsBanner`. |
| `EffectDirectionBadge.svelte` | 1.1 KB | 34 | Badge displaying effect direction with plain-language explanation. |
| `EvidenceBadge.svelte` | 1017 B | 31 | Badge displaying scientific evidence tier (Tiers A-D) with tooltip. |
| `ReferenceIndex.svelte` | 5.7 KB | 208 | Svelte UI component `ReferenceIndex`. |
| `ReportExportActions.styles.test.ts` | 1.2 KB | 28 | CSS/layout tests for `ReportExportActions`. |
| `ReportExportActions.svelte` | 5.7 KB | 204 | Svelte UI component `ReportExportActions`. |
| `ReportHeader.styles.test.ts` | 3.6 KB | 70 | CSS/layout tests for `ReportHeader`. |
| `ReportHeader.svelte` | 11.7 KB | 426 | Report header dashboard presenting summary statistics. |
| `ReportView.controls.test.ts` | 5.3 KB | 91 | Tests for `ReportView.controls`. |
| `ReportView.styles.test.ts` | 5.8 KB | 104 | CSS/layout tests for `ReportView`. |
| `ReportView.svelte` | 44.3 KB | 1,366 | Orchestrator component displaying the completed genetic trait report. |
| `SectionCard.styles.test.ts` | 4.9 KB | 87 | CSS/layout tests for `SectionCard`. |
| `SectionCard.svelte` | 13.7 KB | 465 | Card representing a thematic section of the genetic report. |
| `SourcesList.styles.test.ts` | 999 B | 22 | CSS/layout tests for `SourcesList`. |
| `SourcesList.svelte` | 3.6 KB | 131 | Formatted list of academic/clinical sources for genomic markers. |
| `VariantCard.styles.test.ts` | 6.7 KB | 125 | Reference enrichment chips. |
| `VariantCard.svelte` | 28.6 KB | 896 | Svelte UI component `VariantCard`. |
| `VectorPromotedSection.styles.test.ts` | 1.6 KB | 39 | CSS/layout tests for `VectorPromotedSection`. |
| `VectorPromotedSection.svelte` | 8.9 KB | 268 | Svelte UI component `VectorPromotedSection`. |
| `WarningBlocks.svelte` | 3.1 KB | 86 | Aggregated safety warnings, diagnostic limitations, and clinical validation flags. |
| `WarningBlocks.test.ts` | 1.5 KB | 34 | Tests for `WarningBlocks`. |

## `src/lib/components/research`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `GnomadSetupPanel.svelte` | 15.3 KB | 507 | Svelte UI component `GnomadSetupPanel`. |
| `OfflineDataPanel.svelte` | 19.2 KB | 582 | Svelte UI component `OfflineDataPanel`. |
| `OfflineDataPanel.update-state.test.ts` | 2.7 KB | 40 | Tests for `OfflineDataPanel.update-state`. |
| `PackDraftPanel.svelte` | 6.8 KB | 221 | Export reviewable marker-pack drafts and merge into research_found only. |
| `ResearchConnectionCard.svelte` | 16.1 KB | 482 | Svelte UI component `ResearchConnectionCard`. |
| `ResearchConnectionEditForm.svelte` | 5.9 KB | 169 | Svelte UI component `ResearchConnectionEditForm`. |
| `ResearchConnectionTooltip.test.ts` | 1.2 KB | 24 | Tests for `ResearchConnectionTooltip`. |
| `ResearchFoundReviewPanel.svelte` | 11.2 KB | 364 | Curate research_found markers — fill alleles, edit, drop, opt-in to reports. |
| `ResearchJobActionsPanel.svelte` | 6.6 KB | 191 | Svelte UI component `ResearchJobActionsPanel`. |
| `ResearchJobCard.svelte` | 3.6 KB | 141 | Svelte UI component `ResearchJobCard`. |
| `ResearchJobControls.svelte` | 6.6 KB | 220 | Svelte UI component `ResearchJobControls`. |
| `ResearchJobProgressPanel.svelte` | 19.1 KB | 524 | Svelte UI component `ResearchJobProgressPanel`. |
| `ResearchJobProgressTooltip.test.ts` | 801 B | 15 | Tests for `ResearchJobProgressTooltip`. |
| `ResearchLiveFindings.svelte` | 5.3 KB | 139 | Svelte UI component `ResearchLiveFindings`. |
| `ResearchLiveLog.svelte` | 2.8 KB | 96 | Svelte UI component `ResearchLiveLog`. |
| `ResearchPanel.svelte` | 24.7 KB | 773 | Svelte UI component `ResearchPanel`. |
| `ResearchScopeSection.styles.test.ts` | 1.1 KB | 23 | CSS/layout tests for `ResearchScopeSection`. |
| `ResearchScopeSection.svelte` | 15.3 KB | 464 | Svelte UI component `ResearchScopeSection`. |
| `ResearchStandaloneTooltip.test.ts` | 780 B | 23 | Tests for `ResearchStandaloneTooltip`. |
| `ResearchSurfaceTooltip.test.ts` | 1023 B | 19 | Tests for `ResearchSurfaceTooltip`. |
| `VectorViewerPanel.svelte` | 7.7 KB | 257 | Browse indexed vectors + optional Qdrant named-vector semantic search. |

## `src/lib/components/samples`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `SampleList.svelte` | 3.1 KB | 91 | Displays a list of active imported genomic profiles. |

## `src/lib/components/search`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `LocalReferenceBrowser.svelte` | 11.0 KB | 286 | Svelte UI component `LocalReferenceBrowser`. |
| `VariantSearchPanel.svelte` | 9.6 KB | 260 | Premium raw genotype + local reference database browser. |

## `src/lib/components/settings`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `ConnectionsPanel.styles.test.ts` | 1013 B | 21 | CSS/layout tests for `ConnectionsPanel`. |
| `ConnectionsPanel.svelte` | 29.2 KB | 883 | Advanced Connections hub for Ollama + multi-provider vector DB. |

## `src/lib/components/sidebar`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `ProgressTrack.svelte` | 1.4 KB | 46 | Svelte UI component `ProgressTrack`. |
| `Sidebar.svelte` | 63.9 KB | 1,550 | Sidebar panel container aggregating assembly settings, file import, profiles, and reference DB downloader. |
| `Sidebar.update-state.test.ts` | 6.0 KB | 87 | Tests for `Sidebar.update-state`. |

## `src/lib/constants`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `chromosomeLayout.ts` | 779 B | 18 | GRCh38 chromosome lengths and display order for the genome map heatbar. |
| `traitCategories.test.ts` | 1.1 KB | 28 | Tests for `traitCategories`. |
| `traitCategories.ts` | 564 B | 18 | UI discovery categories derived from the shared research taxonomy resource. |

## `src/lib/marker-packs`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `actionability_guidance.json` | 132.4 KB | 3,612 | Food, supplement, lab, and medication prompt rules. |
| `activity_guardrails.json` | 20.1 KB | 422 | Support resource `activity_guardrails`: Use genotype as a weak context modifier for training questions while keeping symptoms, medical history, progression, and professional clearance ahead of genetic optimization. |
| `agent_research_prompts.json` | 5.1 KB | 47 | Curated marker pack `agent_research_prompts`: Agent Research Prompt Policy. |
| `ai_prompt_helpers.json` | 3.2 KB | 63 | Curated marker pack `ai_prompt_helpers`: Dynamic AI Helper Prompts. |
| `ai_prompt_policy.json` | 9.3 KB | 59 | Default AI instructions and forbidden-action policy. |
| `allergy_atopy_mast_cell.json` | 60.6 KB | 1,423 | Curated marker pack `allergy_atopy_mast_cell`: Allergy, Atopy & Mast-Cell Context. |
| `allergy_sensitivity_catalog.json` | 13.0 KB | 217 | Curated marker pack `allergy_sensitivity_catalog`: Allergy & Sensitivity Guidance Catalog. |
| `bone_growth_mineral_density.json` | 45.5 KB | 1,110 | Curated marker pack `bone_growth_mineral_density`: Bone Growth, Mineral Density & Skeletal Development. |
| `callability_rules.json` | 11.6 KB | 326 | When a consumer-array row can or cannot score an assertion. |
| `cancer_confirmation_only.json` | 222.5 KB | 4,767 | Curated marker pack `cancer_confirmation_only`: High-Stakes Cancer Predisposition. |
| `cardiovascular.json` | 149.3 KB | 3,375 | Curated marker pack `cardiovascular`: Cardiovascular & Lipid Transport. |
| `condition_integrations.json` | 14.3 KB | 220 | Curated marker pack `condition_integrations`: condition_integrations. |
| `connective_tissue.json` | 134.5 KB | 2,925 | Curated marker pack `connective_tissue`: Connective Tissue Laxity. |
| `consultation_modes.json` | 5.7 KB | 82 | Specialty-mode labels and routing for Connected Chat. |
| `core.json` | 58.7 KB | 1,372 | Curated marker pack `core`: Core Physiology Traits. |
| `cycle_support_guidance.json` | 72.8 KB | 1,400 | Support resource `cycle_support_guidance`: Provide cycle-, hormone-, androgen-, pelvic-, and fertility-context questions, local symptom tracking, low-risk support options, safety escalation, and clinical follow-up prompts…. |
| `dental_oral_health.json` | 34.9 KB | 843 | Curated marker pack `dental_oral_health`: Dental, Oral Health & Taste Context. |
| `diet_pattern_profiles.json` | 12.3 KB | 502 | Support resource `diet_pattern_profiles`: Reusable diet-pattern templates for user-declared values, medical constraints, allergies/intolerances, and biohacking goals. |
| `dietary_requirements.json` | 55.1 KB | 1,600 | Support resource `dietary_requirements`: Convert genetics, labs, symptoms, medical conditions, allergies, medications, values, religion/culture and user preference into food-level allow/avoid/emphasize/substitute…. |
| `digestive_gut_microbiome.json` | 61.4 KB | 1,462 | Curated marker pack `digestive_gut_microbiome`: Digestive, Gut & Microbiome Context. |
| `discovery_catalog.json` | 376.2 KB | 7,774 | Curated discovery targets for catalog scans and research UI. |
| `evidence_policy.json` | 12.3 KB | 264 | Shared evidence-tier, severity, and claim-frame wording. |
| `food_nutrient_matrix.json` | 27.0 KB | 1,169 | Support resource `food_nutrient_matrix`: Provide food-group tags, nutrient strengths, caution flags, and substitution logic for local meal filtering and food requirement reasoning. |
| `food_requirement_prompts.json` | 4.9 KB | 171 | Support resource `food_requirement_prompts`: Structured questions to capture hard food requirements, values, allergies, symptoms, labs and goals before generating diet guidance. |
| `hormones_reproductive.json` | 116.6 KB | 2,391 | Curated marker pack `hormones_reproductive`: Hormones, Reproductive Biology & PMDD Context. |
| `immune_autoimmune_general.json` | 68.1 KB | 1,574 | Curated marker pack `immune_autoimmune_general`: Immune & Autoimmune General Susceptibility. |
| `inflammation_support_guidance.json` | 10.3 KB | 156 | Support resource `inflammation_support_guidance`: Connect inflammatory-pathway markers to plain-language context, a short set of foundational food and routine cues, and measured follow-up without treating DNA as a measure of…. |
| `kidney_fluid_electrolytes.json` | 46.8 KB | 1,140 | Curated marker pack `kidney_fluid_electrolytes`: Kidney, Fluid Balance & Electrolytes. |
| `lab_overlays.json` | 4.9 KB | 191 | Curated marker pack `lab_overlays`: Lab Overlay Map. |
| `layperson_translations.json` | 212.9 KB | 3,038 | Plain-English marker copy for Simple mode. |
| `longevity_aging_resilience.json` | 62.7 KB | 1,434 | Curated marker pack `longevity_aging_resilience`: Longevity, Aging Resilience & Healthspan Context. |
| `manifest.json` | 7.9 KB | 165 | Pack catalog: ids, titles, and which JSON files load at runtime. |
| `meal_planning_rules.json` | 5.0 KB | 156 | Support resource `meal_planning_rules`: Score, filter, and explain meal/food recommendations using hard requirements, soft goals, genetic context, lab context, and user preferences. |
| `metabolic.json` | 114.3 KB | 2,626 | Curated marker pack `metabolic`: Metabolic Health & T2D. |
| `muscle_performance_recovery.json` | 66.1 KB | 1,555 | Curated marker pack `muscle_performance_recovery`: Muscle Performance, Hypertrophy & Recovery. |
| `neuropsych.json` | 159.2 KB | 3,379 | Curated marker pack `neuropsych`: Neurotransmitter & Mood Resiliency. |
| `nutrients.json` | 85.8 KB | 2,081 | Curated marker pack `nutrients`: Nutrients & One-Carbon Methylation. |
| `pain_migraine_sensory.json` | 56.4 KB | 1,317 | Curated marker pack `pain_migraine_sensory`: Pain, Migraine & Sensory Processing. |
| `pgx.json` | 172.7 KB | 3,680 | Curated marker pack `pgx`: Pharmacogenomics (PGx). |
| `pgx_diplotype_guidance.json` | 13.2 KB | 152 | Support resource `pgx_diplotype_guidance`: Explain when the curated PGx marker components are incomplete and identify the clinical inputs required before a drug-specific phenotype or prescribing decision can be considered. |
| `phenotype_prompts.json` | 27.8 KB | 709 | Curated marker pack `phenotype_prompts`: Phenotype Prompt Library. |
| `prs_registry.json` | 3.2 KB | 133 | Curated marker pack `prs_registry`: PRS Registry. |
| `research_found.json` | 69 B | 4 | Protected pack for research-found drafts only. |
| `research_taxonomy.json` | 10.8 KB | 206 | Shared discovery labels, keyword routing, pack links, consultation hints. |
| `respiratory_airway.json` | 53.3 KB | 1,215 | Curated marker pack `respiratory_airway`: Respiratory, Airway & Oxygenation Context. |
| `runtime_validation_summary.json` | 5.6 KB | 142 | Curated source resource `runtime_validation_summary.json`. |
| `safety_guardrails.json` | 9.0 KB | 215 | Curated marker pack `safety_guardrails`: Global Safety Guardrails. |
| `skin_hair_dermatology.json` | 56.1 KB | 1,312 | Curated marker pack `skin_hair_dermatology`: Skin, Hair & Dermatology Context. |
| `sleep.json` | 70.3 KB | 1,695 | Curated marker pack `sleep`: Sleep & Circadian Rhythms. |
| `source_registry.json` | 66.0 KB | 981 | Registered citation sources for packs and support resources. |
| `supplement_safety.json` | 17.9 KB | 344 | Support resource `supplement_safety`: Keep genotype-informed supplement prompts food-first, lab-aware, medication-aware, and bounded by nutrient safety limits. |
| `thyroid_autoimmune.json` | 108.2 KB | 2,466 | Curated marker pack `thyroid_autoimmune`: Thyroid Hormone & Autoimmune Risk. |
| `user_diet_profile_schema.json` | 3.6 KB | 159 | Curated marker pack `user_diet_profile_schema`: User Diet Profile Schema. |

## `src/lib/research`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `liveProgress.svelte.ts` | 8.5 KB | 232 | Shared live sweep progress from Tauri `research:progress` events. |
| `mergeJobFromProgress.test.ts` | 1.9 KB | 65 | Tests for `mergeJobFromProgress`. |
| `mergeJobFromProgress.ts` | 1.9 KB | 60 | Merge a research:progress event into the UI job snapshot. |
| `researchEvents.ts` | 1.5 KB | 50 | Central Tauri event bridge for vector research progress + debug logs. |

## `src/lib/styles`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `bootstrap-animations.css` | 2.9 KB | 148 | Keyframes and motion tokens for the bootstrap splash screen. |
| `print.css` | 6.2 KB | 268 | External stylesheet `print.css`. |
| `printReport.styles.test.ts` | 1.8 KB | 40 | CSS/layout tests for `printReport`. |
| `reportLayout.styles.test.ts` | 5.7 KB | 119 | CSS/layout tests for `reportLayout`. |
| `reportSeverityTheme.test.ts` | 3.6 KB | 82 | Tests for `reportSeverityTheme`. |
| `sidebarTheme.test.ts` | 7.2 KB | 147 | Tests for `sidebarTheme`. |
| `theme.css` | 137.4 KB | 5,671 | Centralized CSS stylesheet for Genomics Caddy glassmorphic theme. |
| `themeControls.styles.test.ts` | 5.9 KB | 144 | CSS/layout tests for `themeControls`. |

## `src/lib/styles/components`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `advanced-settings-section.css` | 5.6 KB | 249 | External stylesheet `advanced-settings-section.css`. |
| `agent-research-panel.css` | 2.8 KB | 164 | External stylesheet `agent-research-panel.css`. |
| `ai-assistant-panel.css` | 294 B | 12 | External stylesheet `ai-assistant-panel.css`. |
| `connections-panel.css` | 8.2 KB | 450 | Advanced → Connections: Ollama + vector DB setup, host probe, model ops. |
| `discovery-panel.css` | 10.9 KB | 590 | Catalog discovery — premium findings browser. |
| `empty-state.css` | 3.0 KB | 168 | External stylesheet `empty-state.css`. |
| `evidence-corpus-overview.css` | 2.3 KB | 139 | External stylesheet `evidence-corpus-overview.css`. |
| `evidence-library-panel.css` | 14.0 KB | 675 | External stylesheet `evidence-library-panel.css`. |
| `findings-navigator.css` | 4.5 KB | 270 | External stylesheet `findings-navigator.css`. |
| `genome-map.css` | 18.8 KB | 947 | Chromosome Map — proportional karyotype visualization. |
| `local-reference-browser.css` | 6.2 KB | 327 | Local reference catalog browser (ClinVar / PharmGKB / ClinGen / GWAS / MANE). |
| `research-connection-card.css` | 6.4 KB | 358 | External stylesheet `research-connection-card.css`. |
| `research-job-controls.css` | 13.8 KB | 717 | External stylesheet `research-job-controls.css`. |
| `research-live-log.css` | 2.2 KB | 114 | External stylesheet `research-live-log.css`. |
| `research-panel.css` | 4.9 KB | 246 | External stylesheet `research-panel.css`. |
| `research-scope-section.css` | 5.9 KB | 325 | External stylesheet `research-scope-section.css`. |
| `sidebar.css` | 19.5 KB | 1,016 | Sidebar chrome (profiles, reference-DB controls, progress, sync callouts). |
| `update-banner.css` | 1.2 KB | 64 | Update banner and app-shell stack layout using theme tokens. |
| `variant-browser.css` | 6.0 KB | 336 | Raw genotype + reference catalog browser — premium chrome. |
| `vector-workbench.css` | 12.4 KB | 725 | Vector Research workbench: viewer, pack drafts, secondary tabs. |

## `src/lib/types`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `agent.ts` | 2.5 KB | 104 | TypeScript interface definitions for Genomics Research Agent and Evidence-Grade Validation. |
| `api.ts` | 1.8 KB | 77 | Typed IPC payloads for Tauri commands in tauri.ts. |
| `genomics.ts` | 15.8 KB | 560 | Centralized TypeScript interface definitions for genetic data structures. |
| `research.ts` | 22.9 KB | 883 | TypeScript types for the Qdrant research loop and autonomous marker enrichment. |

## `src/lib/utils`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `actionabilityEngine.test.ts` | 64.1 KB | 1,516 | Tests for `actionabilityEngine`. |
| `actionabilityEngine.ts` | 79.2 KB | 2,049 | Derive dashboard diet / supplement / lab / activity / medication-safety guidance and top findings from a report. |
| `activityContext.ts` | 2.6 KB | 65 | Frontend logic `activityContext`. |
| `agentApis.ts` | 13.3 KB | 361 | Frontend logic `agentApis`. |
| `agentUiBridge.ts` | 51.6 KB | 1,249 | Dev/agent UI control surface for MCP and automated QA. |
| `agentUiBridge.visibility.test.ts` | 15.8 KB | 246 | Tests for `agentUiBridge.visibility`. |
| `aiAssistantExportActions.ts` | 2.6 KB | 82 | Clipboard copy and markdown export actions for the AI consultation export modal. |
| `aiAssistantModelActions.ts` | 2.3 KB | 73 | Ollama model scan and detail loading for AiAssistantPanel. |
| `aiAssistantPreferences.ts` | 2.6 KB | 61 | Load persisted AI consultation UI preferences from localStorage. |
| `aiAssistantSendActions.ts` | 7.0 KB | 197 | Consultation prompt send/stop actions for AiAssistantPanel. |
| `aiAssistantSessionActions.test.ts` | 2.0 KB | 62 | Tests for `aiAssistantSessionActions`. |
| `aiAssistantSessionActions.ts` | 2.5 KB | 73 | Consultation session load/create/delete actions for AiAssistantPanel. |
| `aiAssistantVectorDiagnostics.ts` | 927 B | 29 | Vector research diagnostics loader for AiAssistantPanel. |
| `aiExport.test.ts` | 3.3 KB | 95 | Tests for `aiExport`. |
| `aiExport.ts` | 11.1 KB | 215 | Frontend logic `aiExport`. |
| `aiPrompt.test.ts` | 12.5 KB | 306 | Tests for `aiPrompt`. |
| `aiPrompt.ts` | 22.8 KB | 642 | Frontend logic `aiPrompt`. |
| `atlasColors.ts` | 939 B | 41 | Category → color mapping for the Vector Atlas scatter plot. |
| `auditMarkerSemantics.test.ts` | 1.7 KB | 41 | Tests for `auditMarkerSemantics`. |
| `callability.test.ts` | 2.1 KB | 41 | Tests for `callability`. |
| `callability.ts` | 6.9 KB | 172 | Frontend logic `callability`. |
| `catalogCategoryRouting.test.ts` | 2.4 KB | 64 | Tests for `catalogCategoryRouting`. |
| `catalogCategoryRouting.ts` | 3.6 KB | 107 | Dynamic routing for the research discovery catalog. |
| `chatParser.test.ts` | 1.9 KB | 52 | Tests for `chatParser`. |
| `chatParser.ts` | 8.3 KB | 238 | Frontend logic `chatParser`. |
| `chatSession.ts` | 1.6 KB | 60 | Consultation Chat Session Serialization & Persistence Helpers. |
| `clinicalPresentation.test.ts` | 850 B | 21 | Tests for `clinicalPresentation`. |
| `clinicalPresentation.ts` | 734 B | 17 | Keep the structured Clinical table actionable without making long follow-up lists widen or dominate a row. |
| `conditionEvidence.test.ts` | 9.1 KB | 259 | Tests for `conditionEvidence`. |
| `conditionEvidence.ts` | 20.1 KB | 505 | Frontend logic `conditionEvidence`. |
| `consultationChat.ts` | 9.7 KB | 309 | Ollama consultation streaming: RAG retrieval, primary response, optional safety review. |
| `consultationSession.svelte.ts` | 3.6 KB | 92 | Manage active and historical chat sessions reactively. |
| `contentQualityAudit.test.ts` | 8.1 KB | 213 | Tests for `contentQualityAudit`. |
| `cycleDiary.test.ts` | 2.0 KB | 55 | Tests for `cycleDiary`. |
| `cycleDiary.ts` | 3.3 KB | 92 | Resource-backed local diary for explicitly selected menstrual/cycle contexts. |
| `cycleDiaryReview.test.ts` | 2.9 KB | 62 | Tests for `cycleDiaryReview`. |
| `cycleDiaryReview.ts` | 7.7 KB | 202 | Produce a bounded, observation-only summary of the local cycle diary. |
| `desktopContextMenu.test.ts` | 1.8 KB | 38 | Tests for `desktopContextMenu`. |
| `desktopContextMenu.ts` | 3.0 KB | 96 | Frontend logic `desktopContextMenu`. |
| `dialogState.svelte.ts` | 2.0 KB | 76 | Manage global alert and confirm dialog states reactively for Svelte 5. |
| `dialogState.test.ts` | 1.8 KB | 59 | Tests for `dialogState`. |
| `evidence.test.ts` | 4.3 KB | 112 | Tests for `evidence`. |
| `evidence.ts` | 9.9 KB | 243 | Evidence tier, direction, and severity display utility. |
| `evidenceSearch.ts` | 4.3 KB | 142 | Evidence library search orchestration (SQLite, Qdrant, workbench hybrid, corpus browse). |
| `findingIdentity.test.ts` | 8.0 KB | 235 | Tests for `findingIdentity`. |
| `findingIdentity.ts` | 12.8 KB | 365 | Canonical finding identity and topic grouping. |
| `findingsCatalog.ts` | 2.9 KB | 86 | Non-AI findings catalog: presets, labels, and sort options for individuals vs clinicians. |
| `findingSemantics.test.ts` | 6.5 KB | 159 | Tests for `findingSemantics`. |
| `findingSemantics.ts` | 4.7 KB | 118 | Frontend logic `findingSemantics`. |
| `genotype.test.ts` | 1.2 KB | 32 | Tests for `genotype`. |
| `genotype.ts` | 2.0 KB | 57 | Genotype extraction and normalization utility functions. |
| `image.ts` | 2.3 KB | 74 | Loads an image file, resizes it using HTML5 Canvas to a maximum dimension of 1024px, and converts it to a standard base64 JPEG format to prevent model context clutter. |
| `importProgress.test.ts` | 1.8 KB | 37 | Tests for `importProgress`. |
| `importProgress.ts` | 4.5 KB | 139 | Import progress vocabulary shared by the desktop import orchestrator and its full-screen pending view. |
| `layperson.test.ts` | 23.2 KB | 520 | Tests for `layperson`. |
| `layperson.ts` | 37.4 KB | 823 | Provide clear, plain-English translations for genetic marker impacts and meanings. |
| `markerPacksState.svelte.ts` | 1.9 KB | 67 | Frontend logic `markerPacksState.svelte`. |
| `offlineUpdates.test.ts` | 4.6 KB | 119 | Tests for `offlineUpdates`. |
| `offlineUpdates.ts` | 5.3 KB | 142 | Build a clear, named list of offline assets that need updates. |
| `ollamaSettings.test.ts` | 2.2 KB | 64 | Tests for `ollamaSettings`. |
| `ollamaSettings.ts` | 2.9 KB | 83 | Shared Ollama connection URL persistence for AI consultation and research panels. |
| `ollamaStream.isolation.test.ts` | 892 B | 25 | Tests for `ollamaStream.isolation`. |
| `ollamaStream.test.ts` | 622 B | 21 | Tests for `ollamaStream`. |
| `ollamaStream.ts` | 1.8 KB | 70 | Namespaced Ollama stream events — each stream uses a unique ID so concurrent consultation and agent runs cannot cross-contaminate chunks. |
| `pageBootstrap.ts` | 3.1 KB | 93 | Application startup bootstrap sequence for the main dashboard page. |
| `pageSampleHandlers.report.test.ts` | 11.3 KB | 336 | Tests for `pageSampleHandlers.report`. |
| `pageSampleHandlers.ts` | 12.8 KB | 395 | Sample import, deletion, and report generation handlers for the main dashboard page. |
| `personalSafetyContext.test.ts` | 2.5 KB | 78 | Tests for `personalSafetyContext`. |
| `personalSafetyContext.ts` | 5.1 KB | 150 | Structured, user-supplied context that can change how genomic findings are interpreted safely. |
| `pgxCoverage.ts` | 4.6 KB | 121 | Frontend logic `pgxCoverage`. |
| `presentationPreferences.test.ts` | 1.1 KB | 31 | Tests for `presentationPreferences`. |
| `presentationPreferences.ts` | 1.1 KB | 29 | Small, defensive preference boundary for report presentation mode. |
| `primaryCatalogs.ts` | 646 B | 19 | Shared definition of the four primary reference catalogs shown in the UI. |
| `profileContext.test.ts` | 7.2 KB | 159 | Tests for `profileContext`. |
| `profileContext.ts` | 9.7 KB | 292 | Canonical, profile-scoped user context. |
| `promptTemplates.test.ts` | 1.5 KB | 34 | Tests for `promptTemplates`. |
| `promptTemplates.ts` | 658 B | 9 | Render a resource-authored prompt template without silently dropping data. |
| `prsReadiness.ts` | 819 B | 28 | Frontend logic `prsReadiness`. |
| `qdrantRag.ts` | 4.8 KB | 130 | Format Qdrant vector hits for AI Consultation system prompts and UI citations. |
| `reportAudienceExport.test.ts` | 9.7 KB | 260 | Tests for `reportAudienceExport`. |
| `reportAudienceExport.ts` | 28.8 KB | 658 | Frontend logic `reportAudienceExport`. |
| `reportBundleExport.test.ts` | 11.6 KB | 326 | Tests for `reportBundleExport`. |
| `reportBundleExport.ts` | 9.3 KB | 232 | Frontend logic `reportBundleExport`. |
| `reportOverview.test.ts` | 3.6 KB | 130 | Tests for `reportOverview`. |
| `reportOverview.ts` | 2.4 KB | 68 | Unique, called signals that have a meaningful review or context route. |
| `reportReferences.test.ts` | 3.8 KB | 121 | Tests for `reportReferences`. |
| `reportReferences.ts` | 5.0 KB | 160 | Frontend logic `reportReferences`. |
| `reportStatuses.test.ts` | 2.1 KB | 56 | Tests for `reportStatuses`. |
| `reportStatuses.ts` | 2.4 KB | 78 | Report status compatibility helpers. |
| `reportTemplate.ts` | 1.5 KB | 43 | Builds the merged marker-pack report template for Tauri report generation. |
| `reproductiveContext.test.ts` | 10.9 KB | 222 | Tests for `reproductiveContext`. |
| `reproductiveContext.ts` | 13.9 KB | 355 | Frontend logic `reproductiveContext`. |
| `reproductiveIntake.test.ts` | 3.0 KB | 67 | Tests for `reproductiveIntake`. |
| `reproductiveIntake.ts` | 2.8 KB | 75 | Resource-backed normalization for optional cycle and hormone context. |
| `researchConnection.ts` | 2.1 KB | 56 | Helpers for Vector Research connection checks (localhost vs remote, invoke timeouts). |
| `researchJobMetrics.bootstrap.test.ts` | 993 B | 27 | Tests for `researchJobMetrics.bootstrap`. |
| `researchJobMetrics.ts` | 13.2 KB | 374 | Pure helpers for research sweep job progress UI (speed, ETA, pipeline phases). |
| `researchRunReadiness.test.ts` | 4.1 KB | 140 | Tests for `researchRunReadiness`. |
| `researchRunReadiness.ts` | 6.3 KB | 174 | Central readiness rules for vector research sweep actions. |
| `researchScopeHelpers.ts` | 2.5 KB | 86 | Pure helpers for research sweep scope configuration and enrichment source presets. |
| `searchOrchestrator.ts` | 8.2 KB | 233 | Search Orchestration Engine for the AI Assistant. |
| `severityGlyphs.ts` | 1.5 KB | 59 | Shared severity / association glyphs for Map, Report, and Browser. |
| `sharedInterpretations.test.ts` | 1.0 KB | 30 | Tests for `sharedInterpretations`. |
| `sharedInterpretations.ts` | 1.2 KB | 37 | Group exact marker interpretations that describe the same family-level context. |
| `supportResourceContext.test.ts` | 28.3 KB | 504 | Tests for `supportResourceContext`. |
| `supportResourceContext.ts` | 21.2 KB | 489 | Selects the evidence, phenotype, laboratory, food-safety, and callability resources that belong in an AI consultation context. |
| `tooltipPosition.test.ts` | 2.1 KB | 68 | Tests for `tooltipPosition`. |
| `tooltipPosition.ts` | 2.9 KB | 88 | Calculate a fixed tooltip panel position without allowing it outside the visible viewport. |
| `uiLabels.test.ts` | 1.1 KB | 21 | Tests for `uiLabels`. |
| `uiLabels.ts` | 796 B | 24 | Normalize user-facing labels for deterministic local UI automation. |
| `updater.test.ts` | 5.2 KB | 148 | Tests for `updater`. |
| `updater.ts` | 4.8 KB | 171 | Quiet GitHub update check, dismissal, and confirm-to-install helpers. |
| `urlSafety.test.ts` | 1.3 KB | 33 | Tests for `urlSafety`. |
| `urlSafety.ts` | 974 B | 26 | Safe URL helpers for dynamic href attributes in the Tauri webview. |
| `userFinding.ts` | 7.2 KB | 173 | Converts normalized evidence cards into user-facing finding summaries. |
| `variantNavigation.ts` | 2.4 KB | 82 | Cross-tab variant navigation helpers for the main dashboard. |
| `viewModels.test.ts` | 3.9 KB | 119 | Tests for `viewModels`. |
| `viewModels.ts` | 7.4 KB | 201 | Derived view-model selector for genomics reports. |
| `warningTaxonomy.test.ts` | 2.1 KB | 49 | Tests for `warningTaxonomy`. |
| `warningTaxonomy.ts` | 3.9 KB | 109 | Presentation policy for caveats carried by genomic resources. |

## `src/routes`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `+layout.ts` | 295 B | 5 | SvelteKit route for the single-page desktop shell. |
| `+page.svelte` | 38.0 KB | 1,049 | SvelteKit route for the single-page desktop shell. |

## `static`

| File | Size | Lines | Role |
| --- | ---: | ---: | --- |
| `favicon.png` | 8.2 KB | — | Webview favicon. |
| `logo.png` | 338.8 KB | — | App logo used for Tauri icon generation and Linux launcher. |
| `svelte.svg` | 1.9 KB | 1 | Static svg asset bundled with the webview. |
| `tauri.svg` | 2.5 KB | 6 | Static svg asset bundled with the webview. |
| `vite.svg` | 1.5 KB | 1 | Static svg asset bundled with the webview. |
