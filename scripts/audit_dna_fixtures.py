#!/usr/bin/env python3
"""Read-only audit of local DNA fixtures against curated rsID coverage.

The audit intentionally reports counts and coverage only. It never prints or
persists raw genotype values, and it does not create/import app database rows.
"""

from __future__ import annotations

import io
import json
import zipfile
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PACK_DIR = ROOT / "src" / "lib" / "marker-packs"


def curated_rsids_by_pack() -> dict[str, set[str]]:
    packs: dict[str, set[str]] = {}
    manifest_path = PACK_DIR / "manifest.json"
    try:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        pack_paths = [PACK_DIR / f"{pack['id']}.json" for pack in manifest.get("packs", [])]
    except (OSError, json.JSONDecodeError, KeyError, TypeError):
        pack_paths = sorted(PACK_DIR.glob("*.json"))

    for path in pack_paths:
        if path.name in {"discovery_catalog.json", "manifest.json"}:
            continue
        try:
            doc = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            continue
        pack_rsids: set[str] = set()
        for marker in doc.get("markers", []):
            rsid = str(marker.get("rsid", "")).lower()
            if rsid.startswith("rs"):
                pack_rsids.add(rsid)
        packs[path.stem] = pack_rsids
    return packs


def candidate_files() -> list[Path]:
    paths = []
    for path in sorted(ROOT.iterdir()):
        lower = path.name.lower()
        if path.suffix.lower() in {".txt", ".zip"} and ("dna" in lower or "ancestry" in lower):
            paths.append(path)
    return paths


def iter_lines(path: Path):
    if path.suffix.lower() == ".zip":
        archive = zipfile.ZipFile(path)
        entries = [info for info in archive.infolist() if info.filename.lower().endswith(".txt")]
        if not entries:
            raise ValueError("ZIP contains no .txt entry")
        with archive.open(entries[0], "r") as raw:
            yield from io.TextIOWrapper(raw, encoding="utf-8", errors="replace")
    else:
        with path.open("r", encoding="utf-8", errors="replace") as handle:
            yield from handle


def audit(
    path: Path,
    pack_rsids_by_pack: dict[str, set[str]],
    pack_rsids: set[str],
    hormone_rsids: set[str],
) -> dict[str, object]:
    rows = 0
    valid = 0
    rsids: set[str] = set()
    chromosomes: Counter[str] = Counter()
    y_calls = 0
    header = ""

    for raw_line in iter_lines(path):
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        columns = line.split("\t")
        lower = line.lower()
        if not header and "rsid" in lower:
            header = lower
            continue
        if len(columns) < 4:
            continue

        rsid = columns[0].strip().lower()
        chromosome = columns[1].strip().upper().removeprefix("CHR")
        try:
            position = int(columns[2].strip())
        except ValueError:
            continue
        genotype = "".join(columns[3:5]).strip() if len(columns) >= 5 else columns[3].strip()
        rows += 1
        if not rsid or not rsid.startswith("rs") or position <= 0:
            continue
        valid += 1
        rsids.add(rsid)
        chromosomes[chromosome] += 1
        if chromosome == "Y" and genotype and not any(token in genotype for token in ("-", "0", "?")):
            y_calls += 1

    pack_coverage = {
        pack_id: {
            "present": len(rsids & rsid_set),
            "total": len(rsid_set),
        }
        for pack_id, rsid_set in pack_rsids_by_pack.items()
        if rsid_set
    }

    return {
        "rows": rows,
        "valid_rows": valid,
        "curated_rsids_present": len(pack_rsids & rsids),
        "curated_rsids_total": len(pack_rsids),
        "hormone_rsids_present": len(hormone_rsids & rsids),
        "hormone_rsids_total": len(hormone_rsids),
        "y_calls": y_calls,
        "chromosomes": ",".join(f"{key}:{value}" for key, value in chromosomes.most_common()),
        "pack_coverage": pack_coverage,
    }


def main() -> int:
    packs = curated_rsids_by_pack()
    pack_rsids = set().union(*packs.values()) if packs else set()
    hormone_rsids = packs.get("hormones_reproductive", set())
    paths = candidate_files()
    if not paths:
        print("No DNA fixtures found in the repository root.")
        return 0
    print(f"Curated standard rsID markers: {len(pack_rsids)}")
    for path in paths:
        try:
            result = audit(path, packs, pack_rsids, hormone_rsids)
        except (OSError, ValueError, zipfile.BadZipFile) as error:
            print(f"{path.name}: ERROR {error}")
            return 1
        print(
            f"{path.name}: rows={result['rows']} valid={result['valid_rows']} "
            f"pack_rsids={result['curated_rsids_present']}/{result['curated_rsids_total']} "
            f"hormone_rsids={result['hormone_rsids_present']}/{result['hormone_rsids_total']} "
            f"y_calls={result['y_calls']}"
        )
        coverage = result["pack_coverage"]
        low_coverage = [
            f"{pack_id}={values['present']}/{values['total']}"
            for pack_id, values in coverage.items()
            if values["total"] >= 10 and values["present"] / values["total"] < 0.5
        ]
        print(
            f"  pack_coverage="
            + ",".join(
                f"{pack_id}:{values['present']}/{values['total']}"
                for pack_id, values in coverage.items()
            )
        )
        if low_coverage:
            print(f"  low_coverage(<50%, >=10 markers)={','.join(low_coverage)}")
    print("Raw genotype values were not emitted or persisted.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
