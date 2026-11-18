"""Local libvirt snapshots and QEMU guest agent. Never bypass locks or force reverts."""
from __future__ import annotations

import base64
import json
import subprocess
import time
import uuid
import xml.etree.ElementTree as ET
from dataclasses import dataclass

from .config import Environment, LabError

OUTPUT_LIMIT = 16000


@dataclass(frozen=True)
class ExecResult:
    code: int
    stdout: str
    stderr: str


class VM:
    def __init__(self, uri: str):
        self.uri = uri

    def virsh(self, *args: str, timeout: float = 60) -> str:
        try:
            proc = subprocess.run(
                ["virsh", "-c", self.uri, *args],
                capture_output=True, text=True, timeout=timeout, check=False,
            )
        except (OSError, subprocess.TimeoutExpired) as exc:
            raise LabError(f"virsh {args[0]} unavailable: {exc}") from exc
        if proc.returncode:
            raise LabError(f"virsh {args[0]} failed: {proc.stderr[:700].strip()}")
        return proc.stdout.strip()

    def state(self, env: Environment) -> str:
        return self.virsh("domstate", env.domain).strip().lower()

    def preflight_disposable(self, env: Environment) -> tuple[str, str]:
        """Reject unexpected domain/storage/firmware topology before snapshot mutation."""
        if not env.domain.startswith("ace-lab-"):
            raise LabError("VM must have a dedicated ace-lab-* domain name")
        try:
            root = ET.fromstring(self.virsh("dumpxml", "--inactive", env.domain))
        except ET.ParseError as exc:
            raise LabError("Invalid libvirt domain XML") from exc
        actual_uuid = self.virsh("domuuid", env.domain)
        try:
            uuid.UUID(actual_uuid)
        except ValueError as exc:
            raise LabError("Invalid libvirt domain UUID") from exc
        if root.findtext("name") != env.domain or root.findtext("uuid") != actual_uuid:
            raise LabError("Domain XML identity mismatch")
        if (root.find("./os/nvram") is not None or root.find("./devices/tpm") is not None
                or root.findall("./devices/filesystem") or root.findall("./devices/hostdev")):
            raise LabError("VM has uncaptured NVRAM/TPM/host resources; use full-VM snapshot backend")
        disks = [disk for disk in root.findall("./devices/disk")
                 if disk.get("device") == "disk"]
        if len(disks) != 1:
            raise LabError("Only single QCOW2 disk disposable VMs are supported")
        disk = disks[0]
        driver = disk.find("driver")
        source = disk.find("source")
        target = disk.find("target")
        if (disk.get("type") != "file" or driver is None or driver.get("type") != "qcow2"
                or source is None or not source.get("file")
                or target is None or not target.get("dev")):
            raise LabError("Only local file-backed QCOW2 snapshots are supported")
        return actual_uuid, target.get("dev")

    def preflight_snapshot(self, env: Environment, domain_uuid: str, disk_target: str) -> None:
        try:
            root = ET.fromstring(self.virsh("snapshot-dumpxml", env.domain, env.snapshot))
        except ET.ParseError as exc:
            raise LabError("Invalid libvirt snapshot XML") from exc
        if (root.findtext("name") != env.snapshot or root.findtext("state") != "shutoff"
                or root.findtext("./domain/uuid") != domain_uuid):
            raise LabError("Snapshot identity or offline state does not match selected domain")
        memory = root.find("memory")
        if memory is not None and memory.get("snapshot") != "no":
            raise LabError("Refusing snapshot with live/memory state")
        disks = root.findall("./disks/disk")
        internal = [d for d in disks if d.get("snapshot") == "internal"]
        if (len(internal) != 1 or internal[0].get("name") != disk_target
                or any(d.get("snapshot") not in {"no", "internal"} for d in disks)):
            raise LabError("Snapshot must cover exactly the single QCOW2 disk internally")

    def snapshot_exists(self, env: Environment) -> bool:
        return env.snapshot in self.virsh("snapshot-list", env.domain, "--name").splitlines()

    def require_off(self, env: Environment) -> None:
        if self.state(env) != "shut off":
            raise LabError("Snapshot create/restore needs a shut-off VM; never force-stop a locked engine")

    def create_snapshot(self, env: Environment) -> None:
        self.require_off(env)
        domain_uuid, disk_target = self.preflight_disposable(env)
        if self.snapshot_exists(env):
            raise LabError(f"Snapshot already exists: {env.snapshot}")
        # Internal libvirt snapshot, for offline qcow2-backed disposable domains.
        self.virsh("snapshot-create-as", "--domain", env.domain, "--name", env.snapshot,
                   "--description", "Disposable Ace Blocker network lab environment", "--atomic", timeout=180)
        self.preflight_snapshot(env, domain_uuid, disk_target)

    def restore_snapshot(self, env: Environment) -> None:
        self.require_off(env)
        domain_uuid, disk_target = self.preflight_disposable(env)
        if not self.snapshot_exists(env):
            raise LabError(f"Missing snapshot {env.snapshot}")
        self.preflight_snapshot(env, domain_uuid, disk_target)
        # --force deliberately omitted. Incompatible storage snapshots fail closed.
        self.virsh("snapshot-revert", env.domain, env.snapshot, timeout=180)

    def start(self, env: Environment) -> None:
        state = self.state(env)
        if state == "shut off":
            self.virsh("start", env.domain)
        elif state != "running":
            raise LabError(f"VM is not startable: {state}")

    def agent(self, env: Environment, operation: str, arguments: dict | None = None) -> dict:
        payload = {"execute": operation}
        if arguments is not None:
            payload["arguments"] = arguments
        result = self.virsh("qemu-agent-command", env.domain, "--timeout", "30",
                            json.dumps(payload), timeout=45)
        try:
            parsed = json.loads(result)
            if "error" in parsed:
                raise LabError(f"Guest agent {operation} error: {str(parsed['error'])[:500]}")
            return parsed["return"]
        except (TypeError, ValueError, KeyError) as exc:
            raise LabError("Invalid guest agent response") from exc

    def wait_agent(self, env: Environment, seconds: float = 90) -> None:
        limit = time.monotonic() + seconds
        while time.monotonic() < limit:
            try:
                self.agent(env, "guest-ping")
                return
            except LabError:
                time.sleep(2)
        raise LabError(f"No QEMU guest agent on {env.domain} after {seconds}s")

    def exec(self, env: Environment, code: str, seconds: float = 120) -> ExecResult:
        if env.distribution == "windows":
            path = "powershell.exe"
            args = ["-NoProfile", "-NonInteractive", "-EncodedCommand",
                    base64.b64encode(code.encode("utf-16le")).decode("ascii")]
        else:
            path = "/bin/bash"
            args = ["-c", code]
        pid = self.agent(env, "guest-exec", {
            "path": path, "arg": args, "capture-output": True,
        }).get("pid")
        if not isinstance(pid, int):
            raise LabError("Guest exec returned no PID")
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            time.sleep(0.5)
            status = self.agent(env, "guest-exec-status", {"pid": pid})
            if status.get("exited"):
                def decode(field: str) -> str:
                    return base64.b64decode(status.get(field, ""), validate=True).decode(
                        "utf-8", "replace"
                    )[:OUTPUT_LIMIT]
                try:
                    return ExecResult(int(status.get("exitcode", 1)),
                                      decode("out-data"), decode("err-data"))
                except (ValueError, TypeError) as exc:
                    raise LabError("Invalid guest exec status") from exc
        raise LabError(f"Guest process timeout after {seconds}s (process may remain running)")
