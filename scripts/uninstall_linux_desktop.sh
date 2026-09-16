#!/usr/bin/env bash
# ./scripts/uninstall_linux_desktop.sh
# Remove FreeDesktop launcher + icons and clear WebKit asset caches so the
# portable DNA-Tools binary is not shadowed by a stale desktop registration.
#
# Usage (from repo root):
#   bash ./scripts/uninstall_linux_desktop.sh
#   pnpm run desktop:linux:uninstall

set -euo pipefail

APP_ID="com.dna.explorer"
ICON_NAME="DNA-Tools"
APPS_DIR="${HOME}/.local/share/applications"
ICON_DIR="${HOME}/.local/share/icons/hicolor"
SHARE_DIR="${HOME}/.local/share/${APP_ID}"

rm -f \
  "${APPS_DIR}/${APP_ID}.desktop" \
  "${APPS_DIR}/genomics-caddy.desktop" \
  "${APPS_DIR}/genomics-caddy-dev.desktop"

if [[ -d "${ICON_DIR}" ]]; then
  find "${ICON_DIR}" -type f \( -name "${APP_ID}.png" -o -name "${ICON_NAME}.png" \) -delete
fi

# Stale WebKit module graphs can keep serving an older splash/report UI even
# after the portable binary is replaced.
rm -rf "${SHARE_DIR}/WebKitCache" "${SHARE_DIR}/CacheStorage"

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "${APPS_DIR}" >/dev/null 2>&1 || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -f -t "${ICON_DIR}" >/dev/null 2>&1 || true
fi
if command -v xdg-desktop-menu >/dev/null 2>&1; then
  xdg-desktop-menu forceupdate >/dev/null 2>&1 || true
fi

echo "Removed desktop launcher for ${APP_ID}."
echo "Cleared WebKit caches under ${SHARE_DIR}."
echo "Launch the portable binary directly: ./builds/linux/DNA-Tools or ./App/DNA-Tools"
echo "Fully quit any open Genomics Caddy window before relaunching."
