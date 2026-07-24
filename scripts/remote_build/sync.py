# ./scripts/remote_build/sync.py
"""
Source packaging and upload helpers.

Responsibilities:
  - Build the Svelte frontend locally (npm run build).
  - Create a filtered workspace tarball (no .git, target, node_modules, etc.).
  - Strip beforeBuildCommand from tauri.conf.json during packaging (remote
    builds must not re-trigger a frontend build inside the VM/host).
  - Upload the tarball to the Ubuntu builder host.
  - Extract + rsync to a target destination (builder filesystem or macOS guest).

Inputs:
  project_root: Path to the DNA_Tools workspace.
  settings:     Remote_Build Settings (host + guest credentials).

Outputs / side effects:
  Tarball at a temp path → uploaded to REMOTE_DIR on the builder.
  Returns the remote path where sources are ready for compilation.
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import tarfile
import tempfile
import uuid
from pathlib import Path
from typing import Sequence

from remote_build.config import Settings, get_settings
from remote_build.upgrade.host import host_bash, host_ssh

# Names never included in the source tarball
_EXCLUDE_NAMES: frozenset[str] = frozenset(
    {".git", "target", "node_modules", "dist", ".svelte-kit", "builds", "App"}
)

# Patterns inside the tarball that identify large data files not needed for compilation
_SKIP_REPORT_PATTERNS = ("report", "ancestry", "Roy", "Taye", "Clara", "Jen")


def build_frontend(project_root: Path) -> None:
    """Run `npm run build` in the project root."""
    print("[SYNC] Building Svelte frontend locally…")
    res = subprocess.run(
        ["npm", "run", "build"],
        cwd=str(project_root),
        capture_output=True,
        text=True,
    )
    if res.stdout:
        print(res.stdout.rstrip())
    if res.returncode != 0:
        print(res.stderr, file=sys.stderr)
        raise SystemExit(f"[FAIL] npm run build exited {res.returncode}")
    print("[SYNC] Frontend build OK.")


def _tar_filter(tarinfo: tarfile.TarInfo) -> tarfile.TarInfo | None:
    parts = Path(tarinfo.name).parts
    for name in _EXCLUDE_NAMES:
        if name in parts:
            return None
    # Skip large data files that are never needed for compilation
    if tarinfo.name.endswith((".zip", ".txt", ".json")) and tarinfo.name not in (
        "package.json",
        "package-lock.json",
    ) and not tarinfo.name.endswith("tauri.conf.json"):
        if any(p in tarinfo.name for p in _SKIP_REPORT_PATTERNS):
            return None
    return tarinfo


def create_tarball(project_root: Path) -> Path:
    """
    Package the workspace into a compressed tarball, temporarily stripping
    beforeBuildCommand from tauri.conf.json so the remote side does not
    re-trigger the frontend build.

    Returns the local tarball path.
    """
    tauri_conf = project_root / "src-tauri" / "tauri.conf.json"
    original = tauri_conf.read_text(encoding="utf-8")

    tarball_name = f"dna_tools_src_{uuid.uuid4().hex[:8]}.tar.gz"
    tarball = Path(tempfile.gettempdir()) / tarball_name
    print(f"[SYNC] Creating workspace tarball → {tarball}")

    try:
        # Temporarily strip beforeBuildCommand so remote builds skip frontend
        conf = json.loads(original)
        if "build" in conf and "beforeBuildCommand" in conf["build"]:
            conf["build"]["beforeBuildCommand"] = ""
        tauri_conf.write_text(json.dumps(conf, indent=2), encoding="utf-8")

        with tarfile.open(tarball, "w:gz") as tar:
            for p in project_root.iterdir():
                if p.name in _EXCLUDE_NAMES:
                    continue
                tar.add(p, arcname=p.name, filter=_tar_filter)
    finally:
        # Always restore original tauri.conf.json
        tauri_conf.write_text(original, encoding="utf-8")

    size_mb = tarball.stat().st_size / 1024 / 1024
    print(f"[SYNC] Tarball ready: {size_mb:.1f} MB")
    return tarball


def upload_to_host(
    tarball: Path,
    remote_dir: str,
    *,
    settings: Settings | None = None,
) -> None:
    """SCP the tarball to the Ubuntu builder host."""
    s = settings or get_settings()
    key = s.ssh_key_expanded
    dest = f"{s.remote_user}@{s.remote_host}:{remote_dir}/{tarball.name}"
    print(f"[SYNC] Uploading {tarball.name} to {s.remote_host}:{remote_dir}…")
    cmd = [
        "scp",
        "-i", key,
        "-o", "StrictHostKeyChecking=no",
        "-o", "BatchMode=yes",
        str(tarball),
        dest,
    ]
    res = subprocess.run(cmd, capture_output=True, text=True)
    if res.returncode != 0:
        print(res.stderr, file=sys.stderr)
        raise SystemExit(f"[FAIL] SCP upload failed: {res.returncode}")
    print("[SYNC] Upload complete.")


def extract_on_host(remote_dir: str, tarball_name: str, *, settings: Settings | None = None) -> None:
    """Extract the tarball on the builder host and delete it."""
    print(f"[SYNC] Extracting on host at {remote_dir}…")
    host_bash(
        f"""
set -euo pipefail
cd '{remote_dir}'
tar -xzf '{tarball_name}'
rm -f '{tarball_name}'
echo "EXTRACTED OK"
""",
        settings=settings,
        check=True,
        timeout=120,
    )


def rsync_host_to_guest(
    remote_dir: str,
    guest_dir: str,
    *,
    settings: Settings | None = None,
) -> None:
    """
    Rsync source tree from the Ubuntu builder host into the macOS guest.
    Runs as a command on the host (via host_bash) so we only need one SSH hop.
    """
    s = settings or get_settings()
    key = "~/.ssh/id_ed25519"  # key on the builder host, not local
    print(f"[SYNC] Rsyncing host:{remote_dir} → guest:{guest_dir}…")
    host_bash(
        f"""
set -euo pipefail
rsync -az --delete \\
  --exclude 'target' --exclude '.git' --exclude 'node_modules' \\
  -e 'ssh -i {key} -o StrictHostKeyChecking=no -o BatchMode=yes' \\
  '{remote_dir}/' '{s.vm_user}@{s.vm_host}:{guest_dir}/'
echo "RSYNC OK"
""",
        settings=settings,
        check=True,
        timeout=300,
    )


def prepare_host_workspace(
    tarball: Path,
    remote_dir: str,
    *,
    settings: Settings | None = None,
) -> None:
    """Full sequence: mkdir → upload → extract on host."""
    s = settings or get_settings()
    host_bash(
        f"mkdir -p '{remote_dir}'",
        settings=s,
        check=True,
        timeout=30,
    )
    upload_to_host(tarball, remote_dir, settings=s)
    extract_on_host(remote_dir, tarball.name, settings=s)


def cleanup_local_tarball(tarball: Path) -> None:
    if tarball.exists():
        tarball.unlink()
        print(f"[SYNC] Deleted local tarball {tarball.name}")
