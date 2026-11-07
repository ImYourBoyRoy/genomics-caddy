# Genomics Caddy project instructions

## Boundary and purpose

This repository contains the local Tauri/Rust + Svelte DNA analysis tool. Stay
inside this repository unless a task identifies a concrete external dependency.
Human and agent operator docs: root `README.md` (download and first launch), `docs/`, and
`ARCHITECTURE.md` (file map). Durable shared backlog lives in `TASKS.md`; local
session notes live in `MEMORY.md` and must stay untracked.

Raw DNA fixtures and local app data are private; do not print genotype values,
copy them into logs, or commit secrets.

The Server_Manager WinServer guest now provides a separate native Windows
compilation lane. This proves source/dependency/build compatibility only; it
does not establish native UI launch, accessibility, installer signing, or
release acceptance. Do not generalize the existing Linux evidence to those
surfaces.

Generated release staging under repository-root `builds/` and `App/` is local
output and must remain untracked. Ignore app databases and SQLite sidecars in
any `App/Data/` tree. The curated runtime mirror at
`src-tauri/App/Data/marker-packs/` is source-managed and must remain tracked.

Remote Linux and Windows builds use the sibling `Remote_Build` toolkit's
`remote_build.cache` policy. This project declares only its exact
`src-tauri/target` and protected checkout; do not replace that with a global
Cargo, `/dev/shm`, or checkout-wide purge. The macOS guest retains its
platform-specific artifact-preserving cleanup.

## Resource workflow

- A successfully parsed GRCh37-to-GRCh38 liftover chain is a hard prerequisite
  before reading/importing a user's DNA or running any sample-based report,
  query, discovery, or research job. Validate the chain contents; file presence
  alone is not sufficient. Enforce the gate in the native command layer as well
  as the UI. Startup may load profile metadata but must not generate a DNA
  report before the validated chain is available.
- Reference catalog downloads may run before liftover or DNA import. The local
  genotype-derived variant index must be skipped until liftover validates, and
  the UI must clearly offer the missing catalog downloads.
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
- Full-genome ClinVar condition discovery joins the local variant summary to
  condition-specific submission assertions. Require a known source build and
  allele orientation, preserve the exact `association_is`, SCV/review/source
  provenance, distinguish variant-wide summaries from condition assertions,
  exclude explicitly somatic/oncogenic records, and never serialize genotype
  alleles in the discovery summary.
- Discovery groups (`may_be_relevant`, `carrier`, `unclear`) combine the
 matched copy count with ClinGen Definitive/Strong/Moderate inheritance and
 require a non-conflicting variant-wide Pathogenic/Likely pathogenic summary;
 benign-consensus variants are dropped and a lone per-submission call never
 promotes a condition. Serialize only the copy count, never bases. Do not infer
 phase or penetrance. Unstated vendor strand may be verified only against
 reference alleles at strand-unambiguous sites. POTS/dysautonomia remain
 clinical-context routes unless validated consumer-array evidence exists.
- Download resumes are safe only when the saved object identity and returned
  byte range validate. Progress rates exclude resumed bytes; unknown sizes or
  row-index imports must not display fabricated percentages or ETAs.
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
direct dependencies use stable version ranges in `package.json`; npm/pnpm and
Cargo lockfiles must not be tracked or committed. The audit preserves existing
ignored local lockfiles. Rust validation/release commands must not use
`--locked`. Use `node ./scripts/pnpm_unlocked.mjs install` for frontend
dependency resolution; pnpm 12's resolver lock is kept in a temporary directory
outside the repository and removed on exit. Run
`node ./scripts/audit_reproducibility.mjs` when changing manifests or release
workflow; it checks version-range policy, untracked/ignored lockfiles, and
unlocked Cargo metadata. SvelteKit 3's current tooling requires TypeScript 6;
keep the TypeScript range within 6.x until the project explicitly migrates.

AI and clinician technical handoffs always include the exact raw calls used by
their findings; the deprecated profile export toggles must not be reintroduced
as suppression controls. Connected AI Chat includes raw calls for the findings
selected by its persisted context mode and must disclose the destination when
the Ollama endpoint is remote or when the ChatGPT plan provider is selected
(OpenAI `api.openai.com`). ChatGPT uses browser OAuth + PKCE with no client
secret and no API key; tokens stay in the OS credential manager and never
enter the webview. Do not invent ChatGPT env secrets for release. Session context mode is persisted with chat
history so reopening a session cannot silently change its genotype scope.

GRCh37 and explicit GRCh38 imports are supported under the parser's
1-based-inclusive contract. GRCh37 sources map forward when the local chain is
available; GRCh38 sources retain their source coordinates and use validated
inverse mapping for GRCh37 when available, leaving unmapped values explicit.

For report/sidebar or other desktop UI changes, also run `pnpm run audit:tauri-ui`
while the Tauri window is running. The audit discovers `http://127.0.0.1:<port>`
from the user-only runtime endpoint file (or `GENOMICS_AGENT_UI_URL` if that URL
is loopback). Do not use `localhost` or guess port 17321. The debug bridge
requires a session token for `/ui/*`; use
`GENOMICS_AGENT_UI_ALLOW_UNAUTHENTICATED=1` only for disposable debug QA.
To exercise the narrow responsive layout, run
`GENOMICS_TAURI_AUDIT_WIDTH=640 GENOMICS_TAURI_AUDIT_HEIGHT=720 pnpm run audit:tauri-ui`;
the audit opens and closes **Data controls** before checking sidebar profile
labels when the mobile layout hides the sidebar.
Stop `tauri:dev` before running `pnpm test`, `pnpm run check`, or
`pnpm run build`, since SvelteKit sync/build output can trigger transient
HMR/IPC reloads in the live window. Restart Tauri afterward for the native UI
audit.

The fixture audit is read-only and must not emit or persist raw genotype values.
Report source/runtime drift, unverified external behavior, warnings, and
remaining coverage limits explicitly. Update `README.md` for externally
visible behavior. After file-layout changes, regenerate `ARCHITECTURE.md`.
Write a concise handoff to local `MEMORY.md` after meaningful work.
GitHub pull requests and `master` pushes must pass `.github/workflows/ci.yml`.
The Windows downloader contract compiles Rust unit tests with the Common
Controls v6 manifest required by Tauri-linked test executables. Keep those
linker flags scoped to the Windows CI test step; do not add them to the app
build script. Cargo's `rustc-link-arg-tests` applies to integration-test
targets, not in-module unit tests.
Tagged `v*` publishes reuse that gate, then draft signed desktop installers
and `latest.json` for in-app updates. Never commit the updater private key;
GitHub secrets `TAURI_SIGNING_PRIVATE_KEY` and
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` sign release artifacts.
