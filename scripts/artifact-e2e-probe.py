#!/usr/bin/env python3
"""Assert real UI state inside the packaged artifact's WebView2 (DOD-004).

Invoked by scripts/artifact-e2e.sh, which has already launched the executable
with a DevTools endpoint. This attaches over the Chrome DevTools Protocol using
only the standard library:

  * HTTP GET  /json        -- list page targets
  * WebSocket /devtools/page/<id> -- evaluate JavaScript in the real page

What it proves that the Playwright suite cannot:

  1. The assertions run inside the PACKAGED EXECUTABLE, not `vite preview`. The
     clause excludes a development server, and the previous suite used one.
  2. The Tauri IPC bridge is present (`window.__TAURI_INTERNALS__`), which only
     exists in the real webview. A browser run cannot demonstrate this.
  3. A backend command round-trips: `get_system_health` is invoked through the
     real IPC bridge and its typed envelope is checked. The UI alone is not
     evidence that the Rust side is wired.

The digest is passed in and re-verified by the caller after the run.
"""
from __future__ import annotations

import base64
import hashlib
import json
import os
import socket
import ssl
import struct
import sys
import urllib.request
from pathlib import Path


def http_json(url: str, timeout: float = 5.0):
    with urllib.request.urlopen(url, timeout=timeout) as resp:
        return json.loads(resp.read().decode("utf-8"))


class WsClient:
    """Minimal WebSocket client (RFC 6455) over a plain TCP socket."""

    def __init__(self, url: str):
        assert url.startswith("ws://"), f"only ws:// supported, got {url}"
        rest = url[len("ws://") :]
        hostport, _, path = rest.partition("/")
        host, _, port = hostport.partition(":")
        self.sock = socket.create_connection((host, int(port or 80)), timeout=10)
        key = base64.b64encode(os.urandom(16)).decode()
        req = (
            f"GET /{path} HTTP/1.1\r\n"
            f"Host: {hostport}\r\n"
            "Upgrade: websocket\r\n"
            "Connection: Upgrade\r\n"
            f"Sec-WebSocket-Key: {key}\r\n"
            "Sec-WebSocket-Version: 13\r\n\r\n"
        )
        self.sock.sendall(req.encode())
        buf = b""
        while b"\r\n\r\n" not in buf:
            chunk = self.sock.recv(4096)
            if not chunk:
                raise RuntimeError("websocket handshake failed")
            buf += chunk
        if b"101" not in buf.split(b"\r\n")[0]:
            raise RuntimeError(f"websocket handshake rejected: {buf[:120]!r}")

    def send(self, payload: str) -> None:
        data = payload.encode("utf-8")
        header = bytearray([0x81])
        mask = os.urandom(4)
        length = len(data)
        if length < 126:
            header.append(0x80 | length)
        elif length < 65536:
            header.append(0x80 | 126)
            header += struct.pack(">H", length)
        else:
            header.append(0x80 | 127)
            header += struct.pack(">Q", length)
        header += mask
        masked = bytes(b ^ mask[i % 4] for i, b in enumerate(data))
        self.sock.sendall(bytes(header) + masked)

    def recv(self) -> str:
        def read(n: int) -> bytes:
            out = b""
            while len(out) < n:
                chunk = self.sock.recv(n - len(out))
                if not chunk:
                    raise RuntimeError("socket closed")
                out += chunk
            return out

        b1, b2 = read(2)
        length = b2 & 0x7F
        if length == 126:
            length = struct.unpack(">H", read(2))[0]
        elif length == 127:
            length = struct.unpack(">Q", read(8))[0]
        return read(length).decode("utf-8", errors="replace")

    def call(self, msg_id: int, method: str, params: dict | None = None) -> dict:
        self.send(json.dumps({"id": msg_id, "method": method, "params": params or {}}))
        for _ in range(50):
            raw = self.recv()
            try:
                obj = json.loads(raw)
            except json.JSONDecodeError:
                continue
            if obj.get("id") == msg_id:
                return obj
        raise RuntimeError(f"no response for {method}")

    def close(self) -> None:
        try:
            self.sock.close()
        except OSError:
            pass


def evaluate(ws: WsClient, expr: str, msg_id: int):
    res = ws.call(
        msg_id,
        "Runtime.evaluate",
        {"expression": expr, "returnByValue": True, "awaitPromise": True},
    )
    result = res.get("result", {}).get("result", {})
    if "value" in result:
        return result["value"]
    return None


def main() -> int:
    port = sys.argv[1]
    exe = sys.argv[2]
    digest = sys.argv[3]
    size = sys.argv[4]
    status_path = Path(sys.argv[5])
    targets_path = Path(sys.argv[6])

    targets = json.loads(targets_path.read_text("utf-8"))
    pages = [t for t in targets if t.get("type") == "page"]
    if not pages:
        print("probe: FAIL -- no page target in the artifact", file=sys.stderr)
        return 1

    ws_url = pages[0]["webSocketDebuggerUrl"]
    ws = WsClient(ws_url)

    # A bare Runtime.evaluate against a WebView2 page returns an empty result
    # unless the Runtime domain is enabled and the target has been resumed. Both
    # are required; without them `document.title` came back as '' and every
    # assertion failed against a context that was never actually attached.
    ws.call(1, "Runtime.enable")
    ws.call(2, "Page.enable")

    checks: list[tuple[str, bool, str]] = []

    # 1. The real UI rendered.
    title = evaluate(ws, "document.title", 10)
    checks.append(("document title", bool(title) and "LINCHPIN" in title, repr(title)))

    heading = evaluate(
        ws,
        "(document.querySelector('h1')||{}).textContent || ''",
        11,
    )
    checks.append(
        (
            "product heading rendered",
            "LINCHPIN Patent Intelligence OS" in (heading or ""),
            repr(heading),
        )
    )

    # 2. The confidentiality boundary is stated in the real UI.
    body = evaluate(ws, "document.body.innerText", 12) or ""
    checks.append(
        (
            "confidentiality boundary stated",
            "Local-First Confidentiality Boundary Active." in body,
            "present" if "Local-First Confidentiality" in body else "MISSING",
        )
    )

    # 3. IPC bridge exists -- impossible in a plain browser.
    has_ipc = evaluate(
        ws,
        "typeof window.__TAURI_INTERNALS__ !== 'undefined'",
        13,
    )
    checks.append(("Tauri IPC bridge present", has_ipc is True, repr(has_ipc)))

    # 4. A real command round-trips through the bridge.
    health = evaluate(
        ws,
        """
        (async () => {
          try {
            const r = await window.__TAURI_INTERNALS__.invoke('get_system_health');
            return JSON.stringify(r);
          } catch (e) { return 'ERROR:' + String(e); }
        })()
        """,
        14,
    )
    ok_health = isinstance(health, str) and health.startswith("{")
    checks.append(
        (
            "get_system_health round-trip",
            ok_health,
            (health or "no result")[:120],
        )
    )
    if ok_health:
        parsed = json.loads(health)
        has_fields = all(k in parsed for k in ("status", "version", "storage_ok"))
        checks.append(
            (
                "health payload typed",
                has_fields,
                f"keys={sorted(parsed)}",
            )
        )
        checks.append(
            (
                "health reports a real status",
                parsed.get("status") in {"OK", "DEGRADED"},
                f"status={parsed.get('status')}",
            )
        )

    # 5. The namespace report round-trips too, proving more than one command.
    namespaces = evaluate(
        ws,
        """
        (async () => {
          try {
            const r = await window.__TAURI_INTERNALS__.invoke('get_namespace_status');
            return JSON.stringify(r);
          } catch (e) { return 'ERROR:' + String(e); }
        })()
        """,
        15,
    )
    ns_ok = isinstance(namespaces, str) and namespaces.startswith("[")
    checks.append(
        (
            "get_namespace_status round-trip",
            ns_ok,
            (namespaces or "no result")[:120],
        )
    )
    if ns_ok:
        ns = json.loads(namespaces)
        implemented = [n for n in ns if n.get("implemented")]
        checks.append(
            (
                "namespace coverage reported",
                len(ns) == 14,
                f"{len(implemented)}/{len(ns)} implemented",
            )
        )

    ws.close()

    failures = [c for c in checks if not c[1]]
    lines = [
        "# DOD-004 Exact-Artifact E2E",
        "",
        "Generated by `scripts/artifact-e2e.sh`. Do not hand-edit.",
        "",
        "## Artifact under test",
        "",
        f"- Path: `{exe}`",
        f"- SHA-256: `{digest}`",
        f"- Size: {size} bytes",
        f"- DevTools endpoint: `http://127.0.0.1:{port}`",
        "- Target: the packaged executable's real WebView2 instance -- NOT a",
        "  development server.",
        "",
        "## Assertions (executed inside the artifact)",
        "",
        "| Check | Result | Observed |",
        "| --- | --- | --- |",
    ]
    for name, ok, observed in checks:
        lines.append(f"| {name} | {'PASS' if ok else 'FAIL'} | `{observed}` |")

    lines += [
        "",
        "## Verdict",
        "",
        (
            f"PASS — {len(checks)} assertion(s) executed against digest `{digest}`"
            if not failures
            else f"FAIL — {len(failures)} of {len(checks)} assertion(s) failed"
        ),
        "",
        "## Why this is stronger than the Playwright suite",
        "",
        "The Playwright suite runs against `vite preview`, which DOD-004 excludes",
        "as a development server. This probe drives the packaged executable and",
        "additionally proves the Tauri IPC bridge exists and that two backend",
        "commands round-trip, which no browser-based run can demonstrate.",
        "",
        "## Limitation",
        "",
        "The executable is launched from the build output rather than installed",
        "from the MSI. `scripts/smoke-installed-artifact.sh` covers the install",
        "path. A virgin clean-room install (DOD-034) remains EXTERNAL_REQUIRED.",
        "",
    ]
    status_path.parent.mkdir(parents=True, exist_ok=True)
    status_path.write_text("\n".join(lines), "utf-8")

    for name, ok, observed in checks:
        print(f"  {'PASS' if ok else 'FAIL'}: {name} -- {observed}")
    print(f"probe: wrote {status_path}")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
