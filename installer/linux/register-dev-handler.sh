#!/usr/bin/env bash
# Register the debug Ace authentication callback in the *desktop user's* MIME
# database. This is intentionally separate from the privileged Linux installer.
set -euo pipefail

if (( EUID == 0 )); then
    echo "Run this as your desktop user, not with sudo." >&2
    exit 1
fi

for command in xdg-mime desktop-file-validate; do
    if ! command -v "$command" >/dev/null 2>&1; then
        echo "Required command not found: $command" >&2
        exit 1
    fi
done

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
redirect="$repo_root/target/debug/ace-redirect"
if [[ ! -x "$redirect" ]]; then
    echo "Missing debug callback receiver: $redirect" >&2
    echo "Build it first: cargo build --locked -p ace-engine --bin ace-redirect" >&2
    exit 1
fi

# Never override a working production callback association with a development
# binary, even when the helper is accidentally run on an installed machine.
current="$(xdg-mime query default x-scheme-handler/aceblocker || true)"
if [[ "$current" == "ace-blocker-handler.desktop" &&
      -x /opt/ace-blocker/ace-redirect ]] &&
   { [[ -r /usr/local/share/applications/ace-blocker-handler.desktop ]] ||
     [[ -r /usr/share/applications/ace-blocker-handler.desktop ]]; }; then
    echo "Production Ace callback handler is already installed. Leaving it unchanged."
    exit 0
fi

# Niri and other nonstandard desktop sessions take xdg-open's "generic"
# branch. Its parser cannot resolve quoted .desktop Exec paths, even though
# GIO accepts them. Use an unquoted absolute path with only safe characters
# so both xdg-open and GIO can launch the callback. Fail with an explanation
# rather than generating an invalid or vulnerable shell command.
if [[ ! "$redirect" =~ ^[a-zA-Z0-9_./+-]+$ ]]; then
    echo "The development path contains characters incompatible with generic xdg-open:" >&2
    echo "  $redirect" >&2
    echo "Move the checkout to a path without spaces or shell metacharacters." >&2
    exit 1
fi

application_dir="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
desktop_name="ace-blocker-dev-handler.desktop"
desktop_path="$application_dir/$desktop_name"
mkdir -p -- "$application_dir"

tmp_file="$(mktemp "$application_dir/.ace-blocker-dev-handler.XXXXXX.desktop")"
trap 'rm -f -- "$tmp_file"' EXIT
cat > "$tmp_file" <<EOF
[Desktop Entry]
Type=Application
Version=1.0
Name=Ace Blocker (development login callback)
Comment=Return sign-in from the browser to the running Ace Engine
Exec=$redirect %u
Terminal=false
NoDisplay=true
MimeType=x-scheme-handler/aceblocker;
Categories=Development;
EOF

desktop-file-validate "$tmp_file"
chmod 0644 "$tmp_file"
mv -f -- "$tmp_file" "$desktop_path"

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$application_dir"
fi
xdg-mime default "$desktop_name" x-scheme-handler/aceblocker

registered="$(xdg-mime query default x-scheme-handler/aceblocker)"
if [[ "$registered" != "$desktop_name" ]]; then
    echo "Registration failed: xdg-mime selected '${registered:-none}' instead of $desktop_name." >&2
    exit 1
fi

echo "Registered $desktop_name for aceblocker:// in the user session."
echo "Callback binary: $redirect"
echo "The Engine's existing account/lock authorization is unchanged."

# The callback may be launched successfully but still fail to authenticate if
# its *user* lacks the installation capability. Diagnose without showing the
# secret, altering privileged filesystem permissions, or replacing the Engine.
capability_path="/usr/local/share/AceBlocker/capability.token"
if [[ -e "$capability_path" && ! -r "$capability_path" ]]; then
    echo "WARNING: This user cannot read $capability_path." >&2
    echo "Browser callback dispatch is fixed, but account activation also needs" >&2
    echo "the installed 'ace-blocker' group/permission setup and a fresh login." >&2
fi
