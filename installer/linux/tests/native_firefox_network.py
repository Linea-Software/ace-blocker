#!/usr/bin/env python3
"""Native Firefox HTTP/HTTPS smoke for disposable installed Linux QA guests.

Run as the desktop test user with the example.org QA block enabled.
This uses Firefox's WebDriver BiDi endpoint, not an explicit browser proxy.
"""
import base64
import hashlib
import json
import os
import socket
import struct
import subprocess
import tempfile
import time
from pathlib import Path


class WebSocket:
    def __init__(self, port):
        self.sock = socket.create_connection(("127.0.0.1", port), timeout=5)
        self.reader = self.sock.makefile("rb")
        key = base64.b64encode(os.urandom(16)).decode()
        self.sock.sendall(
            (f"GET /session HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n"
             f"Upgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: {key}\r\n"
             "Sec-WebSocket-Version: 13\r\n\r\n").encode()
        )
        status = self.reader.readline()
        if b" 101 " not in status:
            raise RuntimeError(status.decode(errors="replace"))
        headers = {}
        while True:
            line = self.reader.readline()
            if line == b"\r\n":
                break
            if not line:
                raise EOFError("WebSocket handshake ended")
            name, value = line.decode().split(":", 1)
            headers[name.lower()] = value.strip()
        expected = base64.b64encode(
            hashlib.sha1((key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").encode()).digest()
        ).decode()
        if headers.get("sec-websocket-accept") != expected:
            raise RuntimeError("invalid WebSocket handshake")

    def send(self, payload, opcode=1):
        mask = os.urandom(4)
        size = len(payload)
        if size < 126:
            length = bytes([128 | size])
        elif size < 65536:
            length = b"\xfe" + struct.pack("!H", size)
        else:
            length = b"\xff" + struct.pack("!Q", size)
        self.sock.sendall(
            bytes([128 | opcode]) + length + mask
            + bytes(value ^ mask[i % 4] for i, value in enumerate(payload))
        )

    def receive(self):
        data = bytearray()
        while True:
            header = self.reader.read(2)
            if len(header) != 2:
                raise EOFError("WebSocket closed")
            opcode, size = header[0] & 15, header[1] & 127
            if size == 126:
                size = struct.unpack("!H", self.reader.read(2))[0]
            elif size == 127:
                size = struct.unpack("!Q", self.reader.read(8))[0]
            if size > 8_000_000:
                raise ValueError("oversized WebSocket frame")
            payload = self.reader.read(size)
            if len(payload) != size:
                raise EOFError("truncated WebSocket frame")
            if opcode == 8:
                raise EOFError("WebSocket closed")
            if opcode == 9:
                self.send(payload, 10)
                continue
            if opcode == 10:
                continue
            data.extend(payload)
            if header[0] & 128:
                return json.loads(data)

    def close(self):
        self.reader.close()
        self.sock.close()


def check_case(url, expected):
    with tempfile.TemporaryDirectory(prefix="ace-firefox-qa-") as profile:
        # Only suppress automatic HTTPS upgrades, so HTTP exercises TCP/80.
        # Do not change routing, proxy settings or certificate validation.
        Path(profile, "user.js").write_text(
            'user_pref("dom.security.https_only_mode", false);\n'
            'user_pref("dom.security.https_first", false);\n'
            'user_pref("browser.shell.checkDefaultBrowser", false);\n'
        )
        with socket.socket() as reserve:
            reserve.bind(("127.0.0.1", 0))
            port = reserve.getsockname()[1]
        process = subprocess.Popen(
            ["firefox", "--headless", "--no-remote", "--profile", profile,
             "--remote-debugging-port", str(port), "about:blank"],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        )
        ws = None
        start = time.monotonic()
        status, error = None, None
        try:
            deadline = start + 30
            while time.monotonic() < deadline:
                if process.poll() is not None:
                    raise RuntimeError("Firefox exited")
                try:
                    ws = WebSocket(port)
                    break
                except (ConnectionError, OSError):
                    time.sleep(.25)
            if ws is None:
                raise RuntimeError("BiDi unavailable")
            seq = 0
            events = []

            def call(method, params):
                nonlocal seq
                seq += 1
                ws.sock.settimeout(35)
                ws.send(json.dumps({"id": seq, "method": method, "params": params}).encode())
                while True:
                    value = ws.receive()
                    if value.get("id") == seq:
                        if value.get("type") == "error":
                            raise RuntimeError(value)
                        return value.get("result", {})
                    events.append(value)

            call("session.new", {"capabilities": {"alwaysMatch": {"acceptInsecureCerts": False}}})
            context = call("browsingContext.getTree", {})["contexts"][0]["context"]
            call("session.subscribe", {
                "events": ["network.responseCompleted", "network.fetchError"],
                "contexts": [context],
            })
            call("browsingContext.navigate", {"context": context, "url": url, "wait": "complete"})
            deadline = time.monotonic() + 5
            while True:
                matching = [e for e in events if
                            e.get("method") == "network.responseCompleted"
                            and e.get("params", {}).get("response", {}).get("url") == url]
                if matching:
                    status = matching[-1]["params"]["response"]["status"]
                    break
                if time.monotonic() >= deadline:
                    raise RuntimeError("no exact document response")
                ws.sock.settimeout(max(.1, deadline - time.monotonic()))
                events.append(ws.receive())
        except Exception as exc:
            error = str(exc)
        finally:
            if ws:
                ws.close()
            process.terminate()
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
        return {
            "url": url, "expected": expected, "observed": status, "error": error,
            "seconds": round(time.monotonic() - start, 2),
            "pass": status == expected and error is None,
        }


def main():
    results = []
    for url, expected in (
        ("http://example.org/", 404), ("http://example.com/", 200),
        ("https://example.org/", 404), ("https://example.com/", 200),
    ):
        row = check_case(url, expected)
        results.append(row)
        print(json.dumps(row), flush=True)
    return 0 if all(row["pass"] for row in results) else 1


if __name__ == "__main__":
    raise SystemExit(main())
