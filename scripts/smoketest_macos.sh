#!/usr/bin/env bash
# ./scripts/smoketest_macos.sh
#
# Purpose: Smoke-test the macOS Universal .app bundle for Genomics Caddy.
#   Connects via SSH through the Ubuntu relay host to the macOS VM guest
#   and validates every critical pre-flight condition before giving the
#   bundle to a friend.
#
# Usage:
#   bash scripts/smoketest_macos.sh [--verbose]
#
# Inputs:
#   - Root `.env` REMOTE_*/VM_*/GUEST_DIR or exported environment values
#   - SSH key from VM_SSH_KEY (default ~/.ssh/id_ed25519)
#
# Outputs:
#   - PASS/FAIL report to stdout
#   - Exit code 0 = all tests passed, 1 = one or more failures
#
# Notes:
#   - Run this BEFORE copying the DMG to flash drive.
#   - Gatekeeper (spctl) failure is expected — app is unsigned.
#   - Friend MUST right-click → Open on first launch on their Mac.
#   - Only probes guest dna_tools paths; never biolume.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
ENV_FILE="$REPO_ROOT/.env"

# Load toolkit-native keys from .env without overriding an already-exported value.
if [[ -f "$ENV_FILE" ]]; then
  while IFS= read -r line || [[ -n "$line" ]]; do
    [[ -z "$line" || "$line" =~ ^[[:space:]]*# ]] && continue
    [[ "$line" != *=* ]] && continue
    key="${line%%=*}"
    val="${line#*=}"
    key="$(echo "$key" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
    val="$(echo "$val" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//;s/^"//;s/"$//;s/^'"'"'//;s/'"'"'$//')"
    case "$key" in
      REMOTE_HOST|REMOTE_USER|REMOTE_DIR|VM_HOST|VM_USER|VM_SSH_KEY|VM_NAME|GUEST_DIR)
        if [[ -z "${!key:-}" ]]; then
          export "$key=$val"
        fi
        ;;
    esac
  done < "$ENV_FILE"
fi

REMOTE_HOST="${REMOTE_HOST:?Set REMOTE_HOST in .env or the environment}"
REMOTE_USER="${REMOTE_USER:?Set REMOTE_USER in .env or the environment}"
VM_HOST="${VM_HOST:?Set VM_HOST in .env or the environment}"
VM_USER="${VM_USER:?Set VM_USER in .env or the environment}"
VM_SSH_KEY="${VM_SSH_KEY:-$HOME/.ssh/id_ed25519}"
GUEST_DIR="${GUEST_DIR:?Set GUEST_DIR in .env or the environment}"

# Expand ~ in key path
SSH_KEY="${VM_SSH_KEY/#\~/$HOME}"
RELAY="${REMOTE_USER}@${REMOTE_HOST}"
VM="${VM_USER}@${VM_HOST}"

case "$GUEST_DIR" in
  *biolume*)
    echo "[FAIL] GUEST_DIR must not target biolume (other-project tree)." >&2
    exit 2
    ;;
esac
if [[ "$GUEST_DIR" != *dna_tools* ]]; then
  echo "[FAIL] GUEST_DIR must contain dna_tools (DNA isolation)." >&2
  exit 2
fi

APP_BUNDLE="${GUEST_DIR}/src-tauri/target/universal-apple-darwin/release/bundle/macos/Genomics Caddy.app"
APP_BIN="$APP_BUNDLE/Contents/MacOS/DNA-Tools"
INSPECT_BIN="$APP_BUNDLE/Contents/MacOS/inspect_db"
APPDATA="$HOME/Library/Application Support/Genomics Caddy"
DMG_PATH="${GUEST_DIR}/src-tauri/target/universal-apple-darwin/release/bundle/dmg/Genomics Caddy_0.2.0_universal.dmg"

VERBOSE=false
[[ "${1:-}" == "--verbose" ]] && VERBOSE=true

PASS=0
FAIL=0

# ── Helpers ────────────────────────────────────────────────────────────────────

vm_run() {
    ssh -i "$SSH_KEY" -o StrictHostKeyChecking=no "$RELAY" \
        "ssh -i ~/.ssh/id_ed25519 -o StrictHostKeyChecking=no $VM \"$1\"" 2>&1
}

pass() { echo "  ✅ PASS: $1"; ((PASS++)); }
fail() { echo "  ❌ FAIL: $1"; ((FAIL++)); }
info() { echo "  ℹ️  INFO: $1"; }
section() { echo; echo "── $1 ──────────────────────────────────────────"; }

# ── Tests ──────────────────────────────────────────────────────────────────────

section "1. Binary Architecture"
arch_out=$(vm_run "file '$APP_BIN'")
if echo "$arch_out" | grep -q "universal binary with 2 architectures"; then
    pass "Universal binary confirmed (arm64 + x86_64)"
    $VERBOSE && info "$arch_out"
else
    fail "Binary is NOT a universal binary: $arch_out"
fi
if echo "$arch_out" | grep -q "arm64"; then
    pass "arm64 (Apple Silicon M1) slice present"
else
    fail "arm64 slice missing — will NOT run natively on M1"
fi
if echo "$arch_out" | grep -q "x86_64"; then
    pass "x86_64 (Intel) slice present"
else
    fail "x86_64 slice missing"
fi

section "2. Bundle Structure"
info_plist=$(vm_run "cat '$APP_BUNDLE/Contents/Info.plist' 2>&1" || true)
if echo "$info_plist" | grep -q "CFBundleExecutable"; then
    pass "Info.plist is present and readable"
else
    fail "Info.plist missing or malformed"
fi
min_ver=$(echo "$info_plist" | grep -A1 "LSMinimumSystemVersion" | grep string | sed 's/.*<string>\(.*\)<\/string>.*/\1/')
info "Minimum macOS version declared: ${min_ver:-unknown}"
if echo "$info_plist" | grep -q "NSHighResolutionCapable"; then
    pass "NSHighResolutionCapable = true (Retina display ready)"
fi
has_icon=$(vm_run "ls '$APP_BUNDLE/Contents/Resources/icon.icns' 2>&1" || true)
if echo "$has_icon" | grep -q "icon.icns"; then
    pass "Application icon (icon.icns) bundled"
else
    fail "icon.icns missing from Resources — app will show generic icon"
fi
has_inspect=$(vm_run "ls '$INSPECT_BIN' 2>&1" || true)
if echo "$has_inspect" | grep -q "inspect_db"; then
    pass "inspect_db sidecar binary present in MacOS/"
else
    fail "inspect_db sidecar missing from bundle"
fi

section "3. Dynamic Library Dependencies"
dylibs=$(vm_run "otool -L '$APP_BIN' 2>&1" || true)
# Strip leading tabs/spaces then filter — otool output has leading \t before each dep path
nonSystem=$(echo "$dylibs" | sed 's/^[[:space:]]*//' | grep -v "^/System\|^/usr/lib\|^@\|Caddy.app\|^$\|Mach-O\|for architecture" || true)
if [[ -n "$nonSystem" ]]; then
    fail "Bundled binary has non-system dylib dependencies (may not work on friend's Mac)"
    $VERBOSE && info "$nonSystem"
else
    pass "All dylib dependencies are system frameworks (no external dylibs required)"
fi
for fw in AppKit WebKit Foundation Security SystemConfiguration; do
    if echo "$dylibs" | grep -q "$fw"; then
        pass "Framework linked: $fw"
    else
        fail "Expected framework NOT linked: $fw"
    fi
done

section "4. Gatekeeper / Code Signing"
gk_out=$(vm_run "spctl --assess --type exec --verbose '$APP_BUNDLE' 2>&1" || true)
info "spctl output: $gk_out"
if echo "$gk_out" | grep -q "accepted"; then
    pass "Gatekeeper: app is signed and accepted"
elif echo "$gk_out" | grep -q "no usable signature"; then
    info "App is UNSIGNED — Gatekeeper will warn on first launch (expected for dev/friend builds)"
    info "Friend must: right-click → Open → Open Anyway on first launch"
    pass "Unsigned status confirmed (expected, not a blocker)"
else
    fail "Unexpected Gatekeeper status: $gk_out"
fi

section "5. Quarantine Attribute"
xattr_out=$(vm_run "xattr -l '$APP_BUNDLE' 2>&1" || true)
if echo "$xattr_out" | grep -q "com.apple.quarantine"; then
    fail "Quarantine attribute found — app may be blocked. Friend should run: xattr -cr '$APP_BUNDLE'"
    info "Or tell friend: right-click → Open (bypasses quarantine)"
else
    pass "No quarantine attribute on the bundle (clean)"
fi

section "6. AppData Write Permission (Fallback Path)"
write_test=$(vm_run "mkdir -p ~/\"Library/Application Support/Genomics Caddy\" && touch ~/\"Library/Application Support/Genomics Caddy/.write_test\" && echo WRITE_OK && rm ~/\"Library/Application Support/Genomics Caddy/.write_test\"" || true)
if echo "$write_test" | grep -q "WRITE_OK"; then
    pass "AppData fallback path is writable: ~/Library/Application Support/Genomics Caddy/"
else
    fail "Cannot write to AppData path on VM: ~/Library/Application Support/Genomics Caddy/"
fi

section "7. SQLite Database Init via inspect_db"
db_out=$(vm_run "'$INSPECT_BIN' 2>&1 | head -20" || true)
if echo "$db_out" | grep -q "Database opened successfully"; then
    pass "SQLite database opened and initialized successfully"
else
    fail "Database init failed: $db_out"
fi
if echo "$db_out" | grep -q "Resolved Data Dir:"; then
    resolved_dir=$(echo "$db_out" | grep "Resolved Data Dir:" | sed 's/.*Resolved Data Dir: //')
    info "Data dir resolved to: $resolved_dir"
    # /Users/*/App/Data is fine on a dev VM — on fresh install it will use ~/Library/Application Support
    if echo "$resolved_dir" | grep -q "/App/Data"; then
        info "Data dir resolved to App/Data (portable dev mode — expected on VM with prior dev session)"
        info "On a fresh Mac install, it will fall back to ~/Library/Application Support/Genomics Caddy/"
        pass "Path resolver working (portable dev mode active on VM)"
    else
        pass "Data dir resolved to user-writable path: $resolved_dir"
    fi
fi
if echo "$db_out" | grep -q "Tables:"; then
    pass "SQLite schema tables present"
    $VERBOSE && info "$(echo "$db_out" | grep 'Tables:')"
fi

section "8. macOS Version Compatibility"
sw_ver=$(vm_run "sw_vers" || true)
info "VM macOS: $sw_ver"
mac_version=$(echo "$sw_ver" | grep ProductVersion | awk '{print $2}')
major=$(echo "$mac_version" | cut -d. -f1)
if [[ "$major" -ge 11 ]]; then
    pass "VM running macOS $mac_version — WebKit2/WKWebView requirements met for Tauri"
else
    fail "macOS $mac_version is too old — Tauri 2 requires macOS 11+"
fi

section "9. WebKit Availability"
webkit_check=$(vm_run "ls /System/Library/Frameworks/WebKit.framework 2>&1" || true)
if echo "$webkit_check" | grep -qv "No such file"; then
    pass "WebKit.framework present on VM"
else
    fail "WebKit.framework NOT found — Tauri WebView will crash"
fi

section "10. DMG File"
dmg_check=$(vm_run "ls -lh '$DMG_PATH' 2>&1" || true)
if echo "$dmg_check" | grep -q "universal.dmg"; then
    dmg_size=$(echo "$dmg_check" | awk '{print $5}')
    pass "DMG file exists (size: $dmg_size)"
    if echo "$dmg_check" | grep -qE "[0-9]+M"; then
        pass "DMG size looks reasonable (non-zero)"
    fi
else
    fail "DMG file not found at expected path"
fi

# ── Summary ────────────────────────────────────────────────────────────────────

echo
echo "════════════════════════════════════════════"
echo " SMOKE TEST COMPLETE"
echo "════════════════════════════════════════════"
echo " ✅ PASSED: $PASS"
echo " ❌ FAILED: $FAIL"
echo "════════════════════════════════════════════"

if [[ $FAIL -gt 0 ]]; then
    echo
    echo "⚠️  Fix the failures above before distributing the build."
    exit 1
else
    echo
    echo "🎉 All smoke tests passed! Bundle is ready to copy to flash drive."
    echo
    echo "IMPORTANT DISTRIBUTION NOTES:"
    echo "  1. App is UNSIGNED — friend must: right-click → Open → 'Open Anyway'"
    echo "  2. On first launch, the database is created in:"
    echo "     ~/Library/Application Support/Genomics Caddy/"
    echo "  3. All genome reference downloads will be stored there."
    exit 0
fi
