"""NovaGPT MCP VM backend. No libvirt, virsh, host-QEMU, or filesystem VM access.

All VM actions use the authenticated NovaGPT Streamable HTTP MCP endpoint.
Only loopback URLs are accepted by config.py; the bearer token comes from the
environment and is never persisted or printed.
"""

from __future__ import annotations

import base64
import json
import os
import time
import urllib.error
import urllib.request
from dataclasses import dataclass
from typing import Any

from .config import Environment, LabError, VMProfile, vm_id

MAX_MCP_RESPONSE = 9 * 1024 * 1024


class NoRedirect(urllib.request.HTTPRedirectHandler):
    """A loopback MCP endpoint must never redirect token-bearing requests."""

    def redirect_request(self, req, fp, code, msg, headers, newurl):
        raise LabError("NovaGPT MCP unexpectedly redirected an authenticated request")


class McpClient:
    def __init__(self, url: str, token_env: str, *, timeout: float = 45.0):
        self.url = url
        self.token_env = token_env
        self.timeout = timeout
        self.session: str | None = None
        self.request_id = 0
        self.ready = False
        # Never allow host HTTP_PROXY settings or redirects to carry secrets
        # away from the explicitly approved local MCP endpoint.
        self.opener = urllib.request.build_opener(
            urllib.request.ProxyHandler({}), NoRedirect()
        )

    @staticmethod
    def _read_sse(reply, expected_id: int | None) -> dict:
        """Read complete SSE frames until the requested JSON-RPC id arrives.

        A Streamable HTTP server may keep its POST response stream open after
        sending the matching event; waiting for EOF would hang unnecessarily.
        """
        total = 0
        data: list[str] = []
        while True:
            raw = reply.readline()
            if not raw:
                raise LabError("NovaGPT MCP SSE ended without the matching response")
            total += len(raw)
            if total > MAX_MCP_RESPONSE:
                raise LabError("NovaGPT MCP SSE response exceeds safe size limit")
            try:
                line = raw.decode("utf-8").rstrip("\r\n")
            except UnicodeDecodeError:
                raise LabError("NovaGPT MCP SSE has invalid UTF-8") from None
            if line.startswith("data:"):
                data.append(line[5:].lstrip(" "))
            elif not line:
                if data:
                    try:
                        value = json.loads("\n".join(data))
                    except (ValueError, TypeError):
                        raise LabError(
                            "NovaGPT MCP SSE contains invalid JSON"
                        ) from None
                    data.clear()
                    if isinstance(value, dict) and value.get("id") == expected_id:
                        return value

    def _exchange(
        self, payload: dict, *, notification: bool = False, timeout: float | None = None
    ) -> dict:
        token = os.environ.get(self.token_env)
        if not token:
            raise LabError(
                f"MCP authentication is missing: set {self.token_env} in this process"
            )
        headers = {
            "Authorization": f"Bearer {token}",
            "Content-Type": "application/json",
            "Accept": "application/json, text/event-stream",
            "MCP-Protocol-Version": "2025-11-25",
        }
        if self.session:
            headers["Mcp-Session-Id"] = self.session
        data = json.dumps(payload, separators=(",", ":")).encode("utf-8")
        req = urllib.request.Request(
            self.url, data=data, headers=headers, method="POST"
        )
        try:
            with self.opener.open(req, timeout=timeout or self.timeout) as reply:
                if session := reply.headers.get("Mcp-Session-Id"):
                    self.session = session
                content_type = reply.headers.get("Content-Type", "")
                if "text/event-stream" in content_type:
                    return self._read_sse(reply, payload.get("id"))
                body = reply.read(MAX_MCP_RESPONSE + 1)
        except urllib.error.HTTPError as exc:
            raise LabError(
                f"NovaGPT MCP HTTP status {exc.code}; check token, URL and dev.py server"
            ) from None
        except (urllib.error.URLError, TimeoutError, OSError) as exc:
            raise LabError(
                f"NovaGPT MCP unavailable over loopback: {type(exc).__name__}"
            ) from None
        if len(body) > MAX_MCP_RESPONSE:
            raise LabError("NovaGPT MCP response exceeds safe size limit")
        if notification and not body:
            return {}
        try:
            return json.loads(body)
        except (UnicodeDecodeError, ValueError, TypeError, AttributeError) as exc:
            raise LabError(
                f"Unexpected NovaGPT MCP response format: {type(exc).__name__}"
            ) from None

    def _request(
        self, method: str, params: dict | None = None, *, timeout: float | None = None
    ) -> Any:
        self.request_id += 1
        rpc_id = self.request_id
        payload = {"jsonrpc": "2.0", "id": rpc_id, "method": method}
        if params is not None:
            payload["params"] = params
        response = self._exchange(payload, timeout=timeout)
        if not isinstance(response, dict) or response.get("id") != rpc_id:
            raise LabError("NovaGPT MCP JSON-RPC id mismatch")
        if "error" in response:
            error = response["error"]
            code = (
                error.get("code", "unknown") if isinstance(error, dict) else "unknown"
            )
            raise LabError(f"NovaGPT MCP {method} failed (RPC code {code})")
        if "result" not in response:
            raise LabError("NovaGPT MCP response lacks result")
        return response["result"]

    def connect(self) -> None:
        if self.ready:
            return
        info = self._request(
            "initialize",
            {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": {"name": "ace-blocker-network-lab", "version": "1"},
            },
        )
        if not isinstance(info, dict) or not info.get("protocolVersion"):
            raise LabError("NovaGPT MCP initialize did not negotiate a protocol")
        self._exchange(
            {"jsonrpc": "2.0", "method": "notifications/initialized"}, notification=True
        )
        self.ready = True

    def tool(self, name: str, arguments: dict) -> dict:
        self.connect()
        timeout = self.timeout
        if name == "vm_exec":
            timeout = max(timeout, arguments.get("timeout_ms", 30_000) / 1000 + 30)
        elif name in {"vm_upload", "vm_download"}:
            timeout = max(timeout, 150)
        elif name == "vm_snapshot" and arguments.get("action") == "promote":
            timeout = max(timeout, 940)
        elif name == "vm" and arguments.get("action") == "create":
            timeout = max(timeout, 360)
        result = self._request(
            "tools/call", {"name": name, "arguments": arguments}, timeout=timeout
        )
        if not isinstance(result, dict):
            raise LabError(f"NovaGPT {name}: invalid tool envelope")
        if result.get("isError") or result.get("is_error"):
            # Never include raw returned error text or secrets in logs.
            raise LabError(f"NovaGPT {name} rejected the requested operation")
        content = result.get("structuredContent", result.get("structured_content"))
        if not isinstance(content, dict):
            text_blocks = result.get("content", [])
            try:
                content = json.loads(
                    next(x["text"] for x in text_blocks if x.get("type") == "text")
                )
            except (StopIteration, KeyError, ValueError, TypeError):
                raise LabError(f"NovaGPT {name}: missing structured result") from None
        if "error" in content:
            raise LabError(f"NovaGPT {name} returned an error")
        return content


@dataclass(frozen=True)
class ExecResult:
    code: int
    stdout: str
    stderr: str
    truncated: bool = False


class VM:
    """Typed facade over NovaGPT 0.5.x VM tools."""

    def __init__(self, client: McpClient):
        self.client = client

    def images(self) -> dict:
        return self.client.tool("vm_image", {})

    def isos(self) -> dict:
        return self.client.tool("vm_iso", {})

    def create(self, profile: VMProfile, *, installer_iso: str | None = None) -> str:
        args = {
            "action": "create",
            "image": profile.image,
            "cpus": profile.cpus,
            "memory_mb": profile.memory_mb,
            "networks": list(profile.networks),
            "display": profile.display,
            "guest_agent": profile.guest_agent,
        }
        if installer_iso:
            args["installer_iso"] = installer_iso
        payload = self.client.tool("vm", args)
        instance = payload.get("instance", {})
        if not isinstance(instance, dict) or instance.get("image") != profile.image:
            raise LabError("NovaGPT fork creation returned a mismatched approved image")
        value = instance.get("id") if isinstance(instance, dict) else None
        return vm_id(value)

    def status(self, vm: str) -> dict:
        value = self.client.tool("vm", {"action": "status", "id": vm_id(vm)})
        instance = value.get("instance")
        if not isinstance(instance, dict) or instance.get("id") != vm:
            raise LabError("NovaGPT VM status returned a mismatched instance")
        return instance

    def require_profile(self, profile: VMProfile) -> dict:
        if not profile.id:
            raise LabError(
                f"VM profile {profile.name} has no registered NovaGPT fork id"
            )
        instance = self.status(profile.id)
        if instance.get("image") != profile.image:
            raise LabError(f"NovaGPT VM image mismatch for profile {profile.name}")
        return instance

    def state(self, vm: str) -> str:
        state = self.status(vm).get("state")
        if state not in {"starting", "running", "stopped", "failed"}:
            raise LabError("NovaGPT VM returned unknown lifecycle state")
        return state

    def _wait_state(self, vm: str, expected: str, seconds: float) -> None:
        deadline = time.monotonic() + seconds
        while True:
            current = self.state(vm)
            if current == expected:
                return
            if current == "failed" or time.monotonic() >= deadline:
                raise LabError(f"NovaGPT fork did not reach {expected} state")
            time.sleep(min(1, max(0, deadline - time.monotonic())))

    def start(self, vm: str) -> None:
        current = self.state(vm)
        if current == "stopped":
            self.client.tool("vm", {"action": "start", "id": vm_id(vm)})
        elif current != "running":
            raise LabError("NovaGPT VM must be stopped or running to start")
        self._wait_state(vm, "running", seconds=90)

    def stop(self, vm: str) -> None:
        current = self.state(vm)
        if current == "running":
            self.client.tool("vm", {"action": "stop", "id": vm_id(vm), "force": False})
        elif current != "stopped":
            raise LabError("Cannot gracefully stop VM in its current state")
        self._wait_state(vm, "stopped", seconds=30)

    def guest_ready(self, vm: str) -> None:
        response = self.client.tool("vm_guest_status", {"id": vm_id(vm)})
        if response.get("available") is not True:
            raise LabError("NovaGPT QEMU Guest Agent is not ready inside the guest")

    def wait_guest(self, vm: str, *, seconds: float = 90) -> None:
        deadline = time.monotonic() + seconds
        while True:
            # An unavailable guest agent is an ordinary boot-time condition.
            # A failed/unauthorized MCP tool call remains fatal immediately.
            response = self.client.tool("vm_guest_status", {"id": vm_id(vm)})
            if response.get("available") is True:
                return
            if time.monotonic() >= deadline:
                raise LabError("NovaGPT Guest Agent did not become ready")
            time.sleep(min(1, max(0, deadline - time.monotonic())))

    def snapshot_list(self, vm: str) -> set[str]:
        response = self.client.tool("vm_snapshot", {"action": "list", "id": vm_id(vm)})
        snapshots = response.get("snapshots", [])
        if not isinstance(snapshots, list):
            raise LabError("NovaGPT snapshot list malformed")
        return {
            item["name"]
            for item in snapshots
            if isinstance(item, dict) and "name" in item
        }

    def snapshot_create(self, vm: str, name: str) -> None:
        if self.state(vm) != "stopped":
            raise LabError("NovaGPT snapshot creation requires a stopped VM")
        if name in self.snapshot_list(vm):
            raise LabError("Snapshot already exists; refusing overwrite")
        self.client.tool("vm_snapshot", {"action": "create", "id": vm, "name": name})

    def snapshot_restore(self, vm: str, name: str) -> None:
        if self.state(vm) != "stopped":
            raise LabError("NovaGPT snapshot restore requires a stopped VM")
        if name not in self.snapshot_list(vm):
            raise LabError("Requested NovaGPT environment snapshot does not exist")
        self.client.tool("vm_snapshot", {"action": "restore", "id": vm, "name": name})

    def snapshot_promote(self, vm: str, name: str, image: str) -> dict:
        if self.state(vm) != "stopped":
            raise LabError("Snapshot promotion requires a stopped VM")
        if name not in self.snapshot_list(vm):
            raise LabError("Snapshot to promote does not exist")
        return self.client.tool(
            "vm_snapshot", {"action": "promote", "id": vm, "name": name, "image": image}
        )

    def exec(
        self, vm: str, env: Environment, code: str, *, seconds: float = 120
    ) -> ExecResult:
        if not 0.1 <= seconds <= 300:
            raise LabError(
                "NovaGPT guest execution timeout must be between 0.1 and 300 seconds"
            )
        credential = self._credential(vm, env)
        # Hyper-V Windows uses credential-scoped PowerShell Direct, not QGA.
        # vm_guest_status intentionally cannot authenticate those guests.
        if not credential:
            self.wait_guest(vm, seconds=45)
        if env.distribution == "windows":
            program = r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe"
            args = [
                "-NoProfile",
                "-NonInteractive",
                "-EncodedCommand",
                base64.b64encode(code.encode("utf-16le")).decode("ascii"),
            ]
        else:
            program = "/bin/bash"
            args = ["-c", code]
        request = {
            "id": vm_id(vm),
            "program": program,
            "args": args,
            "timeout_ms": max(100, min(300_000, round(seconds * 1000))),
        }
        request.update(credential)
        value = self.client.tool("vm_exec", request)
        return ExecResult(
            int(value.get("exit_code", -1)),
            str(value.get("stdout", ""))[:16000],
            str(value.get("stderr", ""))[:16000],
            bool(value.get("truncated", False)),
        )

    def screenshot(self, vm: str) -> bytes:
        response = self.client.tool("vm_desktop_screenshot", {"id": vm_id(vm)})
        encoded = response.get("png_base64") or response.get("data_base64")
        if not isinstance(encoded, str):
            raise LabError("NovaGPT returned no screenshot PNG")
        try:
            raw = base64.b64decode(encoded, validate=True)
        except (ValueError, TypeError):
            raise LabError("Invalid screenshot base64") from None
        if not raw.startswith(b"\x89PNG\r\n\x1a\n") or len(raw) > 6 * 1024 * 1024:
            raise LabError("Invalid or oversized NovaGPT screenshot")
        return raw

    def keys(self, vm: str, keys: list[str]) -> None:
        self.client.tool(
            "vm_desktop_input", {"id": vm_id(vm), "action": "keys", "keys": keys}
        )

    def pointer(self, vm: str, x: int, y: int, button: str | None = None) -> None:
        if not (0 <= x <= 32767 and 0 <= y <= 32767):
            raise LabError("Pointer coordinate out of range")
        args = {"id": vm_id(vm), "action": "pointer", "x": x, "y": y}
        if button:
            args["button"] = button
        self.client.tool("vm_desktop_input", args)

    def _credential(self, vm: str, env: Environment | None) -> dict:
        # NovaGPT Hyper-V Windows guest control requires operation-scoped
        # credentials. A Linux QEMU guest never receives these fields.
        if (
            env is None
            or env.distribution != "windows"
            or self.status(vm).get("backend") != "hyperv"
        ):
            return {}
        username = os.environ.get("NOVAGPT_GUEST_USERNAME")
        password = os.environ.get("NOVAGPT_GUEST_PASSWORD")
        if not username or not password:
            raise LabError(
                "Hyper-V Windows guest requires NOVAGPT_GUEST_USERNAME and NOVAGPT_GUEST_PASSWORD"
            )
        return {"credential": {"username": username, "password": password}}

    @staticmethod
    def _valid_guest_path(path: str) -> bool:
        import re

        if (
            not isinstance(path, str)
            or not path
            or len(path) > 4096
            or any(ord(ch) < 32 for ch in path)
        ):
            return False
        if ".." in path.replace("\\", "/").split("/"):
            return False
        return path.startswith("/") or bool(re.match(r"^[A-Za-z]:[\\\\/]", path))

    def upload(
        self, vm: str, guest_path: str, data: bytes, env: Environment | None = None
    ) -> dict:
        if not self._valid_guest_path(guest_path) or len(data) > 1024 * 1024:
            raise LabError(
                "Guest upload needs a safe absolute path and a file <= 1 MiB"
            )
        request = {
            "id": vm_id(vm),
            "guest_path": guest_path,
            "data_base64": base64.b64encode(data).decode("ascii"),
        }
        request.update(self._credential(vm, env))
        reply = self.client.tool("vm_upload", request)
        if reply.get("bytes") != len(data):
            raise LabError("NovaGPT guest upload reported a byte-count mismatch")
        return reply

    def download(
        self, vm: str, guest_path: str, env: Environment | None = None
    ) -> bytes:
        if not self._valid_guest_path(guest_path):
            raise LabError("Guest download path must be safe and absolute")
        request = {"id": vm_id(vm), "guest_path": guest_path}
        request.update(self._credential(vm, env))
        response = self.client.tool("vm_download", request)
        data = response.get("data_base64")
        if not isinstance(data, str):
            raise LabError("VM download missing file data")
        try:
            raw = base64.b64decode(data, validate=True)
        except ValueError:
            raise LabError("VM download invalid base64") from None
        if len(raw) > 1024 * 1024:
            raise LabError("VM download exceeds 1 MiB")
        return raw
