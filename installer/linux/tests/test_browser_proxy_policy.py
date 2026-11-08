"""Hermetic unit tests for Linux browser VPN-extension policy management.

All test modifications happen in TemporaryDirectory, never in real /etc or /var.
"""
from __future__ import annotations

import importlib.util
import json
import os
from pathlib import Path
import stat
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "browser-proxy-policy.py"
SPEC = importlib.util.spec_from_file_location("ace_browser_policy", SCRIPT)
assert SPEC and SPEC.loader
mod = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(mod)


class LinuxBrowserProxyPolicyTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="ace-browser-policies-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.manager = mod.PolicyManager(self.root)

    def path(self, name):
        return self.manager.location(name)

    def put(self, path, data):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)

    def json(self, path):
        return json.loads(path.read_text(encoding="utf8"))

    def test_all_browser_policies_created_and_reverted(self):
        self.manager.enable(list(mod.BROWSERS))
        for name, filename in mod.BROWSERS.items():
            result = self.json(self.path(filename))
            if name == "firefox":
                policy = result["policies"]
                self.assertEqual(policy["Proxy"], {"Mode": "none", "Locked": True})
            else:
                policy = result
                self.assertEqual(policy["ProxySettings"]["ProxyMode"], "direct")
            self.assertEqual(
                policy["ExtensionSettings"]["*"]["blocked_permissions"], ["proxy"]
            )
        state = self.path(mod.STATE_FILE)
        self.assertTrue(state.exists())
        self.assertEqual(stat.S_IMODE(state.stat().st_mode), 0o600)
        self.manager.disable()
        for target in mod.BROWSERS.values():
            self.assertFalse(self.path(target).exists())
        self.assertFalse(state.exists())

    def test_preserve_unrelated_chrome_managed_policy(self):
        other = self.path("/etc/opt/chrome/policies/managed/admin.json")
        original = b'{"HomepageLocation":"https://example.test"}\n'
        self.put(other, original)
        self.manager.enable(["chrome"])
        self.assertEqual(other.read_bytes(), original)
        self.manager.disable()
        self.assertEqual(other.read_bytes(), original)

    def test_preserve_unrelated_firefox_policy_and_file_mode(self):
        original = b'{ "policies": { "DisableTelemetry": true }, "custom": 1 }\n'
        target = self.path(mod.BROWSERS["firefox"])
        self.put(target, original)
        target.chmod(0o600)
        self.manager.enable(["firefox"])
        result = self.json(target)
        self.assertEqual(result["custom"], 1)
        self.assertTrue(result["policies"]["DisableTelemetry"])
        self.assertEqual(stat.S_IMODE(target.stat().st_mode), 0o600)
        self.manager.disable()
        self.assertEqual(target.read_bytes(), original)
        self.assertEqual(stat.S_IMODE(target.stat().st_mode), 0o600)

    def test_refuse_conflicting_chrome_policy_without_state(self):
        other = self.path("/etc/opt/chrome/policies/managed/corporate.json")
        self.put(other, b'{"ExtensionSettings":{"*":{"installation_mode":"allowed"}}}\n')
        with self.assertRaisesRegex(mod.PolicyError, "Existing chrome policy"):
            self.manager.enable(["chrome"])
        self.assertFalse(self.path(mod.STATE_FILE).exists())
        self.assertFalse(self.path(mod.BROWSERS["chrome"]).exists())

    def test_refuse_conflicting_recommended_proxy_policy(self):
        other = self.path("/etc/chromium/policies/recommended/admin.json")
        self.put(other, b'{"ProxySettings":{"ProxyMode":"system"}}\n')
        with self.assertRaisesRegex(mod.PolicyError, "Existing chromium policy"):
            self.manager.enable(["chromium"])

    def test_refuse_existing_firefox_proxy_policy(self):
        target = self.path(mod.BROWSERS["firefox"])
        original = b'{"policies":{"Proxy":{"Mode":"system","Locked":true}}}\n'
        self.put(target, original)
        with self.assertRaisesRegex(mod.PolicyError, "Existing Firefox Proxy"):
            self.manager.enable(["firefox"])
        self.assertEqual(target.read_bytes(), original)

    def test_refuse_alternate_distribution_file(self):
        existing = self.path("/usr/lib/firefox/distribution/policies.json")
        self.put(existing, b'{"policies":{"DisableTelemetry":true}}\n')
        with self.assertRaisesRegex(mod.PolicyError, "distribution-managed"):
            self.manager.enable(["firefox"])

    def test_refuse_symlink_file_and_directory(self):
        target = self.path(mod.BROWSERS["chrome"])
        fake = self.path("/tmp/fake-admin.json")
        self.put(fake, b"{}")
        target.parent.mkdir(parents=True)
        target.symlink_to(fake)
        with self.assertRaisesRegex(mod.PolicyError, "symbolic link"):
            self.manager.enable(["chrome"])
        target.unlink()
        folder = self.path("/etc/opt/chrome/policies/managed")
        folder.rmdir()
        folder.symlink_to(fake.parent)
        with self.assertRaisesRegex(mod.PolicyError, "symlink directory"):
            self.manager.enable(["chrome"])

    def test_failed_interrupted_enable_can_be_rolled_back(self):
        original = self.manager.atomic_write
        policy_writes = []
        def fail_after_first_policy(path, data, mode, gid=None):
            if path != self.path(mod.STATE_FILE):
                policy_writes.append(path)
                if len(policy_writes) == 2:
                    raise OSError("simulated interrupted enable")
            return original(path, data, mode, gid)
        self.manager.atomic_write = fail_after_first_policy
        with self.assertRaisesRegex(OSError, "simulated interrupted"):
            self.manager.enable(["chrome", "chromium"])
        self.assertTrue(self.path(mod.STATE_FILE).exists())
        self.manager.atomic_write = original
        self.manager.disable()
        self.assertFalse(self.path(mod.STATE_FILE).exists())
        self.assertFalse(self.path(mod.BROWSERS["chrome"]).exists())
        self.assertFalse(self.path(mod.BROWSERS["chromium"]).exists())

    def test_changed_policy_prevents_removing_any_entries(self):
        self.manager.enable(["chrome", "chromium"])
        target = self.path(mod.BROWSERS["chrome"])
        target.write_text('{"ProxySettings":{"ProxyMode":"system"}}\n')
        with self.assertRaisesRegex(mod.PolicyError, "changed externally"):
            self.manager.disable()
        self.assertTrue(self.path(mod.STATE_FILE).exists())
        self.assertTrue(self.path(mod.BROWSERS["chromium"]).exists())

    def test_corrupt_journal_does_not_remove_policies(self):
        self.manager.enable(["chrome"])
        state = self.path(mod.STATE_FILE)
        state.write_text('{"schema":2,"entries":[]}')
        with self.assertRaisesRegex(mod.PolicyError, "journal"):
            self.manager.disable()
        self.assertTrue(self.path(mod.BROWSERS["chrome"]).exists())

    def test_dry_run_does_not_write(self):
        self.manager.enable(["chrome", "firefox"], dry_run=True)
        self.assertFalse(self.path(mod.STATE_FILE).exists())
        self.assertFalse(self.path(mod.BROWSERS["chrome"]).exists())
        self.assertFalse(self.path(mod.BROWSERS["firefox"]).exists())
        self.manager.enable(["chrome"])
        self.manager.disable(dry_run=True)
        self.assertTrue(self.path(mod.STATE_FILE).exists())

    def test_selective_browser_guard_bypasses_other_browser_conflicts(self):
        conflict = self.path(mod.BROWSERS["firefox"])
        self.put(conflict, b'{"policies":{"Proxy":{"Mode":"system"}}}\n')
        self.manager.enable(["chrome", "brave"])
        self.assertTrue(self.path(mod.BROWSERS["chrome"]).exists())
        self.assertTrue(self.path(mod.BROWSERS["brave"]).exists())
        self.assertEqual(self.json(conflict)["policies"]["Proxy"]["Mode"], "system")
        self.manager.disable()
        self.assertEqual(self.json(conflict)["policies"]["Proxy"]["Mode"], "system")


if __name__ == "__main__":
    unittest.main()
