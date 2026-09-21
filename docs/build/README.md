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

Do not ship a binary from plain `cargo build --release` unless the
`custom-protocol` feature is enabled (it is the crate default). Without it
the window loads `http://127.0.0.1:1420` and shows Connection refused.
Prefer `pnpm run build:release-fast` or `pnpm run tauri build`.

NSIS **everyone** stores that account's library under
`%LOCALAPPDATA%\Genomics Caddy\Data`. The left-rail **Library** row can
point at another folder. Uninstall asks before deleting this account's
genomes; other Windows accounts are left alone.

## GitHub tagged release

Push a `v*` tag after `master` CI is green. The tag must match
`package.json` / `src-tauri/tauri.conf.json` (for example `v0.2.4`).
Actions runs the same Validate job as pull requests, then
`tauri-apps/tauri-action@v1` drafts **signed** installers: Windows NSIS
(`Install for me only` or `Install for everyone`), Ubuntu AppImage, and
macOS Universal DMG, plus `latest.json` for in-app updates. GitHub does not
ship MSI, RPM, or `.deb`. A Windows portable zip is attached after the NSIS
build. That zip is a PKZip of `DNA-Tools.exe` plus `Data/.keep` with no `./`
prefixes, so Windows Explorer can open it, and the job signs it with the
same minisign key (`GenomicsCaddy-portable-windows.zip.sig`). Do not create
it with `tar -a` (Windows bsdtar stores `./…` entries that look empty in
Explorer; GNU tar writes a tar archive named `.zip`). The macOS job rebuilds the DMG as a folder containing the `.app` and
an empty `Data/` directory. Tauri deletes the live `.app` after it writes the
DMG, so that rebuild unpacks `Genomics Caddy.app.tar.gz` (and falls back to
the just-built DMG) instead of looking only at `bundle/macos/*.app`.
`bundle.targets` includes `app` so macOS updater artifacts are first-class.
Other formats: [compile_instructions.md](../../compile_instructions.md).
Publish the draft from the GitHub Releases UI after you inspect artifacts.

The desktop app checks that endpoint quietly on launch. A dismissible banner
offers the new version; confirm then downloads the signed artifact. Windows
NSIS, Linux AppImage, and macOS use the plugin updater. A Windows portable
zip copy downloads the signed zip, verifies it, replaces files next to the
exe, and leaves `Data/` alone. Closing the window is blocked while an update
is in progress so the process cannot keep running after the UI is gone. It
does not upload DNA or reports.

`tauri dev` and debug builds may check GitHub but do not apply artifacts.
Copies built before v0.2.3 still follow the NSIS plugin path until someone
replaces them with a newer zip or installer. The first signed release is the
baseline; older unsigned copies will not self-update until someone installs
a signed build once.

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

`desktop:linux` registers `App/DNA-Tools` when that portable binary exists
(so the dock opens `App/Data`). It falls back to `builds/linux/DNA-Tools`.

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
