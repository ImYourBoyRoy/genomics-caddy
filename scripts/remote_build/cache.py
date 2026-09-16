# ./scripts/remote_build/cache.py
"""
Remote build cache purge.

After a successful build + artifact download, this module purges the heavy
incremental build artifacts from both the builder host and the macOS/Windows
VM guest. Keeps the final release binaries for incremental linking next run,
but removes .rlib, build/, and deps/ directories which account for ~80% of
disk usage in the Rust target directory.

Purge modes:
  "incremental" (default) — keeps release binary, removes build/ + deps/ only
  "full"                  — cargo clean (full wipe, next build = cold)

Inputs:
  platform:   "macos" | "linux" | "windows"
  guest_dir:  Path on the remote target (host or VM) where the build ran
  mode:       "incremental" | "full"
  settings:   Remote_Build Settings

Outputs / side effects:
  Prints freed space. Removes build artifacts on the remote machine.
"""
from __future__ import annotations

from remote_build.config import Settings, get_settings
from remote_build.upgrade.host import host_bash
from remote_build.ssh import vm_ssh

# Subdirectories under target/<triple>/release/ that are safe to delete
# (they are regenerated on next build; not needed for incremental linking)
_PURGEABLE_SUBDIRS = ["build", "deps", "incremental", ".fingerprint"]

# macOS: purge all three target directories
_MACOS_PURGE_SCRIPT_INCREMENTAL = """
set -euo pipefail
source ~/.cargo/env 2>/dev/null || true
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:$PATH"

GUEST_DIR='{guest_dir}'
TARGETS=(
    "aarch64-apple-darwin"
    "x86_64-apple-darwin"
    "universal-apple-darwin"
)
SUBDIRS=({subdirs})

echo "[CACHE] Incremental purge — macOS targets"
before=$(df -h "$GUEST_DIR" 2>/dev/null | awk 'NR==2{{print $4}}')

for t in "${{TARGETS[@]}}"; do
    tdir="$GUEST_DIR/src-tauri/target/$t/release"
    if [ ! -d "$tdir" ]; then continue; fi
    for sub in "${{SUBDIRS[@]}}"; do
        subdir="$tdir/$sub"
        if [ -d "$subdir" ]; then
            echo "  rm -rf $subdir"
            rm -rf "$subdir"
        fi
    done
done

# Clean host staging area (tarball was already deleted after extract, but nuke the whole dir)
if [ -d /home/{remote_user}/dna_tools ]; then
    echo "  rm -rf /home/{remote_user}/dna_tools (host staging)"
fi

after=$(df -h "$GUEST_DIR" 2>/dev/null | awk 'NR==2{{print $4}}')
echo "[CACHE] Disk free before=$before after=$after"
echo "PURGE_OK"
"""

_MACOS_PURGE_SCRIPT_FULL = """
set -euo pipefail
source ~/.cargo/env 2>/dev/null || true
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:$PATH"

GUEST_DIR='{guest_dir}'
echo "[CACHE] Full cargo clean — macOS"
before=$(df -h "$GUEST_DIR" 2>/dev/null | awk 'NR==2{{print $4}}')
cd "$GUEST_DIR"
cargo clean --manifest-path src-tauri/Cargo.toml
after=$(df -h "$GUEST_DIR" 2>/dev/null | awk 'NR==2{{print $4}}')
echo "[CACHE] Disk free before=$before after=$after"
echo "PURGE_OK"
"""

# Linux: purge the builder host target directory
_LINUX_PURGE_SCRIPT_INCREMENTAL = """
set -euo pipefail
export PATH="$HOME/.cargo/bin:$PATH"

GUEST_DIR='{guest_dir}'
SUBDIRS=({subdirs})
TRIPLE="x86_64-unknown-linux-gnu"

echo "[CACHE] Incremental purge — Linux x86_64"
before=$(df -h "$GUEST_DIR" 2>/dev/null | awk 'NR==2{{print $4}}')
tdir="$GUEST_DIR/src-tauri/target/$TRIPLE/release"
if [ -d "$tdir" ]; then
    for sub in "${{SUBDIRS[@]}}"; do
        subdir="$tdir/$sub"
        if [ -d "$subdir" ]; then
            echo "  rm -rf $subdir"
            rm -rf "$subdir"
        fi
    done
fi

after=$(df -h "$GUEST_DIR" 2>/dev/null | awk 'NR==2{{print $4}}')
echo "[CACHE] Disk free before=$before after=$after"
echo "PURGE_OK"
"""

_LINUX_PURGE_SCRIPT_FULL = """
set -euo pipefail
export PATH="$HOME/.cargo/bin:$PATH"

GUEST_DIR='{guest_dir}'
echo "[CACHE] Full cargo clean — Linux"
before=$(df -h "$GUEST_DIR" 2>/dev/null | awk 'NR==2{{print $4}}')
cd "$GUEST_DIR"
cargo clean --manifest-path src-tauri/Cargo.toml
after=$(df -h "$GUEST_DIR" 2>/dev/null | awk 'NR==2{{print $4}}')
echo "[CACHE] Disk free before=$before after=$after"
echo "PURGE_OK"
"""

# Host staging area purge (always runs regardless of platform, after artifact download)
_HOST_STAGING_PURGE_SCRIPT = """
set -euo pipefail
REMOTE_DIR='{remote_dir}'
if [ -d "$REMOTE_DIR/builds" ]; then
    echo "[CACHE] Purging host staging builds dir: $REMOTE_DIR/builds"
    rm -rf "$REMOTE_DIR/builds"
    echo "HOST_STAGING_PURGED"
fi
"""


def _subdirs_bash() -> str:
    return " ".join(f'"{s}"' for s in _PURGEABLE_SUBDIRS)


def purge_macos(
    guest_dir: str,
    remote_dir: str,
    *,
    mode: str = "incremental",
    settings: Settings | None = None,
) -> None:
    s = settings or get_settings()
    print(f"[CACHE] Purging macOS guest ({s.vm_host}) — mode={mode}")

    if mode == "full":
        script = _MACOS_PURGE_SCRIPT_FULL.format(guest_dir=guest_dir)
    else:
        script = _MACOS_PURGE_SCRIPT_INCREMENTAL.format(
            guest_dir=guest_dir,
            remote_user=s.remote_user,
            subdirs=_subdirs_bash(),
        )

    vm_ssh(
        host=s.remote_host,
        user=s.remote_user,
        key=s.ssh_key_expanded,
        vm_host=s.vm_host,
        vm_user=s.vm_user,
        guest_script=script,
        check=True,
        timeout=300,
    )
    # Also purge the staging area on the builder host
    purge_host_staging(remote_dir, settings=s)
    print("[CACHE] macOS purge complete.")


def purge_linux(
    guest_dir: str,
    remote_dir: str,
    *,
    mode: str = "incremental",
    settings: Settings | None = None,
) -> None:
    s = settings or get_settings()
    print(f"[CACHE] Purging Linux builder ({s.remote_host}) — mode={mode}")

    script = (
        _LINUX_PURGE_SCRIPT_FULL.format(guest_dir=guest_dir)
        if mode == "full"
        else _LINUX_PURGE_SCRIPT_INCREMENTAL.format(
            guest_dir=guest_dir, subdirs=_subdirs_bash()
        )
    )
    host_bash(script, settings=s, check=True, timeout=300)
    purge_host_staging(remote_dir, settings=s)
    print("[CACHE] Linux purge complete.")


def purge_host_staging(
    remote_dir: str,
    *,
    settings: Settings | None = None,
) -> None:
    """Remove the source staging directory from the Ubuntu builder."""
    s = settings or get_settings()
    script = _HOST_STAGING_PURGE_SCRIPT.format(remote_dir=remote_dir)
    host_bash(script, settings=s, check=False, timeout=60, quiet=True)


def purge(
    platform: str,
    guest_dir: str,
    remote_dir: str,
    *,
    mode: str = "incremental",
    settings: Settings | None = None,
) -> None:
    """Dispatch cache purge to the correct platform."""
    if platform == "macos":
        purge_macos(guest_dir, remote_dir, mode=mode, settings=settings)
    elif platform == "linux":
        purge_linux(guest_dir, remote_dir, mode=mode, settings=settings)
    elif platform == "windows":
        print("[CACHE] Windows cache purge: TODO (platform not yet implemented)")
    else:
        raise ValueError(f"Unknown platform: {platform!r}")
