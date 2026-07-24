#!/usr/bin/env bash
# ./scripts/setup_linux_deps.sh
# One-time Linux/macOS prerequisite installer for Genomics Caddy (Tauri v2).
# Installs system libraries required to compile and link the desktop app.
#
# Usage (from repo root):
#   bash ./scripts/setup_linux_deps.sh
#   bash ./scripts/setup_linux_deps.sh --check-only

set -euo pipefail

CHECK_ONLY=0
for arg in "$@"; do
  case "$arg" in
    --check-only) CHECK_ONLY=1 ;;
    -h|--help)
      echo "Usage: bash ./scripts/setup_linux_deps.sh [--check-only]"
      exit 0
      ;;
  esac
done

step() { printf '\n==> %s\n' "$1"; }
ok() { printf '    %s\n' "$1"; }
fail() { printf '    ERROR: %s\n' "$1" >&2; exit 1; }

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "$1 not found on PATH"
}

check_pkg_config() {
  local pkg="$1"
  if pkg-config --exists "$pkg" 2>/dev/null; then
    ok "$pkg ($(pkg-config --modversion "$pkg"))"
    return 0
  fi
  fail "pkg-config package '$pkg' not found (install -dev packages)"
}

step "Genomics Caddy — Linux system dependency check"

require_cmd pkg-config

MISSING=0
for pkg in webkit2gtk-4.1 gtk+-3.0 libsoup-3.0 librsvg-2.0; do
  if pkg-config --exists "$pkg" 2>/dev/null; then
    ok "$pkg ($(pkg-config --modversion "$pkg"))"
  else
    printf '    MISSING: %s\n' "$pkg" >&2
    MISSING=1
  fi
done

if [[ "$MISSING" -eq 0 ]]; then
  ok "All Tauri GTK/WebKit dev packages are present."
  exit 0
fi

if [[ "$CHECK_ONLY" -eq 1 ]]; then
  fail "Dev packages missing. Run without --check-only to install (requires sudo)."
fi

if [[ "$(uname -s)" != "Linux" ]]; then
  fail "Automatic install is Linux-only. On macOS install Xcode CLT; on other distros see https://v2.tauri.app/start/prerequisites/"
fi

if ! command -v apt-get >/dev/null 2>&1; then
  fail "apt-get not found. Install Tauri prerequisites manually for your distribution."
fi

step "Installing Tauri v2 build dependencies (sudo required)"
sudo apt-get update -qq
sudo DEBIAN_FRONTEND=noninteractive apt-get install -y \
  build-essential \
  curl \
  wget \
  file \
  pkg-config \
  libdbus-1-dev \
  libssl-dev \
  libxdo-dev \
  libgtk-3-dev \
  libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev

step "Verifying pkg-config after install"
check_pkg_config webkit2gtk-4.1
check_pkg_config gtk+-3.0
check_pkg_config libsoup-3.0
check_pkg_config librsvg-2.0

ok "Linux build dependencies ready."
