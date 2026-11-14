"""Static regression checks for installer-only Ace Blocker branding.

These checks do not install the MSI or change production services.
"""
import struct
import unittest
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
INSTALLER = ROOT / "installer"
BRANDING = INSTALLER / "branding"
WXS = INSTALLER / "installer.wxs"
NS = {"w": "http://wixtoolset.org/schemas/v4/wxs"}


class InstallerBrandingTests(unittest.TestCase):
    def setUp(self):
        self.wix = ET.parse(WXS).getroot()

    def test_icon_sources_are_in_repository_and_valid(self):
        ico = (BRANDING / "ace-icon-installer-dark.ico").read_bytes()
        self.assertEqual(struct.unpack_from("<HHH", ico), (0, 1, 7))
        sizes = []
        for i in range(7):
            w, h, colors, reserved, planes, depth, data_size, offset = struct.unpack_from(
                "<BBBBHHII", ico, 6 + i * 16
            )
            width, height = w or 256, h or 256
            self.assertEqual(width, height)
            self.assertEqual(planes, 1)
            self.assertEqual(depth, 32)
            self.assertEqual(ico[offset:offset + 8], b"\x89PNG\r\n\x1a\n")
            self.assertEqual(
                struct.unpack_from(">II", ico, offset + 16), (width, height)
            )
            self.assertLessEqual(offset + data_size, len(ico))
            sizes.append(width)
        self.assertEqual(sizes, [16, 24, 32, 48, 64, 128, 256])

        png = (BRANDING / "ace-icon-installer-dark.png").read_bytes()
        self.assertEqual(png[:8], b"\x89PNG\r\n\x1a\n")
        self.assertEqual(struct.unpack_from(">II", png, 16), (1024, 1024))

        bmp = (BRANDING / "ace-icon-installer-dark.bmp").read_bytes()
        self.assertEqual(bmp[:2], b"BM")
        self.assertEqual(struct.unpack_from("<ii", bmp, 18), (128, 128))

        svg = ET.parse(BRANDING / "ace-icon-installer-dark.svg").getroot()
        tags = {elem.tag.split("}")[-1] for elem in svg.iter()}
        self.assertFalse(tags & {"image", "text", "foreignObject"})
        self.assertEqual(svg.get("viewBox"), "0 0 320 320")

    def test_main_symbol_fills_canvas_without_oversized_download_badge(self):
        svg = ET.parse(BRANDING / "ace-icon-installer-dark.svg").getroot()
        nodes = {
            element.get("data-name"): element
            for element in svg.iter()
            if element.get("data-name")
        }
        ring = nodes["ace-icon-dark · Ring A / upper-left"]
        coords = [
            tuple(float(v) for v in point.split(","))
            for point in ring.get("points").split()
        ]
        # The original logo started at x/y=46; the installer artwork
        # must reduce the navy padding to about 25 on a 320px canvas.
        self.assertLess(min(x for x, _ in coords), 30)
        self.assertLess(min(y for _, y in coords), 30)

        badge = nodes["Installer badge · amber disk"]
        separator = nodes["Installer badge · minimal separation"]
        self.assertLessEqual(float(badge.get("r")) * 2, 100)
        self.assertLessEqual(float(separator.get("r")) - float(badge.get("r")), 4)

        # The download arrow must render above the entire Ace symbol.
        groups = [item.get("data-name") for item in svg if item.tag.endswith("g")]
        self.assertEqual(groups[-1], "80 • Installer download badge")

    def test_icon_used_in_installed_program_metadata(self):
        icons = self.wix.findall(".//w:Icon", NS)
        icons = [icon for icon in icons if icon.get("Id") == "AceInstallerIcon"]
        self.assertEqual(len(icons), 1)
        self.assertEqual(
            icons[0].get("SourceFile"),
            r"branding\ace-icon-installer-dark.ico",
        )
        props = self.wix.findall(".//w:Property", NS)
        arp = [prop for prop in props if prop.get("Id") == "ARPPRODUCTICON"]
        self.assertEqual(len(arp), 1)
        self.assertEqual(arp[0].get("Value"), "AceInstallerIcon")

    def test_setup_dialog_uses_bitmap(self):
        binaries = self.wix.findall(".//w:Binary", NS)
        images = [item for item in binaries if item.get("Id") == "AceInstallerDialogLogo"]
        self.assertEqual(len(images), 1)
        self.assertEqual(images[0].get("SourceFile"),
                         r"branding\ace-icon-installer-dark.bmp")
        controls = self.wix.findall(".//w:Dialog[@Id='InstallConfirmDlg']/w:Control", NS)
        logo = [control for control in controls if control.get("Id") == "InstallerLogo"]
        self.assertEqual(len(logo), 1)
        self.assertEqual(logo[0].get("Type"), "Bitmap")
        self.assertEqual(logo[0].get("Text"), "AceInstallerDialogLogo")

    def test_installer_icon_not_reused_for_application_shortcuts(self):
        shortcuts = self.wix.findall(".//w:Shortcut", NS)
        self.assertEqual({entry.get("Id") for entry in shortcuts},
                         {"SC_StartMenu", "SC_Desktop"})
        for shortcut in shortcuts:
            self.assertIsNone(shortcut.get("Icon"))
            self.assertEqual(shortcut.get("Target"), "[INSTALLFOLDER]ace-app.exe")


if __name__ == "__main__":
    unittest.main()
