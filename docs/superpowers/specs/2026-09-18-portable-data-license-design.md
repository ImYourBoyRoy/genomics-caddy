# Portable data, installers, and personal-use license

Date: 2026-09-18
Status: implemented

## Goal

Genomics Caddy stays a personal, local-first tool. DNA, reports, downloads,
and caches live next to the thing the person launched. Uninstall or folder
delete leaves no personal residue. Companies do not get a commercial license.

## Launch artifacts (GitHub)

Keep three product shapes. Drop MSI, WiX, RPM, and `.deb` from GitHub
releases.

| Platform | Installer | Portable |
| --- | --- | --- |
| Windows | NSIS `-setup.exe` with **Install for me only** or **Install for everyone** (`installMode: both`) | Zip containing the exe and a `Data/` folder beside it |
| Linux | none on GitHub | AppImage; `Data/` beside the `.AppImage` file |
| macOS | none | Universal DMG that contains a folder: `Genomics Caddy.app` + `Data/` |

`latest.json` / in-app update keeps using signed NSIS, AppImage, and the
macOS `.app.tar.gz` for those layouts. A Windows portable zip copy (no
uninstaller, writable folder, not Program Files) downloads the signed
`GenomicsCaddy-portable-windows.zip` and replaces itself in place.

Other OS or package formats (RPM, deb, BSD, etc.) are documented in
`compile_instructions.md`, linked from the root README. They are not CI
release artifacts.

## Data location

Personal files never go in a shared Everyone folder. Defaults:

| How it was launched | Default `Data/` |
| --- | --- |
| Portable zip / local `App/` tree / NSIS **me only** | Next to the exe (`<exe-dir>/Data/`) |
| Linux AppImage | Next to the `.AppImage` file |
| macOS folder from the DMG | Next to the `.app` |
| NSIS **everyone** | Per Windows account: `%LOCALAPPDATA%\Genomics Caddy\Data\` |

`GENOMICS_DATA_DIR` remains the test/agent override.

The person can change the folder from the **left sidebar** (folder picker).
That choice is stored next to the exe as a per-user pointer so a USB copy
keeps its own setting, and Everyone installs do not share one path. Changing
the folder offers **Move existing data here** vs **Use this empty folder**.
If the chosen folder cannot be created or written, show an error; do not
silently fall back.

v0.2.0 OS app-data leftovers are imported once into the new default if the
new `Data/` is empty.

The Advanced settings path list stays as the full technical view. The
sidebar is the everyday control: truncated path, Open folder, Change,
Reset to default.

## Sidebar and chrome

- Drop the visible **Theme** label. Auto / Light / Dark is a full-width
  segmented control in the sidebar footer (`1fr 1fr 1fr`), no extra heading.
- Footer uses the same sidebar surface as the rest of the rail (no second
  dark slab). Tighten gaps on Profiles, import, Connections, Liftover, and
  Data & updates so the column can breathe.
- Add a compact **Library** row above the theme control: where genomes live,
  Open, Change. Keep the noisy full paths in Advanced.
- Flatten the extra dark band under the window title / top chrome so the
  profile header sits on the same app background (the GTK/Tauri titlebar
  stays; the in-app double strip goes away).

## Portable Windows is a folder, not a lone `.exe`

Tauri still needs WebView2. The portable artifact is a zip:

```text
Genomics Caddy/
  DNA-Tools.exe
  Data/
```

Unzip anywhere (including a USB stick) and run the exe. Personal files stay
in that `Data/` folder. A single file `.exe` that also holds SQLite, packs,
and downloads is not a Tauri bundle and is out of scope.

NSIS “Install for me only” uses a user-writable install dir with the same
`exe + Data/` layout. Behavior matches portable, plus Start Menu shortcuts
and an uninstaller.

## Uninstall / erase

No leftover personal data:

- NSIS **me only**: deleting `$INSTDIR` removes the exe and that user's
  `Data/`.
- NSIS **everyone**: remove the Program Files app; checkbox **Also delete
  my genomes and downloads** removes this account's data folder. Other
  Windows accounts' data is left alone.
- In-app **Erase all local data** (sidebar Library / Advanced) deletes the
  resolved `Data/` directory and WebView/WebKit caches this process owns,
  then exits. Used by AppImage, DMG/folder, and portable zip.
- Linux `postRemove` for any future local `.deb` build (compile instructions
  only) must not be the GitHub path; GitHub Linux is AppImage.

Deleting the AppImage or the Mac folder without using Erase leaves `Data/`
behind unless the user deletes the whole folder that contains the app and
`Data/`. README tells people to delete that whole folder or use Erase.

## License

Root `LICENSE`: [PolyForm Noncommercial License 1.0.0](https://polyformproject.org/licenses/noncommercial/1.0.0/).

- People may use, share, and modify for noncommercial purposes.
- Companies may not use it for a commercial purpose (product, service,
  clinic workflow, paid analysis, internal business operations).
- Personal use at home stays allowed even if the person has a job.

README states that in one short paragraph and links `LICENSE`. This is not
medical-device clearance and does not change the existing research warning.

## Docs

- Root README: license line, GitHub artifact table, link to
  `compile_instructions.md`.
- `compile_instructions.md`: how to build the portable tree or additional
  packages (deb/rpm/etc.) locally; lock-free pnpm; no `--locked` Cargo;
  pointer to `docs/build/README.md` for signing/updater.
- `docs/build/README.md`: NSIS both, no MSI/RPM/deb on GitHub, data-next-to-exe,
  uninstall/erase contract.

## Out of scope

- Apple notarization
- Embedding a full WebView2 runtime in the portable zip
- Rewriting existing v0.2.0 GitHub assets in place; next tag (for example
  `v0.2.1`) ships this layout
