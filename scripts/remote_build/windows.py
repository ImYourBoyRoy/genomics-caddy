# ./scripts/remote_build/windows.py
"""
Windows x86_64 cross-compile build phase.

Runs on the Ubuntu builder host (not a VM), cross-compiling to Windows
using mingw-w64 and cargo-xwin or standard x86_64-pc-windows-gnu.

Build sequence:
  1. node ./scripts/pnpm_unlocked.mjs install
  2. cargo tauri build --target x86_64-pc-windows-gnu
     Produces: .exe and .msi/.nsis under target/x86_64-pc-windows-gnu/release/

Inputs:
  remote_dir: Path on the builder host where sources were extracted
  clean:      If True, run `cargo clean` before building
  bundle:     If True, generates full installers instead of just the executable
  settings:   Remote_Build Settings
"""
from __future__ import annotations

from remote_build.config import Settings, get_settings
from remote_build.upgrade.host import host_bash

_BUILD_SCRIPT = r"""
set -euo pipefail
if [ -d "$HOME/.local/share/fnm" ] || [ -f "$HOME/.local/bin/fnm" ]; then
    export PATH="$HOME/.local/bin:$HOME/.local/share/fnm:$PATH"
    eval "$(fnm env 2>/dev/null || true)"
fi
export PATH="$HOME/.cargo/bin:$PATH"

REMOTE_DIR='{remote_dir}'
cd "$REMOTE_DIR"

echo ""
echo "══════════════════════════════════════════"
echo "  WIN (Cross): pnpm install"
echo "══════════════════════════════════════════"
node ./scripts/pnpm_unlocked.mjs install 2>&1 | tail -20

{clean_step}

echo ""
echo "══════════════════════════════════════════"
echo "  WIN (Cross): cargo tauri build"
echo "══════════════════════════════════════════"
cargo tauri build --target x86_64-pc-windows-gnu {bundle_arg} 2>&1

echo ""
BUNDLE_DIR="$REMOTE_DIR/src-tauri/target/x86_64-pc-windows-gnu/release"
echo "══════════════════════════════════════════"
echo "  WIN (Cross): list artifacts"
echo "══════════════════════════════════════════"
find "$BUNDLE_DIR" -maxdepth 2 -type f \( -name '*.exe' -o -name '*.msi' \) \
    -exec ls -lh {{}} \;

echo "WINDOWS_BUILD=OK"
"""

def build_cross(
    remote_dir: str,
    *,
    clean: bool = False,
    bundle: bool = False,
    settings: Settings | None = None,
) -> None:
    """Run the Windows x86_64 cross-compile build on the Ubuntu builder host."""
    s = settings or get_settings()
    clean_step = (
        "cargo clean --manifest-path src-tauri/Cargo.toml && echo '[WIN] cargo clean done'"
        if clean
        else "echo '[WIN] Incremental build (no clean)'"
    )
    bundle_arg = "" if bundle else "--no-bundle"

    print(f"[WIN] Starting x86_64 cross-compile build on {s.remote_host} → {remote_dir}")
    if bundle:
        print("[WIN] Generating full installers (.msi, .nsis)")
    else:
        print("[WIN] Generating raw executable only (--no-bundle)")

    script = _BUILD_SCRIPT.format(
        remote_dir=remote_dir,
        clean_step=clean_step,
        bundle_arg=bundle_arg,
    )

    host_bash(
        script,
        settings=s,
        check=True,
    )
