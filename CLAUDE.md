# Genomics Caddy: Claude Code instructions

**`AGENTS.md` is the source of truth for this repository.** Read it in full at
the start of every session and follow it exactly. It covers the privacy
boundary, the liftover gate, marker-pack synchronization, evidence/clinical
claim rules, the dependency (lock-free) policy, and the required verification
commands. Do not duplicate its rules here; this file only adds how Claude
should treat it.

## Start of session

1. `git status` (preserve unrelated dirty changes), then `AGENTS.md`,
   `README.md`, `ARCHITECTURE.md`.
2. Read local `MEMORY.md` if present. It is untracked: never commit it.
3. Check `TASKS.md` for the shared backlog, then read only files relevant to
   the task.

## Keep AGENTS.md healthy

Treat `AGENTS.md` as living documentation, not a one-time artifact. Update it
in the same change whenever work alters something it describes:

- A new or changed invariant (privacy, evidence tiers, liftover/import gates,
  pack mirroring, discovery rules, download/resume behavior).
- A renamed, added, or removed script, audit, or verification command (verify
  against `package.json` scripts rather than from memory).
- Changes to CI, release, updater, or dependency/lockfile policy.
- A lesson from a mistake or correction that a future agent would repeat.

Rules for editing it:

- Verify any file, script, or flag you mention actually exists before writing it.
- Remove or rewrite statements that have become false; do not leave stale rules.
- Keep entries concise and imperative, grouped under the existing headings.
- Never add genotype values, secrets, tokens, or personal data.
- Mention in your final summary when `AGENTS.md` was changed and why.

If `AGENTS.md` and the code disagree, say so and resolve it (fix the doc, or
flag the code), rather than silently following one.

## Other docs to keep current

- `README.md` for externally visible behavior.
- `ARCHITECTURE.md`: regenerate (`scripts/generate_architecture.py`) after
  file-layout changes.
- `TASKS.md` for durable backlog; local `MEMORY.md` gets a concise handoff
  after meaningful work.

## Claude-specific reminders

- Raw DNA fixtures (`*DNA*.txt`, `*_report*.json`, zips in the repo root) are
  private: never open, print, or quote genotype values.
- Stop `tauri:dev` before `pnpm test`, `pnpm run check`, or `pnpm run build`.
- Commit or push only when asked; GitHub PRs and `master` pushes must pass
  `.github/workflows/ci.yml`.
