"""Safe distro package setup and guest read-only readiness probes.

Install scripts act only inside an explicitly selected disposable guest. Paid and
third-party desktop clients require manual preparation, never unattended login.
"""

from __future__ import annotations

from .config import Environment, LabError

PACKAGES = {
    "direct": ("iproute2", "curl", "nftables"),
    "wireguard": ("wireguard-tools", "iproute2", "nftables", "curl", "tcpdump"),
    "openvpn": ("openvpn", "iproute2", "nftables", "curl", "tcpdump"),
    "tor": ("tor", "torsocks", "curl", "iproute2"),
    "socks": ("openssh-client", "curl", "iproute2"),
}
MANUAL = {"tor-browser", "i2p", "proton", "windscribe"}


def provision_script(env: Environment) -> str:
    if env.distribution == "windows":
        raise LabError(
            "Windows clients require a reviewed installer in a manual VM snapshot"
        )
    if env.provider in MANUAL:
        raise LabError(
            f"{env.provider}: prepare manually; no unattended account or third-party install"
        )
    package_list = PACKAGES[env.provider]
    if env.distribution in {"arch", "cachyos"} and env.provider == "socks":
        package_list = ("openssh", "curl", "iproute2")
    packages = " ".join(package_list)
    if env.distribution == "ubuntu":
        return (
            "set -euo pipefail\nexport DEBIAN_FRONTEND=noninteractive\n"
            "apt-get update\napt-get install -y --no-install-recommends " + packages
        )
    return "set -euo pipefail\npacman -S --needed --noconfirm " + packages


def connect_script(env: Environment) -> str:
    """Use a previously provisioned/configured guest, not unreviewed downloaded configs."""
    if env.distribution == "windows":
        raise LabError("Windows provider connection needs its reviewed desktop client")
    if env.provider == "wireguard":
        return (
            "set -euo pipefail\n"
            "test -f /etc/wireguard/ace-lab.conf || { echo 'Missing lab peer config'; exit 3; }\n"
            "wg-quick up ace-lab\nwg show ace-lab"
        )
    if env.provider == "openvpn":
        return (
            "set -euo pipefail\n"
            "test -f /etc/openvpn/client/ace-lab.conf || { echo 'Missing lab client config'; exit 3; }\n"
            "systemctl start openvpn-client@ace-lab\nsystemctl is-active openvpn-client@ace-lab"
        )
    if env.provider == "tor":
        return (
            "set -euo pipefail\n"
            "tor --verify-config >/dev/null\n"
            "(systemctl start tor@default || systemctl start tor)\n"
            "sleep 2\nss -lntH '( sport = :9050 )' | grep -q ':9050'"
        )
    raise LabError(f"{env.provider}: connection is manual or no tunnel is needed")


def disconnect_script(env: Environment) -> str:
    if env.distribution == "windows":
        raise LabError("Windows disconnect requires reviewed desktop client")
    if env.provider == "wireguard":
        return "wg-quick down ace-lab"
    if env.provider == "openvpn":
        return "systemctl stop openvpn-client@ace-lab"
    if env.provider == "tor":
        return "systemctl stop tor@default || systemctl stop tor"
    raise LabError(f"{env.provider}: no managed tunnel to disconnect")


def probe_script(env: Environment, kind: str) -> str | None:
    """Read-only probes do not prove end-to-end browser enforcement."""
    if kind in ("browser", "manual"):
        return None
    if env.distribution == "windows":
        if kind == "baseline":
            return (
                "$ErrorActionPreference='Stop'; "
                "$s=Get-Service AceEngine,AceEngineWatchdog; "
                "$s | Select-Object Name,Status; "
                "if (@($s | Where-Object Status -ne 'Running').Count -gt 0) { exit 3 }"
            )
        if kind == "routing":
            return "Get-NetRoute -AddressFamily IPv4 | Select-Object DestinationPrefix,NextHop,InterfaceAlias"
        return None
    services = (
        "printf 'ENGINE: '; systemctl is-active ace-engine || true; "
        "printf 'WATCHDOG: '; systemctl is-active ace-watchdog || true; "
        "systemctl is-active --quiet ace-engine && systemctl is-active --quiet ace-watchdog"
    )
    if kind == "baseline":
        return "set -eu\n" + services + "\nip -brief address\nip route show default"
    if kind == "routing":
        script = (
            "set -eu\n"
            + services
            + "\nip -4 route; ip -6 route; ip rule; ip -brief address"
        )
        if env.provider == "wireguard":
            # No handshake means a configured interface is NOT a functioning tunnel.
            script += "\nwg show ace-lab latest-handshakes | awk 'NF>=2 && $2>0 {ok=1} END {exit !ok}'"
        if env.provider == "openvpn":
            script += "\nsystemctl is-active --quiet openvpn-client@ace-lab"
        return script
    if kind == "tor-socks" and env.provider == "tor":
        return "set -eu\ncommand -v tor\ncommand -v torsocks\nss -lntH '( sport = :9050 )' | grep -q ':9050'"
    if kind == "browser-guard":
        return (
            "set -eu\n"
            "test -x /opt/ace-blocker/browser-proxy-policy.py || exit 3\n"
            "/opt/ace-blocker/browser-proxy-policy.py --status"
        )
    return None
