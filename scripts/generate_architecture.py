#!/usr/bin/env python3
# ./scripts/generate_architecture.py
"""
Purpose: Write ARCHITECTURE.md — size, line count, and one-line role for every
non-gitignored toolkit file (tracked plus untracked-but-not-ignored).
How to run: python3 ./scripts/generate_architecture.py
Inputs: git ls-files and git ls-files --others --exclude-standard
Outputs: overwrites ./ARCHITECTURE.md
Notes: MEMORY.md is local-only and is omitted even if present on disk.
"""

from __future__ import annotations

import json
import re
import subprocess
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "ARCHITECTURE.md"
SKIP = {"MEMORY.md", "MEMORY.local.md"}
BIN_EXT = {".png", ".webp", ".ico", ".icns", ".woff", ".woff2", ".wasm"}

EXPLICIT = {
    "README.md": "GitHub-facing product page: badges, per-OS install, first launch, screenshot tour.",
    "CITATION.cff": "Citation File Format 1.2 so GitHub shows Cite this repository.",
    "docs/images/github-social-preview.png": "1280×640 GitHub social-preview master (upload in repo Settings).",
    "AGENTS.md": "Repo-local agent rules: packs, privacy, and validation gates.",
    "ARCHITECTURE.md": "Non-gitignored file map with sizes, line counts, and one-line roles.",
    ".gitignore": "Ignore rules for DNA, reports, builds, env, and local memory.",
    ".env.example": "Documented env keys; copy to local `.env`, never commit secrets.",
    ".npmrc": "pnpm peer-dependency compatibility settings.",
    "package.json": "Frontend package manifest and npm/pnpm scripts.",
    "pnpm-workspace.yaml": "pnpm workspace definition.",
    "tsconfig.json": "TypeScript compiler options for the Svelte app.",
    "svelte.config.js": "SvelteKit adapter and preprocessor config.",
    "vite.config.js": "Vite bundler config for the Tauri webview.",
    "vitest.config.ts": "Vitest test runner config.",
    "src-tauri/Cargo.toml": "Rust crate manifest, edition, dependencies, and production custom-protocol feature.",
    "src-tauri/tauri.conf.json": "Tauri window, CSP, bundle, updater pubkey, and identifier config.",
    "src-tauri/build.rs": "Tauri build script hook.",
    "src-tauri/capabilities/default.json": "Tauri capability permissions including updater and relaunch.",
    "src-tauri/.gitignore": "Ignore Cargo target/ and generated Tauri schemas.",
    "src-tauri/desktop.template": "Linux .desktop template for packaged builds.",
    "src/app.html": "SvelteKit HTML shell: dark first paint and webview title.",
    "walkthrough.md": "Internal notes on config loading and sweep UI safeguards.",
    "paths.md": "Internal research-agent pathway map.",
    "plan.md": "Legacy enhancement checklist kept in the tree.",
    "docs/README.md": "Index of operator docs for humans and agents.",
    "docs/images/readme-report-simple.webp": "README animated Dark/Light Simple-mode shot with the data sidebar visible.",
    "docs/images/readme-chromosome-map.webp": "README animated Dark/Light chromosome map shot with the data sidebar visible.",
    "docs/usage/README.md": "How to import DNA, read reports, and share exports.",
    "docs/mcp/README.md": "MCP server flags, client configs, and tool catalog.",
    "docs/ai-chat/README.md": "Ollama, vector connections, chat, and evidence workbench.",
    "docs/build/README.md": "Release builds, Linux launcher, remote compile, Docker.",
    "docs/development/README.md": "Tests, audits, layout, and lock-free install.",
    "docker/README.md": "Headless sweep worker and optional static UI container.",
    "docker/Dockerfile": "Image for the headless sweep worker.",
    "docker/docker-compose.yml": "Compose service for the sweep worker.",
    "docker/nginx.conf": "Optional nginx config for a static UI container.",
    "scripts/generate_architecture.py": "Regenerate ARCHITECTURE.md from the non-gitignored tree.",
    "src-tauri/dev-bins/src/inspect_db.rs": "CLI helper: print resolved data-dir and genome DB paths.",
    "src-tauri/dev-bins/src/seed_mcp_fixture.rs": "CLI helper: seed a disposable MCP test fixture database.",
    ".cursor/skills/connections-vector-providers/reference.md": "Vector-store adapter dispatch notes for Connections providers.",
    ".github/workflows/release.yml": "Tag publish: signed installers and latest.json for in-app updates.",
    ".github/workflows/ci.yml": "PR and master Validate workflow, including updater-config audit.",
    "scripts/audit_updater_config.mjs": "Static audit of signed updater pubkey, endpoints, and release wiring.",
    "scripts/package_windows_portable.mjs": "Build a USB-clean Windows zip (exe + Data/.keep) for GitHub.",
    "scripts/print_agent_ui_endpoint.mjs": "Print the live 127.0.0.1 agent-UI URL from the runtime endpoint file.",
    "scripts/lib/agentUiEndpoint.mjs": "Discover and canonicalize loopback agent-UI / Vite endpoints.",
    "scripts/sign_updater_artifact.sh": "Minisign one updater artifact without printing the private key.",
    "src-tauri/windows/hooks.nsh": "NSIS uninstall: follow uninstall-library.txt, including custom folders.",
    "src/lib/styles/components/global-dialogs.css": "Alert, confirm, and choice overlay using theme tokens.",
    "src/lib/styles/components/library-panel.css": "Sidebar library-folder control using theme tokens.",
    "src/lib/components/sidebar/LibraryPanel.svelte": "Open, change, reset, and erase the resolved library folder.",
    "src/lib/utils/updater.ts": "Quiet GitHub update check, dismissal, and confirm-to-install helpers.",
    "src/lib/components/common/UpdateBanner.svelte": "Dismissible signed-update notification strip.",
    "src/lib/components/common/AppUpdateHost.svelte": "Launch-time updater host: banner, confirm, download, relaunch.",
    "src/lib/styles/components/update-banner.css": "Update banner and app-shell stack layout using theme tokens.",
    ".vscode/extensions.json": "Recommended VS Code / Cursor extensions.",
    ".vscode/settings.json": "Shared editor settings for this workspace.",
    "src-tauri/linux/post_install.sh": "After Linux package install: canonical FreeDesktop launcher.",
    "src-tauri/linux/post_remove.sh": "After Linux package removal: drop the canonical launcher.",
}

PACK_SUPPORT = {
    "manifest.json": "Pack catalog: ids, titles, and which JSON files load at runtime.",
    "discovery_catalog.json": "Curated discovery targets for catalog scans and research UI.",
    "research_taxonomy.json": "Shared discovery labels, keyword routing, pack links, consultation hints.",
    "research_found.json": "Protected pack for research-found drafts only.",
    "evidence_policy.json": "Shared evidence-tier, severity, and claim-frame wording.",
    "layperson_translations.json": "Plain-English marker copy for Simple mode.",
    "actionability_guidance.json": "Food, supplement, lab, and medication prompt rules.",
    "source_registry.json": "Registered citation sources for packs and support resources.",
    "ai_prompt_policy.json": "Default AI instructions and forbidden-action policy.",
    "consultation_modes.json": "Specialty-mode labels and routing for Connected Chat.",
    "callability_rules.json": "When a consumer-array row can or cannot score an assertion.",
}

RUST_KNOWN = {
    "main.rs": "Binary entry: GUI, MCP, or headless sweep.",
    "lib.rs": "Tauri command registration and shared backend surface.",
    "parser.rs": "Raw DNA file parser (txt/csv/tsv/zip).",
    "liftover.rs": "GRCh37/GRCh38 coordinate mapping via UCSC chain.",
    "db.rs": "SQLite schema, profile DBs, and queries.",
    "mcp.rs": "stdin/stdout MCP JSON-RPC server.",
    "report.rs": "Marker evaluation and report assembly.",
    "config.rs": "App data roots and env loading.",
    "agent_ui.rs": "Loopback HTTP bridge to drive and inspect the live UI.",
    "agent_ui_capture.rs": "Linux WebKit viewport PNG capture for agent QA and README shots.",
}


def git_lines(*args: str) -> list[str]:
    return subprocess.check_output(["git", *args], cwd=ROOT, text=True).splitlines()


def list_files() -> list[str]:
    tracked = git_lines("ls-files")
    extra = git_lines("ls-files", "--others", "--exclude-standard")
    files = []
    seen = set()
    for rel in tracked + extra:
        if rel in SKIP or rel in seen:
            continue
        seen.add(rel)
        files.append(rel)
    if "ARCHITECTURE.md" not in seen:
        files.append("ARCHITECTURE.md")
    return sorted(files)


def first_sentence(text: str, limit: int = 180) -> str:
    text = re.sub(r"\s+", " ", text).strip(" -*")
    text = re.sub(r"(?i)^purpose:\s*", "", text)
    if not text:
        return ""
    cut = re.split(r"(?<=[.!?])\s+", text, maxsplit=1)[0]
    if len(cut) > limit:
        cut = cut[: limit - 1].rsplit(" ", 1)[0] + "…"
    if cut and cut[-1] not in ".!?":
        cut += "."
    return cut


HEADER_LINES = 80
FILE_HEADER_LINES = 12


def header_text(text: str, n: int = HEADER_LINES) -> str:
    return "\n".join(text.splitlines()[:n])


def extract_purpose_field(head: str) -> str:
    lines = head.splitlines()
    for i, ln in enumerate(lines):
        m = re.match(r"(?i)\s*(?:\*|//|#)?\s*Purpose:\s*(.+)$", ln)
        if not m:
            continue
        parts = [m.group(1).strip()]
        j = i + 1
        while j < len(lines) and parts[-1] and parts[-1][-1] not in ".!?":
            nxt = re.sub(r"^\s*(?:\*|//|#)?\s*", "", lines[j]).strip()
            nxt = nxt.rstrip("*/").strip()
            if not nxt or nxt.lower().startswith(
                ("usage", "how to", "inputs", "outputs", "privacy", "responsibilities", "key inputs", "key outputs")
            ):
                break
            parts.append(nxt)
            j += 1
        return first_sentence(" ".join(parts))
    return ""


def extract_hash_or_slash_comments(text: str) -> str:
    comments: list[str] = []
    for ln in text.splitlines():
        stripped = ln.strip()
        if stripped.startswith("#!"):
            continue
        if re.match(r"^#\s*\./", stripped) or re.match(r"^//\s*\./", stripped):
            continue
        if stripped.startswith("#") or stripped.startswith("//"):
            body = stripped.lstrip("#/").strip()
            if body.lower().startswith("usage"):
                break
            if body:
                comments.append(body)
            continue
        if stripped == "":
            if comments:
                break
            continue
        break
    return first_sentence(" ".join(comments[:4])) if comments else ""


def extract_block_comment(head: str) -> str:
    m = re.search(r"(?s)/\*\*?(.*?)\*/", head)
    if not m:
        return ""
    lines = []
    for raw in m.group(1).splitlines():
        line = re.sub(r"^\s*\*\s?", "", raw).strip()
        if not line or line.startswith("@") or line.startswith("./") or line.startswith("# ./"):
            continue
        if line.lower().startswith(("usage:", "inputs:", "outputs:", "privacy:", "how to run:")):
            break
        lines.append(re.sub(r"(?i)^purpose:\s*", "", line))
    return first_sentence(" ".join(lines[:4])) if lines else ""


def extract_purpose(path: Path, text: str) -> str:
    head = header_text(text)
    file_head = header_text(text, FILE_HEADER_LINES)

    purpose_field = extract_purpose_field(header_text(text, 40))
    if purpose_field:
        return purpose_field

    m = re.search(r"(?ms)^---\n(.*?)\n---", text)
    if m:
        block = m.group(1)
        dm = re.search(r"(?ms)^description:\s*>-\s*(.+?)(?=\n[a-zA-Z_]+:|\Z)", block)
        if not dm:
            dm = re.search(r"(?m)^description:\s*(.+)$", block)
        if dm:
            return first_sentence(dm.group(1))

    block = extract_block_comment(file_head)
    if block:
        return block

    m = re.search(r'(?s)"""(.*?)"""', head)
    if m:
        body_lines = [
            ln.strip()
            for ln in m.group(1).splitlines()
            if ln.strip() and not ln.strip().startswith("./") and not ln.strip().startswith("# ./")
        ]
        if body_lines:
            return first_sentence(body_lines[0])

    m = re.search(
        r"(?is)<#\s*\.SYNOPSIS\s*(.*?)\s*(?:\.DESCRIPTION\s*(.*?))?\s*(?:\.EXAMPLE|#>)",
        head,
    )
    if m:
        return first_sentence((m.group(2) or m.group(1) or "").strip())

    suffix = path.suffix.lower()
    if suffix in {".sh", ".bash", ".ps1"} or text.startswith("#!/") or suffix in {".mjs", ".cjs", ".js", ".py"}:
        commented = extract_hash_or_slash_comments(file_head if suffix != ".py" else head)
        if commented:
            return commented
    return ""


def pack_role(rel: str, name: str, text: str) -> str:
    if name in PACK_SUPPORT:
        role = PACK_SUPPORT[name]
        if rel.startswith("src-tauri/App/Data/marker-packs/"):
            return f"Runtime mirror: {role[0].lower() + role[1:]}"
        return role
    if name.endswith(".json"):
        label = ""
        purpose = ""
        try:
            data = json.loads(text)
        except json.JSONDecodeError:
            data = None
        if isinstance(data, dict):
            label = str(data.get("name") or data.get("label") or "").strip()
            purpose = str(data.get("purpose") or "").strip()
        if not label:
            nm = re.search(r'"name"\s*:\s*"([^"]+)"', text[:2000])
            if nm:
                label = nm.group(1).strip()
        if not purpose:
            pm = re.search(r'"purpose"\s*:\s*"([^"]+)"', text[:4000])
            if pm:
                purpose = pm.group(1).strip()
        prefix = "Runtime mirror of curated pack" if rel.startswith("src-tauri/") else "Curated marker pack"
        if purpose and (data.get("type") == "support_engine" if isinstance(data, dict) else "guidance" in name):
            kind = "Runtime mirror of support resource" if rel.startswith("src-tauri/") else "Support resource"
            return f"{kind} `{Path(name).stem}`: {first_sentence(purpose)}"
        if label:
            return f"{prefix} `{Path(name).stem}`: {first_sentence(label)}"
        prefix = "Runtime mirror of" if rel.startswith("src-tauri/") else "Curated source resource"
        return f"{prefix} `{name}`."
    return ""


def fallback(rel: str) -> str:
    p = rel.replace("\\", "/")
    name = Path(p).name
    stem = Path(p).stem
    ext = Path(p).suffix.lower()

    if "/icons/android/" in p:
        dens = Path(p).parent.name
        if "_round." in name or name.endswith("_round.png"):
            kind = "round launcher"
        elif "foreground" in name:
            kind = "foreground"
        else:
            kind = "launcher"
        return f"Android {dens} {kind} icon."
    if "/icons/ios/" in p:
        return f"iOS app-icon raster ({name})."
    if p.startswith("src-tauri/icons/") and ext in BIN_EXT:
        return f"Desktop/store icon asset ({name})."
    if p.startswith("static/"):
        if name == "logo.png":
            return "App logo used for Tauri icon generation and Linux launcher."
        if name == "favicon.png":
            return "Webview favicon."
        return f"Static {ext[1:]} asset bundled with the webview."
    if p.startswith("reference_files/"):
        return "Source brand-logo master."
    if p.startswith("src-tauri/testdata/"):
        return "Synthetic parser fixture; no user DNA."
    if p.endswith(".styles.test.ts"):
        return f"CSS/layout tests for `{stem.replace('.styles.test', '')}`."
    if p.endswith(".test.ts") or p.endswith(".spec.ts"):
        target = stem.replace(".test", "").replace(".spec", "")
        return f"Tests for `{target}`."
    if p.startswith("src/lib/styles/"):
        return f"External stylesheet `{name}`."
    if p.startswith("src/lib/types/"):
        return f"Shared TypeScript types `{stem}`."
    if p.startswith("src/lib/api/"):
        return f"Tauri IPC client `{stem}`."
    if p.startswith("src/lib/utils/"):
        return f"Frontend logic `{stem}`."
    if p.startswith("src/routes/"):
        return "SvelteKit route for the single-page desktop shell."
    if "/components/" in p and ext == ".svelte":
        return f"Svelte UI component `{stem}`."
    if ext == ".svelte":
        return f"Svelte view `{name}`."
    if p.startswith("src-tauri/src/research/"):
        return f"Rust research/enrichment `{stem}`."
    if p.startswith("src-tauri/src/offline/"):
        return f"Rust offline catalog/update `{stem}`."
    if p.startswith("src-tauri/src/") and ext == ".rs":
        return RUST_KNOWN.get(name, f"Rust backend `{stem}`.")
    if p.startswith("scripts/remote_build/"):
        return f"Remote/cross-compile helper `{stem}`."
    if p.startswith("scripts/"):
        return f"Operator script `{name}`."
    if p.startswith(".github/"):
        return "GitHub Actions workflow."
    if p.startswith(".cursor/"):
        return "Project Cursor skill for a repeatable local workflow."
    if p.startswith(".vscode/"):
        return "Shared editor setting or extension recommendation."
    if p.startswith("docker/"):
        return f"Docker packaging file `{name}`."
    if ext == ".css":
        return f"CSS `{name}`."
    if ext == ".json":
        return f"JSON `{name}`."
    if ext == ".rs":
        return f"Rust `{name}`."
    if ext == ".ts":
        return f"TypeScript `{name}`."
    return f"Tracked file `{name}`."


def describe(rel: str, text: str) -> str:
    if rel in EXPLICIT:
        return EXPLICIT[rel]
    name = Path(rel).name
    if "marker-packs/" in rel.replace("\\", "/") and name.endswith(".json"):
        role = pack_role(rel, name, text)
        if role:
            return role
    extracted = extract_purpose(ROOT / rel, text) if text else ""
    if extracted and len(extracted) > 18:
        return extracted
    return fallback(rel)


def fmt_size(n: int) -> str:
    if n < 1024:
        return f"{n} B"
    if n < 1024 * 1024:
        return f"{n / 1024:.1f} KB"
    return f"{n / (1024 * 1024):.1f} MB"


def anchor(parent: str) -> str:
    slug = parent.lower()
    for ch in "/() ":
        slug = slug.replace(ch, "-")
    return slug.strip("-")


def collect(files: list[str]) -> list[tuple[str, int, int | None, str]]:
    rows = []
    for rel in files:
        path = ROOT / rel
        if not path.exists():
            rows.append((rel, 0, 0, describe(rel, "")))
            continue
        size = path.stat().st_size
        raw = path.read_bytes()
        ext = Path(rel).suffix.lower()
        is_bin = ext in BIN_EXT or (b"\x00" in raw[:128])
        if is_bin:
            rows.append((rel, size, None, describe(rel, "")))
            continue
        text = raw.decode("utf-8", errors="replace")
        lines = text.count("\n") + (0 if text.endswith("\n") or text == "" else 1)
        if text == "":
            lines = 0
        rows.append((rel, size, lines, describe(rel, text)))
    return rows


def render(rows: list[tuple[str, int, int | None, str]]) -> str:
    groups: dict[str, list[tuple[str, int, int | None, str]]] = defaultdict(list)
    for row in rows:
        parent = str(Path(row[0]).parent)
        if parent == ".":
            parent = "(repository root)"
        groups[parent].append(row)

    out = [
        "# Architecture",
        "",
        "Inventory of **non-gitignored** toolkit files (git-tracked plus local",
        "untracked files that are not ignored). Local-only paths are omitted:",
        "`MEMORY.md`, raw DNA, reports, `.env`, `App/`, `builds/`, and other gitignored data.",
        "",
        f"- Files: **{len(rows)}**",
        f"- Total size: **{fmt_size(sum(r[1] for r in rows))}** ({sum(r[1] for r in rows):,} bytes)",
        f"- Text lines (non-binary): **{sum(r[2] or 0 for r in rows):,}**",
        f"- Directories: **{len(groups)}**",
        "",
        "Sizes are on-disk bytes. Line counts are newline-based. Binary icons show `—`.",
        "Use `docs/` for how to run the app. Regenerate with `python3 ./scripts/generate_architecture.py`.",
        "",
        "## Directories",
        "",
    ]
    order = sorted(groups, key=lambda s: (s != "(repository root)", s.lower()))
    for parent in order:
        n = len(groups[parent])
        bytes_ = sum(r[1] for r in groups[parent])
        out.append(f"- [`{parent}`](#{anchor(parent)}) — {n} files, {fmt_size(bytes_)}")
    out.append("")

    for parent in order:
        out += [f"## `{parent}`", "", "| File | Size | Lines | Role |", "| --- | ---: | ---: | --- |"]
        for rel, size, lines, desc in sorted(groups[parent], key=lambda r: r[0].lower()):
            fn = Path(rel).name
            ln = "—" if lines is None else f"{lines:,}"
            desc = desc.replace("|", "\\|")
            out.append(f"| `{fn}` | {fmt_size(size)} | {ln} | {desc} |")
        out.append("")
    return "\n".join(out).rstrip() + "\n"


def main() -> None:
    files = list_files()
    # Two passes so ARCHITECTURE.md's own size/line row matches the written file.
    for _ in range(2):
        rows = collect(files)
        OUT.write_text(render(rows), encoding="utf-8")
    print(f"Wrote {OUT.relative_to(ROOT)} ({len(files)} files)")


if __name__ == "__main__":
    main()
