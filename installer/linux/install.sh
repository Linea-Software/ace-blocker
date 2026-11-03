#!/usr/bin/env bash
# Fresh-install helper for disposable Linux/systemd test machines.
# Upgrades and removal require authenticated engine maintenance.
set -euo pipefail

if (( EUID != 0 )); then
  echo "Run this installer as root (for example: sudo ./install.sh)." >&2
  exit 1
fi

package_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
install_dir=/opt/ace-blocker

if [[ -e "$install_dir/ace-engine" ]] ||
   systemctl is-active --quiet ace-engine.service ||
   systemctl is-active --quiet ace-watchdog.service; then
  echo "ACE is already installed or running; refusing to replace protected binaries." >&2
  echo "Use authenticated lock-aware maintenance/uninstall first." >&2
  exit 1
fi

for binary in ace-engine ace-watchdog ace-app ace-redirect install-cert; do
  if [[ ! -f "$package_dir/$binary" ]]; then
    echo "Missing release binary: $binary" >&2
    exit 1
  fi
done
for asset in ace-blocker.desktop ace-blocker.png; do
  if [[ ! -f "$package_dir/$asset" ]]; then
    echo "Missing release asset: $asset" >&2
    exit 1
  fi
done
if [[ ! -d "$package_dir/bundled-packs" ]]; then
  echo "Missing bundled offline site packs" >&2
  exit 1
fi

install -d -m 0755 "$install_dir"
for binary in ace-engine ace-watchdog ace-app ace-redirect install-cert; do
  install -m 0755 "$package_dir/$binary" "$install_dir/$binary"
done
cp -R "$package_dir/bundled-packs" "$install_dir/"
install -D -m 0644 "$package_dir/ace-blocker.desktop" /usr/local/share/applications/ace-blocker.desktop
install -D -m 0644 "$package_dir/ace-blocker.png" /usr/local/share/icons/hicolor/128x128/apps/ace-blocker.png

"$install_dir/ace-engine" --install
systemctl daemon-reload
systemctl is-enabled --quiet ace-watchdog.service
systemctl is-enabled --quiet ace-engine.service
systemctl start ace-watchdog.service ace-engine.service
systemctl is-active --quiet ace-watchdog.service
systemctl is-active --quiet ace-engine.service
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database /usr/local/share/applications
fi

echo "ACE installed: services enabled and started; ACE Blocker launcher registered."
