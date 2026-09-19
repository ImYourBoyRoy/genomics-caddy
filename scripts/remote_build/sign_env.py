# ./scripts/remote_build/sign_env.py
"""
Stdin-only updater signing env for remote DNA builds.

Reads the operator key from ~/.config/genomics-caddy/ without printing it.
The bash snippet is delivered through host_bash / vm_ssh stdin (argv stays clean).
Never echo TAURI_SIGNING_PRIVATE_KEY in remote scripts.
"""
from __future__ import annotations

import base64
from pathlib import Path

_KEY = Path.home() / ".config/genomics-caddy/tauri-updater.key"
_PASSWORD = Path.home() / ".config/genomics-caddy/tauri-updater.key.password"


def bash_export_snippet() -> str:
    if not _KEY.is_file():
        return 'echo "[SIGN] no local updater key; signed updater artifacts will fail after the bundle"\n'
    key_b64 = base64.b64encode(_KEY.read_bytes()).decode("ascii")
    lines = [
        f"export TAURI_SIGNING_PRIVATE_KEY=$(printf '%s' '{key_b64}' | base64 -d)",
    ]
    if _PASSWORD.is_file():
        pw_b64 = base64.b64encode(_PASSWORD.read_bytes()).decode("ascii")
        lines.append(
            f"export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=$(printf '%s' '{pw_b64}' | base64 -d | tr -d '\\n\\r')"
        )
    lines.append('echo "[SIGN] updater private key loaded for this build session"')
    return "\n".join(lines) + "\n"


def inject(script: str) -> str:
    needle = "set -euo pipefail\n"
    if needle not in script:
        return bash_export_snippet() + script
    return script.replace(needle, needle + bash_export_snippet(), 1)
