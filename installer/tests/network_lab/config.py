"""TOML-only configuration for NovaGPT-managed, approved disposable VM forks."""

from __future__ import annotations

import ipaddress
import re
import tomllib
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import urlsplit

IDENT = re.compile(r"^[A-Za-z0-9][A-Za-z0-9_.-]{0,79}$")
VM_ID = re.compile(r"^vm-[0-9a-f]{16}$")
SEGMENT = re.compile(r"^[A-Za-z0-9_-]{1,48}$")
PROVIDERS = {"direct", "wireguard", "openvpn", "tor", "tor-browser",
             "socks", "i2p", "proton", "windscribe"}
DISTRIBUTIONS = {"ubuntu", "arch", "cachyos", "windows"}
KINDS = {"baseline", "routing", "tor-socks", "browser-guard", "browser", "manual"}


class LabError(RuntimeError):
    """Invalid configuration or rejected NovaGPT operation."""


def ident(value: str, label: str = "identifier") -> str:
    if not isinstance(value, str) or not IDENT.fullmatch(value):
        raise LabError(f"Invalid {label}: {value!r}")
    return value


def vm_id(value: str) -> str:
    if not isinstance(value, str) or not VM_ID.fullmatch(value):
        raise LabError("Configured VM id must match vm-<16 lowercase hex digits>")
    return value


def networks(values: list[str]) -> tuple[str, ...]:
    if not isinstance(values, list) or not values or len(values) > 4:
        raise LabError("VM networks must be a nonempty array of up to four NICs")
    if values == ["none"]:
        return tuple(values)
    if "none" in values:
        raise LabError("'none' is only valid as the single network entry")
    for item in values:
        if item == "nat":
            continue
        if not isinstance(item, str) or not (
            (item.startswith("listen:") and SEGMENT.fullmatch(item[7:])) or
            (item.startswith("connect:") and SEGMENT.fullmatch(item[8:]))
        ):
            raise LabError(f"Invalid NovaGPT NIC specification: {item!r}")
    return tuple(values)


@dataclass(frozen=True)
class VMProfile:
    name: str
    image: str
    id: str
    cpus: int
    memory_mb: int
    networks: tuple[str, ...]
    display: bool
    guest_agent: bool


@dataclass(frozen=True)
class Environment:
    name: str
    vm: str
    distribution: str
    provider: str
    snapshot: str


@dataclass(frozen=True)
class Case:
    id: str
    kind: str
    environments: tuple[str, ...]
    expected: str


@dataclass(frozen=True)
class WireGuardPeer:
    iface: str
    port: int
    subnet: str
    gateway_address: str
    client_address: str
    gateway_link: str
    client_link: str


@dataclass(frozen=True)
class WireGuardLab:
    gateway_vm: str
    peers: dict[str, WireGuardPeer]


@dataclass(frozen=True)
class Config:
    url: str
    token_env: str
    artifacts: Path
    vms: dict[str, VMProfile]
    environments: dict[str, Environment]
    cases: dict[str, Case]
    wireguard: WireGuardLab | None


def parse_wireguard(raw: dict, profiles: dict[str, VMProfile]) -> WireGuardLab | None:
    if "wireguard_lab" not in raw:
        return None
    lab = raw["wireguard_lab"]
    if not isinstance(lab, dict) or not isinstance(lab.get("peers"), dict):
        raise LabError("wireguard_lab.peers must be a TOML table")
    gateway = ident(lab["gateway_vm"], "gateway VM profile")
    if gateway not in profiles:
        raise LabError("WireGuard gateway VM profile is missing")
    peers = {}
    for distro, raw_peer in lab["peers"].items():
        if distro not in {"ubuntu", "arch", "cachyos"} or not isinstance(raw_peer, dict):
            raise LabError("WireGuard peers must be distro-specific TOML tables")
        iface = ident(raw_peer["iface"])
        if len(iface) > 15 or not re.fullmatch(r"[A-Za-z][A-Za-z0-9_-]*", iface):
            raise LabError("Invalid WireGuard gateway interface")
        port = raw_peer["port"]
        if type(port) is not int or not 1024 <= port <= 65535:
            raise LabError("WireGuard UDP port must be 1024..65535")
        subnet = ipaddress.IPv4Network(raw_peer["subnet"], strict=True)
        server = ipaddress.IPv4Interface(raw_peer["gateway_address"])
        client = ipaddress.IPv4Interface(raw_peer["client_address"])
        if (server.network != subnet or client.network.prefixlen != 32
                or server.ip not in subnet or client.ip not in subnet
                or server.ip == client.ip):
            raise LabError("Invalid WireGuard peer subnet/address combination")
        gateway_link = ipaddress.IPv4Interface(raw_peer["gateway_link"])
        client_link = ipaddress.IPv4Interface(raw_peer["client_link"])
        if (gateway_link.network != client_link.network or gateway_link.ip == client_link.ip
                or gateway_link.network.overlaps(subnet)):
            raise LabError("WireGuard private Ethernet link must be a separate shared subnet")
        peers[distro] = WireGuardPeer(iface, port, str(subnet), str(server), str(client),
                                     str(gateway_link), str(client_link))
    if len({p.iface for p in peers.values()}) != len(peers) or len({p.port for p in peers.values()}) != len(peers):
        raise LabError("WireGuard gateway interface and UDP port must be unique")
    subnets = [ipaddress.IPv4Network(p.subnet) for p in peers.values()]
    links = [ipaddress.IPv4Interface(p.gateway_link).network for p in peers.values()]
    for networks_to_check in (subnets, links):
        if any(a.overlaps(b) for i, a in enumerate(networks_to_check)
               for b in networks_to_check[i + 1:]):
            raise LabError("WireGuard tunnel or transport subnets overlap")
    return WireGuardLab(gateway, peers)


def load(path: Path) -> Config:
    try:
        path = path.resolve(strict=True)
        raw = tomllib.loads(path.read_text(encoding="utf-8"))
        lab = raw["lab"]
        url = lab.get("mcp_url", "http://127.0.0.1:8787/mcp")
        parsed = urlsplit(url)
        if (parsed.scheme != "http" or parsed.hostname not in {"127.0.0.1", "localhost", "::1"}
                or parsed.username or parsed.password or parsed.query or parsed.fragment
                or parsed.path != "/mcp"):
            raise LabError("mcp_url must be a plain loopback HTTP /mcp URL without credentials")
        token_env = ident(lab.get("token_env", "NOVAGPT_HTTP_TOKEN"), "token environment variable")
        artifact_name = lab.get("artifacts", ".artifacts")
        if not isinstance(artifact_name, str) or not re.fullmatch(r"\.?[A-Za-z0-9][A-Za-z0-9_.-]{0,79}", artifact_name):
            raise LabError("artifacts must be one relative directory name")

        vms: dict[str, VMProfile] = {}
        for key, item in raw["vms"].items():
            name = ident(key, "VM profile")
            if not isinstance(item, dict):
                raise LabError(f"VM profile {name} must be a TOML table")
            image = ident(item["image"], "approved VM image")
            current_id = item.get("id", "")
            if current_id:
                vm_id(current_id)
            cpus = item.get("cpus", 4)
            memory = item.get("memory_mb", 8192)
            if type(cpus) is not int or not 1 <= cpus <= 64:
                raise LabError(f"{name}: cpus out of range")
            if type(memory) is not int or not 256 <= memory <= 262144:
                raise LabError(f"{name}: memory_mb out of range")
            display = item.get("display", True)
            guest_agent = item.get("guest_agent", True)
            if type(display) is not bool or type(guest_agent) is not bool:
                raise LabError(f"{name}: display/guest_agent must be boolean")
            vms[name] = VMProfile(name, image, current_id, cpus, memory,
                                  networks(item.get("networks", ["nat"])), display, guest_agent)

        assigned_ids = [p.id for p in vms.values() if p.id]
        if len(assigned_ids) != len(set(assigned_ids)):
            raise LabError("Two NovaGPT VM profiles cannot reference the same fork ID")
        envs: dict[str, Environment] = {}
        for item in raw["environments"]:
            name = ident(item["name"], "environment")
            vm_name = ident(item["vm"], "VM profile")
            dist = item["distribution"]
            provider = item["provider"]
            if vm_name not in vms or dist not in DISTRIBUTIONS or provider not in PROVIDERS or name in envs:
                raise LabError(f"Duplicate or unknown environment {name}")
            envs[name] = Environment(name, vm_name, dist, provider, ident(item["snapshot"], "snapshot"))
        cases: dict[str, Case] = {}
        for item in raw["cases"]:
            cid = ident(item["id"], "test ID")
            kind = item["kind"]
            targets = item["environments"]
            expected = item["expected"]
            if (cid in cases or kind not in KINDS or not isinstance(targets, list)
                    or not targets or any(t not in envs for t in targets)
                    or not isinstance(expected, str) or not expected.strip()):
                raise LabError(f"Invalid case: {cid}")
            cases[cid] = Case(cid, kind, tuple(targets), expected)
        if not vms or not envs or not cases:
            raise LabError("TOML requires VMs, environments and test cases")
        wg = parse_wireguard(raw, vms)
        if wg:
            gateway_networks = vms[wg.gateway_vm].networks
            for distro in wg.peers:
                link = f"listen:ace-{distro}"
                if link not in gateway_networks:
                    raise LabError(f"Gateway requires private NIC {link}")
                clients = [e for e in envs.values() if e.provider == "wireguard" and e.distribution == distro and e.vm != wg.gateway_vm]
                if not clients or any(f"connect:ace-{distro}" not in vms[e.vm].networks for e in clients):
                    raise LabError(f"WireGuard clients for {distro} require matching private NIC")
        return Config(url, token_env, path.parent / artifact_name, vms, envs, cases, wg)
    except (OSError, ValueError, TypeError, KeyError, AttributeError) as exc:
        raise LabError(f"Invalid lab TOML: {exc}") from exc
