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

# Limit shared data and the installation capability to authorized desktop users.
if ! getent group ace-blocker >/dev/null; then
  groupadd --system ace-blocker
fi
data_dir=/usr/local/share/AceBlocker
install -d -o root -g ace-blocker -m 2750 "$data_dir"
# Reinstalling after a legitimate uninstall may retain an existing root:root
# token. Preserve its contents and grant read access only to the approved group.
token_file="$data_dir/capability.token"
if [[ -e "$token_file" ]]; then
  if [[ ! -f "$token_file" || -L "$token_file" ]]; then
    echo "Unexpected capability token file type; refusing installation." >&2
    exit 1
  fi
  chgrp ace-blocker "$token_file"
  chmod 0640 "$token_file"
fi
# Database and private CA key belong to the Engine, not the desktop UI.
for private_file in "$data_dir/engine_db.sql" "$data_dir/certs/mitm_ca.key"; do
  if [[ -e "$private_file" ]]; then
    if [[ ! -f "$private_file" || -L "$private_file" ]]; then
      echo "Unexpected protected data file type: $private_file" >&2
      exit 1
    fi
    chmod 0600 "$private_file"
  fi
done
if [[ -n "${SUDO_USER:-}" && "$SUDO_USER" != root ]]; then
  usermod -a -G ace-blocker "$SUDO_USER"
  echo "Log out and back in to refresh ACE group membership."
else
  echo "Add your desktop user to the 'ace-blocker' group, then log in again." >&2
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
