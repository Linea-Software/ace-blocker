"""Strict stdlib TOML configuration for VM/provider suites."""
from __future__ import annotations

import ipaddress
import re
import tomllib
from dataclasses import dataclass
from pathlib import Path

ID = re.compile(r"^[A-Za-z0-9][A-Za-z0-9_.-]{0,79}$")
PROVIDERS = {"direct", "wireguard", "openvpn", "tor", "tor-browser",
             "socks", "i2p", "proton", "windscribe"}
DISTROS = {"ubuntu", "arch", "cachyos", "windows"}
KINDS = {"baseline", "routing", "tor-socks", "browser-guard", "browser", "manual"}


class LabError(RuntimeError):
    pass


def ident(value: str) -> str:
    if not isinstance(value, str) or not ID.fullmatch(value):
        raise LabError(f"Invalid identifier: {value!r}")
    return value


@dataclass(frozen=True)
class Environment:
    name: str
    domain: str
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


@dataclass(frozen=True)
class WireGuardLab:
    gateway_domain: str
    gateway_distribution: str
    endpoint_ipv4: str
    peers: dict[str, WireGuardPeer]


@dataclass(frozen=True)
class Config:
    uri: str
    artifacts: Path
    environments: dict[str, Environment]
    cases: dict[str, Case]
    wireguard: WireGuardLab | None = None


def parse_wireguard(raw: dict) -> WireGuardLab | None:
    if "wireguard_lab" not in raw:
        return None
    wg = raw["wireguard_lab"]
    if not isinstance(wg, dict) or not isinstance(wg.get("peers"), dict):
        raise LabError("[wireguard_lab.peers] must be a TOML table")
    gateway = ident(wg["gateway_domain"])
    distro = wg["gateway_distribution"]
    if distro not in {"ubuntu", "arch", "cachyos"}:
        raise LabError("WireGuard gateway must be a supported Linux guest")
    endpoint = str(ipaddress.IPv4Address(wg["endpoint_ipv4"]))
    peers = {}
    for name, item in wg["peers"].items():
        if not isinstance(item, dict):
            raise LabError("WireGuard peer configuration must be a TOML table")
        if name not in DISTROS:
            raise LabError(f"Unknown WireGuard peer OS: {name}")
        iface = ident(item["iface"])
        if len(iface) > 15 or not re.fullmatch(r"[A-Za-z][A-Za-z0-9_-]*", iface):
            raise LabError("WireGuard interface must be a short safe Linux device name")
        port = item["port"]
        if not isinstance(port, int) or not 1024 <= port <= 65535:
            raise LabError("WireGuard port must be 1024..65535")
        net = ipaddress.IPv4Network(item["subnet"], strict=True)
        gateway_addr = ipaddress.IPv4Interface(item["gateway_address"])
        client_addr = ipaddress.IPv4Interface(item["client_address"])
        if (gateway_addr.network != net or client_addr.network.prefixlen != 32
                or gateway_addr.ip not in net or client_addr.ip not in net
                or gateway_addr.ip == client_addr.ip):
            raise LabError("WireGuard gateway and /32 client must belong to the configured subnet")
        peers[name] = WireGuardPeer(iface, port, str(net),
                                   str(gateway_addr), str(client_addr))
    if len({p.port for p in peers.values()}) != len(peers) or len({p.iface for p in peers.values()}) != len(peers):
        raise LabError("WireGuard gateway ports/interfaces must be unique per distro")
    networks = [ipaddress.IPv4Network(peer.subnet) for peer in peers.values()]
    if any(left.overlaps(right) for i, left in enumerate(networks) for right in networks[i+1:]):
        raise LabError("WireGuard gateway peer subnets must not overlap")
    return WireGuardLab(gateway, distro, endpoint, peers)


def load(path: Path) -> Config:
    try:
        path = path.resolve(strict=True)
        raw = tomllib.loads(path.read_text(encoding="utf-8"))
        lab = raw["lab"]
        uri = lab.get("libvirt_uri", "qemu:///session")
        if uri not in {"qemu:///session", "qemu:///system"}:
            raise LabError("Only local qemu libvirt URIs are supported")
        artifact_name = lab.get("artifacts", ".artifacts")
        if not isinstance(artifact_name, str) or not re.fullmatch(r"\.?[A-Za-z0-9][A-Za-z0-9_.-]{0,79}", artifact_name):
            raise LabError("artifacts must be a single relative directory name")
        artifact = path.parent / artifact_name
        envs: dict[str, Environment] = {}
        for item in raw["environments"]:
            name = ident(item["name"])
            dist = item["distribution"]
            provider = item["provider"]
            if dist not in DISTROS or provider not in PROVIDERS or name in envs:
                raise LabError(f"Duplicate or unsupported environment: {name}")
            envs[name] = Environment(name, ident(item["domain"]), dist, provider, ident(item["snapshot"]))
        cases: dict[str, Case] = {}
        for item in raw["cases"]:
            cid = ident(item["id"])
            kind = item["kind"]
            targets = tuple(item["environments"])
            expected = item["expected"]
            if cid in cases or kind not in KINDS or not targets or not expected:
                raise LabError(f"Invalid case {cid}")
            if any(target not in envs for target in targets):
                raise LabError(f"Unknown environment in case {cid}")
            cases[cid] = Case(cid, kind, targets, expected)
        if not envs or not cases:
            raise LabError("Need at least one environment and case")
        return Config(uri, artifact, envs, cases, parse_wireguard(raw))
    except (OSError, ValueError, TypeError, KeyError, AttributeError) as exc:
        raise LabError(f"Invalid TOML config: {exc}") from exc
