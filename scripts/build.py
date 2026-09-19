# ./scripts/build.py
"""
DNA-Tools remote build engine — CLI entry point.

Builds Genomics Caddy for macOS (Universal), Linux (x86_64), and/or
Windows (x86_64) using the Remote_Build toolkit primitives.

Usage (from repo root):
  python scripts/build.py --target mac
  python scripts/build.py --target linux
  python scripts/build.py --target windows    # requires WIN_* in .env
  python scripts/build.py --target all        # mac + linux + windows
  python scripts/build.py --target mac linux  # multi-target

Flags:
  --clean            cargo clean before build (cold build, slower)
  --skip-frontend    skip `pnpm run build` local frontend step
  --skip-upgrade     skip rustup/cargo-tauri/pnpm upgrade step
  --skip-lint        skip clippy + svelte-check
  --strict-lint      fail build on any lint warning (default: report-only)
  --no-purge-after   keep remote build caches after success (for debugging)
  --lint-only        run lint and exit, do not build

Config (DNA_Tools/.env or Remote_Build/.env):
  REMOTE_HOST          Ubuntu builder hostname or IP
  REMOTE_USER          builder SSH user
  REMOTE_DIR           staging directory on builder
  VM_HOST              macOS guest hostname or IP
  VM_USER              macOS guest SSH user
  VM_SSH_KEY           SSH key path         (default: ~/.ssh/id_ed25519)
  GUEST_DIR            macOS guest work directory
  WIN_VM_HOST          WinServer guest IP   (unset = Windows disabled)
  WIN_VM_USER          WinServer SSH user   (unset = Windows disabled)
  WIN_GUEST_DIR        WinServer workdir    (default: C:/Users/{WIN_VM_USER}/dna_tools)

Safety:
  - REMOTE_DIR must contain 'dna_tools' and must not target 'biolume'.
  - All SSH commands are delivered via stdin (bash -s) — no secrets in argv.

Outputs:
  builds/macOS/   — folder DMG (.app + Data/)
  builds/linux/   — .AppImage
  builds/windows/ — NSIS .exe + portable zip
"""
from __future__ import annotations

import argparse
import importlib.util
import os
import sys
import time
from pathlib import Path
from types import ModuleType

# ════════════════════════════════════════════════════════════════════════════
# BOOTSTRAP: env + sys.path must be set before ANY project imports
# ════════════════════════════════════════════════════════════════════════════

_PROJECT_ROOT = Path(__file__).resolve().parent.parent
_RB_TOOLKIT_ROOT = _PROJECT_ROOT.parent / "Remote_Build"
_SCRIPTS_DIR = _PROJECT_ROOT / "scripts"
_DNA_BUILD_PKG = _SCRIPTS_DIR / "remote_build"


def _load_dotenv(path: Path, override: bool = False) -> None:
    """Minimal dotenv loader — sets key in env (override=True forces project settings)."""
    if not path.is_file():
        return
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, val = line.split("=", 1)
        key = key.strip()
        val = val.strip().strip('"').strip("'")
        if key:
            if override or key not in os.environ:
                os.environ[key] = val


# Load Remote_Build .env first (base defaults), then DNA_Tools .env with override=True
_load_dotenv(_RB_TOOLKIT_ROOT / ".env", override=False)
_load_dotenv(_PROJECT_ROOT / ".env", override=True)

# Put the Remote_Build toolkit root at sys.path[0] so `import remote_build`
# resolves to the TOOLKIT package, not the DNA build package in scripts/.
# IMPORTANT: do NOT add _SCRIPTS_DIR to sys.path — that would cause the
# name collision. We load DNA build modules by absolute path below.
_rb_str = str(_RB_TOOLKIT_ROOT)
if _rb_str not in sys.path:
    sys.path.insert(0, _rb_str)


def _load_dna_module(name: str) -> ModuleType:
    """Load a module from scripts/remote_build/ by absolute path."""
    full_name = f"_dna_build.{name}"
    if full_name in sys.modules:
        return sys.modules[full_name]
    file_path = _DNA_BUILD_PKG / f"{name}.py"
    spec = importlib.util.spec_from_file_location(full_name, file_path)
    assert spec and spec.loader, f"Cannot find module: {file_path}"
    mod = importlib.util.module_from_spec(spec)
    sys.modules[full_name] = mod
    spec.loader.exec_module(mod)  # type: ignore[union-attr]
    return mod


# ════════════════════════════════════════════════════════════════════════════
# Toolkit imports (only after sys.path is configured correctly)
# ════════════════════════════════════════════════════════════════════════════
from remote_build.config import Settings, get_settings  # noqa: E402

# ════════════════════════════════════════════════════════════════════════════
# DNA build module imports (by absolute path — no sys.path collision)
# ════════════════════════════════════════════════════════════════════════════
_sync = _load_dna_module("sync")
_toolchain = _load_dna_module("toolchain")
_lint = _load_dna_module("lint")
_macos = _load_dna_module("macos")
_linux = _load_dna_module("linux")
_windows = _load_dna_module("windows")
_cache = _load_dna_module("cache")
_collect = _load_dna_module("collect")

# ════════════════════════════════════════════════════════════════════════════
# Safety constants
# ════════════════════════════════════════════════════════════════════════════
_FORBIDDEN_SLUGS = ("biolume",)


def _assert_dna_isolation(remote_dir: str, guest_dir: str) -> None:
    for label, path in (("REMOTE_DIR", remote_dir), ("GUEST_DIR", guest_dir)):
        norm = path.rstrip("/").lower()
        for slug in _FORBIDDEN_SLUGS:
            if norm.endswith(f"/{slug}") or norm == slug:
                sys.exit(
                    f"[FAIL] {label}={path!r} targets forbidden slug {slug!r}. "
                    "DNA_Tools must stay under dna_tools only."
                )
        if "dna_tools" not in norm:
            sys.exit(
                f"[FAIL] {label}={path!r} must contain 'dna_tools' "
                "(DNA isolation on the shared builder)."
            )


def _resolve_config(s: Settings) -> tuple[str, str, str, str, str]:
    """Return (remote_dir, guest_dir, win_host, win_user, win_guest_dir)."""
    vm_user = os.environ.get("VM_USER", s.vm_user)
    remote_dir = os.environ.get("REMOTE_DIR", f"/home/{s.remote_user}/dna_tools")
    guest_dir = os.environ.get("GUEST_DIR", f"/Users/{vm_user}/dna_tools")
    win_host = os.environ.get("WIN_VM_HOST", "")
    win_user = os.environ.get("WIN_VM_USER", "")
    win_guest_dir = os.environ.get(
        "WIN_GUEST_DIR",
        f"C:/Users/{win_user}/dna_tools" if win_user else "",
    )
    return remote_dir, guest_dir, win_host, win_user, win_guest_dir


# ════════════════════════════════════════════════════════════════════════════
# Per-platform build orchestrators
# ════════════════════════════════════════════════════════════════════════════

def _build_macos(
    s: Settings,
    remote_dir: str,
    guest_dir: str,
    args: argparse.Namespace,
) -> None:
    print(f"\n{'═'*54}")
    print("  TARGET: macOS Universal (arm64 + x86_64)")
    print(f"{'═'*54}")

    if not args.skip_upgrade:
        _toolchain.upgrade_macos(guest_dir, settings=s)

    tarball = _sync.create_tarball(_PROJECT_ROOT)
    try:
        _sync.prepare_host_workspace(tarball, remote_dir, settings=s)
        _sync.rsync_host_to_guest(remote_dir, guest_dir, settings=s)
    finally:
        _sync.cleanup_local_tarball(tarball)

    if not args.skip_lint:
        _lint.lint_macos(guest_dir, strict=args.strict_lint, settings=s)

    if args.lint_only:
        print("[LINT-ONLY] macOS done.")
        return

    _macos.build(guest_dir, clean=args.clean, bundle=args.create_bundle, settings=s)

    local_dir = str(_PROJECT_ROOT / "builds" / "macOS")
    _collect.collect_macos(guest_dir, remote_dir, local_dir, settings=s)

    if not args.no_purge_after:
        _cache.purge_macos(
            guest_dir, remote_dir,
            mode="full" if args.clean else "incremental",
            settings=s,
        )

    print("\n[SUCCESS] macOS Universal → builds/macOS/")


def _build_linux(
    s: Settings,
    remote_dir: str,
    args: argparse.Namespace,
) -> None:
    print(f"\n{'═'*54}")
    print("  TARGET: Linux x86_64")
    print(f"{'═'*54}")

    # Linux builds run ON the builder host — guest_dir == remote_dir
    guest_dir = remote_dir

    if not args.skip_upgrade:
        _toolchain.upgrade_linux(guest_dir, settings=s)

    tarball = _sync.create_tarball(_PROJECT_ROOT)
    try:
        _sync.prepare_host_workspace(tarball, remote_dir, settings=s)
    finally:
        _sync.cleanup_local_tarball(tarball)

    if not args.skip_lint:
        _lint.lint_linux(guest_dir, strict=args.strict_lint, settings=s)

    if args.lint_only:
        print("[LINT-ONLY] Linux done.")
        return

    _linux.build(guest_dir, clean=args.clean, bundle=args.create_bundle, settings=s)

    local_dir = str(_PROJECT_ROOT / "builds" / "linux")
    _collect.collect_linux(guest_dir, remote_dir, local_dir, settings=s)

    if not args.no_purge_after:
        _cache.purge_linux(
            guest_dir, remote_dir,
            mode="full" if args.clean else "incremental",
            settings=s,
        )

    print("\n[SUCCESS] Linux x86_64 → builds/linux/")


def _build_windows(
    s: Settings,
    remote_dir: str,
    args: argparse.Namespace,
) -> None:
    print(f"\n{'═'*54}")
    print("  TARGET: Windows x86_64 (Cross-Compiled on Linux)")
    print(f"{'═'*54}")

    # For cross-compilation, the guest is the same Linux builder.
    guest_dir = remote_dir

    if not args.skip_upgrade:
        _toolchain.upgrade_linux(guest_dir, settings=s)

    tarball = _sync.create_tarball(_PROJECT_ROOT)
    try:
        _sync.prepare_host_workspace(tarball, remote_dir, settings=s)
    finally:
        _sync.cleanup_local_tarball(tarball)

    if not args.skip_lint:
        _lint.run_lint("windows", guest_dir, strict=args.strict_lint, settings=s)

    if args.lint_only:
        print("[LINT-ONLY] Windows done.")
        return

    # Use the linux builder to cross-compile to windows
    _windows.build_cross(remote_dir, clean=args.clean, bundle=args.create_bundle, settings=s)

    local_dir = str(_PROJECT_ROOT / "builds" / "windows")
    _collect.collect_linux_cross_windows(remote_dir, local_dir, settings=s)

    if not args.no_purge_after:
        _cache.purge_linux(
            guest_dir, remote_dir,
            mode="full" if args.clean else "incremental",
            settings=s,
        )

    print("\n[SUCCESS] Windows x86_64 → builds/windows/")


# ════════════════════════════════════════════════════════════════════════════
# CLI
# ════════════════════════════════════════════════════════════════════════════

def main() -> None:
    parser = argparse.ArgumentParser(
        description="DNA-Tools remote build engine (macOS / Linux / Windows)",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--target", "-t",
        nargs="+",
        default=["mac"],
        choices=["mac", "linux", "windows", "all"],
        metavar="PLATFORM",
        help="Build target(s): mac linux windows all  (default: mac)",
    )
    parser.add_argument("--clean", action="store_true", help="cargo clean before build (cold)")
    parser.add_argument("--create-bundle", action="store_true", help="Create GitHub-shaped installers (NSIS, AppImage, folder DMG). Default is to build raw executable only.")
    parser.add_argument("--skip-frontend", action="store_true", help="Skip pnpm run build locally")
    parser.add_argument("--skip-upgrade", action="store_true", help="Skip toolchain upgrade step")
    parser.add_argument("--skip-lint", action="store_true", help="Skip clippy + svelte-check")
    parser.add_argument("--strict-lint", action="store_true", default=True, help="Treat lint warnings as errors (default: True)")
    parser.add_argument("--no-purge-after", action="store_true", help="Keep remote caches after build")
    parser.add_argument("--lint-only", action="store_true", help="Run lint only, skip compile")
    args = parser.parse_args()

    # Expand 'all'
    raw_targets: list[str] = []
    for t in args.target:
        raw_targets.extend(["mac", "linux", "windows"] if t == "all" else [t])
    targets = list(dict.fromkeys(raw_targets))  # dedupe, preserve order

    # Config
    s = get_settings()
    remote_dir, guest_dir, win_host, win_user, win_guest_dir = _resolve_config(s)

    print("╔══════════════════════════════════════════════════════╗")
    print("║       DNA-Tools Remote Build Engine                  ║")
    print("╚══════════════════════════════════════════════════════╝")
    print(f"  Targets     : {', '.join(targets)}")
    print(f"  Builder     : {s.remote_user}@{s.remote_host}")
    print(f"  Remote dir  : {remote_dir}")
    print(f"  macOS guest : {s.vm_user}@{s.vm_host}  →  {guest_dir}")
    if win_host:
        print(f"  WinServer   : {win_user}@{win_host}  →  {win_guest_dir}")
    else:
        print("  WinServer   : not configured (set WIN_VM_HOST + WIN_VM_USER in .env)")
    print(f"  --clean     : {args.clean}")
    print(f"  --skip-upg  : {args.skip_upgrade}")
    print(f"  --skip-lint : {args.skip_lint}")
    print(f"  --strict    : {args.strict_lint}")
    print(f"  --no-purge  : {args.no_purge_after}")
    print(f"  --lint-only : {args.lint_only}")
    print()

    # Safety gate
    _assert_dna_isolation(remote_dir, guest_dir)

    # Local frontend build (once, before all targets)
    if not args.skip_frontend and not args.lint_only:
        _sync.build_frontend(_PROJECT_ROOT)

    start = time.monotonic()
    errors: list[str] = []

    for target in targets:
        try:
            if target == "mac":
                _build_macos(s, remote_dir, guest_dir, args)
            elif target == "linux":
                _build_linux(s, remote_dir, args)
            elif target == "windows":
                _build_windows(s, remote_dir, args)
        except SystemExit as exc:
            msg = str(exc).strip()
            print(f"\n[FAIL] Target {target!r}: {msg}", file=sys.stderr)
            errors.append(f"{target}: {msg}")

    elapsed = time.monotonic() - start
    print()
    print(f"{'═'*54}")
    print(f"  Total time: {int(elapsed//60)}m {int(elapsed%60)}s")
    if errors:
        print(f"  FAILED targets ({len(errors)}):")
        for e in errors:
            print(f"    ✗ {e}")
        print(f"{'═'*54}")
        sys.exit(1)
    print("  ALL TARGETS SUCCEEDED ✓")
    print(f"{'═'*54}")


if __name__ == "__main__":
    main()
