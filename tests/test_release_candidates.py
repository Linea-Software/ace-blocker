import hashlib
import importlib.util
import io
import json
import tarfile
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location(
    "verify_release_candidates", ROOT / "installer/verify_release_candidates.py"
)
verifier = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(verifier)


class ReleaseCandidateTests(unittest.TestCase):
    def make_candidates(self, directory: Path, version: str = "1.2.3"):
        windows = directory / f"AceBlocker-{version}.msi"
        header = bytearray(512)
        header[:8] = verifier.MSI_MAGIC
        header[28:30] = b"\xfe\xff"
        header[30:32] = b"\x09\x00"
        windows.write_bytes(header)

        linux = directory / f"AceBlocker-{version}-linux-x64.tar.gz"
        with tarfile.open(linux, "w:gz") as archive:
            for name in sorted(verifier.REQUIRED_LINUX):
                entry = tarfile.TarInfo(f"AceBlocker-{version}-linux-x64/{name}")
                if name in verifier.ELF_FILES:
                    data = bytearray(24)
                    data[:6] = bytes((127, 69, 76, 70, 2, 1))
                    data[18:20] = b"\x3e\x00"
                else:
                    data = b"test"
                entry.size = len(data)
                archive.addfile(entry, io.BytesIO(data))

        manifests = []
        for platform, path in (("windows", windows), ("linux", linux)):
            manifest = directory / f"release-candidate-{platform}.json"
            manifest.write_text(json.dumps({
                "version": version, "os": platform, "architecture": "x64",
                "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                "sourceCommit": "a" * 40, "sourceRepository": "Linea-Software/ace-blocker",
                "engineCommit": "a" * 40, "appCommit": "a" * 40,
            }))
            manifests.append(manifest)
        return windows, linux, manifests

    def test_accepts_both_candidates_with_matching_provenance(self):
        with tempfile.TemporaryDirectory() as tmp:
            windows, linux, manifests = self.make_candidates(Path(tmp))
            for path, manifest, platform in zip((windows, linux), manifests, ("windows", "linux")):
                verifier.verify_candidate(
                    path, manifest, "1.2.3", platform,
                    "a" * 40, "Linea-Software/ace-blocker"
                )

    def test_rejects_fake_msi_even_with_matching_hash(self):
        with tempfile.TemporaryDirectory() as tmp:
            windows, _, manifests = self.make_candidates(Path(tmp))
            windows.write_bytes(b"MZ" + b"0" * 510)
            payload = json.loads(manifests[0].read_text())
            payload["sha256"] = hashlib.sha256(windows.read_bytes()).hexdigest()
            manifests[0].write_text(json.dumps(payload))
            with self.assertRaisesRegex(ValueError, "MSI"):
                verifier.verify_candidate(
                    windows, manifests[0], "1.2.3", "windows",
                    "a" * 40, "Linea-Software/ace-blocker"
                )

    def test_rejects_provenance_mismatch(self):
        with tempfile.TemporaryDirectory() as tmp:
            windows, _, manifests = self.make_candidates(Path(tmp))
            with self.assertRaisesRegex(ValueError, "sourceCommit"):
                verifier.verify_candidate(
                    windows, manifests[0], "1.2.3", "windows",
                    "b" * 40, "Linea-Software/ace-blocker"
                )

    def test_rejects_linux_traversal(self):
        with tempfile.TemporaryDirectory() as tmp:
            _, linux, _ = self.make_candidates(Path(tmp))
            with tarfile.open(linux, "w:gz") as archive:
                bad = tarfile.TarInfo("AceBlocker-1.2.3-linux-x64/../outside")
                bad.size = 4
                archive.addfile(bad, io.BytesIO(b"test"))
            with self.assertRaisesRegex(ValueError, "Unsafe"):
                verifier.verify_linux(linux, "1.2.3")


if __name__ == "__main__":
    unittest.main()
