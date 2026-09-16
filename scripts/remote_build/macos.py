# ./scripts/remote_build/macos.py
"""
macOS Universal (arm64 + x86_64) build phase.

Build sequence:
  1. cargo build --release --target aarch64-apple-darwin
  2. cargo build --release --target x86_64-apple-darwin
  3. lipo -create → universal-apple-darwin/release/DNA-Tools
  4. lipo -create → universal-apple-darwin/release/inspect_db  (if present)
  5. cargo tauri build --target universal-apple-darwin
     - If the packaging step fails (headless AppleScript / hdiutil), applies the
       bundle_dmg.sh Jenkins-bypass patch and re-runs directly.

All commands are delivered via stdin (bash -s) through the builder jump-host,
so no secrets appear in argv or SSH command strings.

Inputs:
  guest_dir:  Path on the macOS guest where sources are ready (e.g. ~/dna_tools)
  clean:      If True, run `cargo clean` before building (cold build)
  settings:   Remote_Build Settings

Outputs / side effects:
  Built bundles at {guest_dir}/src-tauri/target/universal-apple-darwin/release/bundle/
  Prints build progress + any errors.
"""
from __future__ import annotations

from remote_build.config import Settings, get_settings
from remote_build.ssh import vm_ssh

_BUILD_SCRIPT = r"""
set -euo pipefail
source ~/.cargo/env 2>/dev/null || true
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:$PATH"

GUEST_DIR='{guest_dir}'
cd "$GUEST_DIR/src-tauri"

{clean_step}

echo ""
echo "══════════════════════════════════════════"
echo "  BUILD: aarch64-apple-darwin"
echo "══════════════════════════════════════════"
cargo build --release --target aarch64-apple-darwin

echo ""
echo "══════════════════════════════════════════"
echo "  BUILD: x86_64-apple-darwin"
echo "══════════════════════════════════════════"
cargo build --release --target x86_64-apple-darwin

echo ""
echo "══════════════════════════════════════════"
echo "  LIPO: creating universal binaries"
echo "══════════════════════════════════════════"
UNIV="$GUEST_DIR/src-tauri/target/universal-apple-darwin/release"
mkdir -p "$UNIV"

lipo -create -output "$UNIV/DNA-Tools" \
    "$GUEST_DIR/src-tauri/target/aarch64-apple-darwin/release/DNA-Tools" \
    "$GUEST_DIR/src-tauri/target/x86_64-apple-darwin/release/DNA-Tools"
echo "  lipo DNA-Tools OK"

# inspect_db is an optional secondary binary — skip if not present
ARM_INSPECT="$GUEST_DIR/src-tauri/target/aarch64-apple-darwin/release/inspect_db"
X64_INSPECT="$GUEST_DIR/src-tauri/target/x86_64-apple-darwin/release/inspect_db"
if [ -f "$ARM_INSPECT" ] && [ -f "$X64_INSPECT" ]; then
    lipo -create -output "$UNIV/inspect_db" "$ARM_INSPECT" "$X64_INSPECT"
    echo "  lipo inspect_db OK"
fi

echo ""
echo "══════════════════════════════════════════"
echo "  TAURI: cargo tauri build --target universal-apple-darwin"
echo "══════════════════════════════════════════"
cd "$GUEST_DIR"
set +e
cargo tauri build --target universal-apple-darwin {bundle_arg} 2>&1
TAURI_EXIT=$?
set -e

if [ $TAURI_EXIT -eq 0 ]; then
    echo "TAURI_BUILD=OK"
    exit 0
fi

# Packaging failed — likely AppleScript/hdiutil in headless VM.
# Patch bundle_dmg.sh to bypass the Jenkins AppleScript check.
echo "[MACOS] Tauri packaging failed (exit=$TAURI_EXIT). Applying headless patch…"
BUNDLE_DMG="$GUEST_DIR/src-tauri/target/universal-apple-darwin/release/bundle/dmg/bundle_dmg.sh"
if [ ! -f "$BUNDLE_DMG" ]; then
    echo "[FAIL] bundle_dmg.sh not found at $BUNDLE_DMG" >&2
    exit $TAURI_EXIT
fi

# Patch: force SKIP_JENKINS=1 so AppleScript block is always skipped
sed -i.bak 's/SKIP_JENKINS -eq 0/1 -eq 0/' "$BUNDLE_DMG"
BUNDLE_DIR="$(dirname "$BUNDLE_DMG")"
cd "$BUNDLE_DIR"

# Detect the actual DMG + app name from what tauri generated
DMG_NAME=$(ls ./*.dmg 2>/dev/null | head -1 | xargs basename 2>/dev/null || echo "")
if [ -z "$DMG_NAME" ]; then
    # Construct name from Cargo.toml if no DMG was started
    DMG_NAME="Genomics Caddy_0.2.0_universal.dmg"
fi

APP_PATH="../macos/Genomics Caddy.app"
echo "[MACOS] Running patched bundle_dmg.sh: $DMG_NAME"
./bundle_dmg.sh "$DMG_NAME" "$APP_PATH"
BUNDLE_EXIT=$?

if [ $BUNDLE_EXIT -eq 0 ]; then
    echo "TAURI_BUILD=OK_PATCHED"
    exit 0
fi

echo "[FAIL] bundle_dmg.sh patched run also failed (exit=$BUNDLE_EXIT)" >&2
exit $BUNDLE_EXIT
"""


def build(
    guest_dir: str,
    *,
    clean: bool = False,
    bundle: bool = False,
    settings: Settings | None = None,
) -> None:
    """Run the macOS Universal build on the macOS guest via builder jump."""
    s = settings or get_settings()
    
    clean_step = (
        "cargo clean --manifest-path src-tauri/Cargo.toml && echo '[MAC] cargo clean done'"
        if clean
        else "echo '[MAC] Incremental build (no clean)'"
    )
    bundle_arg = "" if bundle else "--no-bundle"

    print(f"[MAC] Starting Universal build on {s.vm_host} → {guest_dir}")
    if bundle:
        print("[MAC] Generating full installers (.dmg)")
    else:
        print("[MAC] Generating raw executable only (--no-bundle)")

    script = _BUILD_SCRIPT.format(
        guest_dir=guest_dir,
        clean_step=clean_step,
        bundle_arg=bundle_arg,
    )

    vm_ssh(
        host=s.remote_host,
        user=s.remote_user,
        key=s.ssh_key_expanded,
        vm_host=s.vm_host,
        vm_user=s.vm_user,
        guest_script=script,
        check=True,
        timeout=5400,  # 90 min: universal builds take ~30 min on 16 vCPU
    )
    print("[MACOS] Universal build complete.")
