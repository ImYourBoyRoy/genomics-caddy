#!/usr/bin/env bash
# ./scripts/system_check.sh
# Quick environment diagnostic for Genomics Caddy builds (Linux/macOS).

set -u

step() { printf '\n==> %s\n' "$1"; }
ok() { printf '    OK  %s\n' "$1"; }
warn() { printf '    WARN %s\n' "$1"; }
bad() { printf '    FAIL %s\n' "$1"; FAIL=1; }

FAIL=0
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

step "Host"
ok "$(uname -s) $(uname -m) — $(uname -r)"
df -h "$REPO_ROOT" | tail -1 | awk '{printf "    Disk: %s free on %s\n", $4, $1}'
free -h | awk '/^Mem:/ {printf "    RAM: %s total, %s available\n", $2, $7}'

step "Toolchain"
for tool in git node npm python3 cargo rustc pkg-config; do
  if command -v "$tool" >/dev/null 2>&1; then
    ver=$("$tool" --version 2>/dev/null | head -1)
    ok "$tool — $ver"
  else
    bad "$tool — not installed"
  fi
done

step "Node engine (package.json requires >=26)"
if command -v node >/dev/null 2>&1; then
  major=$(node -p "process.versions.node.split('.')[0]")
  if [[ "$major" -ge 26 ]]; then ok "Node major $major"; else bad "Node $major < 26"; fi
fi

step "Rust MSRV (Cargo.toml rust-version)"
if command -v rustc >/dev/null 2>&1 && [[ -f "$REPO_ROOT/src-tauri/Cargo.toml" ]]; then
  msrv=$(grep -E '^rust-version' "$REPO_ROOT/src-tauri/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')
  cur=$(rustc --version | awk '{print $2}')
  ok "rustc $cur (MSRV $msrv)"
fi

step "Tauri Linux libraries (pkg-config)"
if [[ "$(uname -s)" == "Linux" ]]; then
  for pkg in webkit2gtk-4.1 gtk+-3.0 libsoup-3.0 librsvg-2.0; do
    if pkg-config --exists "$pkg" 2>/dev/null; then
      ok "$pkg $(pkg-config --modversion "$pkg")"
    else
      bad "$pkg — install via: bash ./scripts/setup_linux_deps.sh"
    fi
  done
else
  warn "Skipped GTK/WebKit check (not Linux)"
fi

step "Project dependencies"
if [[ -d "$REPO_ROOT/node_modules" ]]; then ok "node_modules present"; else warn "run: npm install"; fi

step "Summary"
if [[ "$FAIL" -eq 0 ]]; then
  ok "System ready for Genomics Caddy build."
  exit 0
fi
bad "Fix failures above before running: npm run build:release"
exit 1
