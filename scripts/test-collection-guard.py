#!/usr/bin/env python3
"""Verify the test suite actually collected the expected number of tests.

GraphLock context (DOD-007): "The harness fails when zero tests or fewer than
the expected manifest are collected. BECAUSE: Many runners exit zero for empty,
misconfigured, or partially discovered suites."

This was a real, observed defect in this repository: all three JS packages ran
`vitest run --passWithNoTests`, so `pnpm -r test:unit` printed "No test files
found, exiting with code 0" and the whole suite passed green with ZERO tests
collected. See .agent/evidence/DOD-007-collection-guard-finding.md.

This script runs the Rust suite, parses the runner's own per-binary counts, and
fails unless the collected total meets the committed manifest. It refuses to
report success on an empty collection.

Usage:
  python3 scripts/test-collection-guard.py            # run and verify
  python3 scripts/test-collection-guard.py --check    # verify manifest shape only
"""
from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

MANIFEST = Path(".agent/verification/state/TEST_COLLECTION_MANIFEST.json")

# Parsed from `cargo test` output: "test result: ok. N passed; N failed; N ignored"
RESULT_RE = re.compile(
    r"test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored"
)
BINARY_RE = re.compile(r"Running (?:unittests|tests) .*?\(([^)]+)\)")


def main() -> int:
    if "--check" in sys.argv:
        if not MANIFEST.exists():
            print("collection guard: FAIL (manifest missing)", file=sys.stderr)
            return 1
        data = json.loads(MANIFEST.read_text("utf-8"))
        if not data.get("expected_min_passed"):
            print("collection guard: FAIL (manifest has no expected count)", file=sys.stderr)
            return 1
        print(f"collection guard manifest: ok (min {data['expected_min_passed']} passed)")
        return 0

    proc = subprocess.run(
        ["cargo", "test", "--workspace", "--locked"],
        capture_output=True,
        text=True,
    )
    out = proc.stdout + proc.stderr

    suites = RESULT_RE.findall(out)
    if not suites:
        print(
            "collection guard: FAIL -- runner produced no parsable test results. "
            "An empty or misconfigured suite exits zero, so this is not a pass.",
            file=sys.stderr,
        )
        return 1

    passed = sum(int(p) for p, _, _ in suites)
    failed = sum(int(f) for _, f, _ in suites)
    ignored = sum(int(i) for _, _, i in suites)
    binaries = len(BINARY_RE.findall(out))

    manifest = json.loads(MANIFEST.read_text("utf-8")) if MANIFEST.exists() else {}
    expected = manifest.get("expected_min_passed", 0)

    print(f"collection guard: {len(suites)} result lines, {binaries} test binaries")
    print(f"  passed={passed} failed={failed} ignored={ignored} (manifest min={expected})")

    if passed == 0:
        print(
            "collection guard: FAIL -- zero tests collected. DOD-007 forbids "
            "treating this as a pass.",
            file=sys.stderr,
        )
        return 1
    if passed < expected:
        print(
            f"collection guard: FAIL -- collected {passed} below manifest {expected}",
            file=sys.stderr,
        )
        return 1
    if failed:
        print(f"collection guard: FAIL -- {failed} test(s) failed", file=sys.stderr)
        return 1
    if ignored:
        print(
            f"collection guard: FAIL -- {ignored} test(s) ignored; DOD-006 requires "
            "an approved waiver for any skipped test",
            file=sys.stderr,
        )
        return 1
    if proc.returncode != 0:
        print(
            f"collection guard: FAIL -- runner exit {proc.returncode}",
            file=sys.stderr,
        )
        return 1

    print("collection guard: ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
