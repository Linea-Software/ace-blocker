#!/usr/bin/env python3
"""Ace Blocker VPN/TOR lab using NovaGPT MCP exclusively (no legacy VM backend)."""

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
    from network_lab.config import (
        Case,
        Config,
        Environment,
        LabError,
        VMProfile,
        ident,
        load,
    )  # type: ignore[import-not-found]
    from network_lab.providers import (
        connect_script,
        disconnect_script,
        probe_script,
        provision_script,
    )  # type: ignore[import-not-found]
    from network_lab.vm import McpClient, VM  # type: ignore[import-not-found]
    from network_lab.wireguard_lab import setup as setup_wireguard  # type: ignore[import-not-found]
else:
    from .config import Case, Config, Environment, LabError, VMProfile, ident, load
    from .providers import (
        connect_script,
        disconnect_script,
        probe_script,
        provision_script,
    )
    from .vm import McpClient, VM
    from .wireguard_lab import setup as setup_wireguard


def git_revision() -> str:
    try:
        result = subprocess.run(
            [
                "git",
                "-C",
                str(Path(__file__).resolve().parents[3]),
                "rev-parse",
                "HEAD",
            ],
            capture_output=True,
            text=True,
            timeout=6,
            check=False,
        )
        return result.stdout.strip() if result.returncode == 0 else "unknown"
    except (OSError, subprocess.TimeoutExpired):
        return "unknown"


def artifact_directory(directory: Path) -> Path:
    if directory.is_symlink():
        raise LabError("Refusing symlink artifact path")
    if directory.exists():
        if not directory.is_dir() or directory.stat().st_mode & 0o077:
            raise LabError("Existing artifact directory must already be private (0700)")
    else:
        directory.mkdir(parents=True, exist_ok=False, mode=0o700)
    return directory


def save_binary(directory: Path, name: str, data: bytes) -> Path:
    artifact_directory(directory)
    if (
        not isinstance(name, str)
        or not name
        or "/" in name
        or "\\" in name
        or name in {".", ".."}
    ):
        raise LabError("Artifact name must be a safe basename")
    path = directory / name
    fd = os.open(
        path, os.O_CREAT | os.O_EXCL | os.O_WRONLY | getattr(os, "O_NOFOLLOW", 0), 0o600
    )
    with os.fdopen(fd, "wb") as out:
        out.write(data)
    return path


def save_report(directory: Path, data: dict) -> Path:
    stamp = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
    return save_binary(
        directory,
        f"{data['environment']}-{stamp}.json",
        json.dumps(data, indent=2, ensure_ascii=False).encode("utf-8"),
    )


def authorize(
    args: argparse.Namespace, profile: VMProfile, *, destructive: bool = False
) -> bool:
    if not args.execute:
        print(
            f"PLAN ONLY: NovaGPT profile {profile.name}, image={profile.image}, id={profile.id or '<not configured>'}"
        )
        if destructive:
            print("Snapshot restore is destructive and requires --accept-data-loss")
        return False
    if not profile.id:
        raise LabError(
            f"VM profile {profile.name} has no id. Run fork-create and set it in lab.toml"
        )
    if args.confirm_vm_id != profile.id:
        raise LabError(
            "Refusing guest mutation: --confirm-vm-id must match the exact TOML VM id"
        )
    if destructive and not args.accept_data_loss:
        raise LabError("Restoring a NovaGPT snapshot requires --accept-data-loss")
    return True


def list_plan(cfg: Config) -> None:
    print("NovaGPT VM profiles:")
    for profile in cfg.vms.values():
        print(
            f"  {profile.name}: image={profile.image}, id={profile.id or 'UNCONFIGURED'}, networks={list(profile.networks)}"
        )
    print("Environment / provider / snapshot / cases:")
    for env in cfg.environments.values():
        count = sum(env.name in case.environments for case in cfg.cases.values())
        print(f"  {env.name} / {env.provider} / {env.snapshot} / {count} cases")
    print(
        f"Configured: {len(cfg.vms)} VM profiles, {len(cfg.environments)} environments, {len(cfg.cases)} cases"
    )


def report_cases(
    vm: VM,
    profile: VMProfile,
    env: Environment,
    cases: list[Case],
    cfg: Config,
    require_coverage: bool,
) -> int:
    rows: list[dict] = []
    for case in cases:
        row = {"case": case.id, "expected": case.expected, "enforcement_proven": False}
        script = probe_script(env, case.kind)
        if script is None:
            row.update(
                status="NOT_RUN",
                note="Requires supported browser and controlled origin-server evidence",
            )
        else:
            try:
                result = vm.exec(profile.id, env, script, seconds=90)
                row.update(
                    status="INFRA_OK"
                    if result.code == 0 and not result.truncated
                    else "INFRA_FAILED",
                    exit_code=result.code,
                    stdout=result.stdout,
                    stderr=result.stderr,
                    truncated=result.truncated,
                    note="Infrastructure only: this is not an anti-bypass browser PASS",
                )
            except LabError:
                row.update(
                    status="BLOCKED",
                    reason="NovaGPT guest execution unavailable or rejected",
                )
        rows.append(row)
        print(f"{case.id}: {row['status']}")
    data = {
        "environment": env.name,
        "vm_profile": profile.name,
        "vm_id": profile.id,
        "image": profile.image,
        "networks": list(profile.networks),
        "snapshot": env.snapshot,
        "provider": env.provider,
        "distribution": env.distribution,
        "git_commit": git_revision(),
        "collected_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "results": rows,
    }
    location = save_report(cfg.artifacts, data)
    checked = sum(r["status"] == "INFRA_OK" for r in rows)
    not_run = sum(r["status"] == "NOT_RUN" for r in rows)
    print(
        f"NovaGPT infrastructure OK: {checked}; NOT_RUN: {not_run}; browser enforcement PASS: 0"
    )
    print(f"Private evidence: {location}")
    if any(r["status"] in {"BLOCKED", "INFRA_FAILED"} for r in rows):
        return 2
    if checked == 0 or (require_coverage and not_run):
        return 3
    return 0


def _profile(cfg: Config, name: str) -> VMProfile:
    if name not in cfg.vms:
        raise LabError(f"Unknown NovaGPT VM profile: {name}")
    return cfg.vms[name]


def _environment(cfg: Config, name: str) -> tuple[Environment, VMProfile]:
    if name not in cfg.environments:
        raise LabError(f"Unknown environment: {name}")
    env = cfg.environments[name]
    return env, _profile(cfg, env.vm)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--config", type=Path, default=Path(__file__).with_name("lab.toml")
    )
    sub = parser.add_subparsers(dest="action", required=True)
    sub.add_parser("list")
    inventory = sub.add_parser(
        "inventory", help="Read approved NovaGPT images and ISO inventory"
    )
    inventory.add_argument(
        "--iso", action="store_true", help="Also list approved installer ISO names"
    )
    create = sub.add_parser(
        "fork-create", help="Create an owned fork; copy the returned ID into TOML"
    )
    create.add_argument("--vm", required=True, help="VM profile key from [vms.NAME]")
    create.add_argument(
        "--installer-iso", help="Approved NovaGPT ISO name, never a host path"
    )
    create.add_argument("--execute", action="store_true")
    create.add_argument("--confirm-image", help="Exact human-approved image name")
    for action in (
        "doctor",
        "fork-start",
        "fork-stop",
        "provision",
        "connect",
        "disconnect",
        "wireguard-setup",
        "snapshot-save",
        "snapshot-restore",
        "snapshot-promote",
        "run",
        "screenshot",
        "keys",
        "pointer",
        "upload",
        "download",
    ):
        arg = sub.add_parser(action)
        arg.add_argument("--env", required=True)
        if action not in {"doctor"}:
            arg.add_argument("--execute", action="store_true")
            arg.add_argument("--confirm-vm-id", help="Exact NovaGPT vm-... ID")
        if action in {"snapshot-restore", "run"}:
            arg.add_argument("--accept-data-loss", action="store_true")
        if action == "snapshot-promote":
            arg.add_argument(
                "--image", required=True, help="New approved immutable image name"
            )
        if action == "wireguard-setup":
            arg.add_argument(
                "--confirm-gateway-vm-id", help="Exact NovaGPT gateway VM id"
            )
        if action == "run":
            arg.add_argument("--case", action="append", default=[])
            arg.add_argument("--restore", action="store_true")
            arg.add_argument("--require-coverage", action="store_true")
        if action == "keys":
            arg.add_argument(
                "--key", action="append", required=True, help="NovaGPT QMP keycode"
            )
        if action == "pointer":
            arg.add_argument("--x", type=int, required=True)
            arg.add_argument("--y", type=int, required=True)
            arg.add_argument("--button", choices=["left", "right", "middle"])
        if action in {"upload", "download"}:
            arg.add_argument(
                "--guest-path", required=True, help="Absolute guest file path"
            )
            arg.add_argument(
                "--file",
                type=Path,
                required=True,
                help="Explicitly selected local path",
            )
    args = parser.parse_args()
    cfg = load(args.config)
    if args.action == "list":
        list_plan(cfg)
        return 0
    vm = VM(McpClient(cfg.url, cfg.token_env))

    if args.action == "inventory":
        inventory = vm.images()
        print("Approved NovaGPT images:")
        for row in inventory.get("images", []):
            print(" ", row.get("name", "<unnamed>"))
        if args.iso:
            print("Approved installer media:")
            for row in vm.isos().get("isos", []):
                print(" ", row.get("name", "<unnamed>"))
        return 0

    if args.action == "fork-create":
        profile = _profile(cfg, args.vm)
        if profile.id:
            raise LabError(
                "Refusing to create another fork for a VM profile with an existing id"
            )
        if not args.execute:
            print(
                f"PLAN ONLY: NovaGPT fork-create image={profile.image}, networks={list(profile.networks)}"
            )
            return 0
        if args.confirm_image != profile.image:
            raise LabError("--confirm-image must equal the approved image in TOML")
        approved = {item.get("name") for item in vm.images().get("images", [])}
        if profile.image not in approved:
            raise LabError(
                f"NovaGPT image {profile.image} is not in the approved VM registry"
            )
        ident(args.installer_iso, "installer ISO") if args.installer_iso else None
        if "connect:" in " ".join(profile.networks):
            if cfg.wireguard is None:
                raise LabError("Private QEMU client requires a configured gateway")
            gateway = _profile(cfg, cfg.wireguard.gateway_vm)
            if not gateway.id or vm.state(gateway.id) != "running":
                raise LabError(
                    "Start the registered gateway fork before creating a point-to-point client"
                )
            vm.require_profile(gateway)
        created = vm.create(profile, installer_iso=args.installer_iso)
        print(
            f'NovaGPT created owned fork {created}. Set [vms.{profile.name}].id = "{created}" in TOML.'
        )
        return 0

    env, profile = _environment(cfg, args.env)
    if args.action == "doctor":
        if not profile.id:
            print(
                f"{env.name}: UNCONFIGURED (set [vms.{profile.name}].id after fork-create)"
            )
            return 2
        instance = vm.require_profile(profile)
        print(
            f"NovaGPT VM {profile.id}: state={instance['state']}, image={instance['image']}"
        )
        print(
            f"NovaGPT snapshots: {', '.join(sorted(vm.snapshot_list(profile.id))) or '(none)'}"
        )
        if instance["state"] == "running":
            if env.distribution == "windows" and instance.get("backend") == "hyperv":
                print(
                    "Hyper-V Windows guest: PowerShell Direct requires operation-scoped credentials"
                )
            else:
                try:
                    vm.guest_ready(profile.id)
                    print("QEMU guest agent: responding")
                except LabError:
                    print("QEMU guest agent: unavailable")
                    return 2
        return 0

    destructive = args.action == "snapshot-restore" or (
        args.action == "run" and args.restore
    )
    if not authorize(args, profile, destructive=destructive):
        if args.action == "run":
            for case in cfg.cases.values():
                if env.name in case.environments and (
                    not args.case or case.id in args.case
                ):
                    print(f"  {case.id}: {case.kind} / {case.expected}")
        return 0

    vm.require_profile(profile)
    if args.action == "fork-start":
        vm.start(profile.id)
        print(f"NovaGPT owned fork {profile.id}: running")
        return 0
    if args.action == "fork-stop":
        vm.stop(profile.id)
        print(f"NovaGPT owned fork {profile.id}: stopped cleanly")
        return 0
    if args.action == "wireguard-setup":
        if not cfg.wireguard:
            raise LabError("No [wireguard_lab] in TOML")
        gateway = _profile(cfg, cfg.wireguard.gateway_vm)
        if not gateway.id or args.confirm_gateway_vm_id != gateway.id:
            raise LabError("WireGuard gateway requires exact --confirm-gateway-vm-id")
        vm.require_profile(gateway)
        print(json.dumps(setup_wireguard(vm, cfg, env), indent=2))
        return 0

    if args.action == "snapshot-save":
        vm.snapshot_create(profile.id, env.snapshot)
        print(f"Saved NovaGPT snapshot {env.snapshot} in fork {profile.id}")
        return 0
    if args.action == "snapshot-restore":
        vm.snapshot_restore(profile.id, env.snapshot)
        print(f"Restored NovaGPT snapshot {env.snapshot} in fork {profile.id}")
        return 0
    if args.action == "snapshot-promote":
        ident(args.image, "approved promoted image")
        promoted = vm.snapshot_promote(profile.id, env.snapshot, args.image)
        print(f"Promoted NovaGPT snapshot to approved image: {promoted.get('name')}")
        return 0
    if args.action == "run":
        cases = [
            case
            for case in cfg.cases.values()
            if env.name in case.environments and (not args.case or case.id in args.case)
        ]
        if args.case and set(args.case) - {case.id for case in cases}:
            raise LabError("Unknown or incompatible case ID")
        if not cases:
            raise LabError("No test cases selected")
        if args.restore:
            vm.snapshot_restore(profile.id, env.snapshot)
            vm.start(profile.id)
        elif vm.state(profile.id) != "running":
            raise LabError("VM must be running or --restore must be requested")
        if not (
            env.distribution == "windows"
            and vm.status(profile.id).get("backend") == "hyperv"
        ):
            vm.wait_guest(profile.id, seconds=90)
        return report_cases(vm, profile, env, cases, cfg, args.require_coverage)

    if args.action in {"provision", "connect", "disconnect"}:
        script = {
            "provision": provision_script,
            "connect": connect_script,
            "disconnect": disconnect_script,
        }[args.action](env)
        if vm.state(profile.id) != "running":
            raise LabError("NovaGPT fork must be running for guest operations")
        result = vm.exec(
            profile.id, env, script, seconds=300 if args.action == "provision" else 120
        )
        if result.code or result.truncated:
            raise LabError(f"Guest operation {args.action} failed (exit={result.code})")
        print(f"NovaGPT guest {args.action}: succeeded on {profile.id}")
        return 0

    if args.action == "screenshot":
        stamp = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
        path = save_binary(
            cfg.artifacts, f"{env.name}-{stamp}.png", vm.screenshot(profile.id)
        )
        print(f"Private guest screenshot: {path}")
        return 0
    if args.action == "keys":
        vm.keys(profile.id, args.key)
        print("NovaGPT guest keyboard event sent")
        return 0
    if args.action == "pointer":
        vm.pointer(profile.id, args.x, args.y, args.button)
        print("NovaGPT guest pointer event sent")
        return 0
    if args.action == "upload":
        if args.file.is_symlink() or not args.file.is_file():
            raise LabError("Upload requires a local regular file, not a symlink")
        if args.file.stat().st_size > 1024 * 1024:
            raise LabError("NovaGPT guest upload limit is 1 MiB")
        vm.upload(profile.id, args.guest_path, args.file.read_bytes(), env)
        print("NovaGPT guest upload complete; no data printed")
        return 0
    if args.action == "download":
        raw = vm.download(profile.id, args.guest_path, env)
        if args.file.is_absolute() or args.file.parent != Path("."):
            raise LabError(
                "Download destination must be a basename inside the private lab artifacts directory"
            )
        path = save_binary(cfg.artifacts, args.file.name, raw)
        print(f"Private guest download stored at {path}")
        return 0

    raise LabError("Unhandled lab command")


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (LabError, OSError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        raise SystemExit(2) from None
