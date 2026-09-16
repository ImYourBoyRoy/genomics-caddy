#!/usr/bin/env bash
# ./scripts/refresh_linux_portable_ui.sh
#
# Purpose: Refresh the FreeDesktop launcher/icon to match the current portable
# binary, then clear WebKit UI caches. Keeps the branded dock entry installed.
# Does not delete App/Data profiles or genomes.
#
# Usage (from repo root):
#   bash ./scripts/refresh_linux_portable_ui.sh
#   bash ./scripts/refresh_linux_portable_ui.sh --status
#   bash ./scripts/refresh_linux_portable_ui.sh --verify
#   pnpm run desktop:linux:refresh
#
# Exit codes: 0 success, 1 usage/verify failure.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_ID="com.dna.explorer"
SHARE_DIR="${HOME}/.local/share/${APP_ID}"
APPS_DIR="${HOME}/.local/share/applications"
STATUS_ONLY=0
VERIFY=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --status|-s)
      STATUS_ONLY=1
      shift
      ;;
    --verify|-v)
      VERIFY=1
      shift
      ;;
    -h|--help)
      sed -n '2,16p' "$0"
      exit 0
      ;;
    *)
      echo "Unknown arg: $1" >&2
      echo "Usage: bash ./scripts/refresh_linux_portable_ui.sh [--status] [--verify]" >&2
      exit 1
      ;;
  esac
done

print_status() {
  echo "==> Portable binaries"
  for path in "${ROOT}/App/DNA-Tools" "${ROOT}/builds/linux/DNA-Tools"; do
    if [[ -x "${path}" ]]; then
      local sha
      sha="$(sha256sum "${path}" | awk '{print $1}')"
      echo "  OK  ${path}"
      echo "      sha256 ${sha}"
      echo "      mtime $(stat -c '%y' "${path}" 2>/dev/null || true)"
    else
      echo "  MISSING  ${path}"
    fi
  done

  echo "==> Desktop launcher"
  local launcher="${APPS_DIR}/${APP_ID}.desktop"
  if [[ -f "${launcher}" ]]; then
    echo "  PRESENT  ${launcher}"
    grep -E '^(Exec|Icon|Name)=' "${launcher}" 2>/dev/null | sed 's/^/    /' || true
  else
    echo "  absent   ${launcher}"
  fi

  echo "==> WebKit / UI caches under ${SHARE_DIR}"
  for name in WebKitCache CacheStorage GPUCache; do
    local target="${SHARE_DIR}/${name}"
    if [[ -e "${target}" ]]; then
      echo "  PRESENT  ${target}"
    else
      echo "  absent   ${target}"
    fi
  done

  if command -v pgrep >/dev/null 2>&1; then
    # Exact portable/release binary paths only (path mentions in git/diff args are ignored).
    local running=""
    local candidate
    for candidate in \
      "${ROOT}/App/DNA-Tools" \
      "${ROOT}/builds/linux/DNA-Tools" \
      "${ROOT}/src-tauri/target/release/DNA-Tools" \
      "${ROOT}/src-tauri/target/debug/DNA-Tools"
    do
      running+="$(pgrep -af -- "${candidate}" 2>/dev/null || true)\n"
    done
    running="$(printf '%b' "${running}" | awk 'NF && $0 !~ /refresh_linux_portable_ui/ && $0 !~ /pgrep/ {print}' | sort -u)"
    if [[ -n "${running}" ]]; then
      echo "==> Running processes (quit these before relaunch)"
      printf '%s\n' "${running}" | sed 's/^/  /'
    else
      echo "==> Running processes: none matched"
    fi
  fi
}

verify_frontend_markers() {
  local build_dir="${ROOT}/build"
  if [[ ! -d "${build_dir}" ]]; then
    echo "VERIFY FAIL: ${build_dir} missing — run a frontend/release build first." >&2
    return 1
  fi

  local markers=(
    "bootstrap-content"
    "bootstrap-helix-slot"
    "lab-spotlight"
    "report-marker-ref-text"
  )
  local missing=0
  echo "==> Verify splash/report markers in build/"
  for marker in "${markers[@]}"; do
    if rg -q --fixed-strings "${marker}" "${build_dir}" 2>/dev/null; then
      echo "  OK  ${marker}"
    else
      echo "  MISSING  ${marker}"
      missing=1
    fi
  done
  return "${missing}"
}

print_status

if [[ "${STATUS_ONLY}" -eq 1 ]]; then
  exit 0
fi

echo "==> Refresh branded desktop launcher/icon"
if [[ -x "${ROOT}/builds/linux/DNA-Tools" || -x "${ROOT}/App/DNA-Tools" || \
      -x "${ROOT}/src-tauri/target/release/DNA-Tools" || \
      -x "${ROOT}/src-tauri/target/debug/DNA-Tools" ]]; then
  bash "${ROOT}/scripts/install_linux_desktop.sh"
else
  echo "WARN: no portable binary found; leaving any existing launcher/icon unchanged."
fi

echo "==> Clear WebKit / UI caches (launcher and branded icons are preserved)"
rm -rf \
  "${SHARE_DIR}/WebKitCache" \
  "${SHARE_DIR}/CacheStorage" \
  "${SHARE_DIR}/GPUCache" \
  "${HOME}/.cache/${APP_ID}" 2>/dev/null || true

echo "==> Post-refresh status"
print_status

if [[ "${VERIFY}" -eq 1 ]]; then
  verify_frontend_markers
fi

echo
echo "Fully quit Genomics Caddy, then relaunch it from Applications/the dock (Genomics Caddy)."
echo "Direct binary launch is also available:"
echo "  ${ROOT}/App/DNA-Tools"
echo "  or ${ROOT}/builds/linux/DNA-Tools"
echo "The desktop launcher now points to the current portable binary and uses the DNA logo."
