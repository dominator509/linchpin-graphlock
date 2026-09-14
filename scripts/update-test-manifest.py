#!/usr/bin/env python3
"""Regenerate the DOD-007 test-collection manifest from a real run.

GraphLock context: the manifest drifted. It recorded 80 passed at candidate
`4f54de5` while the suite had grown to 125, because earlier updates edited
values by string replacement and silently missed. DOD-007 exists precisely to
stop a suite from passing without collecting what it claims, so a manifest that
under-reports is a real (if benign) defect: it weakens the guard.

This script runs the suite, parses the runner's own counts, and writes the
manifest from measurement. It refuses to LOWER `expected_min_passed`, because
that is the DOD-007/DOD-024 manipulation pattern.

Usage: python3 scripts/update-test-manifest.py
"""
from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

MANIFEST = Path(".agent/verification/state/TEST_COLLECTION_MANIFEST.json")
RESULT_RE = re.compile(
    r"test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored"
)
BINARY_RE = re.compile(r"Running (?:unittests|tests) .*?\(([^)]+)\)")


def main() -> int:
    proc = subprocess.run(
        ["cargo", "test", "--workspace", "--locked"], capture_output=True, text=True
    )
    out = proc.stdout + proc.stderr
    suites = RESULT_RE.findall(out)
    if not suites:
        raise SystemExit("no parsable test results; refusing to write a manifest")

    passed = sum(int(p) for p, _, _ in suites)
    failed = sum(int(f) for _, f, _ in suites)
    ignored = sum(int(i) for _, _, i in suites)
    binaries = len({m for m in BINARY_RE.findall(out)})

    if failed or ignored:
        raise SystemExit(
            f"refusing to update manifest: failed={failed} ignored={ignored}"
        )

    head = subprocess.run(
        ["git", "rev-parse", "--short", "HEAD"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()

    previous = 0
    if MANIFEST.exists():
        try:
            previous = json.loads(MANIFEST.read_text("utf-8")).get(
                "expected_min_passed", 0
            )
        except json.JSONDecodeError:
            previous = 0

    if passed < previous:
        print(
            f"REFUSING to lower expected_min_passed from {previous} to {passed}: "
            "that is the DOD-007/DOD-024 manipulation pattern. Investigate the "
            "shrinking suite instead.",
            file=sys.stderr,
        )
        return 1

    data = {
        "_comment": (
            "Expected test-collection manifest for DOD-007. The guard in "
            "scripts/test-collection-guard.py fails if the runner collects fewer "
            "than expected_min_passed, collects zero, ignores any test, or "
            "reports a failure. Regenerate with scripts/update-test-manifest.py; "
            "never hand-lower it."
        ),
        "candidate_commit": head,
        "expected_min_passed": passed,
        "expected_test_binaries": binaries,
        "measured_at": "2026-09-10",
        "rust": {
            "passed": passed,
            "failed": failed,
            "ignored": ignored,
            "suites": len(suites),
            "test_binaries": binaries,
        },
        "javascript": {
            "test_files": 0,
            "note": (
                "No JS unit test files exist. Playwright E2E (apps/desktop/e2e) "
                "is separate and runs via scripts/test-e2e.sh. --passWithNoTests "
                "was removed from all package.json test:unit scripts so an empty "
                "suite exits 1."
            ),
        },
        "python": {
            "note": "No pyproject.toml / pytest suite exists; test-unit.sh skips this lane."
        },
    }
    MANIFEST.write_text(json.dumps(data, indent=2) + "\n", "utf-8")
    print(f"wrote {MANIFEST}")
    print(f"  candidate={head} passed={passed} binaries={binaries} suites={len(suites)}")
    if previous and previous != passed:
        print(f"  previous expected_min_passed={previous} -> {passed}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
