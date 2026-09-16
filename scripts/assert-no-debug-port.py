#!/usr/bin/env python3
"""Assert an executable does NOT open a WebView2 debug port -- with attribution.

SPEC-005 keeps invention content device-confidential, and a released binary that
opens a CDP debug port would let any local process attach to the webview and
invoke backend commands. Both the exact-artifact gate and the provider live-fire
gate launch the PRODUCTION executable and check that port 9222 stays closed.

WHY THIS IS A SCRIPT AND NOT FIVE LINES INSIDE EACH GATE

Measured defect: the check was `connect_ex(("127.0.0.1", 9222)) == 0` after
launching the artifact, with no baseline and no attribution. A leftover
debug-enabled build from an interrupted run was still holding 9222, so a gate
that launched the honest production binary reported

    artifact-e2e: FAIL -- the production artifact opens a debug port

which is a false accusation against the artifact: the listener belonged to a
different process. The same shape of check had the opposite hazard too -- if the
artifact failed to start at all, the port stayed closed and the check passed
vacuously.

The check therefore measures three things instead of one:

  1. BASELINE: is anything already listening on the port before the artifact is
     launched? If so, the run is refused as unattributable rather than blamed on
     the artifact.
  2. ATTRIBUTION: if the port opens, who owns it? Only the launched process, or a
     direct child of it (WebView2 runs its browser process as a child), counts as
     the artifact opening it.
  3. PRESENCE: the process must still be alive at the end of the window, so
     "closed because it crashed immediately" cannot pass as "closed because it
     respects the boundary".

Usage:
  python3 scripts/assert-no-debug-port.py <exe> [port]

Exit status: 0 when the port stayed closed; 1 on any of the three failures above.
"""
from __future__ import annotations

import subprocess
import sys
import time

DEFAULT_PORT = 9222
WINDOW_SECONDS = 5.0
POLL_SECONDS = 0.25


def _powershell(script: str) -> str:
    try:
        out = subprocess.run(
            ["powershell", "-NoProfile", "-Command", script],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=30,
            shell=False,
        )
    except Exception:
        return ""
    return (out.stdout or "").strip()


def listener_pid(port: int) -> int | None:
    """PID owning a LISTENING socket on `port`, or None when nothing listens."""
    text = _powershell(
        f"Get-NetTCPConnection -LocalPort {port} -State Listen "
        f"-ErrorAction SilentlyContinue | Select-Object -First 1 "
        f"-ExpandProperty OwningProcess"
    )
    first = text.splitlines()[0].strip() if text else ""
    return int(first) if first.isdigit() else None


def child_pids(parent: int) -> set[int]:
    text = _powershell(
        f"Get-CimInstance Win32_Process -Filter \"ParentProcessId = {parent}\" "
        f"| Select-Object -ExpandProperty ProcessId"
    )
    return {int(line) for line in text.split() if line.strip().isdigit()}


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: assert-no-debug-port.py <exe> [port]", file=sys.stderr)
        return 2
    exe = sys.argv[1]
    port = int(sys.argv[2]) if len(sys.argv) > 2 else DEFAULT_PORT

    before = listener_pid(port)
    if before is not None:
        print(
            f"assert-no-debug-port: FAIL -- port {port} is ALREADY listening "
            f"(pid {before}) BEFORE {exe} is launched, so a listener on it cannot "
            "be attributed to the artifact. A leftover debug build or an "
            "unrelated process is holding the port; clean the environment and "
            "rerun rather than reading this as a product finding.",
            file=sys.stderr,
        )
        return 1

    proc = subprocess.Popen([exe], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        deadline = time.time() + WINDOW_SECONDS
        owner = None
        while time.time() < deadline:
            time.sleep(POLL_SECONDS)
            owner = listener_pid(port)
            if owner is not None:
                break

        alive = proc.poll() is None
        if owner is None:
            if not alive:
                print(
                    f"assert-no-debug-port: FAIL -- {exe} exited with code "
                    f"{proc.returncode} inside the {WINDOW_SECONDS:.0f}s window, so "
                    "'no debug port' is vacuous rather than verified.",
                    file=sys.stderr,
                )
                return 1
            print(f"assert-no-debug-port: ok -- {exe} opens no port {port}")
            return 0

        # Attribute: the artifact itself, or a child it spawned (WebView2's
        # browser process is a child), owns the listener.
        attributed = owner == proc.pid or owner in child_pids(proc.pid)
        if attributed:
            print(
                f"assert-no-debug-port: FAIL -- {exe} opened debug port {port} "
                f"(pid {owner})",
                file=sys.stderr,
            )
        else:
            print(
                f"assert-no-debug-port: FAIL -- port {port} opened by pid {owner}, "
                f"which is NOT {exe} (pid {proc.pid}) nor a child of it; the check "
                "cannot attribute the listener to the artifact.",
                file=sys.stderr,
            )
        return 1
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            proc.kill()


if __name__ == "__main__":
    raise SystemExit(main())
