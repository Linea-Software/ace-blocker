#!/usr/bin/env python3
r"""Windows 11 VM desktop test controls using libvirt/QMP, without in-guest automation.

Coordinates use the 960x600 screenshot preview, independent of guest resolution.
Screenshots stay on the host; never transmit user secrets or browser data.
Examples:
  python3 installer/tests/windows_vm_desktop.py screenshot
  python3 installer/tests/windows_vm_desktop.py click 312 437
  python3 installer/tests/windows_vm_desktop.py double-click 25 182
  python3 installer/tests/windows_vm_desktop.py key KEY_LEFTMETA KEY_R
  python3 installer/tests/windows_vm_desktop.py type 'C:\Program Files\Ace Blocker\ace-app.exe'
"""

import argparse
import json
from pathlib import Path
import subprocess
import time

DEFAULT_VM = "novagpt-install-win11-ace-test"
PREVIEW_WIDTH = 960
PREVIEW_HEIGHT = 600


def virsh(vm: str, *args: str) -> None:
    result = subprocess.run(
        ["virsh", "-c", "qemu:///session", *args],
        text=True,
        capture_output=True,
        timeout=25,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(result.stderr.strip() or result.stdout.strip())


def qmp(vm: str, events: list[dict]) -> None:
    virsh(
        vm,
        "qemu-monitor-command",
        vm,
        json.dumps(
            {
                "execute": "input-send-event",
                "arguments": {"events": events},
            }
        ),
    )


def click(vm: str, x: int, y: int, count: int) -> None:
    if not 0 <= x < PREVIEW_WIDTH or not 0 <= y < PREVIEW_HEIGHT:
        raise ValueError("coordinates must be inside the 960x600 preview")
    qmp(
        vm,
        [
            {
                "type": "abs",
                "data": {"axis": "x", "value": round(x / PREVIEW_WIDTH * 32767)},
            },
            {
                "type": "abs",
                "data": {"axis": "y", "value": round(y / PREVIEW_HEIGHT * 32767)},
            },
        ],
    )
    time.sleep(0.12)
    for _ in range(count):
        qmp(vm, [{"type": "btn", "data": {"button": "left", "down": True}}])
        time.sleep(0.08)
        qmp(vm, [{"type": "btn", "data": {"button": "left", "down": False}}])
        time.sleep(0.16)


def key(vm: str, keys: list[str]) -> None:
    virsh(vm, "send-key", vm, *keys)


def type_text(vm: str, value: str) -> None:
    punct = {
        ":": ["KEY_LEFTSHIFT", "KEY_SEMICOLON"],
        "\\": ["KEY_BACKSLASH"],
        "-": ["KEY_MINUS"],
        ".": ["KEY_DOT"],
        "/": ["KEY_SLASH"],
        " ": ["KEY_SPACE"],
        '"': ["KEY_LEFTSHIFT", "KEY_APOSTROPHE"],
        "_": ["KEY_LEFTSHIFT", "KEY_MINUS"],
        "=": ["KEY_EQUAL"],
        "@": ["KEY_LEFTSHIFT", "KEY_2"],
    }
    for char in value:
        if char in punct:
            keys = punct[char]
        elif char.isalnum() and char.isascii():
            keys = ["KEY_" + char.upper()]
            if char.isupper():
                keys.insert(0, "KEY_LEFTSHIFT")
        else:
            raise ValueError(f"Unsupported keyboard character: {char!r}")
        key(vm, keys)
        time.sleep(0.025)


def screenshot(vm: str, out: Path) -> None:
    full = out.with_name(out.stem + ".original.png")
    full.parent.mkdir(parents=True, exist_ok=True)
    try:
        virsh(vm, "screenshot", vm, str(full))
        process = subprocess.run(
            [
                "magick",
                str(full),
                "-resize",
                f"{PREVIEW_WIDTH}x{PREVIEW_HEIGHT}",
                "-strip",
                str(out),
            ],
            check=False,
            text=True,
            capture_output=True,
            timeout=30,
        )
        if process.returncode:
            raise RuntimeError(process.stderr)
    finally:
        full.unlink(missing_ok=True)
    print(out.resolve())


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--vm", default=DEFAULT_VM)
    sub = parser.add_subparsers(dest="action", required=True)
    for action in ("click", "double-click"):
        p = sub.add_parser(action)
        p.add_argument("x", type=int)
        p.add_argument("y", type=int)
    p = sub.add_parser("key")
    p.add_argument("keys", nargs="+")
    p = sub.add_parser("type")
    p.add_argument("text")
    p = sub.add_parser("screenshot")
    p.add_argument(
        "--out", type=Path, default=Path(__file__).with_name(".vm-preview.png")
    )
    args = parser.parse_args()
    if args.action in ("click", "double-click"):
        click(args.vm, args.x, args.y, 2 if args.action == "double-click" else 1)
    elif args.action == "key":
        key(args.vm, args.keys)
    elif args.action == "type":
        type_text(args.vm, args.text)
    else:
        screenshot(args.vm, args.out)


if __name__ == "__main__":
    main()
