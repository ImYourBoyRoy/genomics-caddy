---
name: vector-research-sweep-qa
description: >-
  Genomics Caddy Vector Research sweep QA: start/pause/resume/cancel races,
  agent UI bridge (dynamic 127.0.0.1 port), SQLite research_jobs assertions, and known
  cancel wind-down pitfalls. Use when testing or fixing Cancel Sweep, Pause,
  Resume, Run Controller progress, RESEARCH_RUNNING/CANCELLED flags, or
  research:progress UI races.
---

# Vector Research sweep QA

Project skill for Genomics Caddy. Read this before diagnosing or regressing sweep control bugs.

## When to use

- User reports Cancel → Resume/paused, Start stuck, or “already running”
- Editing `research/{state,job,sweep,commands}.rs` or Run Controller Svelte
- Manual / agent UI verification of Start → Pause → Resume → Cancel

## Preconditions

1. Kill stale `tauri`/`vite` processes; start fresh: `npm run tauri:dev` (log to `/tmp/tauri-cancel-test.log` if useful).
2. Wait for Agent UI: `pnpm run agent-ui:url` then `curl -s http://127.0.0.1:<port>/health` → `"ok":true`. Never use `localhost` or guess `:17321`.
3. Sample selected (default sample id `1`); Vector Research tab open.
4. Qdrant + Ollama live. Prefer **Test Connections** before Start.
5. Wait until Start is clickable (scope preview non-empty, e.g. `83,820 queued`). Scope recount can take several seconds after boot/HMR.

## Agent UI control plane

| Call | Purpose |
|------|---------|
| `GET /health` | Bridge up + window count |
| `POST /ui/setTab` `{"tab":"research"}` | Open Vector Research |
| `POST /ui/clickText` `{"text":"…"}` | Click enabled control (skips disabled) |
| `POST /ui/queryText` `{"text":"…"}` | Count visible text hits (substring; noisy for short needles like `Start`) |

Prefer exact labels: `Start Autonomous Sweep`, `Pause Sweep`, `Resume Sweep`, `Cancel Sweep`.

## SQLite truth

```bash
sqlite3 App/Data/samples/1/genome.db \
  "SELECT job_id,status,error_message,enriched_count,total_markers
   FROM research_jobs ORDER BY started_at DESC LIMIT 1;"
```

| Expected after | `status` | UI |
|----------------|----------|-----|
| Cancel settled | `idle` + `Sweep cancelled by user.` | **Start** only (no Resume) |
| Pause settled | `paused` | **Resume** + Cancel |
| Resume / Start active | `running` | Pause + Cancel |
| Cancel mid-flight | may briefly stay `running` then `idle` | Cancelling… / wind-down |

**Fail if** cancel leaves `paused` or shows **Resume Sweep**.

## Required E2E sequences

Run against live GUI + DB (not unit tests alone).

### A — Cancel

1. Start → wait until DB `running` **and** Cancel visible.
2. Confirm status does **not** flip to `paused` before Cancel.
3. Cancel → poll until `idle` + Start visible + Resume count `0`.
4. Assert no `paused` in the cancel status sequence.

### B — Pause / Resume / Cancel

1. Start → `running`.
2. Pause → `paused` + Resume visible.
3. Resume → `running` within a few seconds (early running save on resume preflight).
4. Cancel → `idle` + Start only.

### C — Early cancel

1. Start → Cancel within ~1s (bootstrap/preflight).
2. Settle `idle` without sticky Resume. Retry Cancel if button appears only after `running`.

## Known race map (do not reintroduce)

| Bug | Mechanism | Guard |
|-----|-----------|-------|
| Start → instant `paused` | Poll reads old `idle`/`paused` → `clear_stale_running_flag` clears `RESEARCH_RUNNING` → normalize writes `paused` | Never clear RUNNING on `idle`/`paused` (only `complete`/`error`) |
| Cancel → Resume | Cancel shared pause latch / late `paused` progress / normalize | `RESEARCH_CANCELLED`; cancel→`idle`; drop late running/paused emits; merge ignores resurrecting settled jobs |
| Start right after Cancel → “already running” | UI forced `loop_active: false` while worker still winding down | Keep backend `loop_active` during wind-down; block Start while `idle`+`loop_active`+cancel message |
| Start disabled forever | Scope preview `Counting…` with empty preview, or recount after cancel | Allow Start when `total_unique > 0` even if recount loading; clear previewLoading if sweepRunning |
| Resume silent no-op | `ensureConnectionsBeforeRun()` returned false without log | Must pushLog why resume blocked |

## Key files

| Area | Path |
|------|------|
| Flags | `src-tauri/src/research/state.rs` |
| Normalize / cancel DB | `src-tauri/src/research/job.rs` |
| Loop stop / emit | `src-tauri/src/research/sweep.rs` |
| Commands | `src-tauri/src/research/commands.rs` |
| Progress merge | `src/lib/research/mergeJobFromProgress.ts` |
| Run readiness | `src/lib/utils/researchRunReadiness.ts` |
| Cancel/resume UI | `src/lib/components/research/ResearchPanel.svelte` |
| Agent bridge | `src/lib/utils/agentUiBridge.ts`, `src-tauri/src/agent_ui.rs` |

## Unit checks (fast)

```bash
npx vitest run src/lib/research/mergeJobFromProgress.test.ts src/lib/utils/researchRunReadiness.test.ts
cd src-tauri && cargo test research::state::tests -- --nocapture
npm run cargo:clippy
npm run check
```

## Pass criteria

- Cancel never lands on `paused` / Resume.
- After cancel settles: DB `idle`, Start enabled, Cancel/Resume gone.
- Pause → Resume returns to `running`; Cancel from either state → `idle`.
- No “already running” immediately after a settled cancel (wait for wind-down if Start is briefly blocked).
- During Qdrant bootstrap: hero % matches “already in Qdrant” baseline (not partial chunk discoveries); batch bar shows chunks scanned, not fake 100% upsert.
