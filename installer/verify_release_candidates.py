#!/usr/bin/env python3
"""Validate desktop release candidates before assembling a GitHub draft.

A candidate is not an approved, Authenticode-signed public release.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import tarfile
from pathlib import Path

MAX_PACKAGE_BYTES = 100 * 1024 * 1024
MAX_EXPANDED_BYTES = 512 * 1024 * 1024
REQUIRED_LINUX = frozenset({
    "ace-engine", "ace-watchdog", "ace-redirect", "install-cert", "ace-app",
    "install.sh", "browser-proxy-policy.py", "migrate-legacy-data.sh",
    "ace-blocker.desktop", "ace-blocker-handler.desktop", "ace-blocker.png",
    "RELEASING.md", "bundled-packs/manifest.json",
})
ELF_FILES = frozenset({"ace-engine", "ace-watchdog", "ace-redirect", "install-cert", "ace-app"})
MSI_MAGIC = bytes.fromhex("d0cf11e0a1b11ae1")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def verify_msi(path: Path) -> None:
    with path.open("rb") as source:
        header = source.read(512)
    if (len(header) != 512 or header[:8] != MSI_MAGIC or
        header[28:30] != b"\xfe\xff" or
        header[30:32] not in (b"\x09\x00", b"\x0c\x00")):
        raise ValueError("Not an MSI compound-file header")


def verify_linux(path: Path, version: str) -> None:
    root = f"AceBlocker-{version}-linux-x64"
    seen: set[str] = set()
    total = 0
    with tarfile.open(path, mode="r|gz") as archive:
        for index, member in enumerate(archive):
            if index >= 4096:
                raise ValueError("Too many tar entries")
            parts = member.name.removesuffix("/").split("/")
            if (not parts or parts[0] != root or
                any(part in ("", ".", "..") for part in parts) or
                "\\" in member.name or member.name.startswith("/")):
                raise ValueError(f"Unsafe or out-of-root tar path: {member.name!r}")
            if not (member.isfile() or member.isdir()):
                raise ValueError("Archive contains a link or special file")
            if member.isdir():
                continue
            name = "/".join(parts[1:])
            if not name or name in seen:
                raise ValueError("Duplicate or root-level file entry")
            seen.add(name)
            total += member.size
            if total > MAX_EXPANDED_BYTES:
                raise ValueError("Uncompressed archive exceeds 512 MiB")
            if name in REQUIRED_LINUX and member.size == 0:
                raise ValueError(f"Required file is empty: {name}")
            source = archive.extractfile(member)
            if source is None:
                raise ValueError("Unreadable tar member")
            prefix = source.read(20)
            if name in ELF_FILES and (
                prefix[:6] != bytes((127, 69, 76, 70, 2, 1)) or
                prefix[18:20] != b"\x3e\x00"
            ):
                raise ValueError(f"Not an x86-64 Linux ELF executable: {name}")
            # Drain content so gzip integrity/truncation is checked.
            while source.read(1024 * 1024):
                pass
    missing = REQUIRED_LINUX - seen
    if missing:
        raise ValueError(f"Linux archive missing required files: {sorted(missing)}")


def verify_candidate(
    path: Path, manifest_path: Path, version: str, platform: str, commit: str, repository: str
) -> None:
    expected = (f"AceBlocker-{version}.msi" if platform == "windows" else
                f"AceBlocker-{version}-linux-x64.tar.gz")
    if path.name != expected or not 0 < path.stat().st_size <= MAX_PACKAGE_BYTES:
        raise ValueError(f"Expected nonempty {expected} (max 100 MiB)")
    manifest = json.loads(manifest_path.read_text(encoding="utf-8-sig"))
    digest = sha256_file(path)
    for field, value in {
        "version": version,
        "os": platform,
        "architecture": "x64",
        "sha256": digest,
        "sourceCommit": commit,
        "sourceRepository": repository,
        "engineCommit": commit,
        "appCommit": commit,
    }.items():
        if manifest.get(field) != value:
            raise ValueError(f"{platform} provenance mismatch: {field}")
    if platform == "windows":
        verify_msi(path)
    else:
        verify_linux(path, version)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--version", required=True)
    parser.add_argument("--commit", required=True)
    parser.add_argument("--repository", required=True)
    parser.add_argument("--windows", required=True, type=Path)
    parser.add_argument("--linux", required=True, type=Path)
    parser.add_argument("--windows-manifest", required=True, type=Path)
    parser.add_argument("--linux-manifest", required=True, type=Path)
    args = parser.parse_args()
    if not re.fullmatch(r"\d+\.\d+\.\d+", args.version):
        parser.error("Version must be X.Y.Z")
    verify_candidate(args.windows, args.windows_manifest, args.version, "windows", args.commit, args.repository)
    verify_candidate(args.linux, args.linux_manifest, args.version, "linux", args.commit, args.repository)
    print("Both Windows and Linux release artifacts passed structure, hash and provenance checks.")


if __name__ == "__main__":
    main()
