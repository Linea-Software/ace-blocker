#!/usr/bin/env bash
# One-time migration after authorized shutdown of both protected services.
set -euo pipefail
if (( EUID != 0 )) || (( $# != 2 )) || [[ "$1" != "--from" ]]; then
  echo "Usage: sudo migrate-legacy-data.sh --from /path/to/AceBlocker" >&2
  exit 2
fi
source="$(realpath -e -- "$2")"
dest=/usr/local/share/AceBlocker
if [[ "${source##*/}" != AceBlocker || "$source" == "$dest" ]]; then
  echo "Invalid legacy data directory" >&2; exit 2
fi
require_stopped() {
  if systemctl is-active --quiet ace-engine.service ||
     systemctl is-active --quiet ace-watchdog.service; then
    echo "Protected services active; refuse migration" >&2; exit 1
  fi
}
require_stopped
if [[ ! -f "$source/engine_db.sql" || ! -f "$source/capability.token" ]] ||
   [[ -n "$(find "$source" -type l -print -quit)" ]]; then
  echo "Missing legacy data or unsafe symlink" >&2; exit 1
fi
# Use the original Engine *before upgrading binaries* to check both databases.
check_locks() {
  XDG_DATA_DIRS="${1%/AceBlocker}:/usr/share" /opt/ace-blocker/ace-engine --check-locks
}
check_locks "$source"
if [[ -f "$dest/engine_db.sql" ]]; then check_locks "$dest"; fi
getent group ace-blocker >/dev/null || groupadd --system ace-blocker
staging="$(mktemp -d -p /usr/local/share .AceBlocker-stage.XXXXXXXX)"
backup=""
cleanup() {
  status=$?
  if (( status != 0 )) && [[ -n "$backup" && ! -e "$dest" ]]; then
    mv -T -- "$backup" "$dest" || true
  fi
  [[ ! -d "$staging" ]] || rm -rf -- "$staging"
}
trap cleanup EXIT
cp -a -- "$source/." "$staging/"
chown root:ace-blocker "$staging"
chmod 2750 "$staging"
chgrp ace-blocker "$staging/capability.token"
chmod 0640 "$staging/capability.token"
chmod 0600 "$staging/engine_db.sql"
if [[ -f "$staging/certs/mitm_ca.key" ]]; then chmod 0600 "$staging/certs/mitm_ca.key"; fi
require_stopped
if [[ -e "$dest" ]]; then
  backup="$(mktemp -d -p /usr/local/share .AceBlocker-backup.XXXXXXXX)"
  rmdir -- "$backup"
  mv -T -- "$dest" "$backup"
fi
mv -T -- "$staging" "$dest"
staging=""
echo "Migrated: $dest. Source preserved: $source. Previous canonical backup: ${backup:-none}"
echo "Install new Engine binaries and units before restarting services."
