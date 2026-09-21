# Compile instructions

GitHub releases ship three shapes: Windows NSIS (`Install for me only` or
`Install for everyone`), a Windows portable zip, a Linux AppImage, and a
macOS universal DMG. Use this page when you want another package format or a
local replica of those artifacts.

Needs **Node 26+**, current **Rust** (see `src-tauri/Cargo.toml`), and **pnpm**.
The repo is lock-free: do not pass `--locked` to Cargo, and resolve JavaScript
with `node ./scripts/pnpm_unlocked.mjs install`.

## Portable tree (any OS)

```bash
node ./scripts/pnpm_unlocked.mjs install
pnpm run build:release
```

The binary lands in `App/` with personal files in `App/Data/`. Linux:
`App/DNA-Tools`. Windows: `App/DNA-Tools.exe`.

Faster iteration: `pnpm run build:release-fast`.
A plain `cargo build --release` now enables `custom-protocol` by default so
the window uses the embedded UI instead of `http://127.0.0.1:1420`.

## GitHub-shaped bundles

From `src-tauri` after the frontend build (`pnpm run build`):

```bash
cargo tauri build --bundles nsis      # Windows installer (me / everyone)
cargo tauri build --bundles appimage  # Linux portable
cargo tauri build --bundles app,dmg   # macOS .app + DMG (updater needs app)
```

Windows portable zip of the staged exe + empty `Data/` folder (includes
`Data/.keep` so the directory survives zip). `--upload` minisign-signs the
zip so in-app portable copies can replace themselves:

```bash
node ./scripts/package_windows_portable.mjs
node ./scripts/package_windows_portable.mjs --self-test
```

macOS folder DMG (`.app` + empty `Data/`):

```bash
bash ./scripts/package_macos_folder_dmg.sh
```

Signed updater artifacts and `latest.json` are documented in
[docs/build/README.md](docs/build/README.md). Never commit the updater private
key.

## Other package formats (not GitHub)

These stay off GitHub because `/usr` cannot keep DNA next to the binary.

```bash
cargo tauri build --bundles deb
cargo tauri build --bundles rpm
```

Linux post-install/remove scripts live under `src-tauri/linux/`. A `.deb` or
`.rpm` is a local convenience, not the supported personal-data layout. Prefer
the AppImage if you want USB-clean storage.

BSD and other targets: install the OS WebKit/GTK (or WebView) dependencies,
then `cargo tauri build` for that host. There is no CI matrix for them.

## Tests before you ship a custom package

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
