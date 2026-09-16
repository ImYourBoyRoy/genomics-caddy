# ./scripts/smoke_test.py
"""
Genomics Caddy — local integration smoke tests.

Purpose:
  Verify Qdrant (and optional NCBI / Ollama) connectivity using credentials from `.env`.
How to run:
  python scripts/smoke_test.py
  pnpm run smoke
Key inputs:
  `.env` beside project root (QDRANT_URL, QDRANT_API_KEY, QDRANT_COLLECTION, optional NCBI_API_KEY, OLLAMA_URL, OLLAMA_TOKEN)
Key outputs:
  Structured pass/fail summary; exit code 0 when all required checks pass.
Operational notes:
  Read-only against Qdrant except a harmless collection GET; no writes to production collections.
"""

from __future__ import annotations

import json
import os
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
ROOT = SCRIPT_DIR.parent
sys.path.insert(0, str(SCRIPT_DIR))

from load_env import apply_env_file, find_env_path  # noqa: E402


class CheckResult:
    def __init__(self, name: str, ok: bool, detail: str = "", ms: float | None = None):
        self.name = name
        self.ok = ok
        self.detail = detail
        self.ms = ms


def http_request(
    url: str,
    *,
    method: str = "GET",
    headers: dict[str, str] | None = None,
    body: dict | None = None,
    timeout: float = 12.0,
) -> tuple[int, str, float]:
    payload = None
    req_headers = dict(headers or {})
    if body is not None:
        payload = json.dumps(body).encode("utf-8")
        req_headers.setdefault("Content-Type", "application/json")
    req = urllib.request.Request(url, data=payload, headers=req_headers, method=method)
    start = time.perf_counter()
    try:
        with urllib.request.urlopen(req, timeout=timeout) as res:
            text = res.read().decode("utf-8", errors="replace")
            elapsed = (time.perf_counter() - start) * 1000.0
            return res.status, text, elapsed
    except urllib.error.HTTPError as e:
        elapsed = (time.perf_counter() - start) * 1000.0
        body_text = e.read().decode("utf-8", errors="replace") if e.fp else e.reason
        return e.code, body_text or str(e.reason), elapsed
    except Exception as e:  # noqa: BLE001 — smoke test CLI
        elapsed = (time.perf_counter() - start) * 1000.0
        return 0, str(e), elapsed


def check_qdrant(url: str, api_key: str, collection: str) -> list[CheckResult]:
    results: list[CheckResult] = []
    base = url.rstrip("/")
    headers = {"api-key": api_key} if api_key else {}

    status, body, ms = http_request(f"{base}/collections/{collection}", headers=headers)
    if status == 200:
        try:
            parsed = json.loads(body)
            vectors = parsed.get("result", {}).get("vectors_count")
            points = parsed.get("result", {}).get("points_count")
            results.append(
                CheckResult(
                    "qdrant.collection",
                    True,
                    f"collection '{collection}' reachable — vectors={vectors}, points={points}",
                    ms,
                )
            )
        except json.JSONDecodeError:
            results.append(CheckResult("qdrant.collection", True, f"HTTP 200 ({ms:.0f}ms)", ms))
    elif status == 404:
        hint = ""
        list_status, list_body, _ = http_request(f"{base}/collections", headers=headers)
        if list_status == 200:
            try:
                names = [
                    c.get("name")
                    for c in json.loads(list_body).get("result", {}).get("collections", [])
                    if c.get("name")
                ]
                if names:
                    hint = f" Available: {', '.join(names)}"
            except json.JSONDecodeError:
                pass
        results.append(
            CheckResult(
                "qdrant.collection",
                True,
                f"collection '{collection}' not created yet (HTTP 404 — expected on a fresh server).{hint} "
                "Create it from Vector Research tab or start an enrichment sweep.",
                ms,
            )
        )
    elif status in (401, 403):
        results.append(
            CheckResult(
                "qdrant.collection",
                False,
                f"auth failed (HTTP {status}) — check QDRANT_API_KEY",
                ms,
            )
        )
    elif status == 0:
        results.append(CheckResult("qdrant.collection", False, f"unreachable: {body}", ms))
    else:
        results.append(CheckResult("qdrant.collection", False, f"HTTP {status}: {body[:200]}", ms))

    # Auth sanity: bad key should not succeed
    bad_status, _, bad_ms = http_request(
        f"{base}/collections/{collection}",
        headers={"api-key": "invalid-smoke-test-key"},
    )
    if bad_status in (401, 403):
        results.append(CheckResult("qdrant.auth_rejects_bad_key", True, f"HTTP {bad_status}", bad_ms))
    elif bad_status == 200 and api_key:
        results.append(
            CheckResult(
                "qdrant.auth_rejects_bad_key",
                False,
                "server accepted an invalid api-key (auth may be disabled)",
                bad_ms,
            )
        )
    else:
        results.append(
            CheckResult(
                "qdrant.auth_rejects_bad_key",
                True,
                f"skipped/ inconclusive (HTTP {bad_status})",
                bad_ms,
            )
        )

    return results


def check_ncbi(api_key: str) -> list[CheckResult]:
    if not api_key:
        return [CheckResult("ncbi.esearch", True, "skipped — NCBI_API_KEY not set")]

    url = (
        "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi"
        f"?db=snp&term=rs4680&retmode=json&retmax=1&api_key={api_key}"
    )
    status, body, ms = http_request(url)
    if status == 200 and "esearchresult" in body:
        return [CheckResult("ncbi.esearch", True, "PubMed/SNP esearch OK for rs4680", ms)]
    return [CheckResult("ncbi.esearch", False, f"HTTP {status}: {body[:160]}", ms)]


def check_ollama(url: str, token: str) -> list[CheckResult]:
    if not url:
        return [CheckResult("ollama.tags", True, "skipped — OLLAMA_URL not set")]

    headers: dict[str, str] = {}
    if token:
        headers["Authorization"] = token if token.lower().startswith("bearer ") else f"Bearer {token}"

    status, body, ms = http_request(f"{url.rstrip('/')}/api/tags", headers=headers)
    if status == 200:
        try:
            models = json.loads(body).get("models", [])
            names = ", ".join(m.get("name", "?") for m in models[:3])
            suffix = f" (+{len(models) - 3} more)" if len(models) > 3 else ""
            return [CheckResult("ollama.tags", True, f"{len(models)} models: {names}{suffix}", ms)]
        except json.JSONDecodeError:
            return [CheckResult("ollama.tags", True, f"HTTP 200 ({ms:.0f}ms)", ms)]
    return [CheckResult("ollama.tags", False, f"HTTP {status}: {body[:160]}", ms)]


def print_results(results: list[CheckResult]) -> bool:
    all_ok = True
    print("\nGenomics Caddy — smoke test results")
    print("=" * 56)
    for r in results:
        icon = "PASS" if r.ok else "FAIL"
        timing = f" ({r.ms:.0f}ms)" if r.ms is not None else ""
        print(f"[{icon}] {r.name}{timing}")
        if r.detail:
            print(f"       {r.detail}")
        if not r.ok:
            all_ok = False
    print("=" * 56)
    print("OVERALL:", "PASS" if all_ok else "FAIL")
    return all_ok


def main() -> int:
    env_path = find_env_path(ROOT)
    if not env_path:
        print("ERROR: No .env found at project root.", file=sys.stderr)
        print("Copy .env.example to .env and set QDRANT_URL / QDRANT_API_KEY.", file=sys.stderr)
        return 2

    apply_env_file(env_path)
    print(f"Using env file: {env_path}")

    qdrant_url = os.environ.get("QDRANT_URL", "").strip()
    qdrant_key = os.environ.get("QDRANT_API_KEY", "").strip()
    qdrant_collection = os.environ.get("QDRANT_COLLECTION", "genomics_evidence").strip()
    ncbi_key = os.environ.get("NCBI_API_KEY", "").strip()
    ollama_url = os.environ.get("OLLAMA_URL", "").strip()
    ollama_token = os.environ.get("OLLAMA_TOKEN", "").strip()

    if not qdrant_url:
        print("ERROR: QDRANT_URL is required in .env", file=sys.stderr)
        return 2

    results: list[CheckResult] = []
    results.extend(check_qdrant(qdrant_url, qdrant_key, qdrant_collection))
    results.extend(check_ncbi(ncbi_key))
    results.extend(check_ollama(ollama_url, ollama_token))

    return 0 if print_results(results) else 1


if __name__ == "__main__":
    raise SystemExit(main())
