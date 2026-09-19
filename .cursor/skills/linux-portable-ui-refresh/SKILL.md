---
name: linux-portable-ui-refresh
description: >-
  Genomics Caddy Linux portable UI refresh: refresh the matching FreeDesktop
  launcher and DNA icon, clear WebKit/CacheStorage/GPU caches, verify
  splash/report assets, and relaunch the current portable build. Use when the
  icon is generic, splash shows helix-only, UI looks stale after rebuild,
  import progress is missing, or user mentions WebKitCache / desktop:linux.
---

# Linux portable UI refresh

Project skill for Genomics Caddy on Linux. Read this before diagnosing
helix-only splash, stale report UI after rebuild, or portable-vs-installed
launcher confusion.

## When to use

- Splash is DNA helix only (no status text / checklist)
- UI looks unchanged after `build:release-fast` / binary copy
- Import progress missing from sidebar; full-screen helix during import
- User asks to remove installed launcher / clear cache / use portable only
- Editing bootstrap overlay, helix backdrop, or Linux desktop scripts

## Hard facts (do not rediscover)

| Fact | Truth |
|------|--------|
| Canonical profile data | Repo `App/Data` (not `builds/linux/App/Data`) |
| Portable binaries | `./App/DNA-Tools` and `./builds/linux/DNA-Tools` |
| Desktop launcher | `pnpm run desktop:linux` installs/refreshes `~/.local/share/applications/com.dna.explorer.desktop` pointing at `App/DNA-Tools` when present (canonical `App/Data`), else `builds/linux/DNA-Tools` |
| Opening portable | Does **not** install a launcher |
| Stale UI after rebuild | Often `~/.local/share/com.dna.explorer/WebKitCache` (+ CacheStorage / GPUCache) |
| No `.deb` required | “Installed” usually means FreeDesktop entry + icons |

## Splash / import contract

- Status panel is **left** with inline live status text (literal colors; no opacity fade-in).
- Helix is a **right-column** slot — never full-screen center replacing status.
- Normal DNA import progress lives in sidebar `GenomeImportPanel`.
- Full-screen `BootstrapOverlay` for import only on **error** (or intentional resource sync).
- After splash/import UI edits: rebuild portable binary, then run the refresh tool before asking the user to relaunch.

Key files:

- `src/lib/components/common/AppBootstrapScreen.svelte`
- `src/lib/components/common/bootstrap/BootstrapOverlay.svelte`
- `src/lib/components/common/bootstrap/BootstrapHelixBackdrop.svelte`
- `src/lib/components/import/GenomeImportPanel.svelte`
- `src/routes/+page.svelte` (`showImportOverlay`)

## Tool (preferred)

From repo root:

```bash
pnpm run desktop:linux:refresh
# or:
bash ./scripts/refresh_linux_portable_ui.sh
```

Useful flags:

```bash
bash ./scripts/refresh_linux_portable_ui.sh --status   # report only
bash ./scripts/refresh_linux_portable_ui.sh --verify    # also check build/ for splash markers
```

What it does:

1. Refreshes the identifier-matched FreeDesktop launcher and DNA logo from the current portable binary when one is available
2. Clears WebKitCache, CacheStorage, and GPUCache under `~/.local/share/com.dna.explorer` while preserving the launcher/icons
3. Prints which portable binaries exist and how to launch
4. Reminds: fully quit Genomics Caddy before relaunch

Does **not** delete `App/Data` profiles or genomes.
Use `pnpm run desktop:linux:uninstall` only when intentionally removing the dock entry and its user-installed icons.

## Agent workflow after UI rebuild

```text
1. pnpm run build:release-fast   # or copy release DNA-Tools into App/ + builds/linux/
2. Fully quit any running Genomics Caddy process
3. pnpm run desktop:linux:refresh
4. Verify the launcher/icon remain present and point at the current binary
5. Tell user to relaunch from the desktop entry or `./App/DNA-Tools`
```

## Report UX reminders (related stale-cache symptoms)

- Labs open by default: `genomics_dashboard_collapsed_v3_*`
- Marker/rsID refs: `--report-marker-ref-text` (burnt yellow)
- Lab spotlight: `lab-spotlight` under Review first

## Anti-patterns

- Do not tell the user to “just reopen the dock icon” after a splash fix
- Do not reinstall `desktop:linux` as part of a cache fix
- Do not wipe `App/Data` to fix UI cache
- Do not reintroduce opacity/`animation-fill-mode: both` entrance fades on bootstrap status
