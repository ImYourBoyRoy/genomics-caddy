# Development

For product usage see the [root README](../../README.md) and
[../usage/README.md](../usage/README.md). This page is for people and agents
changing the repo.

## Rules of the road

- Stay in this repository unless a task names an external dependency.
- Never print, log, or commit raw genotype values or user DNA files.
- `src/lib/marker-packs/` is curated source. Keep
  `src-tauri/App/Data/marker-packs/` mirrored. Never delete a pack for
  cleanup.
- Lockfiles are intentionally absent. Install with
  `node ./scripts/pnpm_unlocked.mjs install`, not a committed `pnpm-lock.yaml`.
- Full agent workflow: [AGENTS.md](../../AGENTS.md).

## Install

```bash
node ./scripts/pnpm_unlocked.mjs install
```

Use that Node entry when `node_modules` is missing. `pnpm run` can auto-install
and leave a lockfile.

## Everyday commands

| Command | Purpose |
| --- | --- |
| `pnpm run tauri:dev` | Desktop app (real DNA import) |
| `pnpm run dev` | Vite preview only; no Tauri import |
| `pnpm run mcp` | Headless MCP (see [../mcp/README.md](../mcp/README.md)) |
| `pnpm test` | Frontend tests |
| `pnpm run check` | Svelte / TypeScript |
| `pnpm run build` | Frontend production bundle |
| `pnpm run cargo:test` | Rust tests |
| `pnpm run cargo:clippy` | Strict Clippy |
| `pnpm run validate:packs` | Marker-pack and support-resource schemas |
| `pnpm run audit:markers` | Marker semantics; no genotype print |
| `pnpm run audit:resources` | Pack/resource quality |
| `pnpm run audit:content` | Copy quality; no DNA fixtures |
| `pnpm run audit:dna-fixtures` | Local ignored DNA files; counts only |
| `node ./scripts/audit_reproducibility.mjs` | Lock-free install contract |
| `pnpm run smoke` | Optional Ollama/Qdrant/NCBI reachability |

Resource or marker-pack edits should run the pack/resource/DNA-fixture audits
plus `pnpm test`, `pnpm run check`, `pnpm run build`, `pnpm run cargo:test`,
and `git diff --check`.

Desktop UI fixture audit needs a running Tauri window and
`GENOMICS_AGENT_UI_TOKEN`. See `pnpm run audit:tauri-ui`. Do not set
`GENOMICS_AGENT_UI_ALLOW_UNAUTHENTICATED=1` except for disposable local QA.

## `.env`

Copy `.env.example` → `.env`. UI-saved Connections override `.env`. Never
commit `.env`. Details: [../ai-chat/README.md](../ai-chat/README.md).

## Layout (agents)

```
src/lib/marker-packs/          curated packs and support JSON
src-tauri/App/Data/marker-packs/  required runtime mirror
src-tauri/testdata/            synthetic import fixtures only
src-tauri/src/                 Rust: parser, liftover, db, report, MCP
src/lib/components/            Svelte UI
docs/                          human and AI operator docs
ARCHITECTURE.md                file-by-file size, lines, and role
```

Generated and private (gitignored): `App/`, `builds/`, `.env`, raw DNA,
reports, local SQLite, `MEMORY.md`.

File map: [../../ARCHITECTURE.md](../../ARCHITECTURE.md). Refresh it with
`python3 ./scripts/generate_architecture.py`.

Deep clean (preview first): `node ./scripts/clean_deep.mjs --dry-run`.
It does not delete `App/Data` or marker-pack trees.

## Dependency bumps

```bash
pnpm run update:all:dry
node ./scripts/update_all.mjs
```

No `--locked` Cargo flags. After manifest changes run
`node ./scripts/audit_reproducibility.mjs`.

## Publishing

Push only `master` to GitHub:

```bash
git push -u origin master
```

Do not `git push --mirror`. Local `refs/codex/*` recovery refs are not part of
the public history and must not be pushed.
