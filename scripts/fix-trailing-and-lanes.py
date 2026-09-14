#!/usr/bin/env python3
"""Rewrite trailing `[ -f X ] && cmd` lanes under `set -eu` as explicit ifs.

THE BUG (found four times in this repository before the pattern was recognised):

    #!/usr/bin/env bash
    set -eu
    [ -f Cargo.toml ] && cargo fetch --locked
    [ -f package.json ] && pnpm install --frozen-lockfile
    [ -f pyproject.toml ] && uv sync --frozen      # <-- absent in this repo

Under `set -eu`, a false test in an `&&` list IS the command's exit status. So
the script exits 1 whenever the LAST such file is absent -- reporting failure
even when every lane that actually ran passed. That is a gate that lies, and it
masked genuine results in test-unit.sh, security-check.sh, verify.sh and
install.sh. Measured: scripts/install.sh exits 1 while printing "Done in 702ms",
because pyproject.toml does not exist.

This rewrites each such line as:

    if [ -f X ]; then
      cmd
    fi

so the lane is skipped cleanly when its file is absent. The transformation is
deliberately narrow: only lines that are EXACTLY a file test followed by `&&` and
a command are touched, and only in files that already declare `set -eu`. The
`|| { ...; exit N; }` guard form is left alone.

Usage: python3 scripts/fix-trailing-and-lanes.py [--check]
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

SCRIPTS = Path("scripts")
LANE_RE = re.compile(r"^(\s*)\[\s+-([fd])\s+(\"[^\"]+\"|\S+)\s+\]\s*&&\s*(.+?)\s*$")


def main() -> int:
    check_only = "--check" in sys.argv
    offenders: list[tuple[Path, int, str]] = []

    for path in sorted(SCRIPTS.glob("*.sh")):
        text = path.read_text("utf-8")
        if not re.search(r"^set -eu", text, re.M):
            continue
        for i, line in enumerate(text.splitlines(), 1):
            m = LANE_RE.match(line)
            if not m:
                continue
            # Leave the guard form `[ ... ] && ... || { ... }` alone: it already
            # handles its own failure.
            if "||" in line:
                continue
            offenders.append((path, i, line))

    if check_only:
        if offenders:
            for path, i, line in offenders:
                print(f"{path}:{i}: {line.strip()}", file=sys.stderr)
            print(
                f"trailing-and check: FAIL -- {len(offenders)} lane(s) would exit "
                "non-zero when their file is absent",
                file=sys.stderr,
            )
            return 1
        print("trailing-and check: ok (no trailing &&-test lanes)")
        return 0

    changed = 0
    touched = sorted({p for p, _, _ in offenders})
    for path in touched:
        lines = path.read_text("utf-8").splitlines(keepends=True)
        out: list[str] = []
        for line in lines:
            stripped = line.rstrip("\n").rstrip("\r")
            m = LANE_RE.match(stripped)
            if not m or "||" in stripped:
                out.append(line)
                continue
            indent, flag, target, cmd = m.groups()
            # Rebuild the test clause verbatim from the original line, so the
            # file path and any quoting are preserved exactly.
            test_clause = stripped.split("]")[0].lstrip() + " ]"
            out.append(f"{indent}if {test_clause}; then\n")
            out.append(f"{indent}  {cmd}\n")
            out.append(f"{indent}fi\n")
            changed += 1
        path.write_text("".join(out), "utf-8")
        print(f"rewrote {path}")

    print(f"trailing-and: rewrote {changed} lane(s) across {len(touched)} file(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
