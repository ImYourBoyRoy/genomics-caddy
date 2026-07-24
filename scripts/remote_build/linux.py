# ./scripts/remote_build/linux.py
"""
Linux x86_64 build phase.

Runs on the Ubuntu builder host (not a VM). The builder IS a Linux x86_64
machine, so no cross-compilation is needed — we build natively.

Build sequence:
  1. npm install (ensure deps in case node_modules was excluded from tarball)
  2. cargo tauri build --target x86_64-unknown-linux-gnu
     Produces: .deb, .rpm, .AppImage under target/x86_64-unknown-linux-gnu/release/bundle/

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

_BUILD_SCRIPT = r"""
set -euo pipefail
if [ -d "$HOME/.local/share/fnm" ] || [ -f "$HOME/.local/bin/fnm" ]; then
    export PATH="$HOME/.local/bin:$HOME/.local/share/fnm:$PATH"
    eval "$(fnm env 2>/dev/null || true)"
fi
export PATH="$HOME/.cargo/bin:$PATH"

GUEST_DIR='{guest_dir}'
cd "$GUEST_DIR"

echo ""
echo "══════════════════════════════════════════"
echo "  LINUX: npm install"
echo "══════════════════════════════════════════"
npm install --prefer-offline 2>&1 | tail -20

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
find "$BUNDLE_DIR" -type f \( -name '*.deb' -o -name '*.rpm' -o -name '*.AppImage' \) \
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
        print("[LINUX] Generating full installers (.deb, .AppImage)")
    else:
        print("[LINUX] Generating raw executable only (--no-bundle)")

    script = _BUILD_SCRIPT.format(
        guest_dir=guest_dir,
        clean_step=clean_step,
        bundle_arg=bundle_arg,
    )

    host_bash(
        script,
        settings=s,
        check=True,
        timeout=5400,  # 90 min
    )
    print("[LINUX] x86_64 build complete.")
