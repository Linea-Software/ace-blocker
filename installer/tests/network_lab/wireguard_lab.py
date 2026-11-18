"""Optional two-VM WireGuard lab provisioning; all private keys remain in guests.

Configure gateway endpoint and per-distro peer interface/subnet in lab.toml.
Only owned disposable libvirt VMs may be passed to this function.
"""
from __future__ import annotations

import ipaddress
import re

from .config import Environment, LabError, WireGuardLab
from .providers import provision_script
from .vm import VM

PUBLIC_KEY = re.compile(r"[A-Za-z0-9+/]{43}=")
DEVICE = re.compile(r"[A-Za-z0-9_.-]{1,15}")


def must_exec(vm: VM, env: Environment, script: str, *, seconds: int = 120) -> str:
    result = vm.exec(env, script, seconds=seconds)
    if result.code != 0:
        raise LabError(f"{env.name}: WireGuard lab setup failed (exit {result.code}); "
                       f"{result.stderr.strip()[:200]}")
    return result.stdout.strip()


def public_key(vm: VM, env: Environment, interface: str) -> str:
    # Outputs ONLY the public key; 0600 private key remains at the guest.
    script = (f"set -euo pipefail\numask 077\n"
              f"install -d -m 0700 /etc/wireguard\n"
              f"key=/etc/wireguard/{interface}.key\n"
              f"if ! test -f \"$key\"; then wg genkey > \"$key\"; fi\n"
              f"chmod 0600 \"$key\"\nwg pubkey < \"$key\"")
    key = must_exec(vm, env, script)
    if not PUBLIC_KEY.fullmatch(key):
        raise LabError(f"{env.name}: invalid public key received from guest")
    return key


def install_config(vm: VM, env: Environment, interface: str, lines: list[str]) -> None:
    """Create exact config; refusing to silently overwrite a different peer."""
    desired = "\n".join(lines).replace("__PRIVATE_KEY__", f"$(cat /etc/wireguard/{interface}.key)")
    script = (f"set -euo pipefail\numask 077\n"
              f"dest=/etc/wireguard/{interface}.conf\n"
              f"tmp=$(mktemp /etc/wireguard/.ace-lab.XXXXXX)\n"
              f"trap 'rm -f \"$tmp\"' EXIT\n"
              f"cat > \"$tmp\" <<ACE_CONFIG\n{desired}\nACE_CONFIG\n"
              f"if test -e \"$dest\"; then\n"
              f"  cmp -s \"$tmp\" \"$dest\" || {{ echo 'Different peer configuration already present'; exit 3; }}\n"
              f"else mv \"$tmp\" \"$dest\"; fi\n"
              f"chmod 0600 \"$dest\"\necho CONFIG_READY")
    must_exec(vm, env, script)


def setup(vm: VM, client: Environment, lab: WireGuardLab) -> dict:
    if client.provider != "wireguard" or client.distribution == "windows":
        raise LabError("Automatic WireGuard lab setup currently supports Linux WireGuard clients")
    if client.distribution not in lab.peers:
        raise LabError(f"No lab WireGuard peer defined for {client.distribution}")
    if client.domain == lab.gateway_domain:
        raise LabError("WireGuard gateway and client must be separate disposable VMs")
    if not client.domain.startswith("ace-lab-") or not lab.gateway_domain.startswith("ace-lab-"):
        raise LabError("WireGuard client/gateway must be owned ace-lab-* disposable domains")
    peer = lab.peers[client.distribution]
    gateway = Environment(
        "lab-gateway", lab.gateway_domain, lab.gateway_distribution,
        "wireguard", "lab-gateway-ready",
    )
    for env in (gateway, client):
        if vm.state(env) != "running":
            raise LabError(f"{env.domain}: start the owned VM and its QEMU guest agent first")
        vm.wait_agent(env)
        if must_exec(vm, env, "id -u") != "0":
            raise LabError(f"{env.domain}: guest agent must execute as root for package setup")
        must_exec(vm, env, provision_script(env), seconds=600)

    endpoint = str(ipaddress.IPv4Address(lab.endpoint_ipv4))
    # Ensure the configured endpoint is actually on the gateway; no silent
    # assumptions about bridged/host networking.
    must_exec(
        vm, gateway,
        "ip -4 -o addr show | awk '{print $4}' | cut -d/ -f1 | grep -Fx " + endpoint,
    )
    wan = must_exec(
        vm, gateway,
        """ip -4 route show default | awk 'NR==1 {for (i=1;i<=NF;i++) if ($i=="dev") {print $(i+1); exit}}'""",
    )
    if not DEVICE.fullmatch(wan):
        raise LabError("Cannot identify safe gateway outbound interface")
    public_gateway = public_key(vm, gateway, peer.iface)
    public_client = public_key(vm, client, "ace-lab")
    nat = "ace_lab_nat_" + peer.iface
    client_ip = str(ipaddress.IPv4Interface(peer.client_address).ip)
    server_lines = [
        "[Interface]",
        f"Address = {peer.gateway_address}",
        f"ListenPort = {peer.port}",
        "PrivateKey = __PRIVATE_KEY__",
        f"PostUp = sysctl -w net.ipv4.ip_forward=1 >/dev/null; nft add table ip {nat}; nft 'add chain ip {nat} postrouting {{ type nat hook postrouting priority 100; policy accept; }}'; nft add rule ip {nat} postrouting ip saddr {peer.subnet} oifname {wan} masquerade",
        f"PostDown = nft delete table ip {nat}",
        "",
        "[Peer]",
        f"PublicKey = {public_client}",
        f"AllowedIPs = {client_ip}/32",
    ]
    client_lines = [
        "[Interface]",
        f"Address = {peer.client_address}",
        "PrivateKey = __PRIVATE_KEY__",
        "",
        "[Peer]",
        f"PublicKey = {public_gateway}",
        f"Endpoint = {endpoint}:{peer.port}",
        "AllowedIPs = 0.0.0.0/0",
        "PersistentKeepalive = 25",
    ]
    install_config(vm, gateway, peer.iface, server_lines)
    install_config(vm, client, "ace-lab", client_lines)
    # Only the owned gateway starts forwarding; the client interface remains
    # down so a provider-ready snapshot can be taken without stale tunnel state.
    must_exec(vm, gateway, f"systemctl enable --now wg-quick@{peer.iface}\nsystemctl is-active --quiet wg-quick@{peer.iface}")
    return {"gateway": gateway.domain, "gateway_interface": peer.iface,
            "client": client.domain, "client_interface": "ace-lab",
            "gateway_endpoint": endpoint, "port": peer.port}
