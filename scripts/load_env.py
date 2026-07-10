# ./scripts/load_env.py
"""
Load key=value pairs from a `.env` file without third-party dependencies.
Used by local smoke tests; does not override existing process environment variables.
"""

from __future__ import annotations

from pathlib import Path


def load_env_file(path: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    if not path.is_file():
        return values
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        if "=" not in line:
            continue
        key, _, value = line.partition("=")
        key = key.strip()
        value = value.strip().strip('"').strip("'")
        if key:
            values[key] = value
    return values


def apply_env_file(path: Path) -> dict[str, str]:
    """Apply `.env` to os.environ for keys not already set; return parsed values."""
    import os

    parsed = load_env_file(path)
    for key, value in parsed.items():
        os.environ.setdefault(key, value)
    return parsed


def find_env_path(start: Path | None = None) -> Path | None:
    root = start or Path(__file__).resolve().parent.parent
    candidate = root / ".env"
    return candidate if candidate.is_file() else None
