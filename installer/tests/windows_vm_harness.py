#!/usr/bin/env python3
r"""Run repeatable Windows 11 checks over libvirt's QEMU guest agent.

Prerequisite: an owned disposable Windows VM with qemu-ga running.
This tool does not weaken service ACLs, locks, or installer gates.
Usage:
  python3 installer/tests/windows_vm_harness.py smoke
  python3 installer/tests/windows_vm_harness.py ps --script installer/tests/win11_smoke.ps1
  python3 installer/tests/windows_vm_harness.py install --msi 'C:\Windows\Temp\AceBlocker-0.1.0.msi'
  python3 installer/tests/windows_vm_harness.py uninstall --msi 'C:\Windows\Temp\AceBlocker-0.1.0.msi'
"""
import argparse
import base64
import json
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path

DEFAULT_VM = "novagpt-install-win11-ace-test"


class GuestError(RuntimeError):
    pass


@dataclass(frozen=True)
class ExecResult:
    exitcode: int
    stdout: str
    stderr: str


class Guest:
    def __init__(self, vm: str, connect: str = "qemu:///session"):
        self.vm = vm
        self.connect = connect

    def request(self, execute: str, arguments: dict | None = None) -> dict:
        request = {"execute": execute}
        if arguments is not None:
            request["arguments"] = arguments
        result = subprocess.run(
            ["virsh", "-c", self.connect, "qemu-agent-command", self.vm, "--timeout", "30", json.dumps(request)],
            check=False,
            text=True,
            capture_output=True,
            timeout=40,
        )
        if result.returncode:
            raise GuestError(result.stderr.strip()[:800])
        payload = json.loads(result.stdout)
        if "error" in payload:
            raise GuestError(str(payload["error"])[:800])
        return payload["return"]

    def powershell(self, code: str, timeout: float = 120) -> ExecResult:
        encoded = base64.b64encode(code.encode("utf-16le")).decode("ascii")
        info = self.request(
            "guest-exec",
            {
                "path": "powershell.exe",
                "arg": ["-NoLogo", "-NoProfile", "-NonInteractive", "-EncodedCommand", encoded],
                "capture-output": True,
            },
        )
        pid = info["pid"]
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            time.sleep(0.5)
            status = self.request("guest-exec-status", {"pid": pid})
            if status.get("exited", False):
                def decode(name: str) -> str:
                    return base64.b64decode(status.get(name, "")).decode("utf-8", "replace")
                return ExecResult(status.get("exitcode", 1), decode("out-data"), decode("err-data"))
        raise GuestError(f"Guest process {pid} did not finish within {timeout}s; may still be running")

    def ping(self) -> None:
        self.request("guest-ping")


def quoted_ps(value: str) -> str:
    return "'" + value.replace("'", "''") + "'"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--vm", default=DEFAULT_VM, help="libvirt domain name")
    parser.add_argument("--connect", default="qemu:///session")
    parser.add_argument("--timeout", type=int, default=180)
    sub = parser.add_subparsers(dest="action", required=True)
    sub.add_parser("ping")
    sub.add_parser("smoke")
    ps = sub.add_parser("ps")
    inp = ps.add_mutually_exclusive_group(required=True)
    inp.add_argument("--script", type=Path)
    inp.add_argument("--command")
    for action in ("install", "uninstall"):
        install = sub.add_parser(action)
        install.add_argument("--msi", required=True, help="Windows guest-local MSI path")
        install.add_argument("--log", default=f"C:\\Windows\\Temp\\AceBlocker-{action}.log")

    args = parser.parse_args()
    guest = Guest(args.vm, args.connect)
    try:
        if args.action == "ping":
            guest.ping()
            print("OK: QEMU guest agent responsive")
            return 0
        if args.action == "smoke":
            source = Path(__file__).with_name("win11_smoke.ps1").read_text(encoding="utf-8")
        elif args.action == "ps":
            source = args.script.read_text(encoding="utf-8") if args.script else args.command
        else:
            verb = "/i" if args.action == "install" else "/x"
            # Execute the canonical MSI; never force service shutdown or bypass the uninstall lock check.
            source = (
                '$ErrorActionPreference = "Stop"; '
                f'$args = {quoted_ps(verb + " " + chr(34) + args.msi + chr(34) + " /qn /norestart /L*v " + chr(34) + args.log + chr(34))}; '
                '$p = Start-Process -FilePath "msiexec.exe" -ArgumentList $args -Wait -PassThru; '
                'Write-Output ("MSI_EXIT_CODE=" + $p.ExitCode); '
                'if ($p.ExitCode -ne 0 -and $p.ExitCode -ne 3010) { exit $p.ExitCode }'
            )
        result = guest.powershell(source, timeout=args.timeout)
        if result.stdout:
            print(result.stdout.rstrip())
        if result.stderr:
            print(result.stderr[:6000].rstrip(), file=sys.stderr)
        return result.exitcode
    except (GuestError, OSError, ValueError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
