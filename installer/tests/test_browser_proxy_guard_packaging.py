"""Regression checks for the opt-in browser proxy guard's MSI packaging.

These are static cross-platform checks. They do NOT claim that Windows browser
enterprise policies were applied or honored at runtime.
"""
from pathlib import Path
import unittest
import xml.etree.ElementTree as ET


ROOT = Path(__file__).resolve().parents[2]
MSI = ROOT / "installer" / "installer.wxs"
PACKAGER = ROOT / "installer" / "package.ps1"
SCRIPT = ROOT / "installer" / "browser-proxy-extension-policy.ps1"
NAMESPACE = {"w": "http://wixtoolset.org/schemas/v4/wxs"}


class BrowserProxyGuardPackagingTests(unittest.TestCase):
    def test_policy_script_is_staged_and_included(self):
        contents = PACKAGER.read_text(encoding="utf-8")
        self.assertIn('Copy-Item -LiteralPath $BrowserProxyGuardScript -Destination $Artifacts', contents)
        root = ET.parse(MSI).getroot()
        matching_files = root.findall(
            ".//w:File[@Name='browser-proxy-extension-policy.ps1']", NAMESPACE
        )
        self.assertEqual(len(matching_files), 1)
        self.assertEqual(
            matching_files[0].get("Source"),
            r"artifacts\browser-proxy-extension-policy.ps1",
        )

    def test_msi_never_enables_policy_silently(self):
        root = ET.parse(MSI).getroot()
        actions = root.findall(".//w:CustomAction", NAMESPACE)
        self.assertFalse(any(
            "browser-proxy-extension-policy.ps1" in action.get("ExeCommand", "")
            and "-Action Enable" in action.get("ExeCommand", "")
            for action in actions
        ))
        actions_with_cleanup = [
            action for action in actions
            if action.get("Id") == "CA_RemoveBrowserProxyGuard"
        ]
        self.assertEqual(len(actions_with_cleanup), 1)
        self.assertIn("-Action DisableIfOwned", actions_with_cleanup[0].get("ExeCommand", ""))
        sequences = root.findall(
            ".//w:Custom[@Action='CA_RemoveBrowserProxyGuard']", NAMESPACE
        )
        self.assertEqual(len(sequences), 1)
        self.assertIn("NOT UPGRADINGPRODUCTCODE", sequences[0].get("Condition", ""))
        self.assertIn("REMOVE", sequences[0].get("Condition", ""))

    def test_guard_requires_opt_in_and_detects_existing_policies(self):
        script = SCRIPT.read_text(encoding="utf-8")
        self.assertIn("'Enable', 'Disable', 'DisableIfOwned'", script)
        self.assertIn("-IUnderstand", script)
        self.assertIn("Assert-NoConflicts", script)
        self.assertIn("Assert-NoUnownedChanges", script)
        self.assertIn("Write-State", script)
        self.assertIn("Read-State", script)
        self.assertIn('blocked_permissions', script)
        self.assertIn('Firefox 153', script)
        self.assertIn("ProxyMode", script)
        self.assertIn("Firefox proxy locked", script)


if __name__ == "__main__":
    unittest.main()
