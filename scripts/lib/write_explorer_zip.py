#!/usr/bin/env python3
# ./scripts/lib/write_explorer_zip.py
"""Write a PKZip archive Windows Explorer can open.

How to run:
  python3 ./scripts/lib/write_explorer_zip.py create STAGING ZIP
  python3 ./scripts/lib/write_explorer_zip.py audit ZIP
  python3 ./scripts/lib/write_explorer_zip.py self-test

create: store files only, relative forward-slash names, no ``./`` prefix,
        no duplicate names. Directory entries are omitted; ``Data/.keep``
        is enough for the empty library folder.

audit: exit 0 when the zip is Explorer-safe (real PKZip, unique names,
       DNA-Tools.exe + Data/.keep). Exit 1 with problems listed.

self-test: create a good zip, audit it, then prove a ``./``-prefixed zip fails.
"""

from __future__ import annotations

import os
import sys
import tempfile
import time
import zipfile
from pathlib import Path

REQUIRED_MEMBERS = ("DNA-Tools.exe", "Data/.keep")


def normalize_arcname(name: str) -> str:
    arc = name.replace("\\", "/").lstrip("/")
    while arc.startswith("./"):
        arc = arc[2:]
    return arc


def iter_files(staging: Path) -> list[tuple[Path, str]]:
    files: list[tuple[Path, str]] = []
    for root, dirnames, filenames in os.walk(staging):
        dirnames.sort()
        filenames.sort()
        for filename in filenames:
            full = Path(root) / filename
            arc = normalize_arcname(str(full.relative_to(staging)))
            if arc.startswith("./"):
                arc = arc[2:]
            if not arc or arc in {".", "./"}:
                continue
            files.append((full, arc))
    files.sort(key=lambda item: item[1].lower())
    return files


def create_zip(staging: Path, zip_path: Path) -> list[str]:
    if not staging.is_dir():
        raise SystemExit(f"write_explorer_zip: staging is not a directory: {staging}")
    files = iter_files(staging)
    if not files:
        raise SystemExit(f"write_explorer_zip: staging has no files: {staging}")
    zip_path.parent.mkdir(parents=True, exist_ok=True)
    if zip_path.exists():
        zip_path.unlink()
    with zipfile.ZipFile(
        zip_path,
        mode="w",
        compression=zipfile.ZIP_DEFLATED,
        allowZip64=False,
    ) as zf:
        for full, arc in files:
            modified = time.localtime(full.stat().st_mtime)
            info = zipfile.ZipInfo(arc, date_time=modified[:6])
            info.compress_type = zipfile.ZIP_DEFLATED
            info.create_system = 0  # MS-DOS; Windows Explorer is picky about Unix metadata
            info.external_attr = 0x20  # FILE_ATTRIBUTE_ARCHIVE
            info.flag_bits = 0
            zf.writestr(info, full.read_bytes(), compress_type=zipfile.ZIP_DEFLATED)
    return [arc for _full, arc in files]


def audit_zip(zip_path: Path) -> list[str]:
    problems: list[str] = []
    if not zip_path.is_file():
        return [f"missing zip {zip_path}"]
    magic = zip_path.read_bytes()[:2]
    if magic != b"PK":
        problems.append("file is not a PKZip (missing PK magic)")
        return problems
    try:
        with zipfile.ZipFile(zip_path) as zf:
            infos = zf.infolist()
    except zipfile.BadZipFile as error:
        return [f"not a readable zip: {error}"]

    names = [info.filename for info in infos]
    if not names:
        problems.append("zip has no entries")
    seen: dict[str, str] = {}
    for info in infos:
        raw = info.filename
        if "\\" in raw:
            problems.append(f"backslash in entry name: {raw!r}")
        if raw in {".", "./", ""} or raw.startswith("./"):
            problems.append(f"Explorer-hostile ./ prefix: {raw!r}")
        if info.is_dir() and raw in {"./", "."}:
            problems.append(f"root directory entry: {raw!r}")
        key = normalize_arcname(raw).rstrip("/").lower()
        if key in seen:
            problems.append(f"duplicate entry {raw!r} collides with {seen[key]!r}")
        else:
            seen[key] = raw
    normalized = {normalize_arcname(name) for name in names}
    for required in REQUIRED_MEMBERS:
        if required not in normalized:
            problems.append(f"missing {required}")
    return problems


def cmd_create(staging: str, zip_path: str) -> int:
    entries = create_zip(Path(staging), Path(zip_path))
    print("\n".join(entries))
    return 0


def cmd_audit(zip_path: str) -> int:
    problems = audit_zip(Path(zip_path))
    if problems:
        print("write_explorer_zip audit failed:", file=sys.stderr)
        for problem in problems:
            print(f"- {problem}", file=sys.stderr)
        return 1
    with zipfile.ZipFile(zip_path) as zf:
        print("\n".join(zf.namelist()))
    return 0


def cmd_self_test() -> int:
    with tempfile.TemporaryDirectory(prefix="explorer-zip-") as tmp:
        staging = Path(tmp) / "staging"
        (staging / "Data").mkdir(parents=True)
        (staging / "Data" / ".keep").write_bytes(b"")
        (staging / "DNA-Tools.exe").write_bytes(b"fake")
        good = Path(tmp) / "good.zip"
        create_zip(staging, good)
        problems = audit_zip(good)
        if problems:
            print("good zip failed audit:", problems, file=sys.stderr)
            return 1
        hostile = Path(tmp) / "hostile.zip"
        with zipfile.ZipFile(hostile, "w") as zf:
            zf.writestr("./", b"")
            zf.writestr("./Data/", b"")
            zf.writestr("./DNA-Tools.exe", b"fake")
            zf.writestr("./Data/.keep", b"")
        hostile_problems = audit_zip(hostile)
        if not hostile_problems:
            print("hostile ./ zip was accepted; auditor is too weak", file=sys.stderr)
            return 1
    print("write_explorer_zip self-test: Explorer-safe writer rejects ./ prefixes")
    return 0


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print(__doc__.strip(), file=sys.stderr)
        return 2
    command = argv[1]
    if command == "create" and len(argv) == 4:
        return cmd_create(argv[2], argv[3])
    if command == "audit" and len(argv) == 3:
        return cmd_audit(argv[2])
    if command == "self-test" and len(argv) == 2:
        return cmd_self_test()
    print(__doc__.strip(), file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
