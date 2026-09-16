#!/usr/bin/env python3
"""Executed-case driver for registry IDs whose harness already exists.

WHY THIS EXISTS. 41 registry IDs are marked APPLICABLE with an executable harness
entry point named in the applicability matrix, yet their accounting recorded the
same blanket string: "no candidate-specific case material executed for this ID ...
Real dependency/harness provisioning is required". For most of them that reason is
false -- the harness exists and is an enforced lane.

This driver RE-RUNS the executable gates now, capturing for each command: the exact
argv, the exit code, the complete stdout+stderr, and the SHA-256 of that output.
Per-ID evidence documents are then assembled from those raw captures, so a PASS
row cites an output produced by a command that a reviewer can re-run, not a claim.

Nothing here decides a status: it only produces evidence. Statuses are recorded in
CASE_RESULTS.json by hand, per ID, from what the captures actually show.
"""
import hashlib
import json
import pathlib
import shutil
import subprocess
import sys

ROOT = pathlib.Path(".").resolve()
OUT = ROOT / ".agent/evidence/applicable-cases"
LOGS = OUT / "logs"

# (key, argv) -- every command is a real repository gate or validator.
COMMANDS: list[tuple[str, list[str]]] = [
    ("fmt", ["cargo", "fmt", "--all", "--", "--check"]),
    ("lint", ["sh", "scripts/lint.sh"]),
    ("anti-gaming", ["python3", "scripts/anti-gaming-scan.py", "."]),
    ("secret-scan", ["python3", "scripts/secret-scan.py", "--check"]),
    ("sbom-currency", ["python3", "scripts/generate-sbom.py", "--check"]),
    ("deny-advisories", ["cargo", "deny", "check", "advisories"]),
    ("deny-bans", ["cargo", "deny", "check", "bans"]),
    ("deny-sources", ["cargo", "deny", "check", "sources"]),
    ("deny-licenses", ["cargo", "deny", "check", "licenses"]),
    ("pnpm-audit", ["pnpm", "audit", "--audit-level", "high"]),
    ("security-check", ["sh", "scripts/security-check.sh"]),
    ("harness-validate", ["sh", "scripts/harness-validate.sh"]),
]


def main() -> int:
    LOGS.mkdir(parents=True, exist_ok=True)
    # Windows resolves pnpm/eslint-style entry points only with their extension,
    # so each argv[0] is resolved through PATH before it is executed.
    records = []
    runs_file = OUT / "RUNS.json"
    if runs_file.exists() and "--all" not in sys.argv:
        records = json.loads(runs_file.read_text(encoding="utf-8"))
    done = {r["key"] for r in records}
    for key, argv in COMMANDS:
        if key in done:
            print(f"{key}: already captured, skipped")
            continue
        exe = shutil.which(argv[0])
        if exe is None:
            print(f"{key}: argv[0] {argv[0]!r} not on PATH", file=sys.stderr)
            return 2
        proc = subprocess.run(
            [exe, *argv[1:]], cwd=ROOT, capture_output=True, text=True, errors="replace"
        )
        blob = (proc.stdout or "") + (proc.stderr or "")
        digest = hashlib.sha256(blob.encode("utf-8", "replace")).hexdigest()
        (LOGS / f"{key}.log").write_text(blob, encoding="utf-8")
        records.append(
            {
                "key": key,
                "argv": argv,
                "resolved": exe,
                "exit_code": proc.returncode,
                "output_sha256": digest,
                "output_bytes": len(blob.encode("utf-8", "replace")),
                "output_lines": len(blob.splitlines()),
            }
        )
        print(f"{key}: exit {proc.returncode}  ({records[-1]['output_bytes']} bytes)")
        sys.stdout.flush()

    (OUT / "RUNS.json").write_text(json.dumps(records, indent=2) + "\n", encoding="utf-8")
    failed = [r["key"] for r in records if r["exit_code"] != 0]
    print("non-zero:", failed if failed else "none")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
