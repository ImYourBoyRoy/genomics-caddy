#!/usr/bin/env bash
# ./scripts/install_linux_desktop.sh
# Install FreeDesktop .desktop entry + hicolor icons so Genomics Caddy shows
# the DNA logo in the GNOME/Ubuntu dock (Wayland) instead of a generic gear.
#
# Tauri enableGTKAppId → Wayland app_id = com.dna.explorer.
# GNOME matches desktop *filename* == app_id, so the launcher MUST be
# named com.dna.explorer.desktop (not genomics-caddy.desktop).
#
# Usage (from repo root):
#   bash ./scripts/install_linux_desktop.sh
#   bash ./scripts/install_linux_desktop.sh --exe /path/to/DNA-Tools
#   npm run desktop:linux

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ICON_SRC="${ROOT}/src-tauri/icons"
APP_ID="com.dna.explorer"
ICON_NAME="DNA-Tools"
DESKTOP_NAME="${APP_ID}.desktop"

EXE=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --exe)
      EXE="${2:-}"
      shift 2
      ;;
    -h|--help)
      echo "Usage: bash ./scripts/install_linux_desktop.sh [--exe /path/to/DNA-Tools]"
      exit 0
      ;;
    *)
      echo "Unknown arg: $1" >&2
      exit 1
      ;;
  esac
done

pick_default_exe() {
  if [[ -x "${ROOT}/src-tauri/target/debug/DNA-Tools" ]]; then
    echo "${ROOT}/src-tauri/target/debug/DNA-Tools"
  elif [[ -x "${ROOT}/App/DNA-Tools" ]]; then
    echo "${ROOT}/App/DNA-Tools"
  elif [[ -x "${ROOT}/src-tauri/target/release/DNA-Tools" ]]; then
    echo "${ROOT}/src-tauri/target/release/DNA-Tools"
  else
    echo ""
  fi
}

if [[ -z "${EXE}" ]]; then
  EXE="$(pick_default_exe)"
  if [[ -z "${EXE}" ]]; then
    echo "ERROR: No DNA-Tools binary found. Build first (npm run tauri:dev / build:release-fast) or pass --exe." >&2
    exit 1
  fi
fi

EXE="$(readlink -f "${EXE}")"
ICON_DIR="${HOME}/.local/share/icons/hicolor"
APPS_DIR="${HOME}/.local/share/applications"
ICON_FILE_128="${ICON_DIR}/128x128/apps/${APP_ID}.png"
mkdir -p "${APPS_DIR}"

# Ensure hicolor theme index exists (required for reliable icon lookup).
if [[ ! -f "${ICON_DIR}/index.theme" ]]; then
  if [[ -f /usr/share/icons/hicolor/index.theme ]]; then
    mkdir -p "${ICON_DIR}"
    cp /usr/share/icons/hicolor/index.theme "${ICON_DIR}/index.theme"
  else
    mkdir -p "${ICON_DIR}"
    cat > "${ICON_DIR}/index.theme" <<'EOF'
[Icon Theme]
Name=Hicolor
Comment=Fallback icon theme
Directories=16x16/apps,32x32/apps,64x64/apps,128x128/apps,256x256/apps,512x512/apps
Hidden=true
Example=folder

[16x16/apps]
Size=16
Context=Applications
Type=Threshold

[32x32/apps]
Size=32
Context=Applications
Type=Threshold

[64x64/apps]
Size=64
Context=Applications
Type=Threshold

[128x128/apps]
Size=128
Context=Applications
Type=Threshold

[256x256/apps]
Size=256
Context=Applications
Type=Threshold

[512x512/apps]
Size=512
Context=Applications
Type=Threshold
EOF
  fi
fi

install_size() {
  local size="$1"
  local src="$2"
  local dest_dir="${ICON_DIR}/${size}/apps"
  mkdir -p "${dest_dir}"
  install -m 0644 "${src}" "${dest_dir}/${ICON_NAME}.png"
  install -m 0644 "${src}" "${dest_dir}/${APP_ID}.png"
}

install_size "32x32" "${ICON_SRC}/32x32.png"
install_size "64x64" "${ICON_SRC}/64x64.png"
install_size "128x128" "${ICON_SRC}/128x128.png"
install_size "256x256" "${ICON_SRC}/128x128@2x.png"
install_size "512x512" "${ICON_SRC}/icon.png"

rm -f "${APPS_DIR}/genomics-caddy.desktop" "${APPS_DIR}/genomics-caddy-dev.desktop"

# Absolute Icon= path is the most reliable on GNOME Wayland when theme caches lag.
cat > "${APPS_DIR}/${DESKTOP_NAME}" <<EOF
[Desktop Entry]
Type=Application
Version=1.0
Name=Genomics Caddy
Comment=Local genomics analysis desktop app
Exec=${EXE}
Icon=${ICON_FILE_128}
Terminal=false
Categories=Science;Education;
StartupWMClass=${APP_ID}
StartupNotify=true
DBusActivatable=false
EOF
chmod 0644 "${APPS_DIR}/${DESKTOP_NAME}"

# Stale WebKit module graph can keep requesting old Vite virtual CSS URLs.
rm -rf "${HOME}/.local/share/${APP_ID}/WebKitCache" "${HOME}/.local/share/${APP_ID}/CacheStorage"

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "${APPS_DIR}" >/dev/null 2>&1 || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -f -t "${ICON_DIR}" >/dev/null 2>&1 || true
fi
if command -v xdg-desktop-menu >/dev/null 2>&1; then
  xdg-desktop-menu forceupdate >/dev/null 2>&1 || true
fi

echo "Installed desktop entry: ${APPS_DIR}/${DESKTOP_NAME}"
echo "Executable: ${EXE}"
echo "app_id / StartupWMClass: ${APP_ID}"
echo "Icon (absolute): ${ICON_FILE_128}"
echo "Cleared WebKit cache under ~/.local/share/${APP_ID}/"
echo "Fully quit Genomics Caddy and relaunch if the dock still shows a generic icon."
