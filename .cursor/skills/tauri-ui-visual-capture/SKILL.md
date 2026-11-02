---
name: tauri-ui-visual-capture
description: >-
  Genomics Caddy visual QA: relaunch the Tauri debug app, capture a timed series
  of window PNGs through the loopback agent UI bridge (/ui/capture), and tile
  them into a contact sheet to diagnose startup flicker, double loading, phase
  regressions, choppy animation, and layout problems. Use when the user says the
  UI looks wrong, loading restarts, animation is choppy, or asks for visual
  references from inside the app.
---

# Tauri UI visual capture timeline

Evidence loop for anything the user "sees" in the desktop window. Read the frames
before changing code; re-capture after the fix.

## Fast path (loader overlay only)

When the question is loader layout / telemetry (not native WebKit paint or
real Sync All Missing), use the permanent Vite fixture first:

```bash
pnpm run loading-lab
# scenarios: ?s=res-scan|res-download|res-index|startup|import
```

`src/lib/dev/loadingLabScenarios.ts` feeds the real `BootstrapOverlay`.
Dev-gated; no DNA, no downloads. Then run the native capture below for
Tauri/WebKit confirmation.

## Workflow

1. Stop any running instance and `tauri:dev` (they fight over the bridge and HMR).
2. Launch the debug binary with a disposable data dir and the sample fixture only:
   ```bash
   GENOMICS_DATA_DIR=$HOME/.cache/gc-visual-audit \
   GENOMICS_AGENT_UI_ALLOW_UNAUTHENTICATED=1 \
   src-tauri/target/debug/DNA-Tools &
   ```
3. Immediately capture (the script waits up to 30 s for the endpoint file):
   ```bash
   node scripts/ui_capture_timeline.mjs --frames 24 --interval-ms 400 \
     --out /tmp/gc-startup --sheet
   ```
4. Open `/tmp/gc-startup/contact-sheet.png` and `timeline.json`. Look for:
   - checklist/phase resetting backwards (a ticker re-firing `onPhase`)
   - progress percent jumping or stalling vs backend `[CMD_GENERATE_REPORT]` logs
   - a frame with an empty/white window (webview not yet painted)
   - layout shift between consecutive frames
5. Fix, rebuild, repeat with a new `--out` so before/after sheets can be compared.
6. Interact via the bridge when needed: `POST /ui/clickText`, `/ui/resize`,
   `GET /ui/snapshot`. Narrow layout: `GENOMICS_TAURI_AUDIT_WIDTH=640 GENOMICS_TAURI_AUDIT_HEIGHT=720`.
7. Finish with `pnpm run audit:tauri-ui` (default and 640x720).

## Rules

- Discover the URL from `pnpm run agent-ui:url`; never use `localhost` or guess 17321.
- Frames can show genotype-derived UI. Keep them in `/tmp`, never commit them,
  never paste values into chat or `MEMORY.md`. Use the sample fixture only.
- Capture is Linux/WebKitGTK verified; do not generalize to Windows/macOS.
- Frame cadence is limited by capture cost (~100-300 ms); use it for phase and
  layout evidence, not for judging frame-rate smoothness (see the global
  `css-animation-performance` skill for that).
