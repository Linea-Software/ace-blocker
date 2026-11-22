"""Offline tests for NovaGPT MCP-only network lab (never touch real VMs)."""

from __future__ import annotations

import argparse
import base64
import copy
import json
import os
import subprocess
import sys
import tempfile
import unittest
from dataclasses import replace
from pathlib import Path
from unittest.mock import Mock, patch

from installer.tests.network_lab.config import Environment, LabError, load, networks
from installer.tests.network_lab.providers import (
    connect_script,
    probe_script,
    provision_script,
)
from installer.tests.network_lab.run import (
    authorize,
    report_cases,
    save_report,
    save_binary,
)
from installer.tests.network_lab.vm import ExecResult, McpClient, VM
from installer.tests.network_lab.wireguard_lab import nic_mac, setup as setup_wireguard

ROOT = Path(__file__).resolve().parent
TOML = ROOT / "lab.toml"
ID = "vm-0123456789abcdef"
GW_ID = "vm-fedcba9876543210"


def temporary_toml(changed: str):
    class CaseFile:
        def __enter__(self):
            self.temp = tempfile.TemporaryDirectory()
            self.path = Path(self.temp.name) / "lab.toml"
            self.path.write_text(changed)
            return self.path

        def __exit__(self, *args):
            self.temp.cleanup()

    return CaseFile()


class ConfigTests(unittest.TestCase):
    def test_matrix(self):
        cfg = load(TOML)
        self.assertEqual(
            (len(cfg.vms), len(cfg.environments), len(cfg.cases)), (8, 32, 28)
        )
        self.assertEqual(cfg.vms["ubuntu-wg"].networks, ("nat", "connect:ace-ubuntu"))
        self.assertIn("listen:ace-cachyos", cfg.vms["gateway"].networks)
        self.assertEqual(cfg.wireguard.peers["cachyos"].port, 51823)
        self.assertEqual(cfg.environments["arch-wireguard"].vm, "arch-wg")
        self.assertIn("I2P-01", cfg.cases)
        self.assertIn("TOR-03", cfg.cases)

    def test_reject_bad_provider(self):
        with temporary_toml(
            TOML.read_text().replace('provider = "tor"', 'provider = "unknown"', 1)
        ) as file:
            with self.assertRaises(LabError):
                load(file)

    def test_reject_bad_case_target(self):
        with temporary_toml(
            TOML.read_text().replace('"ubuntu-direct"', '"nonexistent"', 1)
        ) as file:
            with self.assertRaises(LabError):
                load(file)

    def test_reject_remote_and_credential_urls(self):
        original = TOML.read_text()
        for bad in [
            "http://example.com/mcp",
            "http://127.0.0.1:8787/mcp?novagpt_token=secret",
            "https://127.0.0.1:8787/mcp",
            "http://someone:secret@127.0.0.1/mcp",
        ]:
            with (
                self.subTest(url=bad),
                temporary_toml(
                    original.replace(
                        'mcp_url = "http://127.0.0.1:8787/mcp"', f'mcp_url = "{bad}"'
                    )
                ) as file,
            ):
                with self.assertRaises(LabError):
                    load(file)

    def test_invalid_vm_id(self):
        bad = TOML.read_text().replace('id = ""', 'id = "novagpt-install-win11"', 1)
        with temporary_toml(bad) as file:
            with self.assertRaises(LabError):
                load(file)

    def test_no_unsupported_host_network(self):
        for value in [["nat", "none"], ["bridge:host"], ["tap0"], [], ["nat"] * 5]:
            with self.subTest(value=value), self.assertRaises(LabError):
                networks(value)
        self.assertEqual(networks(["none"]), ("none",))
        self.assertEqual(networks(["nat", "listen:test-1"]), ("nat", "listen:test-1"))

    def test_invalid_peer_or_network(self):
        original = TOML.read_text()
        for changed in [
            original.replace("port = 51822", "port = 51821"),
            original.replace(
                'gateway_link = "172.29.10.1/30"', 'gateway_link = "172.29.10.1/29"'
            ),
            original.replace(
                'client_link = "172.29.10.2/30"', 'client_link = "10.66.10.2/30"'
            ),
            original.replace(
                'networks = ["nat", "listen:ace-ubuntu", "listen:ace-arch", "listen:ace-cachyos"]',
                'networks = ["nat"]',
            ),
        ]:
            with temporary_toml(changed) as file:
                with self.assertRaises(LabError):
                    load(file)

    def test_reject_duplicate_owned_vm_ids(self):
        bad = TOML.read_text().replace('id = ""', f'id = "{ID}"', 2)
        with temporary_toml(bad) as file:
            with self.assertRaises(LabError):
                load(file)

    def test_artifact_path_traversal(self):
        with temporary_toml(
            TOML.read_text().replace('artifacts = ".artifacts"', 'artifacts = "../out"')
        ) as file:
            with self.assertRaises(LabError):
                load(file)


class ProviderTests(unittest.TestCase):
    def env(self, distro, provider):
        return Environment(
            f"{distro}-{provider}", distro, distro, provider, "test-ready"
        )

    def test_package_install(self):
        self.assertIn(
            "apt-get install", provision_script(self.env("ubuntu", "wireguard"))
        )
        self.assertIn(
            "pacman -S --needed", provision_script(self.env("arch", "openvpn"))
        )
        self.assertIn("openssh ", provision_script(self.env("cachyos", "socks")))
        self.assertNotIn(
            "openssh-client", provision_script(self.env("cachyos", "socks"))
        )

    def test_manually_authenticated_provider_is_never_installed(self):
        for name in ["proton", "windscribe", "tor-browser", "i2p"]:
            with self.subTest(provider=name):
                with self.assertRaises(LabError):
                    provision_script(self.env("ubuntu", name))

    def test_connection_prerequisites(self):
        self.assertIn(
            "wg-quick up ace-lab", connect_script(self.env("ubuntu", "wireguard"))
        )
        self.assertIn(
            "test -f /etc/openvpn/client/ace-lab.conf",
            connect_script(self.env("ubuntu", "openvpn")),
        )
        self.assertIn("9050", connect_script(self.env("ubuntu", "tor")))

    def test_probe_is_not_a_security_pass(self):
        self.assertIsNone(probe_script(self.env("ubuntu", "tor"), "browser"))
        self.assertIsNone(probe_script(self.env("ubuntu", "tor"), "manual"))
        self.assertIn(
            "wg show ace-lab latest-handshakes",
            probe_script(self.env("ubuntu", "wireguard"), "routing"),
        )
        self.assertIn(
            "systemctl is-active --quiet ace-engine",
            probe_script(self.env("ubuntu", "direct"), "baseline"),
        )


class McpProtocolTests(unittest.TestCase):
    def test_init_then_tool_with_bearer_and_session(self):
        sent = []

        class Response:
            headers = {
                "Content-Type": "application/json",
                "Mcp-Session-Id": "private-session",
            }

            def __init__(self, payload):
                self.payload = json.dumps(payload).encode("utf-8")

            def __enter__(self):
                return self

            def __exit__(self, *_):
                return False

            def read(self, *_):
                return self.payload

        def urlopen(req, timeout):
            payload = json.loads(req.data)
            sent.append(
                (
                    payload,
                    req.get_header("Authorization"),
                    req.get_header("Mcp-session-id"),
                )
            )
            if payload.get("method") == "initialize":
                content = {
                    "protocolVersion": "2025-11-25",
                    "serverInfo": {"name": "novagpt"},
                }
            elif payload.get("method") == "tools/call":
                content = {"structuredContent": {"images": [{"name": "approved"}]}}
            else:
                content = {}
            return Response(
                {"jsonrpc": "2.0", "id": payload.get("id"), "result": content}
            )

        client = McpClient("http://127.0.0.1:8787/mcp", "NOVAGPT_HTTP_TOKEN")
        with (
            patch.dict(os.environ, {"NOVAGPT_HTTP_TOKEN": "X" * 32}),
            patch.object(client.opener, "open", side_effect=urlopen),
        ):
            payload = client.tool("vm_image", {})
        self.assertEqual(payload["images"][0]["name"], "approved")
        self.assertEqual(
            [x[0]["method"] for x in sent],
            ["initialize", "notifications/initialized", "tools/call"],
        )
        self.assertTrue(all(x[1] == "Bearer " + "X" * 32 for x in sent))
        self.assertEqual(sent[-1][2], "private-session")

    def test_no_token_never_opens_network(self):
        client = McpClient("http://127.0.0.1:8787/mcp", "NOVAGPT_HTTP_TOKEN")
        with (
            patch.dict(os.environ, {"NOVAGPT_HTTP_TOKEN": ""}),
            patch("urllib.request.urlopen") as urlopen,
        ):
            with self.assertRaises(LabError):
                client.connect()
            urlopen.assert_not_called()

    def test_sse_jsonrpc_envelope(self):
        client = McpClient("http://127.0.0.1:8787/mcp", "NOVAGPT_HTTP_TOKEN")

        class Resp:
            headers = {"Content-Type": "text/event-stream"}

            def __init__(self):
                import io

                # A following live event would keep the SSE stream open; the
                # client must stop after receiving the requested response.
                self.lines = io.BytesIO(
                    b'event: message\ndata: {"jsonrpc":"2.0","id":4,"result":{}}\n\n'
                    b'event: message\ndata: {"jsonrpc":"2.0","id":5,"result":{"ok":true}}\n\n'
                    b'event: ping\ndata: {"id":6}\n\n'
                )

            def __enter__(self):
                return self

            def __exit__(self, *_):
                return False

            def readline(self):
                return self.lines.readline()

            def read(self, *_):
                raise AssertionError("Must parse SSE incrementally")

        with (
            patch.dict(os.environ, {"NOVAGPT_HTTP_TOKEN": "Y" * 32}),
            patch.object(client.opener, "open", return_value=Resp()),
        ):
            reply = client._exchange(
                {"jsonrpc": "2.0", "id": 5, "method": "tools/list"}
            )
        self.assertEqual(reply["result"]["ok"], True)

    def test_redirect_and_proxy_are_disabled(self):
        from installer.tests.network_lab.vm import NoRedirect
        import urllib.request

        with patch(
            "urllib.request.build_opener", wraps=urllib.request.build_opener
        ) as builder:
            client = McpClient("http://127.0.0.1:8787/mcp", "NOVAGPT_HTTP_TOKEN")
        installed = builder.call_args.args
        proxies = [h for h in installed if isinstance(h, urllib.request.ProxyHandler)]
        self.assertEqual(len(proxies), 1)
        self.assertEqual(proxies[0].proxies, {})
        redirects = [h for h in installed if isinstance(h, NoRedirect)]
        self.assertEqual(len(redirects), 1)
        with self.assertRaises(LabError):
            redirects[0].redirect_request(
                None, None, 302, "redirect", {}, "https://external.example/"
            )

    def test_long_running_guest_operations_extend_http_timeout(self):
        client = McpClient("http://127.0.0.1:8787/mcp", "NOVAGPT_HTTP_TOKEN")
        client.ready = True
        client._request = Mock(return_value={"structuredContent": {"exit_code": 0}})
        client.tool("vm_exec", {"id": ID, "timeout_ms": 290000})
        self.assertGreaterEqual(client._request.call_args.kwargs["timeout"], 320)
        client.tool("vm_snapshot", {"id": ID, "action": "promote"})
        self.assertGreaterEqual(client._request.call_args.kwargs["timeout"], 940)

    def test_is_error_never_returns_tool_success(self):
        client = McpClient("http://127.0.0.1:8787/mcp", "NOVAGPT_HTTP_TOKEN")
        client.ready = True
        client._request = Mock(
            return_value={
                "isError": True,
                "content": [{"type": "text", "text": "private!"}],
            }
        )
        with self.assertRaises(LabError) as caught:
            client.tool("vm_exec", {"id": ID})
        self.assertNotIn("private!", str(caught.exception))

    def test_mismatched_rpc_id_fails(self):
        client = McpClient("http://127.0.0.1:8787/mcp", "NOVAGPT_HTTP_TOKEN")
        client._exchange = Mock(
            return_value={"jsonrpc": "2.0", "id": 999, "result": {}}
        )
        with self.assertRaises(LabError):
            client._request("tools/list")


class VmBackendTests(unittest.TestCase):
    def setUp(self):
        self.cfg = load(TOML)
        self.client = Mock()
        self.vm = VM(self.client)

    def test_create_structured_nics(self):
        self.client.tool.return_value = {
            "instance": {"id": ID, "image": "ubuntu-ace-ready"}
        }
        created = self.vm.create(self.cfg.vms["ubuntu-wg"])
        self.assertEqual(created, ID)
        name, payload = self.client.tool.call_args.args
        self.assertEqual(name, "vm")
        self.assertEqual(payload["action"], "create")
        self.assertEqual(payload["networks"], ["nat", "connect:ace-ubuntu"])
        self.assertNotIn("domain", payload)

    def test_status_mismatched_identity_denied(self):
        self.client.tool.return_value = {"instance": {"id": GW_ID, "state": "stopped"}}
        with self.assertRaises(LabError):
            self.vm.status(ID)

    def test_wrong_approved_image_denied_even_for_valid_id(self):
        self.client.tool.return_value = {
            "instance": {
                "id": ID,
                "state": "running",
                "image": "unrelated-approved-image",
            }
        }
        with self.assertRaises(LabError):
            self.vm.require_profile(replace(self.cfg.vms["ubuntu"], id=ID))

    def test_create_requires_novagpt_fork_id(self):
        self.client.tool.return_value = {"instance": {"id": "novagpt-install-win11"}}
        with self.assertRaises(LabError):
            self.vm.create(self.cfg.vms["ubuntu"])

    def test_guest_exec_uses_owned_vm_tool(self):
        env = self.cfg.environments["ubuntu-direct"]
        self.client.tool.side_effect = [
            {"available": True},
            {"exit_code": 0, "stdout": "READY\n", "stderr": "", "truncated": False},
        ]
        output = self.vm.exec(ID, env, "printf READY")
        self.assertEqual(output.code, 0)
        tools = self.client.tool.call_args_list
        self.assertEqual(tools[0].args[0], "vm_guest_status")
        self.assertEqual(tools[1].args[0], "vm_exec")
        self.assertEqual(tools[1].args[1]["program"], "/bin/bash")
        self.assertEqual(tools[1].args[1]["args"], ["-c", "printf READY"])

    def test_failed_guest_status_fails_closed(self):
        self.vm.wait_guest = Mock(side_effect=LabError("Guest Agent unavailable"))
        with self.assertRaises(LabError):
            self.vm.exec(ID, self.cfg.environments["ubuntu-direct"], "id")
        self.client.tool.assert_not_called()

    def test_vm_start_waits_for_running(self):
        states = iter(["stopped", "starting", "running"])
        self.vm.state = Mock(side_effect=lambda _: next(states))
        with patch("installer.tests.network_lab.vm.time.sleep", return_value=None):
            self.vm.start(ID)
        self.client.tool.assert_called_once_with("vm", {"action": "start", "id": ID})

    def test_graceful_stop_waits_for_stopped(self):
        states = iter(["running", "running", "stopped"])
        self.vm.state = Mock(side_effect=lambda _: next(states))
        with patch("installer.tests.network_lab.vm.time.sleep", return_value=None):
            self.vm.stop(ID)
        self.client.tool.assert_called_once_with(
            "vm", {"action": "stop", "id": ID, "force": False}
        )

    def test_guest_agent_waits_through_boot(self):
        self.client.tool.side_effect = [{"available": False}, {"available": True}]
        with patch("installer.tests.network_lab.vm.time.sleep", return_value=None):
            self.vm.wait_guest(ID, seconds=2)
        self.assertEqual(self.client.tool.call_count, 2)

    def test_snapshot_requires_stopped(self):
        self.vm.state = Mock(return_value="running")
        self.vm.snapshot_list = Mock(return_value={"clean-os"})
        with self.assertRaises(LabError):
            self.vm.snapshot_restore(ID, "clean-os")
        with self.assertRaises(LabError):
            self.vm.snapshot_create(ID, "other")
        self.client.tool.assert_not_called()

    def test_snapshot_restore_never_uses_host_virsh(self):
        self.vm.state = Mock(return_value="stopped")
        self.vm.snapshot_list = Mock(return_value={"ubuntu-tor-ready"})
        self.vm.snapshot_restore(ID, "ubuntu-tor-ready")
        self.client.tool.assert_called_once_with(
            "vm_snapshot", {"action": "restore", "id": ID, "name": "ubuntu-tor-ready"}
        )

    def test_screenshot_png(self):
        self.client.tool.return_value = {
            "mime_type": "image/png",
            "width": 100,
            "height": 100,
            "data_base64": base64.b64encode(b"\x89PNG\r\n\x1a\n123").decode(),
        }
        self.assertEqual(self.vm.screenshot(ID)[:8], b"\x89PNG\r\n\x1a\n")

    def test_windows_hyperv_credentials_are_operation_scoped(self):
        env = self.cfg.environments["windows-direct"]
        self.vm.wait_guest = Mock(return_value=None)
        self.vm.status = Mock(
            return_value={"id": ID, "backend": "hyperv", "image": "windows-ace-ready"}
        )
        self.client.tool.return_value = {"exit_code": 0, "stdout": "OK", "stderr": ""}
        with patch.dict(
            os.environ,
            {"NOVAGPT_GUEST_USERNAME": "Tester", "NOVAGPT_GUEST_PASSWORD": "secret"},
        ):
            answer = self.vm.exec(ID, env, "Write-Output OK")
        self.assertEqual(answer.code, 0)
        args = self.client.tool.call_args.args[1]
        self.assertEqual(args["credential"]["username"], "Tester")
        self.assertEqual(args["credential"]["password"], "secret")
        self.assertIn("-EncodedCommand", args["args"])
        self.vm.wait_guest.assert_not_called()
        with patch.dict(
            os.environ, {"NOVAGPT_GUEST_USERNAME": "", "NOVAGPT_GUEST_PASSWORD": ""}
        ):
            with self.assertRaises(LabError):
                self.vm.exec(ID, env, "Write-Output OK")

    def test_guest_upload_and_download(self):
        self.client.tool.return_value = {"action": "upload", "bytes": 3}
        self.vm.upload(ID, "/tmp/fixture", b"abc")
        self.assertEqual(self.client.tool.call_args.args[1]["data_base64"], "YWJj")
        self.client.tool.return_value = {"data_base64": "YWJj"}
        self.assertEqual(self.vm.download(ID, "/tmp/fixture"), b"abc")
        with self.assertRaises(LabError):
            self.vm.upload(ID, "../host", b"abc")
        with self.assertRaises(LabError):
            self.vm.upload(ID, r"C:\temp\..\private.txt", b"abc")
        self.assertTrue(VM._valid_guest_path(r"C:\Windows\Temp\fixture.txt"))
        self.client.tool.return_value = {"bytes": 2}
        with self.assertRaises(LabError):
            self.vm.upload(ID, "/tmp/fixture", b"abc")

    def test_desktop_coordinates_bounded(self):
        with self.assertRaises(LabError):
            self.vm.pointer(ID, 40000, 10)
        self.client.tool.assert_not_called()


class WireGuardTests(unittest.TestCase):
    def prepare(self):
        cfg = load(TOML)
        profiles = dict(cfg.vms)
        profiles["gateway"] = replace(profiles["gateway"], id=GW_ID)
        profiles["ubuntu-wg"] = replace(profiles["ubuntu-wg"], id=ID)
        return replace(cfg, vms=profiles)

    def test_private_link_mac_is_deterministic(self):
        self.assertEqual(nic_mac(ID, 0), nic_mac(ID, 0))
        self.assertNotEqual(nic_mac(ID, 0), nic_mac(ID, 1))

    def test_setup_requires_approved_fork_ids(self):
        cfg = load(TOML)
        vm = Mock()
        with self.assertRaises(LabError):
            setup_wireguard(vm, cfg, cfg.environments["ubuntu-wireguard"])
        vm.assert_not_called() if callable(vm) else None

    def test_gateway_and_client_guest_scripts(self):
        cfg = self.prepare()
        env = cfg.environments["ubuntu-wireguard"]
        commands = []

        class FakeVm:
            def state(self, *_):
                return "running"

            def wait_guest(self, *_, **__):
                return None

            def exec(self, ident, environment, script, seconds=120):
                commands.append((ident, script))
                if script == "id -u":
                    return ExecResult(0, "0\n", "")
                if "wg pubkey <" in script:
                    return ExecResult(
                        0, ("A" if ident == GW_ID else "B") * 43 + "=\n", ""
                    )
                if "ip -4 route show default" in script:
                    return ExecResult(0, "eth0\n", "")
                if "for iface in /sys/class/net/" in script:
                    return ExecResult(0, "eth1\n", "")
                return ExecResult(0, "", "")

        info = setup_wireguard(FakeVm(), cfg, env)
        self.assertEqual(info["transport"], "NovaGPT isolated QEMU NICs")
        self.assertEqual(info["gateway_vm"], GW_ID)
        self.assertFalse(info["ipv6_covered"])
        self.assertTrue(any("Endpoint = 172.29.10.1:51821" in x[1] for x in commands))
        self.assertTrue(any("wg-quick@acegw0" in x[1] for x in commands))
        self.assertTrue(any("$(cat /etc/wireguard/" in x[1] for x in commands))
        for _, script in commands:
            syntax = subprocess.run(
                ["bash", "-n"], input=script, text=True, capture_output=True, timeout=4
            )
            self.assertEqual(syntax.returncode, 0, syntax.stderr)

    def test_same_fork_id_fails(self):
        cfg = self.prepare()
        profiles = dict(cfg.vms)
        profiles["gateway"] = replace(profiles["gateway"], id=ID)
        changed = replace(cfg, vms=profiles)
        with self.assertRaises(LabError):
            setup_wireguard(Mock(), changed, changed.environments["ubuntu-wireguard"])


class SafetyAndCliTests(unittest.TestCase):
    def setUp(self):
        self.cfg = load(TOML)
        self.profile = replace(self.cfg.vms["ubuntu"], id=ID)

    def cli(self, *args):
        return subprocess.run(
            [sys.executable, str(ROOT / "run.py"), "--config", str(TOML), *args],
            capture_output=True,
            text=True,
            timeout=12,
        )

    def test_default_plan(self):
        self.assertEqual(self.cli("list").returncode, 0)
        response = self.cli("run", "--env", "ubuntu-tor", "--case", "TOR-03")
        self.assertEqual(response.returncode, 0, response.stderr)
        self.assertIn("PLAN ONLY", response.stdout)
        self.assertIn("TOR-03", response.stdout)
        self.assertEqual(self.cli("fork-create", "--vm", "ubuntu").returncode, 0)

    def test_authorize_exact_vm_id_and_data_loss(self):
        args = argparse.Namespace(
            execute=True, confirm_vm_id=GW_ID, accept_data_loss=False
        )
        with self.assertRaises(LabError):
            authorize(args, self.profile)
        args.confirm_vm_id = ID
        self.assertTrue(authorize(args, self.profile))
        with self.assertRaises(LabError):
            authorize(args, self.profile, destructive=True)
        args.accept_data_loss = True
        self.assertTrue(authorize(args, self.profile, destructive=True))
        args.execute = False
        self.assertFalse(authorize(args, self.profile))

    def test_private_artifact_0600_and_no_symlink_follow(self):
        with tempfile.TemporaryDirectory() as d:
            private = Path(d) / "private"
            out = save_report(private, {"environment": "ubuntu-tor", "results": []})
            self.assertEqual(json.loads(out.read_text())["results"], [])
            if os.name == "posix":
                self.assertEqual(out.stat().st_mode & 0o777, 0o600)
            symlink = Path(d) / "symlink"
            symlink.symlink_to(private, target_is_directory=True)
            with self.assertRaises(LabError):
                save_binary(symlink, "abc", b"hello")

    def test_not_run_suite_is_not_a_security_pass(self):
        cfg = self.cfg
        vm = Mock()
        profile = replace(cfg.vms["ubuntu"], id=ID)
        env = cfg.environments["ubuntu-tor"]
        case = cfg.cases["TOR-02"]
        with tempfile.TemporaryDirectory() as d:
            cfg = replace(cfg, artifacts=Path(d) / "private")
            result = report_cases(vm, profile, env, [case], cfg, False)
        self.assertEqual(result, 3)
        vm.exec.assert_not_called()

    def test_infra_check_is_not_end_to_end_pass(self):
        vm = Mock()
        vm.exec.return_value = ExecResult(0, "READY", "")
        profile = self.profile
        env = self.cfg.environments["ubuntu-direct"]
        with tempfile.TemporaryDirectory() as d:
            cfg = replace(self.cfg, artifacts=Path(d) / "private")
            result = report_cases(vm, profile, env, [cfg.cases["LAB-01"]], cfg, False)
            report = next(cfg.artifacts.glob("*.json"))
            rows = json.loads(report.read_text())["results"]
        self.assertEqual(result, 0)
        self.assertEqual(rows[0]["status"], "INFRA_OK")
        self.assertFalse(rows[0]["enforcement_proven"])


if __name__ == "__main__":
    unittest.main()
