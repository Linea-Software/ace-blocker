"""Regression checks for the branded, self-contained Windows Burn setup bundle."""
from pathlib import Path
import unittest
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]
BUNDLE = ROOT / "installer" / "bundle.wxs"
PACKAGE = ROOT / "installer" / "package.ps1"
RELEASE = ROOT / ".github" / "workflows" / "release.yml"
W = {"w": "http://wixtoolset.org/schemas/v4/wxs",
     "bal": "http://wixtoolset.org/schemas/v4/wxs/bal"}


class BurnBundleTests(unittest.TestCase):
    def setUp(self):
        self.root = ET.parse(BUNDLE).getroot()

    def test_embeds_only_existing_msi_with_no_download_fallback(self):
        bundles = self.root.findall("w:Bundle", W)
        self.assertEqual(len(bundles), 1)
        bundle = bundles[0]
        self.assertEqual(bundle.get("IconSourceFile"),
                         r"branding\ace-icon-installer-dark.ico")
        self.assertEqual(bundle.get("Compressed"), "yes")
        self.assertEqual(bundle.get("Version"), "__ACE_VERSION__")
        self.assertTrue(bundle.get("UpgradeCode"))
        chain = bundle.find("w:Chain", W)
        self.assertIsNotNone(chain)
        children = list(chain)
        self.assertEqual(len(children), 1)
        msi = children[0]
        self.assertEqual(msi.tag.rsplit("}", 1)[-1], "MsiPackage")
        self.assertEqual(msi.get("SourceFile"), "__ACE_MSI_PATH__")
        self.assertEqual(msi.get("Compressed"), "yes")
        self.assertEqual(msi.get("Vital"), "yes")
        self.assertEqual(msi.get("Visible"), "no")
        self.assertIsNone(msi.get("DownloadUrl"))

    def test_custom_icon_is_in_bootstrapper_ui(self):
        ba = self.root.find(".//bal:WixStandardBootstrapperApplication", W)
        self.assertIsNotNone(ba)
        self.assertEqual(ba.get("LogoFile"),
                         r"branding\ace-icon-installer-dark.png")
        self.assertEqual(ba.get("SuppressOptionsUI"), "yes")
        self.assertEqual(ba.get("LicenseUrl"), "")

    def test_packager_builds_msi_before_burn_exe(self):
        text = PACKAGE.read_text(encoding="utf-8")
        self.assertLess(text.index("Building MSI"), text.index("bundle.generated.wxs"))
        self.assertIn("WixToolset.Bal.wixext", text)
        self.assertIn("AceBlocker-Setup-{0}.exe", text)
        self.assertIn("$bundleXml.Replace('__ACE_VERSION__', $Version)", text)
        self.assertIn(".Replace('__ACE_MSI_PATH__', $msiPath)", text)
        self.assertIn("Remove-Item -LiteralPath $GeneratedBundleSource", text)

    def test_ci_includes_and_verifies_both_windows_artifacts(self):
        text = RELEASE.read_text(encoding="utf-8")
        self.assertIn("WixToolset.Bal.wixext/4.0.5", text)
        self.assertIn("installer/dist/AceBlocker-Setup-", text)
        self.assertIn('setupSha256 = $setupSha', text)
        self.assertIn('--windows-setup "dist/AceBlocker-Setup-$VERSION.exe"', text)
        self.assertIn('sha256sum *.exe *.msi *.tar.gz', text)
        # The website staging endpoint still requires a separately signed MSI.
        self.assertIn('file = Get-Item "installer\\dist\\AceBlocker-$env:VERSION.msi"', text)


if __name__ == "__main__":
    unittest.main()
