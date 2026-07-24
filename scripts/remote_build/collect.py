# ./scripts/remote_build/collect.py
"""
Artifact download helpers.

After a successful remote build, this module copies the finished bundles from:
  macOS:   VM guest → builder host → local builds/macOS/
  Linux:   builder host directly  → local builds/linux/
  Windows: VM guest → builder host → local builds/windows/

Also cleans up macOS DMG scratch files (rw.*.dmg) left by hdiutil.

Inputs:
  platform:    "macos" | "linux" | "windows"
  guest_dir:   Path on the VM where bundles were produced
  remote_dir:  Staging path on the builder host
  local_dir:   Local destination (e.g. "builds/macOS")
  settings:    Remote_Build Settings

Outputs / side effects:
  Artifacts in local_dir/. Prints downloaded file paths and sizes.
"""
from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

from remote_build.config import Settings, get_settings
from remote_build.upgrade.host import host_bash, host_ssh


def _scp_from_host(
    remote_path: str,
    local_dir: str,
    *,
    settings: Settings,
) -> None:
    """SCP a file or directory from the builder host to a local directory."""
    key = settings.ssh_key_expanded
    src = f"{settings.remote_user}@{settings.remote_host}:{remote_path}"
    res = subprocess.run(
        [
            "scp",
            "-i", key,
            "-o", "StrictHostKeyChecking=no",
            "-o", "BatchMode=yes",
            "-r",
            src,
            local_dir,
        ],
        capture_output=True,
        text=True,
    )
    if res.returncode != 0:
        print(res.stderr, file=sys.stderr)
        raise SystemExit(f"[FAIL] scp download from host exited {res.returncode}")


def _clean_scratch_dmgs(directory: Path) -> None:
    """Remove hdiutil scratch files (rw.*.dmg) left in the bundle directory."""
    removed = 0
    for f in directory.rglob("rw.*.dmg"):
        size_mb = f.stat().st_size / 1024 / 1024
        print(f"[COLLECT] Removing scratch DMG {f.name} ({size_mb:.0f} MB)")
        f.unlink()
        removed += 1
    if removed:
        print(f"[COLLECT] Removed {removed} scratch DMG(s).")


def collect_macos(
    guest_dir: str,
    remote_dir: str,
    local_dir: str,
    *,
    settings: Settings | None = None,
) -> None:
    """
    Copy macOS bundles:  guest → host staging → local builds/macOS/
    """
    s = settings or get_settings()
    guest_bundle_dir = f"{guest_dir}/src-tauri/target/universal-apple-darwin/release/bundle"
    remote_bundle_dir = f"{remote_dir}/builds/macOS"

    print(f"[COLLECT] macOS: guest:{guest_bundle_dir} → host:{remote_bundle_dir}")
    # 1. Copy from guest to builder host (runs on the host — one SSH hop)
    host_bash(
        f"""
set -euo pipefail
mkdir -p '{remote_bundle_dir}'
scp -i ~/.ssh/id_ed25519 -o StrictHostKeyChecking=no -o BatchMode=yes -r \\
    '{s.vm_user}@{s.vm_host}:{guest_bundle_dir}/.' '{remote_bundle_dir}/'
echo "GUEST_TO_HOST_OK"
""",
        settings=s,
        check=True,
        timeout=300,
    )

    # 2. Copy from host to local
    os.makedirs(local_dir, exist_ok=True)
    print(f"[COLLECT] macOS: host:{remote_bundle_dir} → local:{local_dir}")
    _scp_from_host(f"{remote_bundle_dir}/.", local_dir, settings=s)

    # 3. Clean scratch DMGs
    _clean_scratch_dmgs(Path(local_dir))

    _report_local(local_dir)


def collect_linux(
    guest_dir: str,
    remote_dir: str,
    local_dir: str,
    *,
    settings: Settings | None = None,
) -> None:
    """
    Copy Linux bundles: builder host → local builds/linux/
    Linux builds run on the host itself, so no VM hop needed.
    """
    s = settings or get_settings()
    triple = "x86_64-unknown-linux-gnu"
    remote_release_dir = f"{guest_dir}/src-tauri/target/{triple}/release"
    remote_bundle_dir = f"{remote_release_dir}/bundle"
    remote_staging = f"{remote_dir}/builds/linux"

    print(f"[COLLECT] Linux: host:{remote_release_dir} → local:{local_dir}")

    # Gather the bundles and raw executables into a staging dir on the host
    host_bash(
        f"""
set -euo pipefail
mkdir -p '{remote_staging}'
# Copy bundles (.deb, .rpm, .AppImage) and avoid pulling unpacked intermediate directories
if [ -d "{remote_bundle_dir}" ]; then
    find "{remote_bundle_dir}" -type f \\( -name '*.deb' -o -name '*.rpm' -o -name '*.AppImage' \\) -exec cp {{}} '{remote_staging}/' \\;
fi
# Copy raw executables (maxdepth 1 excludes deps/, build/, etc)
find "{remote_release_dir}" -maxdepth 1 -type f -executable -exec cp {{}} '{remote_staging}/' \\;
echo "LINUX_STAGING_OK"
""",
        settings=s,
        check=True,
        timeout=60,
    )

    os.makedirs(local_dir, exist_ok=True)
    _scp_from_host(f"{remote_staging}/.", local_dir, settings=s)
    _report_local(local_dir)


def collect_linux_cross_windows(
    remote_dir: str,
    local_dir: str,
    *,
    settings: Settings | None = None,
) -> None:
    """Copy Windows bundles: builder host → local builds/windows/"""
    s = settings or get_settings()
    remote_release_dir = f"{remote_dir}/src-tauri/target/x86_64-pc-windows-gnu/release"
    remote_staging = f"{remote_dir}/builds/windows"

    print(f"[COLLECT] Windows Cross: host:{remote_release_dir} → local:{local_dir}")
    host_bash(
        f"""
set -euo pipefail
mkdir -p '{remote_staging}'
# Copy executable and MSI/NSIS from release and bundle dirs, excluding deps
find "{remote_release_dir}" -type f -not -path "*/deps/*" -not -path "*/build/*" \\( -name '*.exe' -o -name '*.msi' \\) -exec cp {{}} '{remote_staging}/' \\;
echo "WIN_CROSS_STAGING_OK"
""",
        settings=s,
        check=True,
        timeout=60,
    )

    os.makedirs(local_dir, exist_ok=True)
    _scp_from_host(f"{remote_staging}/.", local_dir, settings=s)
    _report_local(local_dir)


def collect(
    platform: str,
    guest_dir: str,
    remote_dir: str,
    local_dir: str,
    *,
    settings: Settings | None = None,
    win_host: str = "",
    win_user: str = "",
) -> None:
    """Dispatch artifact collection to the correct platform handler."""
    if platform == "macos":
        collect_macos(guest_dir, remote_dir, local_dir, settings=settings)
    elif platform == "linux":
        collect_linux(guest_dir, remote_dir, local_dir, settings=settings)
    elif platform == "windows":
        if not win_host or not win_user:
            raise SystemExit("[FAIL] Windows collect requires WIN_VM_HOST and WIN_VM_USER")
        collect_windows(guest_dir, remote_dir, local_dir, win_host, win_user, settings=settings)
    else:
        raise ValueError(f"Unknown platform: {platform!r}")


def _report_local(local_dir: str) -> None:
    """Print sizes of collected artifacts."""
    p = Path(local_dir)
    files = sorted(p.rglob("*"))
    files = [f for f in files if f.is_file()]
    if not files:
        print(f"[COLLECT] Warning: no files found in {local_dir}")
        return
    print(f"[COLLECT] Artifacts in {local_dir}:")
    for f in files:
        size_mb = f.stat().st_size / 1024 / 1024
        print(f"  {size_mb:6.1f} MB  {f.relative_to(p)}")
