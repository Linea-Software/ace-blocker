"""Two-NovaGPT-fork WireGuard lab, using private QEMU point-to-point NICs only.

No libvirt and no host routing edits. Gateway/client are already-running,
human-approved NovaGPT QEMU forks. Private WG keys remain in guest files.
"""

from __future__ import annotations

import hashlib
import ipaddress
import re

from .config import Config, Environment, LabError, VMProfile, WireGuardLab
from .providers import provision_script
from .vm import VM

PUBLIC_KEY = re.compile(r"[A-Za-z0-9+/]{43}=")
DEVICE = re.compile(r"[A-Za-z0-9_.-]{1,15}")


def require_id(profile: VMProfile) -> str:
    if not profile.id:
        raise LabError(f"VM profile {profile.name} has no NovaGPT fork id in TOML")
    return profile.id


def guest_exec(
    vm: VM, vm_id: str, env: Environment, script: str, timeout: int = 120
) -> str:
    result = vm.exec(vm_id, env, script, seconds=timeout)
    if result.code != 0 or result.truncated:
        raise LabError(f"NovaGPT guest setup failed ({env.name}, exit {result.code})")
    return result.stdout.strip()


def nic_mac(vm_id: str, index: int) -> str:
    """Match the deterministic QEMU NIC MAC implementation in novagpt-vm/linux_qemu.rs."""
    digest = hashlib.sha256(vm_id.encode("utf-8") + bytes([index])).digest()
    return "52:54:00:" + ":".join(f"{value:02x}" for value in digest[:3])


def guest_nic(vm: VM, vm_id: str, env: Environment, index: int) -> str:
    mac = nic_mac(vm_id, index)
    script = (
        "set -euo pipefail\nfor iface in /sys/class/net/*; do\n"
        f'  if [ "$(cat "$iface/address")" = \'{mac}\' ]; then basename "$iface"; fi\n'
        "done"
    )
    interface = guest_exec(vm, vm_id, env, script)
    if not DEVICE.fullmatch(interface):
        raise LabError(
            f"Cannot identify private QEMU NIC for {env.name} (index {index})"
        )
    return interface


def configure_link(
    vm: VM, vm_id: str, env: Environment, interface: str, address: str
) -> None:
    """Persist a private guest NIC address across snapshot restore and guest reboot.

    The provider snapshot is always taken with the guest powered off; an
    ephemeral 'ip address add' would disappear and break future WireGuard
    connection attempts. Install an exact, idempotent guest-only systemd unit.
    """
    if not DEVICE.fullmatch(interface):
        raise LabError("Unsafe guest NIC device name")
    address = str(ipaddress.IPv4Interface(address))
    unit = f"ace-lab-link-{interface}.service"
    script = (
        "set -euo pipefail\n"
        "ip_bin=$(command -v ip)\n"
        'case "$ip_bin" in /*) ;; *) exit 3;; esac\n'
        f"dest=/etc/systemd/system/{unit}\n"
        "tmp=$(mktemp /etc/systemd/system/.ace-lab.XXXXXX)\n"
        "trap 'rm -f \"$tmp\"' EXIT\n"
        'cat > "$tmp" <<ACE_LAB_LINK\n'
        "[Unit]\n"
        f"Description=Ace test-only isolated NIC {interface}\n"
        "After=network.target\n"
        "[Service]\n"
        "Type=oneshot\n"
        "RemainAfterExit=yes\n"
        f"ExecStart=$ip_bin link set dev {interface} up\n"
        f"ExecStart=$ip_bin address replace {address} dev {interface}\n"
        "[Install]\n"
        "WantedBy=multi-user.target\n"
        "ACE_LAB_LINK\n"
        'if test -e "$dest"; then\n'
        '  cmp -s "$tmp" "$dest" || { echo \'Existing link unit differs\'; exit 3; }\n'
        'else mv "$tmp" "$dest"; fi\n'
        'chmod 0644 "$dest"\n'
        "systemctl daemon-reload\n"
        f"systemctl enable --now {unit}\n"
    )
    guest_exec(vm, vm_id, env, script)


def public_key(vm: VM, vm_id: str, env: Environment, interface: str) -> str:
    script = (
        f"set -euo pipefail\numask 077\ninstall -d -m 0700 /etc/wireguard\n"
        f"key=/etc/wireguard/{interface}.key\n"
        'if ! test -f "$key"; then wg genkey > "$key"; fi\n'
        'chmod 0600 "$key"\nwg pubkey < "$key"'
    )
    value = guest_exec(vm, vm_id, env, script)
    if not PUBLIC_KEY.fullmatch(value):
        raise LabError("QGA did not return a valid public WireGuard key")
    return value


def install_config(
    vm: VM, vm_id: str, env: Environment, interface: str, lines: list[str]
) -> None:
    # Command substitution is expanded by the guest shell into a private 0600
    # temporary file, NOT into the MCP tool argument or tool response.
    desired = "\n".join(lines).replace(
        "__PRIVATE_KEY__", f"$(cat /etc/wireguard/{interface}.key)"
    )
    script = (
        f"set -euo pipefail\numask 077\n"
        f"dest=/etc/wireguard/{interface}.conf\n"
        f"tmp=$(mktemp /etc/wireguard/.ace-lab.XXXXXX)\n"
        f"trap 'rm -f \"$tmp\"' EXIT\n"
        f'cat > "$tmp" <<ACE_LAB_CONF\n{desired}\nACE_LAB_CONF\n'
        'if test -e "$dest"; then\n'
        ' cmp -s "$tmp" "$dest" || { echo \'Existing WireGuard config differs\'; exit 3; }\n'
        'else mv "$tmp" "$dest"; fi\n'
        'chmod 0600 "$dest"\n'
    )
    guest_exec(vm, vm_id, env, script)


def setup(vm: VM, cfg: Config, client_env: Environment) -> dict:
    lab = cfg.wireguard
    if lab is None:
        raise LabError("Missing [wireguard_lab] in TOML")
    if client_env.provider != "wireguard" or client_env.distribution == "windows":
        raise LabError(
            "WireGuard lab supports only Linux WireGuard client environments"
        )
    if client_env.distribution not in lab.peers:
        raise LabError(f"Missing WireGuard peer for {client_env.distribution}")
    gateway_profile = cfg.vms[lab.gateway_vm]
    client_profile = cfg.vms[client_env.vm]
    gateway_id = require_id(gateway_profile)
    client_id = require_id(client_profile)
    if gateway_id == client_id:
        raise LabError("WireGuard gateway and client must be distinct NovaGPT forks")
    peer = lab.peers[client_env.distribution]
    server_link = f"listen:ace-{client_env.distribution}"
    client_link = f"connect:ace-{client_env.distribution}"
    if (
        server_link not in gateway_profile.networks
        or client_link not in client_profile.networks
    ):
        raise LabError("Gateway/client point-to-point QEMU NICs are not declared")
    server_nic_idx = gateway_profile.networks.index(server_link)
    client_nic_idx = client_profile.networks.index(client_link)
    if "nat" not in gateway_profile.networks:
        raise LabError("Gateway needs its own outbound NAT interface")
    server = Environment(
        "wireguard-gateway", lab.gateway_vm, "ubuntu", "wireguard", "gateway-ready"
    )
    for entry_id, env in ((gateway_id, server), (client_id, client_env)):
        if vm.state(entry_id) != "running":
            raise LabError(f"{env.name}: VM must already be running")
        vm.wait_guest(entry_id, seconds=90)
        if guest_exec(vm, entry_id, env, "id -u") != "0":
            raise LabError(
                f"{env.name}: guest agent requires root privileges for lab setup"
            )
        guest_exec(vm, entry_id, env, provision_script(env), timeout=300)

    server_nic = guest_nic(vm, gateway_id, server, server_nic_idx)
    client_nic = guest_nic(vm, client_id, client_env, client_nic_idx)
    configure_link(vm, gateway_id, server, server_nic, peer.gateway_link)
    configure_link(vm, client_id, client_env, client_nic, peer.client_link)
    key_server = public_key(vm, gateway_id, server, peer.iface)
    key_client = public_key(vm, client_id, client_env, "ace-lab")

    nat_table = "ace_lab_nat_" + peer.iface
    nat_dev = guest_exec(
        vm,
        gateway_id,
        server,
        """ip -4 route show default | awk 'NR==1 {for (i=1;i<=NF;i++) if ($i=="dev") {print $(i+1); exit}}'""",
    )
    if not DEVICE.fullmatch(nat_dev):
        raise LabError("Gateway outbound NAT device not found")
    server_lines = [
        "[Interface]",
        f"Address = {peer.gateway_address}",
        f"ListenPort = {peer.port}",
        "PrivateKey = __PRIVATE_KEY__",
        f"PostUp = sysctl -w net.ipv4.ip_forward=1 >/dev/null; nft add table ip {nat_table}; nft 'add chain ip {nat_table} postrouting {{ type nat hook postrouting priority 100; policy accept; }}'; nft add rule ip {nat_table} postrouting ip saddr {peer.subnet} oifname {nat_dev} masquerade",
        f"PostDown = nft delete table ip {nat_table}",
        "",
        "[Peer]",
        f"PublicKey = {key_client}",
        f"AllowedIPs = {ipaddress.IPv4Interface(peer.client_address).ip}/32",
    ]
    client_lines = [
        "[Interface]",
        f"Address = {peer.client_address}",
        "PrivateKey = __PRIVATE_KEY__",
        "",
        "[Peer]",
        f"PublicKey = {key_server}",
        f"Endpoint = {ipaddress.IPv4Interface(peer.gateway_link).ip}:{peer.port}",
        "AllowedIPs = 0.0.0.0/0",
        "PersistentKeepalive = 25",
    ]
    install_config(vm, gateway_id, server, peer.iface, server_lines)
    install_config(vm, client_id, client_env, "ace-lab", client_lines)
    # The WireGuard gateway must wait for its private link address after a
    # reboot; the guest-side systemd drop-in persists through VM snapshots.
    link_unit = f"ace-lab-link-{server_nic}.service"
    guest_exec(
        vm,
        gateway_id,
        server,
        "set -euo pipefail\n"
        f"dir=/etc/systemd/system/wg-quick@{peer.iface}.service.d\n"
        'install -d -m 0755 "$dir"\n'
        'tmp=$(mktemp "$dir"/.ace-lab.XXXXXX)\n'
        "trap 'rm -f \"$tmp\"' EXIT\n"
        "cat > \"$tmp\" <<'ACE_LAB_UNIT'\n"
        "[Unit]\n"
        f"Requires={link_unit}\nAfter={link_unit}\n"
        "ACE_LAB_UNIT\n"
        "dest=$dir/ace-lab-link.conf\n"
        'if test -e "$dest"; then\n'
        ' cmp -s "$tmp" "$dest" || { echo \'Existing WG drop-in differs\'; exit 3; }\n'
        'else mv "$tmp" "$dest"; fi\n'
        'chmod 0644 "$dest"\n'
        "systemctl daemon-reload\n"
        f"systemctl enable --now wg-quick@{peer.iface}\n"
        f"systemctl is-active --quiet wg-quick@{peer.iface}",
    )
    return {
        "gateway_vm": gateway_id,
        "client_vm": client_id,
        "gateway_interface": peer.iface,
        "client_interface": "ace-lab",
        "transport": "NovaGPT isolated QEMU NICs",
        "ipv6_covered": False,
    }
