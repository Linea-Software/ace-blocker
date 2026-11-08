#!/usr/bin/env python3
"""Optional Linux anti-proxy browser policies for Ace Blocker.

No browser policy is installed by the normal Ace installer. Run --enable
explicitly as root. All I/O is confined to system policy directories and a
root-only ownership journal. Tests use PolicyManager(root=temp_dir).
"""
from __future__ import annotations

import argparse
import base64
from contextlib import contextmanager
from datetime import datetime, timezone
import fcntl
import hashlib
import json
import os
from pathlib import Path
import stat
import sys
import tempfile
from typing import Iterator

BROWSERS = {
    "chrome": "/etc/opt/chrome/policies/managed/ace-blocker-proxy.json",
    "chromium": "/etc/chromium/policies/managed/ace-blocker-proxy.json",
    "brave": "/etc/brave/policies/managed/ace-blocker-proxy.json",
    "firefox": "/etc/firefox/policies/policies.json",
}
STATE_FILE = "/var/lib/ace-blocker/browser-proxy-policy-state.json"
LOCK_FILE = "/var/lib/ace-blocker/browser-proxy-policy.lock"
CHROMIUM_POLICY = {
    "ExtensionSettings": {"*": {"blocked_permissions": ["proxy"]}},
    "ProxySettings": {"ProxyMode": "direct"},
}
FIREFOX_POLICY = {
    "ExtensionSettings": {"*": {"blocked_permissions": ["proxy"]}},
    "Proxy": {"Mode": "none", "Locked": True},
}
CHROMIUM_CONFLICTS = frozenset({
    "ExtensionSettings", "ExtensionInstallAllowlist", "ExtensionInstallBlocklist",
    "ExtensionInstallForcelist", "ExtensionInstallSources", "ExtensionAllowedTypes",
    "ProxyMode", "ProxySettings", "ProxyPacUrl", "ProxyServer",
    "ProxyBypassList", "ProxyServerMode",
})
FIREFOX_ALTERNATES = (
    "/usr/lib/firefox/distribution/policies.json",
    "/usr/lib64/firefox/distribution/policies.json",
    "/usr/lib/firefox-esr/distribution/policies.json",
    "/usr/lib64/firefox-esr/distribution/policies.json",
    "/usr/share/firefox/distribution/policies.json",
    "/opt/firefox/distribution/policies.json",
)
MAX_POLICY_BYTES = 1024 * 1024
MAX_JOURNAL_BYTES = 6 * 1024 * 1024


class PolicyError(RuntimeError):
    pass


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def json_bytes(value: object) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n").encode("utf-8")


def parse_object(data: bytes, location: Path) -> dict:
    try:
        value = json.loads(data.decode("utf-8"))
    except (ValueError, UnicodeError) as error:
        raise PolicyError(f"Cannot parse JSON policy {location}: {error}") from error
    if not isinstance(value, dict):
        raise PolicyError(f"Expected JSON object in {location}")
    return value


class PolicyManager:
    def __init__(self, root: Path = Path("/")):
        self.root = root.absolute()
        self.actual_system = self.root == Path("/")

    def location(self, absolute_name: str) -> Path:
        assert absolute_name.startswith("/")
        return self.root / absolute_name.lstrip("/")

    @staticmethod
    def check_regular(path: Path) -> bool:
        """Refuse files, ancestors and other policy containers that are symlinks."""
        if path.is_symlink():
            raise PolicyError(f"Refusing symbolic link: {path}")
        if not path.exists():
            return False
        if not path.is_file():
            raise PolicyError(f"Not a regular file: {path}")
        return True

    def check_ancestors(self, path: Path) -> None:
        # Check every existing component; Path.exists() alone follows symlinks.
        current = self.root
        if current.is_symlink():
            raise PolicyError(f"Refusing symlink root: {current}")
        if not current.is_dir():
            raise PolicyError(f"Policy root is not a directory: {current}")
        try:
            parts = path.relative_to(self.root).parts
        except ValueError as error:
            raise PolicyError("Policy path outside root") from error
        for part in parts[:-1]:
            current /= part
            if current.is_symlink():
                raise PolicyError(f"Refusing symlink directory: {current}")
            if current.exists() and not current.is_dir():
                raise PolicyError(f"Expected directory: {current}")
            if self.actual_system and current.exists():
                info = current.stat()
                if info.st_uid != 0 or info.st_mode & 0o022:
                    raise PolicyError(f"Untrusted writable policy directory: {current}")

    def read_file(self, path: Path, limit: int = MAX_POLICY_BYTES) -> bytes | None:
        self.check_ancestors(path)
        if not self.check_regular(path):
            return None
        info = path.stat()
        if info.st_size > limit:
            raise PolicyError(f"Policy file too large: {path}")
        if self.actual_system and info.st_uid != 0:
            raise PolicyError(f"Untrusted non-root-owned policy file: {path}")
        return path.read_bytes()

    def make_directories(self, path: Path, mode: int) -> None:
        self.check_ancestors(path)
        missing = []
        current = path.parent
        while not current.exists():
            missing.append(current)
            current = current.parent
        for directory in reversed(missing):
            if directory.is_symlink():
                raise PolicyError(f"Refusing symlink directory: {directory}")
            directory.mkdir(mode=mode)
            if self.actual_system:
                os.chown(directory, 0, 0)

    @staticmethod
    def fsync_directory(directory: Path) -> None:
        fd = os.open(directory, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
        try:
            os.fsync(fd)
        finally:
            os.close(fd)

    def atomic_write(self, path: Path, data: bytes, mode: int, gid: int | None = None) -> None:
        self.make_directories(path, 0o700 if path in (self.location(STATE_FILE), self.location(LOCK_FILE)) else 0o755)
        self.check_ancestors(path)
        if path.is_symlink():
            raise PolicyError(f"Refusing symbolic link: {path}")
        fd, temp_path = tempfile.mkstemp(prefix=".ace-policy-", dir=str(path.parent))
        try:
            with os.fdopen(fd, "wb") as stream:
                stream.write(data)
                stream.flush()
                os.fsync(stream.fileno())
            os.chmod(temp_path, mode)
            if gid is not None and self.actual_system:
                os.chown(temp_path, 0, gid)
            os.replace(temp_path, path)
            self.fsync_directory(path.parent)
        finally:
            if os.path.exists(temp_path):
                os.unlink(temp_path)

    @contextmanager
    def locked(self) -> Iterator[None]:
        lock_path = self.location(LOCK_FILE)
        self.make_directories(lock_path, 0o700)
        self.check_ancestors(lock_path)
        if lock_path.is_symlink():
            raise PolicyError(f"Unsafe lock file: {lock_path}")
        fd = os.open(lock_path, os.O_CREAT | os.O_RDWR | getattr(os, "O_NOFOLLOW", 0), 0o600)
        try:
            info = os.fstat(fd)
            if not stat.S_ISREG(info.st_mode) or (self.actual_system and info.st_uid != 0):
                raise PolicyError(f"Unsafe lock file ownership or type: {lock_path}")
            fcntl.flock(fd, fcntl.LOCK_EX)
            yield
        finally:
            os.close(fd)

    def current_state(self) -> dict | None:
        state_path = self.location(STATE_FILE)
        data = self.read_file(state_path, MAX_JOURNAL_BYTES)
        if data is None:
            return None
        state = parse_object(data, state_path)
        if state.get("schema") != 1 or not isinstance(state.get("entries"), list):
            raise PolicyError("Unknown or damaged Ace policy journal; not proceeding")
        seen = set()
        for entry in state["entries"]:
            if not isinstance(entry, dict) or entry.get("browser") not in BROWSERS:
                raise PolicyError("Policy journal contains unknown browser or path")
            browser = entry["browser"]
            if browser in seen:
                raise PolicyError("Duplicate browser in policy journal")
            seen.add(browser)
            if entry.get("path") != BROWSERS[browser]:
                raise PolicyError("Policy journal path does not match allowed policy path")
            before = entry.get("before")
            if before is not None:
                try:
                    raw = base64.b64decode(before, validate=True)
                except (ValueError, TypeError) as error:
                    raise PolicyError("Invalid saved original policy") from error
                if len(raw) > MAX_POLICY_BYTES or digest(raw) != entry.get("before_sha256"):
                    raise PolicyError("Saved original policy checksum mismatch")
            before_mode = entry.get("before_mode")
            before_gid = entry.get("before_gid")
            if before is not None and (
                not isinstance(before_mode, int) or not 0 <= before_mode <= 0o777
                or not isinstance(before_gid, int) or before_gid < 0
            ):
                raise PolicyError("Invalid saved original file permissions")
            if not isinstance(entry.get("after_sha256"), str) or len(entry["after_sha256"]) != 64:
                raise PolicyError("Invalid policy journal checksum")
        if not seen:
            raise PolicyError("Empty browser policy journal")
        return state

    def check_chromium_conflicts(self, browser: str) -> None:
        target = self.location(BROWSERS[browser])
        if self.read_file(target) is not None:
            raise PolicyError(f"Ace policy file already exists without owned journal: {target}")
        for kind in ("managed", "recommended"):
            folder = target.parent if kind == "managed" else target.parent.parent / "recommended"
            self.check_ancestors(folder / ".check")
            if not folder.exists():
                continue
            if folder.is_symlink() or not folder.is_dir():
                raise PolicyError(f"Unexpected policy directory: {folder}")
            for candidate in sorted(folder.iterdir()):
                if candidate.suffix != ".json":
                    continue
                if candidate == target:
                    continue
                data = self.read_file(candidate)
                if data is None:
                    continue
                policy = parse_object(data, candidate)
                overlap = CHROMIUM_CONFLICTS.intersection(policy)
                if overlap:
                    raise PolicyError(
                        f"Existing {browser} policy {candidate} owns {sorted(overlap)}. "
                        "Ace will not override it."
                    )

    def firefox_after(self) -> tuple[bytes | None, bytes]:
        target = self.location(BROWSERS["firefox"])
        for other_name in FIREFOX_ALTERNATES:
            other = self.location(other_name)
            if other != target and self.read_file(other) is not None:
                raise PolicyError(f"Firefox has another distribution-managed policy file: {other}")
        original = self.read_file(target)
        if original is None:
            base: dict = {"policies": {}}
        else:
            base = parse_object(original, target)
            if not isinstance(base.get("policies"), dict):
                raise PolicyError(f"Firefox policies object missing/invalid: {target}")
        policies = base["policies"]
        for name in FIREFOX_POLICY:
            if name in policies:
                raise PolicyError(f"Existing Firefox {name} policy in {target}; refusing takeover")
        updated = dict(base)
        updated["policies"] = {**policies, **FIREFOX_POLICY}
        return original, json_bytes(updated)

    def plan(self, browsers: list[str]) -> list[dict]:
        if len(browsers) != len(set(browsers)) or not browsers:
            raise PolicyError("Browser selection must contain unique supported names")
        entries = []
        for browser in browsers:
            if browser not in BROWSERS:
                raise PolicyError(f"Unsupported browser: {browser}")
            if browser == "firefox":
                before, after = self.firefox_after()
            else:
                self.check_chromium_conflicts(browser)
                before, after = None, json_bytes(CHROMIUM_POLICY)
            target_path = self.location(BROWSERS[browser])
            original_stat = target_path.stat() if before is not None else None
            entries.append({
                "browser": browser,
                "path": BROWSERS[browser],
                "before": base64.b64encode(before).decode("ascii") if before is not None else None,
                "before_sha256": digest(before) if before is not None else None,
                "before_mode": stat.S_IMODE(original_stat.st_mode) if original_stat is not None else None,
                "before_gid": original_stat.st_gid if original_stat is not None else None,
                "after_sha256": digest(after),
                "after": after,
            })
        return entries

    def verify_owned(self, entries: list[dict]) -> list[tuple[dict, bytes | None]]:
        current = []
        for entry in entries:
            path = self.location(entry["path"])
            data = self.read_file(path)
            if data is not None and digest(data) not in (
                entry["after_sha256"], entry["before_sha256"]
            ):
                raise PolicyError(f"Policy changed externally: {path}. No files removed.")
            # If the policy was already restored to its original bytes, it is
            # safe to keep it during a retry after interrupted disable.
            current.append((entry, data))
        return current

    def enable(self, browsers: list[str], dry_run: bool = False) -> None:
        if dry_run:
            if self.current_state() is not None:
                raise PolicyError("Ace policy state exists; disable before changing browser set")
            entries = self.plan(browsers)
            for entry in entries:
                print(f"Would manage {entry['browser']}: {entry['path']}")
            return
        with self.locked():
            if self.current_state() is not None:
                raise PolicyError("Ace policy state already exists; use --disable first")
            entries = self.plan(browsers)
            # Journal before the first policy write. Interrupted enable is
            # recoverable through --disable; nothing unowned is removed.
            journal = {
                "schema": 1,
                "created_utc": datetime.now(timezone.utc).isoformat(),
                "entries": [{key: value for key, value in entry.items() if key != "after"}
                            for entry in entries],
            }
            self.atomic_write(self.location(STATE_FILE), json_bytes(journal), 0o600)
            for entry in entries:
                self.atomic_write(
                    self.location(entry["path"]), entry["after"],
                    entry["before_mode"] if entry["before_mode"] is not None else 0o644,
                    entry["before_gid"],
                )
            print("Browser proxy guard installed. Restart the selected browsers and inspect their policy pages.")

    def disable(self, dry_run: bool = False) -> None:
        if dry_run:
            state = self.current_state()
            if state is None:
                raise PolicyError("No Ace-owned browser proxy policy state")
            self.verify_owned(state["entries"])
            for entry in state["entries"]:
                print(f"Would restore {entry['path']}")
            return
        with self.locked():
            state = self.current_state()
            if state is None:
                raise PolicyError("No Ace-owned browser proxy policies to remove")
            current = self.verify_owned(state["entries"])
            for entry, existing in current:
                path = self.location(entry["path"])
                original = (base64.b64decode(entry["before"], validate=True)
                            if entry["before"] is not None else None)
                if existing == original:
                    continue
                if original is None:
                    # A missing file is also safe after an interrupted disable.
                    if existing is not None:
                        path.unlink()
                        self.fsync_directory(path.parent)
                else:
                    self.atomic_write(path, original, entry["before_mode"], entry["before_gid"])
            self.location(STATE_FILE).unlink()
            self.fsync_directory(self.location(STATE_FILE).parent)
            print("Ace-owned proxy policies restored. Restart browsers to refresh settings.")

    def status(self) -> None:
        state = self.current_state()
        if state is None:
            print("Ace browser proxy guard: not enabled")
            return
        print("Ace browser proxy guard journal: present")
        for entry in state["entries"]:
            data = self.read_file(self.location(entry["path"]))
            if data is None:
                status = "missing (incomplete)"
            elif digest(data) == entry["after_sha256"]:
                status = "configured (registry/file only; verify in browser)"
            elif digest(data) == entry["before_sha256"]:
                status = "restored/original (incomplete)"
            else:
                status = "CHANGED EXTERNALLY"
            print(f"  {entry['browser']}: {status} [{entry['path']}]")
        print("Verify chrome://policy, brave://policy, chromium://policy and about:policies.")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    action = parser.add_mutually_exclusive_group()
    action.add_argument("--status", action="store_true", help="Show stored policy state (default)")
    action.add_argument("--enable", action="store_true", help="Enable ACE browser proxy policy (root)")
    action.add_argument("--disable", action="store_true", help="Restore ACE-owned policy state (root)")
    parser.add_argument("--dry-run", action="store_true", help="Verify without changing policy files")
    parser.add_argument(
        "--browser", action="append", choices=sorted(BROWSERS),
        help="Browser to protect (repeatable); --enable defaults to all supported browsers",
    )
    args = parser.parse_args(argv)
    manager = PolicyManager()
    try:
        if args.enable or args.disable:
            if os.geteuid() != 0:
                raise PolicyError("Run as root (sudo) to change browser policies")
            if args.enable:
                manager.enable(args.browser or list(BROWSERS), dry_run=args.dry_run)
            else:
                if args.browser:
                    raise PolicyError("--browser is only supported with --enable")
                manager.disable(dry_run=args.dry_run)
        else:
            if args.browser or args.dry_run:
                raise PolicyError("--browser/--dry-run require --enable or --disable")
            manager.status()
        return 0
    except (PolicyError, OSError, PermissionError) as error:
        print(f"ACE browser proxy guard: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
