#!/usr/bin/env python3
"""Prepare/assemble an Ace Blocker MSI release envelope without handling private keys.

The payload MUST be signed externally over DOMAIN || payload_bytes with
Ed25519. The authenticated website publishes the envelope only after manual
approval, Authenticode validation and public-key signature verification.
"""
import argparse
import base64
import hashlib
import json
import re
from pathlib import Path
from urllib.parse import urlsplit

DOMAIN = b"ace-blocker-windows-msi-release-v1\x00"
VERSION = re.compile(r"^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$")
HEX256 = re.compile(r"^[a-fA-F0-9]{64}$")


def safe_version(text: str) -> str:
    if not VERSION.fullmatch(text):
        raise ValueError("Expected a stable three-component numeric version")
    return text


def safe_https_url(text: str) -> str:
    value = urlsplit(text)
    if (
        value.scheme != "https" or not value.hostname or value.username
        or value.password or value.fragment or not value.netloc
    ):
        raise ValueError("Package URL must be an absolute credential-free HTTPS URL")
    return text


def prepare(args: argparse.Namespace) -> None:
    source = Path(args.msi)
    if not source.is_file() or source.suffix.lower() != ".msi":
        raise ValueError("Input must be a built Windows MSI")
    if not HEX256.fullmatch(args.publisher_cert_sha256):
        raise ValueError("Publisher certificate SHA-256 is required")
    version = safe_version(args.version)
    minimum = safe_version(args.minimum_installed_version) if args.minimum_installed_version else None
    package_url = safe_https_url(args.package_url)
    digest = hashlib.sha256()
    with source.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    release = {
        "schemaVersion": 1,
        "version": version,
        "channel": "stable",
        "os": "windows",
        "architecture": "x64",
        "packageUrl": package_url,
        "sha256": digest.hexdigest(),
        "publisherCertificateSha256": args.publisher_cert_sha256.lower(),
        "minimumInstalledVersion": minimum,
    }
    output = Path(args.output_dir)
    output.mkdir(parents=True, exist_ok=True)
    payload = json.dumps(release, separators=(",", ":"), ensure_ascii=True, sort_keys=True).encode()
    (output / "release-payload.json").write_bytes(payload)
    (output / "release-signing-input.bin").write_bytes(DOMAIN + payload)
    print("Unsigned payload and domain-separated signing input prepared.")
    print("Sign externally; do not publish without validating the detached signature.")


def assemble(args: argparse.Namespace) -> None:
    payload = Path(args.payload).read_bytes()
    signature = Path(args.signature).read_bytes()
    if len(signature) != 64:
        raise ValueError("Expected exactly 64 raw Ed25519 signature bytes")
    if len(payload) > 16 * 1024:
        raise ValueError("Release payload exceeds the client limit")
    if not re.fullmatch(r"[A-Za-z0-9_-]{1,80}", args.key_id):
        raise ValueError("Invalid signing key identifier")
    release = json.loads(payload)
    if not isinstance(release, dict) or release.get("schemaVersion") != 1:
        raise ValueError("Unsupported signed payload")
    envelope = {
        "payload": base64.b64encode(payload).decode("ascii"),
        "signature": base64.b64encode(signature).decode("ascii"),
        "key_id": args.key_id,
    }
    Path(args.output).write_text(json.dumps(envelope, indent=2) + "\n", encoding="utf-8")
    print("Envelope assembled (cryptographic verification must precede publication).")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="action", required=True)
    prep = commands.add_parser("prepare", help="Prepare authenticated release payload")
    prep.add_argument("--msi", required=True)
    prep.add_argument("--version", required=True)
    prep.add_argument("--package-url", required=True)
    prep.add_argument("--publisher-cert-sha256", required=True)
    prep.add_argument("--minimum-installed-version")
    prep.add_argument("--output-dir", required=True)
    done = commands.add_parser("assemble", help="Wrap an externally signed payload")
    done.add_argument("--payload", required=True)
    done.add_argument("--signature", required=True)
    done.add_argument("--key-id", default="ace-stable-v1")
    done.add_argument("--output", required=True)
    args = parser.parse_args()
    try:
        (prepare if args.action == "prepare" else assemble)(args)
    except (OSError, ValueError, json.JSONDecodeError) as error:
        parser.error(str(error))


if __name__ == "__main__":
    main()
