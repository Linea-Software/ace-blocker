#!/usr/bin/env python3
"""Declarative, guarded libvirt/QEMU network lab for Ace Blocker.

By default no guest, disk, snapshot or OS setting is mutated.
"""
from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import subprocess
import sys
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
    from network_lab.config import Case, Config, Environment, LabError, load  # type: ignore[import-not-found]
    from network_lab.providers import connect_script, disconnect_script, probe_script, provision_script  # type: ignore[import-not-found]
    from network_lab.vm import VM  # type: ignore[import-not-found]
    from network_lab.wireguard_lab import setup as setup_wireguard  # type: ignore[import-not-found]
else:
    from .config import Case, Config, Environment, LabError, load
    from .providers import connect_script, disconnect_script, probe_script, provision_script
    from .vm import VM
    from .wireguard_lab import setup as setup_wireguard


def git_revision() -> str:
    try:
        result = subprocess.run(["git", "-C", str(Path(__file__).resolve().parents[3]), "rev-parse", "HEAD"], capture_output=True,
                                text=True, timeout=8, check=False)
        return result.stdout.strip() if result.returncode == 0 else "unknown"
    except (OSError, subprocess.TimeoutExpired):
        return "unknown"


def save_report(directory: Path, data: dict) -> Path:
    # Reports may contain network metadata. Restrict permissions on host.
    if directory.is_symlink():
        raise LabError("Refusing symlink artifact directory")
    if directory.exists():
        if not directory.is_dir() or directory.stat().st_mode & 0o077:
            raise LabError("Existing artifact directory must already be private (mode 0700)")
    else:
        directory.mkdir(parents=True, exist_ok=False, mode=0o700)
    name = (data["environment"] + "-" +
            dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%S%fZ") + ".json")
    path = directory / name
    fd = os.open(path, os.O_CREAT | os.O_EXCL | os.O_WRONLY | getattr(os, "O_NOFOLLOW", 0), 0o600)
    with os.fdopen(fd, "w", encoding="utf-8") as dest:
        json.dump(data, dest, indent=2, ensure_ascii=False)
    return path


def authorize(args: argparse.Namespace, env: Environment, *, reset: bool = False) -> bool:
    if not args.execute:
        print("PLAN ONLY: no VM action taken; add --execute --confirm-domain", env.domain)
        if reset:
            print("Snapshot reset also requires --accept-data-loss; existing VM state will be replaced")
        return False
    if args.confirm_domain != env.domain:
        raise LabError("Refusing guest changes: --confirm-domain must equal the configured domain name")
    if reset and not args.accept_data_loss:
        raise LabError("Snapshot restore requires --accept-data-loss")
    return True


def confirm_vm_uuid(vm: VM, env: Environment, claimed: str | None) -> None:
    actual = vm.virsh("domuuid", env.domain)
    if not claimed or claimed != actual:
        raise LabError("Snapshot operation requires exact --confirm-uuid from 'virsh domuuid'")

def show_list(cfg: Config) -> None:
    print("Environment / libvirt domain / distro / provider / snapshot")
    for env in cfg.environments.values():
        count = sum(env.name in case.environments for case in cfg.cases.values())
        print(f"{env.name} / {env.domain} / {env.distribution} / {env.provider} / {env.snapshot} ({count} cases)")
    print(f"Configured: {len(cfg.environments)} environments, {len(cfg.cases)} cases")


def doctor(vm: VM, env: Environment) -> int:
    try:
        state = vm.state(env)
        snapshot = vm.snapshot_exists(env)
        actual_uuid = vm.virsh("domuuid", env.domain)
        print(f"{env.name}: domain={env.domain}, uuid={actual_uuid}, state={state}, snapshot={snapshot}")
        try:
            vm.preflight_disposable(env)
            print("  Snapshot backend: supported single-disk disposable QCOW2 VM")
        except LabError as exc:
            print(f"  Snapshot backend: UNSUPPORTED ({exc})")
        if state == "running":
            try:
                vm.agent(env, "guest-ping")
                print("  QEMU guest agent: ready")
            except LabError as exc:
                print(f"  QEMU guest agent: unavailable ({exc})")
                return 2
        return 0
    except LabError as exc:
        print(f"{env.name}: BLOCKED ({exc})")
        return 2


def apply_script(vm: VM, env: Environment, script: str, timeout: int) -> None:
    vm.wait_agent(env)
    result = vm.exec(env, script, seconds=timeout)
    print(f"Guest operation exit_code={result.code}")
    if result.code:
        raise LabError(f"Guest command failed: {result.stderr[:350] or result.stdout[:350]}")


def execute_cases(vm: VM, env: Environment, cases: list[Case], artifacts: Path, *, require_coverage: bool = False) -> int:
    results: list[dict] = []
    for case in cases:
        row = {"case": case.id, "expected": case.expected,
               "enforcement_proven": False}
        script = probe_script(env, case.kind)
        if script is None:
            row.update(status="NOT_RUN", note="Needs browser-origin/server evidence or a manually prepared provider")
        else:
            try:
                result = vm.exec(env, script, seconds=90)
                row.update(status="INFRA_OK" if result.code == 0 else "INFRA_FAILED",
                           exit_code=result.code, stdout=result.stdout, stderr=result.stderr,
                           note="Infrastructure only; no browser blocking claim")
            except LabError as exc:
                row.update(status="BLOCKED", reason=str(exc))
        results.append(row)
        print(f"{case.id}: {row['status']}")
    summary = {"environment": env.name, "domain": env.domain,
               "snapshot": env.snapshot, "provider": env.provider,
               "distribution": env.distribution, "git_commit": git_revision(),
               "collected_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
               "results": results}
    location = save_report(artifacts, summary)
    completed = sum(row["status"] == "INFRA_OK" for row in results)
    incomplete = sum(row["status"] == "NOT_RUN" for row in results)
    print(f"Infrastructure checks OK: {completed}; NOT RUN browser/manual: {incomplete}; enforcement PASS: 0")
    print(f"Private evidence report: {location}")
    if any(row["status"] in {"INFRA_FAILED", "BLOCKED"} for row in results):
        return 2
    if completed == 0 or (require_coverage and incomplete):
        return 3
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", type=Path, default=Path(__file__).with_name("lab.toml"))
    sub = parser.add_subparsers(dest="action", required=True)
    sub.add_parser("list")
    for action in ("doctor", "provision", "connect", "disconnect",
                   "snapshot-save", "snapshot-restore", "wireguard-setup", "run"):
        p = sub.add_parser(action)
        p.add_argument("--env", required=True, help="Named environment from TOML")
        if action not in ("doctor",):
            p.add_argument("--execute", action="store_true", help="Authorize guest changes and execution")
            p.add_argument("--confirm-domain", help="Exact libvirt domain name; required for --execute")
        if action == "wireguard-setup":
            p.add_argument("--confirm-gateway-domain", help="Exact disposable VPN gateway domain")
            p.add_argument("--confirm-gateway-uuid", help="Exact libvirt UUID for owned VPN gateway")
        if action in ("snapshot-save", "snapshot-restore", "run"):
            p.add_argument("--confirm-uuid", help="Exact libvirt domain UUID for snapshot operations")
        if action in ("snapshot-restore", "run"):
            p.add_argument("--accept-data-loss", action="store_true",
                           help="Accept snapshot restore deleting guest state after the snapshot")
        if action == "run":
            p.add_argument("--case", action="append", default=[], help="Run only this test ID (repeatable)")
            p.add_argument("--restore", action="store_true", help="Restore environment snapshot first; VM must be off")
            p.add_argument("--require-coverage", action="store_true",
                           help="Nonzero exit when any requested scenario is NOT_RUN")
    args = parser.parse_args()
    cfg = load(args.config)
    if args.action == "list":
        show_list(cfg)
        return 0
    if args.env not in cfg.environments:
        raise LabError(f"Unknown environment: {args.env}")
    env = cfg.environments[args.env]
    vm = VM(cfg.uri)
    if args.action == "doctor":
        return doctor(vm, env)
    if args.action == "run":
        cases = [case for case in cfg.cases.values()
                 if env.name in case.environments and (not args.case or case.id in args.case)]
        if args.case and set(args.case) - {case.id for case in cases}:
            raise LabError("Requested test ID is absent or not valid for this environment")
        if not cases:
            raise LabError("No cases selected")
        if not authorize(args, env, reset=args.restore):
            for case in cases:
                print(f"  {case.id}: {case.kind} / {case.expected}")
            return 0
        if args.restore:
            confirm_vm_uuid(vm, env, args.confirm_uuid)
            vm.restore_snapshot(env)
            vm.start(env)
        elif vm.state(env) != "running":
            raise LabError("Start the VM yourself or use --restore with a shut-off VM")
        vm.wait_agent(env)
        return execute_cases(vm, env, cases, cfg.artifacts,
                             require_coverage=args.require_coverage)
    if args.action in ("snapshot-save", "snapshot-restore"):
        if not authorize(args, env, reset=args.action == "snapshot-restore"):
            return 0
        confirm_vm_uuid(vm, env, args.confirm_uuid)
        if args.action == "snapshot-save":
            vm.create_snapshot(env)
        else:
            vm.restore_snapshot(env)
        print(f"{args.action}: {env.domain} / {env.snapshot}")
        return 0
    if args.action == "wireguard-setup":
        if cfg.wireguard is None:
            raise LabError("No [wireguard_lab] configured in TOML")
        if env.provider != "wireguard":
            raise LabError("Select a WireGuard client environment")
        if not authorize(args, env):
            print("Planned owned gateway:", cfg.wireguard.gateway_domain)
            return 0
        if args.confirm_gateway_domain != cfg.wireguard.gateway_domain:
            raise LabError("Refusing gateway changes: --confirm-gateway-domain must exactly match TOML")
        gateway = Environment("wireguard-gateway", cfg.wireguard.gateway_domain,
                              cfg.wireguard.gateway_distribution, "wireguard",
                              "wireguard-gateway-base")
        confirm_vm_uuid(vm, gateway, args.confirm_gateway_uuid)
        print(json.dumps(setup_wireguard(vm, env, cfg.wireguard), indent=2))
        return 0
    if args.action == "provision":
        script = provision_script(env)
        timeout = 600
    elif args.action == "connect":
        script = connect_script(env)
        timeout = 150
    else:
        script = disconnect_script(env)
        timeout = 120
    if not authorize(args, env):
        print(f"Planned {args.action}: {env.name} ({env.provider})")
        return 0
    if vm.state(env) != "running":
        raise LabError("Guest must already be running for provisioning or provider control")
    apply_script(vm, env, script, timeout)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (LabError, OSError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        raise SystemExit(2) from None
