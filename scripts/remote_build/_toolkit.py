# ./scripts/remote_build/_toolkit.py
"""
Remote_Build toolkit path shim + re-exports.

The Remote_Build toolkit lives in a sibling checkout's `remote_build` package.
This DNA_Tools sub-package also uses the name `remote_build`.

Python can only resolve `import remote_build` to ONE of these. This shim
ensures the sibling Remote_Build project root is on sys.path at position 0,
so `import remote_build` finds the TOOLKIT, not this package.

NOTE: build.py inserts Remote_Build/ at sys.path[0] BEFORE loading any
module via importlib.util.spec_from_file_location. As a result, each
module in this package imports directly from remote_build.* without going
through this shim. This file is retained as a convenience shim for manual
/ REPL use — importing it from a vanilla Python session with no prior
sys.path setup will still place the toolkit on the path correctly.

Usage (standalone / REPL, without build.py):
  import scripts.remote_build._toolkit  # patches sys.path, then:
  from remote_build.config import Settings, get_settings
  from remote_build.upgrade.host import host_bash, host_ssh
  from remote_build.ssh import vm_ssh
"""
from __future__ import annotations

import sys
from pathlib import Path

# Locate the Remote_Build sibling project root
# scripts/remote_build/_toolkit.py
# → scripts/remote_build/  (parent 0)
# → scripts/               (parent 1)
# → DNA_Tools/             (parent 2)
# → AI/                    (parent 3)
# → AI/Remote_Build/       (sibling)
_TOOLKIT_ROOT = Path(__file__).resolve().parents[3] / "Remote_Build"

if not _TOOLKIT_ROOT.is_dir():
    raise RuntimeError(
        f"Remote_Build toolkit not found at {_TOOLKIT_ROOT}.\n"
        "Expected: Desktop/AI/Remote_Build/ alongside Desktop/AI/DNA_Tools/"
    )

_toolkit_str = str(_TOOLKIT_ROOT)
if _toolkit_str not in sys.path:
    sys.path.insert(0, _toolkit_str)

# After ensuring the toolkit root is on sys.path, we can import toolkit symbols.
# The `remote_build` package Python finds will be Remote_Build/remote_build/
# (not scripts/remote_build/) because:
#   1. _TOOLKIT_ROOT is at index 0 in sys.path
#   2. build.py loads these modules by absolute path, not via `scripts/` on sys.path
from remote_build.config import Settings, get_settings  # noqa: E402
from remote_build.upgrade.host import (  # noqa: E402
    HostResult,
    host_bash,
    host_ssh,
    host_sudo_bash,
    host_write_b64,
)
from remote_build.secrets import redact_text  # noqa: E402
from remote_build.ssh import ssh, vm_ssh  # noqa: E402

__all__ = [
    "Settings",
    "get_settings",
    "HostResult",
    "host_bash",
    "host_ssh",
    "host_sudo_bash",
    "host_write_b64",
    "redact_text",
    "ssh",
    "vm_ssh",
    "_TOOLKIT_ROOT",
]
