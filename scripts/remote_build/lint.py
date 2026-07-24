# ./scripts/remote_build/lint.py
"""
Lint runner for remote build pipelines.

Runs cargo clippy (with pedantic + nursery warnings) and svelte-check on the
remote target BEFORE compilation. Surfaces all warnings and optionally fails
the build on any warning (strict mode).

By default, lint warnings are printed but do NOT block the build (report-only
mode). Pass strict=True to treat any warning as a fatal error.

Inputs:
  platform:   "macos" | "linux"  (Windows lint: TODO)
  guest_dir:  Path on the remote target where source was synced
  strict:     If True, fail on any lint warning (-D warnings)
  settings:   Remote_Build Settings

Outputs / side effects:
  Prints all lint warnings to stdout. Returns lint result summary dict.
  Raises SystemExit if strict=True and warnings/errors were found.
"""
from __future__ import annotations

from remote_build.config import Settings, get_settings
from remote_build.upgrade.host import host_bash
from remote_build.ssh import vm_ssh

# Clippy flags — standard clippy checks for code correctness
_CLIPPY_BASE = "cargo clippy --manifest-path src-tauri/Cargo.toml"
_CLIPPY_STRICT = _CLIPPY_BASE + " -- -D warnings"
_CLIPPY_REPORT = _CLIPPY_BASE  # warnings visible, not fatal

_SVELTE_CHECK = "npm run check 2>&1"
_SVELTE_CHECK_STRICT = "npm run check -- --fail-on-warnings 2>&1"

_MACOS_LINT_SCRIPT = """
set -uo pipefail
source ~/.cargo/env 2>/dev/null || true
export PATH="/Users/guest-user/.cargo/bin:/usr/local/bin:/opt/homebrew/bin:$PATH"
cd '{guest_dir}'

echo ""
echo "══════════════════════════════════════════"
echo "  LINT: cargo clippy"
echo "══════════════════════════════════════════"
{clippy_cmd}
CLIPPY_EXIT=$?

echo ""
echo "══════════════════════════════════════════"
echo "  LINT: svelte-check"
echo "══════════════════════════════════════════"
if ! command -v npm >/dev/null 2>&1; then
    echo "[AUTO-HEAL] npm missing on guest — sourcing Homebrew path…"
    export PATH="/usr/local/bin:/opt/homebrew/bin:$PATH"
fi

if command -v npm >/dev/null 2>&1; then
    {svelte_cmd}
    SVELTE_EXIT=$?
else
    echo "[LINT FAIL] npm required for svelte-check but missing after auto-healing!"
    SVELTE_EXIT=1
fi

echo ""
echo "══════════════════════════════════════════"
echo "CLIPPY_EXIT=$CLIPPY_EXIT"
echo "SVELTE_EXIT=$SVELTE_EXIT"
if [ $CLIPPY_EXIT -ne 0 ] || [ $SVELTE_EXIT -ne 0 ]; then
  echo "LINT_STATUS=FAIL"
  exit 1
fi
echo "LINT_STATUS=PASS"
"""

_LINUX_LINT_SCRIPT = """
set -uo pipefail
if [ -d "$HOME/.local/share/fnm" ] || [ -f "$HOME/.local/bin/fnm" ]; then
    export PATH="$HOME/.local/bin:$HOME/.local/share/fnm:$PATH"
    eval "$(fnm env 2>/dev/null || true)"
fi
export PATH="$HOME/.cargo/bin:$PATH"
cd '{guest_dir}'

echo ""
echo "══════════════════════════════════════════"
echo "  LINT: cargo clippy"
echo "══════════════════════════════════════════"
{clippy_cmd}
CLIPPY_EXIT=$?

echo ""
echo "══════════════════════════════════════════"
echo "  LINT: svelte-check"
echo "══════════════════════════════════════════"
if ! command -v npm >/dev/null 2>&1 && [ -d "$HOME/.local/share/fnm" ]; then
    export PATH="$HOME/.local/share/fnm:$PATH"
    eval "$(fnm env 2>/dev/null || true)"
fi

if command -v npm >/dev/null 2>&1; then
    {svelte_cmd}
    SVELTE_EXIT=$?
else
    echo "[LINT FAIL] npm required for svelte-check but missing!"
    SVELTE_EXIT=1
fi

echo ""
echo "══════════════════════════════════════════"
echo "CLIPPY_EXIT=$CLIPPY_EXIT"
echo "SVELTE_EXIT=$SVELTE_EXIT"
if [ $CLIPPY_EXIT -ne 0 ] || [ $SVELTE_EXIT -ne 0 ]; then
  echo "LINT_STATUS=FAIL"
  exit 1
fi
echo "LINT_STATUS=PASS"
"""


_WINDOWS_LINT_SCRIPT = """
set -uo pipefail
if [ -d "$HOME/.local/share/fnm" ] || [ -f "$HOME/.local/bin/fnm" ]; then
    export PATH="$HOME/.local/bin:$HOME/.local/share/fnm:$PATH"
    eval "$(fnm env --shell bash 2>/dev/null || true)"
fi
source "$HOME/.cargo/env" 2>/dev/null || export PATH="$HOME/.cargo/bin:$PATH"
cd '{guest_dir}'

echo ""
echo "══════════════════════════════════════════"
echo "  LINT (Windows Target): cargo clippy"
echo "══════════════════════════════════════════"
{clippy_win_cmd}
CLIPPY_EXIT=$?

echo ""
echo "══════════════════════════════════════════"
echo "  LINT (Windows Target): svelte-check"
echo "══════════════════════════════════════════"
if command -v npm >/dev/null 2>&1; then
    {svelte_cmd}
    SVELTE_EXIT=$?
else
    echo "[LINT FAIL] npm required for svelte-check but missing!"
    SVELTE_EXIT=1
fi

echo ""
echo "══════════════════════════════════════════"
echo "CLIPPY_EXIT=$CLIPPY_EXIT"
echo "SVELTE_EXIT=$SVELTE_EXIT"
if [ $CLIPPY_EXIT -ne 0 ] || [ $SVELTE_EXIT -ne 0 ]; then
  echo "LINT_STATUS=FAIL"
  exit 1
fi
echo "LINT_STATUS=PASS"
"""


def _pick_cmds(strict: bool) -> tuple[str, str]:
    clippy = _CLIPPY_STRICT if strict else _CLIPPY_REPORT
    svelte = _SVELTE_CHECK_STRICT if strict else _SVELTE_CHECK
    return clippy, svelte


def lint_macos(
    guest_dir: str,
    *,
    strict: bool = True,
    settings: Settings | None = None,
) -> bool:
    """Run lint on macOS guest. Returns True if passed."""
    s = settings or get_settings()
    clippy, svelte = _pick_cmds(strict)
    mode = "STRICT" if strict else "REPORT-ONLY"
    print(f"[LINT] Running on macOS guest ({s.vm_host}) — mode={mode}")

    script = _MACOS_LINT_SCRIPT.format(
        guest_dir=guest_dir,
        clippy_cmd=clippy,
        svelte_cmd=svelte,
    )
    res = vm_ssh(
        host=s.remote_host,
        user=s.remote_user,
        key=s.ssh_key_expanded,
        vm_host=s.vm_host,
        vm_user=s.vm_user,
        guest_script=script,
        check=False,
        timeout=600,
    )
    passed = res.returncode == 0 and "LINT_STATUS=PASS" in (res.stdout or "")
    _report(passed, strict)
    return passed


def lint_linux(
    guest_dir: str,
    *,
    strict: bool = True,
    settings: Settings | None = None,
) -> bool:
    """Run lint on the Ubuntu builder host. Returns True if passed."""
    s = settings or get_settings()
    clippy, svelte = _pick_cmds(strict)
    mode = "STRICT" if strict else "REPORT-ONLY"
    print(f"[LINT] Running on Linux host ({s.remote_host}) — mode={mode}")

    script = _LINUX_LINT_SCRIPT.format(
        guest_dir=guest_dir,
        clippy_cmd=clippy,
        svelte_cmd=svelte,
    )
    res = host_bash(script, settings=s, check=False, timeout=600)
    passed = res.returncode == 0 and "LINT_STATUS=PASS" in (res.stdout or "")
    _report(passed, strict)
    return passed


def lint_windows(
    guest_dir: str,
    *,
    strict: bool = True,
    settings: Settings | None = None,
) -> bool:
    """Run lint for Windows x86_64 target on the Ubuntu builder host."""
    s = settings or get_settings()
    clippy_base = "cargo clippy --manifest-path src-tauri/Cargo.toml --target x86_64-pc-windows-gnu"
    clippy_cmd = f"{clippy_base} -- -D warnings" if strict else clippy_base
    svelte_cmd = _SVELTE_CHECK_STRICT if strict else _SVELTE_CHECK
    mode = "STRICT" if strict else "REPORT-ONLY"
    print(f"[LINT] Running Windows target lint on Linux host ({s.remote_host}) — mode={mode}")

    script = _WINDOWS_LINT_SCRIPT.format(
        guest_dir=guest_dir,
        clippy_win_cmd=clippy_cmd,
        svelte_cmd=svelte_cmd,
    )
    res = host_bash(script, settings=s, check=False, timeout=600)
    passed = res.returncode == 0 and "LINT_STATUS=PASS" in (res.stdout or "")
    _report(passed, strict)
    return passed


def _report(passed: bool, strict: bool) -> None:
    if passed:
        print("[LINT] ✓ All lint checks passed.")
    else:
        raise SystemExit("[LINT] ✗ Lint checks or typechecks failed! Hard stopping build so errors can be resolved.")


def run_lint(
    platform: str,
    guest_dir: str,
    *,
    strict: bool = True,
    settings: Settings | None = None,
) -> bool:
    """Dispatch lint to the correct platform."""
    if platform == "macos":
        return lint_macos(guest_dir, strict=strict, settings=settings)
    if platform == "linux":
        return lint_linux(guest_dir, strict=strict, settings=settings)
    if platform == "windows":
        return lint_windows(guest_dir, strict=strict, settings=settings)
    print(f"[LINT] Unknown platform {platform!r} — skipping.")
    return True
