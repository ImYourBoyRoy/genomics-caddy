#!/bin/sh
set -eu

# Tauri names the generated entry from productName. The running GTK app uses
# identifier/app_id, so keep an identifier-named entry as the canonical
# launcher for GNOME/Wayland matching. The package-owned product-name entry is
# left in place for package-manager ownership and compatibility.
applications_dir="/usr/share/applications"
generated_entry="${applications_dir}/Genomics Caddy.desktop"
canonical_entry="${applications_dir}/com.dna.explorer.desktop"

if [ -f "${generated_entry}" ]; then
  cp "${generated_entry}" "${canonical_entry}"
  chmod 0644 "${canonical_entry}"
fi

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "${applications_dir}" >/dev/null 2>&1 || true
fi
