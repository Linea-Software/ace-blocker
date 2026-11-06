import argparse
import base64
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location(
    "release_manifest", ROOT / "installer" / "release_manifest.py"
)
release_manifest = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(release_manifest)


class ReleaseManifestContractTests(unittest.TestCase):
    def test_prepare_creates_exact_signing_bytes_and_hash(self):
        with tempfile.TemporaryDirectory() as temp:
            folder = Path(temp)
            msi = folder / "AceBlocker-0.2.0.msi"
            msi.write_bytes(b"fixture-msi-not-authenticode-signed")
            args = argparse.Namespace(
                msi=str(msi), version="0.2.0",
                package_url="https://aceblocker.com/release/AceBlocker-0.2.0.msi",
                publisher_cert_sha256="ab" * 32,
                minimum_installed_version="0.1.0",
                output_dir=str(folder / "output"),
            )
            release_manifest.prepare(args)
            payload = (folder / "output/release-payload.json").read_bytes()
            self.assertEqual(
                (folder / "output/release-signing-input.bin").read_bytes(),
                release_manifest.DOMAIN + payload,
            )
            values = json.loads(payload)
            self.assertEqual(values["sha256"], __import__("hashlib").sha256(msi.read_bytes()).hexdigest())
            self.assertEqual(values["minimumInstalledVersion"], "0.1.0")
            sig = folder / "signature.bin"
            sig.write_bytes(b"s" * 64)  # invalid cryptographic signature, assembly only
            release_manifest.assemble(argparse.Namespace(
                payload=str(folder / "output/release-payload.json"), signature=str(sig),
                key_id="ace-stable-v1", output=str(folder / "envelope.json"),
            ))
            envelope = json.loads((folder / "envelope.json").read_text())
            self.assertEqual(base64.b64decode(envelope["payload"]), payload)
            self.assertEqual(base64.b64decode(envelope["signature"]), b"s" * 64)

    def test_rejects_unsafe_url_version_and_missing_signature(self):
        for url in ("http://aceblocker.com/a.msi", "file:///a.msi", "https://u:p@example.com/a.msi"):
            with self.assertRaises(ValueError):
                release_manifest.safe_https_url(url)
        for version in ("0.2", "0.2.0-rc.1", "2.01.0", "../0.2.0"):
            with self.assertRaises(ValueError):
                release_manifest.safe_version(version)
        with tempfile.TemporaryDirectory() as temp:
            folder = Path(temp)
            payload = folder / "payload.json"
            payload.write_text('{"schemaVersion":1}')
            signature = folder / "signature.bin"
            signature.write_bytes(b"")
            with self.assertRaises(ValueError):
                release_manifest.assemble(argparse.Namespace(
                    payload=str(payload), signature=str(signature),
                    key_id="ace-stable-v1", output=str(folder / "env.json"),
                ))

    def test_wix_preserves_cas_and_blocks_locked_upgrades_until_authorized(self):
        tree = ET.parse(ROOT / "installer" / "installer.wxs")
        ns = {"w": "http://wixtoolset.org/schemas/v4/wxs"}
        sequence = tree.find(".//w:InstallExecuteSequence", ns)
        self.assertIsNotNone(sequence)
        conditions = {
            element.get("Action"): element.get("Condition")
            for element in sequence.findall("w:Custom", ns)
        }
        self.assertEqual(conditions["CA_CheckLocks"], "REMOVE")
        self.assertEqual(conditions["CA_PrepareEngineMaintenance"], "REMOVE")
        self.assertEqual(
            conditions["CA_RemoveApiCert"], "REMOVE AND NOT UPGRADINGPRODUCTCODE"
        )
        self.assertEqual(
            conditions["CA_RemoveMitmCert"], "REMOVE AND NOT UPGRADINGPRODUCTCODE"
        )
        sources = {element.get("Name") for element in tree.findall(".//w:File", ns)}
        self.assertIn("verify-product-signature.ps1", sources)


if __name__ == "__main__":
    unittest.main()
