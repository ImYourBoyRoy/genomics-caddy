# Building and packaging

Frontend-only `pnpm run build` writes static assets to `/build`. DNA import
and reports need the Tauri desktop binary.

## Production desktop build

```bash
pnpm run build:release
```

Windows:

```powershell
pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File .\scripts\purge_and_build.ps1
```

Linux / macOS:

```bash
bash ./scripts/purge_and_build.sh
```

Flags (npm script or native wrappers): `--purge-only`, `--skip-purge`,
`--skip-checks`, `--dry-run`.

Output is a portable tree under **`App/`** with persistence in **`App/Data/`**.
Linux binary: `App/DNA-Tools`. Windows: `App/DNA-Tools.exe`. Cargo artifacts
stay in `src-tauri/target/` and are safe to wipe. `App/Data` is not.

## GitHub tagged release

Push a `v*` tag after `master` CI is green. The tag must match
`package.json` / `src-tauri/tauri.conf.json` (for example `v0.2.0`).
Actions runs the same Validate job as pull requests, then `tauri-action`
drafts **signed** installers on Windows (NSIS preferred for `latest.json`),
Ubuntu, and macOS Universal, and uploads `latest.json` for in-app updates.
Publish the draft from the GitHub Releases UI after you inspect artifacts.

The desktop app checks that endpoint quietly on launch. A dismissible banner
offers the new version; Install asks for confirmation, then downloads the
signed artifact and relaunches. It does not upload DNA or reports.

In-app updates apply to GitHub-published installers. A local portable `App/`
copy and `tauri dev` may check GitHub but are not the supported install path
for applying those artifacts. The first signed release is the baseline;
older unsigned copies will not self-update until someone installs a signed
build once.

Signing uses repo secrets `TAURI_SIGNING_PRIVATE_KEY` and
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. The public key lives in
`src-tauri/tauri.conf.json`. Never commit the private key. If either secret
is missing, the publish job fails instead of shipping unsigned updater
artifacts. Platform jobs run one at a time so `latest.json` can merge.

`pnpm run audit:updater-config` checks this wiring without secrets.

Faster iteration (skip cache purge): `pnpm run build:release-fast`.

## Linux desktop

One-time WebKitGTK 4.1 + GTK headers:

```bash
pnpm run setup:linux
```

Optional dock launcher (not created by running the portable binary):

```bash
pnpm run desktop:linux
pnpm run desktop:linux:refresh   # after a rebuild; quit the app first
pnpm run desktop:linux:uninstall # launcher + WebKit UI cache; keeps App/Data
```

`desktop:linux` registers `builds/linux/DNA-Tools` when that artifact exists.

## Remote / Docker / headless

Cross-compile from a configured builder:

```bash
pnpm run build:remote:all
python scripts/build.py --target all --create-bundle
```

Headless enrichment worker and optional static UI: [../../docker/README.md](../../docker/README.md).

Same binary, no Docker:

```bash
export GENOMICS_DATA_DIR=/path/to/App/Data
./App/DNA-Tools --headless-sweep --sample-id=1
```

## Data migration

Legacy `data/` tree:

```powershell
pwsh -File .\scripts\migrate_data_to_app.ps1
```

## Timing (optional)

```bash
pnpm run bench:purge
pnpm run bench:rebuild
```

Reports write under `App/Data/benchmarks/`.
