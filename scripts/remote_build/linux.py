# ./scripts/remote_build/linux.py
"""
Linux x86_64 build phase.

Runs on the Ubuntu builder host (not a VM). The builder IS a Linux x86_64
machine, so no cross-compilation is needed — we build natively.

Build sequence:
  1. node ./scripts/pnpm_unlocked.mjs install (ensure deps in case node_modules was excluded from tarball)
  2. cargo tauri build --target x86_64-unknown-linux-gnu
     Produces: .AppImage under target/x86_64-unknown-linux-gnu/release/bundle/

The source tree was already synced to remote_dir on the host by sync.py.

Inputs:
  guest_dir:  Path on the builder host (== remote_dir, where sources were extracted)
  clean:      If True, run `cargo clean` before building
  settings:   Remote_Build Settings

Outputs / side effects:
  Built bundles at {guest_dir}/src-tauri/target/x86_64-unknown-linux-gnu/release/bundle/
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
    echo "[LINUX] pnpm missing — installing latest into the user npm prefix"
    mkdir -p "$HOME/.npm-global"
    export PATH="$(npm prefix -g)/bin:$PATH"
    npm install --global pnpm@latest
fi
export PNPM_HOME="${{PNPM_HOME:-$HOME/.local/share/pnpm}}"

GUEST_DIR='{guest_dir}'
cd "$GUEST_DIR"

echo ""
echo "══════════════════════════════════════════"
echo "  LINUX: pnpm install"
echo "══════════════════════════════════════════"
node ./scripts/pnpm_unlocked.mjs install 2>&1 | tail -20

{clean_step}

echo ""
echo "══════════════════════════════════════════"
echo "  LINUX: cargo tauri build"
echo "══════════════════════════════════════════"
cargo tauri build --target x86_64-unknown-linux-gnu {bundle_arg} 2>&1

echo ""
BUNDLE_DIR="$GUEST_DIR/src-tauri/target/x86_64-unknown-linux-gnu/release/bundle"
echo "══════════════════════════════════════════"
echo "  LINUX: bundle artifacts"
echo "══════════════════════════════════════════"
find "$BUNDLE_DIR" -type f \( -name '*.AppImage' -o -name '*.AppImage.sig' \) \
    -exec ls -lh {{}} \;

echo "LINUX_BUILD=OK"
"""


def build(
    guest_dir: str,
    *,
    clean: bool = False,
    bundle: bool = False,
    settings: Settings | None = None,
) -> None:
    """Run the Linux x86_64 build on the Ubuntu builder host."""
    s = settings or get_settings()
    clean_step = (
        "cargo clean --manifest-path src-tauri/Cargo.toml && echo '[LINUX] cargo clean done'"
        if clean
        else "echo '[LINUX] Incremental build (no clean)'"
    )
    bundle_arg = "" if bundle else "--no-bundle"

    print(f"[LINUX] Starting x86_64 build on {s.remote_host} → {guest_dir}")
    if clean:
        print("[LINUX] Full clean requested — this will be a cold build.")
    if bundle:
        print("[LINUX] Generating AppImage")
    else:
        print("[LINUX] Generating raw executable only (--no-bundle)")

    script = _with_sign_env(_BUILD_SCRIPT.format(
        guest_dir=guest_dir,
        clean_step=clean_step,
        bundle_arg=bundle_arg,
    ))

    host_bash(
        script,
        settings=s,
        check=True,
        timeout=5400,  # 90 min
    )
    print("[LINUX] x86_64 build complete.")
