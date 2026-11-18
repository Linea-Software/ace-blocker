"""Offline unit tests for the lab: never start, modify or revert a real VM."""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import Mock

from installer.tests.network_lab.config import Environment, LabError, load
from installer.tests.network_lab.providers import connect_script, probe_script, provision_script
from installer.tests.network_lab.run import authorize, save_report
from installer.tests.network_lab.vm import ExecResult, VM
from installer.tests.network_lab.wireguard_lab import setup as setup_wireguard

ROOT = Path(__file__).resolve().parent
TOML = ROOT / "lab.toml"


class ConfigTests(unittest.TestCase):
    def test_sample_matrix(self):
        config = load(TOML)
        self.assertEqual(len(config.environments), 32)
        self.assertEqual(len(config.cases), 28)
        self.assertIn("ubuntu-tor", config.environments)
        self.assertIn("cachyos-wireguard", config.environments)
        self.assertIn("TOR-03", config.cases)
        self.assertIn("I2P-01", config.cases)
        self.assertIsNotNone(config.wireguard)
        self.assertEqual(config.wireguard.peers["cachyos"].port, 51823)

    def test_reject_unsupported_provider(self):
        sample = TOML.read_text().replace('provider = "tor"', 'provider = "not-real"', 1)
        with tempfile.TemporaryDirectory() as d:
            path = Path(d) / "plan.toml"
            path.write_text(sample)
            with self.assertRaises(LabError):
                load(path)

    def test_reject_unknown_case_environment(self):
        sample = TOML.read_text().replace('"ubuntu-direct"', '"unknown-env"', 1)
        with tempfile.TemporaryDirectory() as d:
            path = Path(d) / "plan.toml"
            path.write_text(sample)
            with self.assertRaises(LabError):
                load(path)

    def test_reject_remote_libvirt(self):
        sample = TOML.read_text().replace("qemu:///session", "qemu+ssh://danger/system", 1)
        with tempfile.TemporaryDirectory() as d:
            path = Path(d) / "plan.toml"
            path.write_text(sample)
            with self.assertRaises(LabError):
                load(path)


class ProviderTests(unittest.TestCase):
    def test_ubuntu_wireguard(self):
        env = Environment("ubuntu-wireguard", "ace-lab-ubuntu", "ubuntu", "wireguard", "wg-ready")
        self.assertIn("apt-get install", provision_script(env))
        self.assertIn("wg-quick up ace-lab", connect_script(env))

    def test_arch_openvpn(self):
        env = Environment("arch-openvpn", "ace-lab-arch", "arch", "openvpn", "ovpn-ready")
        self.assertIn("pacman -S --needed", provision_script(env))
        self.assertIn("test -f /etc/openvpn/client/ace-lab.conf", connect_script(env))

    def test_no_unattended_paid_provider_login(self):
        env = Environment("ubuntu-proton", "ace-lab-ubuntu", "ubuntu", "proton", "proton-ready")
        with self.assertRaises(LabError):
            provision_script(env)
        with self.assertRaises(LabError):
            connect_script(env)

    def test_cachyos_socks_uses_arch_openssh_package(self):
        env = Environment("cachyos-socks", "ace-lab-cachyos",
                          "cachyos", "socks", "socks-ready")
        script = provision_script(env)
        self.assertIn("pacman -S --needed --noconfirm openssh ", script)
        self.assertNotIn("openssh-client", script)

    def test_wireguard_probe_requires_a_real_handshake(self):
        env = Environment("ubuntu-wireguard", "ace-lab-ubuntu",
                          "ubuntu", "wireguard", "wg-ready")
        script = probe_script(env, "routing")
        self.assertIn("wg show ace-lab latest-handshakes", script)

    def test_tor_proxy_listener_is_checked(self):
        env = Environment("ubuntu-tor", "ace-lab-ubuntu", "ubuntu", "tor", "tor-ready")
        self.assertIn("9050", probe_script(env, "tor-socks"))
        self.assertIn("grep -q", probe_script(env, "tor-socks"))

    def test_browser_case_cannot_claim_enforcement(self):
        env = Environment("ubuntu-tor", "ace-lab-ubuntu", "ubuntu", "tor", "tor-ready")
        self.assertIsNone(probe_script(env, "browser"))


class SafetyTests(unittest.TestCase):
    def setUp(self):
        self.env = Environment("ubuntu-tor", "ace-lab-ubuntu", "ubuntu", "tor", "tor-ready")

    def test_commands_require_explicit_exact_domain(self):
        args = argparse.Namespace(execute=True, confirm_domain="some-other-vm",
                                  accept_data_loss=False)
        with self.assertRaises(LabError):
            authorize(args, self.env)
        args.execute = False
        self.assertFalse(authorize(args, self.env))

    def test_restore_requires_accept_data_loss(self):
        args = argparse.Namespace(execute=True, confirm_domain=self.env.domain,
                                  accept_data_loss=False)
        with self.assertRaises(LabError):
            authorize(args, self.env, reset=True)

    def test_revert_denies_running_domain(self):
        vm = VM("qemu:///session")
        vm.state = Mock(return_value="running")
        vm.snapshot_exists = Mock(return_value=True)
        vm.virsh = Mock()
        with self.assertRaises(LabError):
            vm.restore_snapshot(self.env)
        vm.virsh.assert_not_called()

    def test_revert_denies_missing_snapshot(self):
        vm = VM("qemu:///session")
        vm.state = Mock(return_value="shut off")
        vm.preflight_disposable = Mock(return_value=("test-uuid", "vda"))
        vm.snapshot_exists = Mock(return_value=False)
        vm.virsh = Mock()
        with self.assertRaises(LabError):
            vm.restore_snapshot(self.env)
        vm.virsh.assert_not_called()

    def test_revert_no_force(self):
        vm = VM("qemu:///session")
        vm.state = Mock(return_value="shut off")
        vm.preflight_disposable = Mock(return_value=("test-uuid", "vda"))
        vm.preflight_snapshot = Mock()
        vm.snapshot_exists = Mock(return_value=True)
        vm.virsh = Mock()
        vm.restore_snapshot(self.env)
        args = vm.virsh.call_args.args
        self.assertNotIn("--force", args)
        self.assertEqual(args[0], "snapshot-revert")

    def test_private_json_report(self):
        with tempfile.TemporaryDirectory() as d:
            path = save_report(Path(d) / "private", {"environment": "ubuntu-tor", "results": []})
            self.assertEqual(json.loads(path.read_text())["results"], [])
            if os.name == "posix":
                self.assertEqual(path.stat().st_mode & 0o777, 0o600)

    def test_cli_defaults_to_plan_only(self):
        proc = subprocess.run([
            sys.executable, str(ROOT / "run.py"), "--config", str(TOML),
            "run", "--env", "ubuntu-tor", "--case", "TOR-03",
        ], capture_output=True, text=True, timeout=15)
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertIn("PLAN ONLY", proc.stdout)


class WireGuardLabTests(unittest.TestCase):
    def test_gateway_client_setup_keeps_private_keys_on_guest(self):
        lab = load(TOML).wireguard
        self.assertIsNotNone(lab)
        env = Environment("ubuntu-wireguard", "ace-lab-ubuntu",
                          "ubuntu", "wireguard", "ubuntu-wireguard-ready")
        scripts: list[tuple[str, str]] = []

        class FakeVM:
            def state(self, e):
                return "running"

            def wait_agent(self, e):
                return None

            def exec(self, e, script, seconds=120):
                scripts.append((e.domain, script))
                if script == "id -u":
                    return ExecResult(0, "0\n", "")
                if "wg pubkey <" in script:
                    key = "A" if e.domain == lab.gateway_domain else "B"
                    return ExecResult(0, key * 43 + "=\n", "")
                if "ip -4 route show default" in script:
                    return ExecResult(0, "ens3\n", "")
                return ExecResult(0, "OK\n", "")

        info = setup_wireguard(FakeVM(), env, lab)
        self.assertEqual(info["gateway_interface"], "acegw0")
        self.assertTrue(any("AllowedIPs = 0.0.0.0/0" in s for _, s in scripts))
        self.assertTrue(any("nft add table ip ace_lab_nat_acegw0" in s for _, s in scripts))
        self.assertTrue(any("$(cat /etc/wireguard/" in s for _, s in scripts))
        self.assertTrue(all("PRIVATE_KEY = A" not in s for _, s in scripts))
        for _, script in scripts:
            syntax = subprocess.run(["bash", "-n"], input=script, text=True,
                                    capture_output=True, timeout=5)
            self.assertEqual(syntax.returncode, 0, syntax.stderr)

    def test_wireguard_setup_rejects_same_vm(self):
        lab = load(TOML).wireguard
        env = Environment("ubuntu-wireguard", lab.gateway_domain,
                          "ubuntu", "wireguard", "ubuntu-wireguard-ready")
        with self.assertRaises(LabError):
            setup_wireguard(VM("qemu:///session"), env, lab)

    def test_reject_overlapping_wireguard_ports(self):
        text = TOML.read_text().replace("port = 51822", "port = 51821", 1)
        with tempfile.TemporaryDirectory() as d:
            path = Path(d) / "plan.toml"
            path.write_text(text)
            with self.assertRaises(LabError):
                load(path)

    def test_wireguard_setup_plans_without_touching_vm(self):
        proc = subprocess.run([
            sys.executable, str(ROOT / "run.py"), "--config", str(TOML),
            "wireguard-setup", "--env", "ubuntu-wireguard",
        ], capture_output=True, text=True, timeout=15)
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertIn("PLAN ONLY", proc.stdout)
        self.assertIn("ace-lab-vpn-gateway", proc.stdout)


class GuestAgentTests(unittest.TestCase):
    def test_linux_guest_exec_uses_structured_qga_arguments(self):
        import base64
        env = Environment("ubuntu-direct", "ace-lab-ubuntu", "ubuntu", "direct", "direct-ready")
        vm = VM("qemu:///session")
        recorded = []

        def mock_agent(guest, op, args=None):
            recorded.append((op, args))
            if op == "guest-exec":
                return {"pid": 123}
            return {"exited": True, "exitcode": 0,
                    "out-data": base64.b64encode(b"READY\n").decode("ascii")}
        vm.agent = mock_agent
        out = vm.exec(env, "printf READY")
        self.assertEqual(out.code, 0)
        self.assertEqual(out.stdout, "READY\n")
        self.assertEqual(recorded[0][1]["path"], "/bin/bash")
        self.assertEqual(recorded[0][1]["arg"], ["-c", "printf READY"])

    def test_running_services_required_for_infra_baseline(self):
        env = Environment("ubuntu-direct", "ace-lab-ubuntu", "ubuntu", "direct", "direct-ready")
        script = probe_script(env, "baseline")
        self.assertIn("systemctl is-active --quiet ace-engine", script)
        self.assertIn("systemctl is-active --quiet ace-watchdog", script)


class SnapshotPreflightTests(unittest.TestCase):
    UUID = "123e4567-e89b-12d3-a456-426614174000"
    DOMAIN = """<domain type='kvm'><name>ace-lab-ubuntu</name><uuid>123e4567-e89b-12d3-a456-426614174000</uuid><devices><disk type='file' device='disk'><driver name='qemu' type='qcow2'/><source file='/tmp/disposable.qcow2'/><target dev='vda'/></disk></devices></domain>"""
    SNAP = """<domainsnapshot><name>ubuntu-tor-ready</name><state>shutoff</state><memory snapshot='no'/><disks><disk name='vda' snapshot='internal'/></disks><domain><uuid>123e4567-e89b-12d3-a456-426614174000</uuid></domain></domainsnapshot>"""

    def test_allowed_disposable_single_disk_snapshot(self):
        vm = VM("qemu:///session")
        env = Environment("ubuntu-tor", "ace-lab-ubuntu", "ubuntu", "tor", "ubuntu-tor-ready")
        def stub(*args, **kwargs):
            if args[0] == "dumpxml":
                return self.DOMAIN
            if args[0] == "domuuid":
                return self.UUID
            if args[0] == "snapshot-dumpxml":
                return self.SNAP
            raise AssertionError(f"unexpected virsh: {args}")
        vm.virsh = stub
        self.assertEqual(vm.preflight_disposable(env), (self.UUID, "vda"))
        vm.preflight_snapshot(env, self.UUID, "vda")

    def test_reject_uefi_and_tpm_uncaptured_state(self):
        vm = VM("qemu:///session")
        vm.virsh = lambda *a, **k: (
            self.DOMAIN.replace("</devices>", "<tpm model='tpm-crb'/></devices>")
            if a[0] == "dumpxml" else self.UUID
        )
        env = Environment("ubuntu-tor", "ace-lab-ubuntu", "ubuntu", "tor", "ubuntu-tor-ready")
        with self.assertRaises(LabError):
            vm.preflight_disposable(env)

    def test_reject_external_or_live_snapshot(self):
        vm = VM("qemu:///session")
        env = Environment("ubuntu-tor", "ace-lab-ubuntu", "ubuntu", "tor", "ubuntu-tor-ready")
        vm.virsh = lambda *a, **k: self.SNAP.replace('snapshot="internal"', 'snapshot="external"').replace("snapshot='internal'", "snapshot='external'")
        with self.assertRaises(LabError):
            vm.preflight_snapshot(env, self.UUID, "vda")
        vm.virsh = lambda *a, **k: self.SNAP.replace("<state>shutoff</state>", "<state>running</state>")
        with self.assertRaises(LabError):
            vm.preflight_snapshot(env, self.UUID, "vda")

    def test_snapshot_save_requires_domain_uuid(self):
        from installer.tests.network_lab.run import confirm_vm_uuid
        vm = VM("qemu:///session")
        vm.virsh = Mock(return_value=self.UUID)
        env = Environment("ubuntu-tor", "ace-lab-ubuntu", "ubuntu", "tor", "ubuntu-tor-ready")
        with self.assertRaises(LabError):
            confirm_vm_uuid(vm, env, None)
        with self.assertRaises(LabError):
            confirm_vm_uuid(vm, env, "another-domain-uuid")
        confirm_vm_uuid(vm, env, self.UUID)

    def test_artifacts_existing_directory_permissions_not_changed(self):
        with tempfile.TemporaryDirectory() as d:
            directory = Path(d) / "existing"
            directory.mkdir(mode=0o755)
            directory.chmod(0o755)
            with self.assertRaises(LabError):
                save_report(directory, {"environment": "ubuntu", "results": []})
            self.assertEqual(directory.stat().st_mode & 0o777, 0o755)

    def test_all_manual_suite_has_nonzero_exit(self):
        from installer.tests.network_lab.run import execute_cases
        from installer.tests.network_lab.config import Case
        env = Environment("ubuntu-tor", "ace-lab-ubuntu", "ubuntu", "tor", "ubuntu-tor-ready")
        case = Case("TOR-01", "browser", ("ubuntu-tor",), "Must verify browser origin")
        with tempfile.TemporaryDirectory() as d:
            self.assertEqual(execute_cases(VM("qemu:///session"), env, [case], Path(d) / "report"), 3)

    def test_malformed_wireguard_peers_and_artifact_path(self):
        src = TOML.read_text()
        bad = [
            src.replace("[wireguard_lab.peers.ubuntu]", "[[wireguard_lab.peers.ubuntu]]", 1),
            src.replace('artifacts = ".artifacts"', 'artifacts = "../outside"', 1),
        ]
        for value in bad:
            with tempfile.TemporaryDirectory() as d:
                path = Path(d) / "lab.toml"
                path.write_text(value)
                with self.assertRaises(LabError):
                    load(path)


if __name__ == "__main__":
    unittest.main()
