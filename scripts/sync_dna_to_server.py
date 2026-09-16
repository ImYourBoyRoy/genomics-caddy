# ./scripts/sync_dna_to_server.py
"""
DEPRECATED — use scripts/build.py instead.

  python scripts/build.py --target mac        # macOS Universal
  python scripts/build.py --target linux      # Linux x86_64
  python scripts/build.py --target all        # all platforms

scripts/build.py uses the Remote_Build toolkit primitives, upgrades the
toolchain before every build, runs lint with warnings, and purges remote
caches after success. This script is kept for backward compatibility only.

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

DNA-Tools remote sync & Universal macOS compile on the Ubuntu builder + KVM guest.

How to run (repo root — DEPRECATED):
  python scripts/sync_dna_to_server.py
  python scripts/sync_dna_to_server.py --clean

Inputs:
  Local DNA_Tools tree + optional root `.env` (REMOTE_*/VM_*/GUEST_DIR).
  Builds Svelte frontend locally, then syncs to host `REMOTE_DIR`
  and guest `GUEST_DIR` (defaults: …/dna_tools).

Outputs / side effects:
  Universal `.app` + DMG under local `builds/macOS/`.
  Never syncs into or cleans remote `biolume` (other-project game tree).

Operational notes:
  Prefer LAN REMOTE_HOST=192.168.1.21. Guest is only reachable via the builder jump.
  Config uses toolkit-native REMOTE_*/VM_* names — not other-project aliases.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path

# Forbidden remote slugs on the shared builder (other projects — do not touch).
_FORBIDDEN_REMOTE_SLUGS = ("biolume",)


def load_dotenv() -> None:
    env_path = Path(".env")
    if not env_path.exists():
        return
    for line in env_path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, val = line.split("=", 1)
        key = key.strip()
        val = val.strip().strip('"').strip("'")
        if key not in os.environ:
            os.environ[key] = val


load_dotenv()

# Toolkit-native keys (aligned with Remote_Build). Defaults keep DNA slug isolation.
REMOTE_HOST = os.environ.get("REMOTE_HOST", "192.168.1.21")
REMOTE_USER = os.environ.get("REMOTE_USER", "builder-user")
REMOTE_DIR = os.environ.get("REMOTE_DIR", "/home/builder-user/dna_tools")
# Production guest is Tahoe domain macOS @ .142 (Sonoma rollback was typically .75).
VM_HOST = os.environ.get("VM_HOST", "192.168.122.142")
VM_USER = os.environ.get("VM_USER", "guest-user")
VM_SSH_KEY = os.environ.get("VM_SSH_KEY", "~/.ssh/id_ed25519")
VM_NAME = os.environ.get("VM_NAME", "macOS")
GUEST_DIR = os.environ.get("GUEST_DIR", f"/Users/{VM_USER}/dna_tools")


def assert_dna_isolation() -> None:
    """Refuse to run if REMOTE_DIR / GUEST_DIR point at another project's tree."""
    for label, path in (("REMOTE_DIR", REMOTE_DIR), ("GUEST_DIR", GUEST_DIR)):
        normalized = path.rstrip("/").lower()
        for slug in _FORBIDDEN_REMOTE_SLUGS:
            if normalized.endswith(f"/{slug}") or normalized == slug:
                print(
                    f"[FAIL] {label}={path!r} targets forbidden remote slug {slug!r}. "
                    "DNA_Tools must stay under dna_tools only.",
                    file=sys.stderr,
                )
                sys.exit(2)
        if "dna_tools" not in normalized:
            print(
                f"[FAIL] {label}={path!r} must contain 'dna_tools' "
                "(DNA isolation on the shared builder).",
                file=sys.stderr,
            )
            sys.exit(2)


def run_local(cmd: list[str], check: bool = True) -> subprocess.CompletedProcess[str]:
    print(f"[LOCAL RUN] {' '.join(cmd)}")
    return subprocess.run(cmd, check=check, capture_output=True, text=True)


def run_ssh_host(cmd_str: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    print(f"[HOST SSH] {cmd_str}")
    ssh_cmd = [
        "ssh",
        "-i",
        os.path.expanduser(VM_SSH_KEY),
        "-o",
        "StrictHostKeyChecking=no",
        f"{REMOTE_USER}@{REMOTE_HOST}",
        cmd_str,
    ]
    res = subprocess.run(ssh_cmd, capture_output=True, text=True)
    if check and res.returncode != 0:
        print(f"Error executing on host:\nStdout: {res.stdout}\nStderr: {res.stderr}")
        sys.exit(res.returncode)
    return res


def main() -> None:
    parser = argparse.ArgumentParser(description="Sync and compile DNA-Tools on macOS VM.")
    parser.add_argument("--clean", action="store_true", help="Clean VM build target first.")
    args = parser.parse_args()

    assert_dna_isolation()
    print(f"[CONFIG] host={REMOTE_USER}@{REMOTE_HOST} REMOTE_DIR={REMOTE_DIR}")
    print(f"[CONFIG] guest={VM_USER}@{VM_HOST} GUEST_DIR={GUEST_DIR} VM_NAME={VM_NAME}")

    # 0. Build the frontend locally first
    print("[LOCAL BUILD] Compiling Svelte frontend assets locally...")
    run_local(["pnpm", "run", "build"])

    # 1. Create tarball of current workspace (excluding build caches, but including the compiled frontend build/ directory)
    print("[PACKAGING] Creating workspace tarball...")
    temp_dir = tempfile.gettempdir()
    tarball_path = os.path.join(temp_dir, "dna_tools_src.tar.gz")

    exclude_names = {".git", "target", "node_modules", "dist", ".svelte-kit", "builds", "App"}

    def tar_filter(tarinfo: tarfile.TarInfo) -> tarfile.TarInfo | None:
        path_parts = Path(tarinfo.name).parts
        for name in exclude_names:
            if name in path_parts:
                return None
        # Exclude large data files not needed for compilation
        if (
            tarinfo.name.endswith((".zip", ".txt", ".json"))
            and tarinfo.name != "package.json"
            and not tarinfo.name.endswith("tauri.conf.json")
        ):
            if "report" in tarinfo.name or "ancestry" in tarinfo.name or "Roy" in tarinfo.name:
                return None
        return tarinfo

    # Temporarily remove beforeBuildCommand in tauri.conf.json for VM compilation
    tauri_conf_path = Path("src-tauri/tauri.conf.json")
    original_tauri_conf = tauri_conf_path.read_text(encoding="utf-8")

    try:
        conf_data = json.loads(original_tauri_conf)
        if "build" in conf_data and "beforeBuildCommand" in conf_data["build"]:
            conf_data["build"]["beforeBuildCommand"] = ""
        tauri_conf_path.write_text(json.dumps(conf_data, indent=2), encoding="utf-8")

        with tarfile.open(tarball_path, "w:gz") as tar:
            for p in Path(".").iterdir():
                if p.name in exclude_names:
                    continue
                tar.add(p, arcname=p.name, filter=tar_filter)
    finally:
        # Restore original tauri.conf.json immediately
        tauri_conf_path.write_text(original_tauri_conf, encoding="utf-8")

    print(f"[PACKAGING] Tarball created at {tarball_path}")

    # 2. Upload tarball to remote Ubuntu Host
    print("[UPLOAD] Uploading source tarball to remote Ubuntu host...")
    run_ssh_host(f"mkdir -p {REMOTE_DIR}")
    scp_cmd = [
        "scp",
        "-i",
        os.path.expanduser(VM_SSH_KEY),
        "-o",
        "StrictHostKeyChecking=no",
        tarball_path,
        f"{REMOTE_USER}@{REMOTE_HOST}:{REMOTE_DIR}/dna_tools_src.tar.gz",
    ]
    run_local(scp_cmd)

    # 3. Extract tarball on remote Ubuntu Host
    print("[EXTRACT] Extracting source tarball on host...")
    run_ssh_host(f"cd {REMOTE_DIR} && tar -xzf dna_tools_src.tar.gz && rm -f dna_tools_src.tar.gz")

    # 4. Sync workspace from remote Ubuntu Host to guest macOS VM
    print("[VM SYNC] Syncing files from host to macOS VM guest...")
    vm_sync_cmd = (
        f"rsync -avz --delete "
        f"--exclude 'target' --exclude '.git' --exclude 'node_modules' "
        f"-e 'ssh -i {VM_SSH_KEY} -o StrictHostKeyChecking=no' "
        f"{REMOTE_DIR}/ {VM_USER}@{VM_HOST}:{GUEST_DIR}/"
    )
    run_ssh_host(vm_sync_cmd)

    # 5. Execute Tauri compilation on guest macOS VM
    print("[VM BUILD] Compiling Tauri macOS Universal release on guest VM...")
    clean_step = "cargo clean && " if args.clean else ""

    # Compile individual architectures first
    compile_targets_cmd = (
        f"ssh -i {VM_SSH_KEY} -o StrictHostKeyChecking=no {VM_USER}@{VM_HOST} "
        f"\"source ~/.profile && "
        f"cd {GUEST_DIR}/src-tauri && "
        f"{clean_step}"
        f"cargo build --release --target aarch64-apple-darwin && "
        f"cargo build --release --target x86_64-apple-darwin\""
    )
    run_ssh_host(compile_targets_cmd)

    # Run lipo to merge both main binary and secondary binary (inspect_db)
    lipo_cmd = (
        f"ssh -i {VM_SSH_KEY} -o StrictHostKeyChecking=no {VM_USER}@{VM_HOST} "
        f"\"source ~/.profile && "
        f"mkdir -p {GUEST_DIR}/src-tauri/target/universal-apple-darwin/release && "
        f"lipo -create -output {GUEST_DIR}/src-tauri/target/universal-apple-darwin/release/DNA-Tools "
        f"{GUEST_DIR}/src-tauri/target/aarch64-apple-darwin/release/DNA-Tools "
        f"{GUEST_DIR}/src-tauri/target/x86_64-apple-darwin/release/DNA-Tools && "
        f"lipo -create -output {GUEST_DIR}/src-tauri/target/universal-apple-darwin/release/inspect_db "
        f"{GUEST_DIR}/src-tauri/target/aarch64-apple-darwin/release/inspect_db "
        f"{GUEST_DIR}/src-tauri/target/x86_64-apple-darwin/release/inspect_db\""
    )
    run_ssh_host(lipo_cmd)

    # Run cargo tauri build. It will bundle the app and then try to run bundle_dmg.sh.
    build_tauri_cmd = (
        f"ssh -i {VM_SSH_KEY} -o StrictHostKeyChecking=no {VM_USER}@{VM_HOST} "
        f"\"source ~/.profile && "
        f"cd {GUEST_DIR} && "
        f"cargo tauri build --target universal-apple-darwin\""
    )
    res = run_ssh_host(build_tauri_cmd, check=False)

    if res.returncode != 0:
        # Check if failure is due to AppleScript in headless VM context
        print("[VM BUILD] Packaging failed. Patching bundle_dmg.sh on VM to bypass AppleScript...")
        patch_cmd = (
            f"ssh -i {VM_SSH_KEY} -o StrictHostKeyChecking=no {VM_USER}@{VM_HOST} "
            f"\"sed -i.bak 's/SKIP_JENKINS -eq 0/1 -eq 0/' "
            f"{GUEST_DIR}/src-tauri/target/universal-apple-darwin/release/bundle/dmg/bundle_dmg.sh && "
            f"cd {GUEST_DIR}/src-tauri/target/universal-apple-darwin/release/bundle/dmg && "
            f"./bundle_dmg.sh 'Genomics Caddy_0.2.0_universal.dmg' '../macos/Genomics Caddy.app'\""
        )
        run_ssh_host(patch_cmd)

    # 6. Copy final bundles back to host server and local builds directory
    print("[DOWNLOAD] Fetching completed macOS bundles...")

    vm_bundle_dir = f"{GUEST_DIR}/src-tauri/target/universal-apple-darwin/release/bundle"
    remote_bundle_dir = f"{REMOTE_DIR}/builds/macOS"
    local_builds_dir = "builds/macOS"

    # Copy from VM to Ubuntu host
    run_ssh_host(f"mkdir -p {remote_bundle_dir}")
    scp_from_vm = (
        f"scp -i {VM_SSH_KEY} -o StrictHostKeyChecking=no -r "
        f"{VM_USER}@{VM_HOST}:{vm_bundle_dir}/* {remote_bundle_dir}/"
    )
    run_ssh_host(scp_from_vm)

    # Copy from Ubuntu host to local builds directory
    os.makedirs(local_builds_dir, exist_ok=True)
    scp_to_local = [
        "scp",
        "-i",
        os.path.expanduser(VM_SSH_KEY),
        "-o",
        "StrictHostKeyChecking=no",
        "-r",
        f"{REMOTE_USER}@{REMOTE_HOST}:{remote_bundle_dir}/*",
        local_builds_dir,
    ]
    run_local(scp_to_local)

    # Clean up local temporary tarball
    if os.path.exists(tarball_path):
        os.remove(tarball_path)

    print("\n[SUCCESS] macOS Universal build finished successfully!")
    print(f"Artifacts downloaded to: {local_builds_dir}")


if __name__ == "__main__":
    main()
