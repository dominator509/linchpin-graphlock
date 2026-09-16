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
    expected_binaries = manifest.get("expected_test_binaries", 0)

    print(f"collection guard: {len(suites)} result lines, {binaries} test binaries")
    print(f"  passed={passed} failed={failed} ignored={ignored} (manifest min={expected})")

    if passed == 0:
        print(
            "collection guard: FAIL -- zero tests collected. DOD-007 forbids "
            "treating this as a pass.",
            file=sys.stderr,
        )
        return 1
    # A partial collection means the runner did not report on every test binary.
    # MEASURED: running this guard while another `cargo test --workspace` held the
    # target directory produced "collected 130 below manifest 216" -- a scary
    # message for an artifact of contention, and one that would be indistinguishable
    # from a genuinely shrinking suite. The binary count separates the two, so the
    # diagnosis names which one it is instead of guessing.
    if expected_binaries and binaries < expected_binaries:
        print(
            f"collection guard: FAIL -- the runner reported on only {binaries} test "
            f"binary/binaries while the manifest expects {expected_binaries}, so this "
            f"collection is PARTIAL ({passed} tests seen) rather than a smaller suite. "
            "Most likely another cargo run held the target directory; rerun this guard "
            "with nothing else building.",
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

    # The manifest used to assert `javascript: {test_files: 0}` with a note
    # claiming no JS unit test files exist, while three vitest files were
    # passing. A guard that only reads the Rust runner cannot notice that, so the
    # JS claim is now checked against the tree: the manifest may not under-report
    # how many JS test files exist.
    js_files = [
        p
        for p in Path(".").glob("**/*.test.ts")
        if "node_modules" not in p.parts and "dist" not in p.parts
    ]
    recorded = (manifest.get("javascript") or {}).get("test_files")
    print(f"  javascript: {len(js_files)} test file(s) in the tree, manifest records {recorded}")
    if js_files and (recorded is None or recorded < len(js_files)):
        print(
            "collection guard: FAIL -- the manifest under-reports the JS lane "
            f"({recorded} recorded vs {len(js_files)} present). Regenerate with "
            "scripts/update-test-manifest.py.",
            file=sys.stderr,
        )
        return 1
    if js_files and not (manifest.get("javascript") or {}).get("tests"):
        print(
            "collection guard: FAIL -- the manifest records no JS test count while "
            "JS test files exist.",
            file=sys.stderr,
        )
        return 1

    print("collection guard: ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
