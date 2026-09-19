# ./scripts/remote_build/windows.py
"""
Windows x86_64 cross-compile build phase.

Runs on the Ubuntu builder host (not a VM), cross-compiling to Windows
using mingw-w64 and cargo-xwin or standard x86_64-pc-windows-gnu.

Build sequence:
  1. node ./scripts/pnpm_unlocked.mjs install
  2. cargo tauri build --target x86_64-pc-windows-gnu
     Produces: .exe and NSIS under target/x86_64-pc-windows-gnu/release/

Inputs:
  remote_dir: Path on the builder host where sources were extracted
  clean:      If True, run `cargo clean` before building
  bundle:     If True, generates full installers instead of just the executable
  settings:   Remote_Build Settings
"""
from __future__ import annotations

from remote_build.config import Settings, get_settings
from remote_build.upgrade.host import host_bash

from pathlib import Path
import importlib.util as _ilu

def _with_sign_env(script: str) -> str:
    spec = _ilu.spec_from_file_location(
        "dna_tools_sign_env", Path(__file__).with_name("sign_env.py")
    )
    mod = _ilu.module_from_spec(spec)
    spec.loader.exec_module(mod)  # type: ignore[union-attr]
    return mod.inject(script)

_BUILD_SCRIPT = r"""
set -euo pipefail
source "$HOME/.cargo/env" 2>/dev/null || true
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$HOME/.local/share/fnm:$HOME/.npm-global/bin:${{PNPM_HOME:-$HOME/.local/share/pnpm}}:$PATH"
if [ -d "$HOME/.local/share/fnm" ] || [ -f "$HOME/.local/bin/fnm" ]; then
    eval "$(fnm env 2>/dev/null || true)"
fi
if ! command -v pnpm >/dev/null 2>&1; then
    echo "[WIN] pnpm missing — installing latest into the user npm prefix"
    mkdir -p "$HOME/.npm-global"
    export PATH="$(npm prefix -g)/bin:$PATH"
    npm install --global pnpm@latest
fi
export PNPM_HOME="${{PNPM_HOME:-$HOME/.local/share/pnpm}}"

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
find "$BUNDLE_DIR" -maxdepth 3 -type f \( -name '*.exe' -o -name '*.zip' -o -name '*.sig' \) \
    ! -path '*/deps/*' ! -path '*/build/*' \
    -exec ls -lh {{}} \;

if [ -f "$REMOTE_DIR/src-tauri/target/x86_64-pc-windows-gnu/release/DNA-Tools.exe" ] \
   || [ -f "$REMOTE_DIR/src-tauri/target/release/DNA-Tools.exe" ]; then
    echo "[WIN] Staging portable zip with Data/.keep"
    node ./scripts/package_windows_portable.mjs
fi

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
        print("[WIN] Generating NSIS installer and portable zip")
    else:
        print("[WIN] Generating raw executable only (--no-bundle)")

    script = _with_sign_env(_BUILD_SCRIPT.format(
        remote_dir=remote_dir,
        clean_step=clean_step,
        bundle_arg=bundle_arg,
    ))

    host_bash(
        script,
        settings=s,
        check=True,
    )
