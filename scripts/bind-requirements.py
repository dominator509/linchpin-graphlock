#!/usr/bin/env python3
"""Bind requirements to EXECUTED tests (DOD-001).

GraphLock context: `REQUIREMENT_TRACEABILITY.csv` mapped each requirement to a
count of *registry IDs*, but those registry IDs are overwhelmingly
NOT_RUN_BLOCKED_MATERIAL -- so the matrix proved nothing about executed
behaviour. 128 named tests actually run, and none of them was bound to a
requirement. DOD-001 requires "a stable requirement ID and at least one
acceptance test", with "executed acceptance evidence".

This script closes that gap mechanically:

  1. Enumerate every test the runner actually collects.
  2. Extract the requirement IDs each test covers, from an explicit marker in
     the test's own source (`REQ-XXX-NNN` appearing in the test body or its
     doc comment). No inference, no fuzzy matching: an unmarked test is
     unbound, and the report says so.
  3. Emit a matrix of requirement -> executed tests -> pass/fail -> file.

A requirement with no bound test is reported as NO_BOUND_TEST rather than
being credited with the registry count.
"""
from __future__ import annotations

import csv
import json
import re
import subprocess
import sys
from pathlib import Path

OUT = Path(".agent/verification/REQUIREMENT_TRACEABILITY.csv")
LEDGER = Path(".agent/verification/state/REQUIREMENT_TEST_BINDINGS.jsonl")

REQ_RE = re.compile(r"\bREQ-[A-Z]+-\d+\b")
TEST_FN_RE = re.compile(r"^\s*(?:async\s+)?fn\s+([a-z0-9_]+)\s*\(", re.MULTILINE)
# Playwright: test("title", async ({ page }) => { ... });
E2E_TEST_RE = re.compile(r"""^\s*test\(\s*["'`]([^"'`]+)["'`]""", re.MULTILINE)

SEARCH_DIRS = [Path("crates"), Path("apps"), Path("packages")]
EXCLUDED = {"node_modules", "target", "dist", "gen", "build"}


def collect_test_ids() -> set[str]:
    """Every test the Rust runner actually collects."""
    proc = subprocess.run(
        ["cargo", "test", "--workspace", "--locked", "--", "--list"],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    out = proc.stdout + proc.stderr
    ids = set()
    for line in out.splitlines():
        line = line.strip()
        if line.endswith(": test"):
            ids.add(line[: -len(": test")])
    return ids


def find_test_sources() -> list[Path]:
    files = []
    for root in SEARCH_DIRS:
        if not root.exists():
            continue
        for path in root.rglob("*.rs"):
            if set(path.parts) & EXCLUDED:
                continue
            files.append(path)
    return files


def crate_module_prefix(path: Path) -> str | None:
    """Module path contributed by the file's own location.

    A test in `apps/desktop/src-tauri/src/commands.rs` is collected as
    `commands::tests::name`, not `tests::name`: the file itself is a module of
    the crate root. Attributing it without that prefix made every collected test
    look uncollected.

    A file under `tests/` is different: it is an INTEGRATION test crate root, so
    cargo lists its functions bare (`name: test`), not `file::name: test`.
    Measured: `crates/platform_windows/tests/licensing_policy.rs` yielded
    `test_dependency_versions_are_pinned_and_never_floated: test` with no prefix,
    so applying one produced `licensing_policy::test_...` and four requirements
    were reported BOUND_NOT_COLLECTED even though their tests ran and passed.
    Both cases are crate roots and contribute no prefix.
    """
    parts = list(path.with_suffix("").parts)
    name = parts[-1]
    if name in {"lib", "main", "mod"}:
        return None
    # `<crate>/tests/<file>.rs` is an integration-test crate root.
    if len(parts) >= 2 and parts[-2] == "tests":
        return None
    return name


def find_e2e_specs() -> list[Path]:
    """Playwright specs under apps/desktop/e2e."""
    root = Path("apps/desktop/e2e")
    if not root.exists():
        return []
    return sorted(root.rglob("*.spec.ts"))


def attached_comment_block(text: str, start: int) -> str:
    """The run of comment lines immediately above position `start`.

    Only lines contiguous with the declaration count: the block ends at the
    first non-comment line. This is what makes attribution unambiguous. An
    earlier version searched from the preceding blank line to the NEXT test's
    declaration, so a test carrying no marker of its own inherited the marker
    written above the following test -- measured, REQ-UI-001 was credited to the
    receipt-import test as well as to the navigation test.
    """
    block: list[str] = []
    for line in reversed(text[:start].splitlines()):
        stripped = line.strip()
        if (
            stripped.startswith("//")
            or stripped.startswith("/*")
            or stripped.startswith("*")
            or stripped.endswith("*/")
        ):
            block.append(line)
            continue
        break
    return "\n".join(reversed(block))


def bind_e2e_requirements() -> dict[str, list[dict[str, str]]]:
    """Map Playwright test titles -> requirement IDs declared inside that test.

    Defect this fixes: the binder only scanned `*.rs` and only collected IDs via
    `cargo test --list`, so the 14 executed Playwright acceptance tests were
    invisible. Requirements they genuinely cover (REQ-UI-002, REQ-UI-003) were
    reported NO_BOUND_TEST even though real, passing, executed tests existed.
    Coverage was UNDERSTATED -- the same class of measurement error as the SBOM
    undercount and the `cargo build` vs `tauri build` mistake.

    Attribution is per-test, not per-file: a REQ marker is counted only when it
    appears inside that test's own body. File-level docblock mentions are
    deliberately NOT counted, because they assert suite-wide scope and would
    bind requirements to tests that do not exercise them.
    """
    bindings: dict[str, list[dict[str, str]]] = {}

    for path in find_e2e_specs():
        text = path.read_text("utf-8", errors="replace")
        starts = list(E2E_TEST_RE.finditer(text))
        for idx, match in enumerate(starts):
            title = match.group(1)
            start = match.start()

            # Attribute the comment block ATTACHED to this declaration, so a
            # `covers:` marker belongs to the test it sits above and to no other.
            body = attached_comment_block(text, start)

            marker = re.search(
                r"(?://|/\*|\*)\s*(?:covers|verifies|reqs?)\s*:\s*([^\n*]*)",
                body,
                re.IGNORECASE,
            )
            if not marker:
                continue
            reqs = sorted(set(REQ_RE.findall(marker.group(1))))
            for req in reqs:
                bindings.setdefault(req, []).append(
                    {"test_id": f"e2e::{title}", "file": str(path).replace("\\", "/")}
                )
    return bindings


def collect_e2e_test_ids() -> set[str]:
    """Playwright test IDs, read from its JSON reporter output when present.

    Only tests Playwright actually RAN count as collected. Presence in a spec
    file is not execution, so without a report the set is empty and any bound
    E2E requirement is reported BOUND_NOT_COLLECTED rather than PASS -- the
    honest outcome when the browser lane has not run for this candidate.

    The walker follows Playwright's real report shape: `suites[]` nest other
    suites AND hold `specs[]`, and each spec carries the test title and `ok`.
    An earlier version iterated suites looking for `tests` directly on them and
    therefore never reached a single spec -- it returned an empty set, which
    went unnoticed because no report existed to read. Fixed once the JSON
    reporter was added and the empty result became visible.
    """
    report = Path("apps/desktop/e2e-report.json")
    if not report.exists():
        report = Path(".agent/state/e2e-report.json")
    if not report.exists():
        return set()
    try:
        data = json.loads(report.read_text("utf-8", errors="replace"))
    except (json.JSONDecodeError, OSError):
        return set()
    ids: set[str] = set()

    def walk_suites(suites: list) -> None:
        for suite in suites or []:
            for spec in suite.get("specs", []) or []:
                title = spec.get("title")
                if title:
                    ids.add(f"e2e::{title}")
            # Nested describes.
            walk_suites(suite.get("suites", []) or [])

    walk_suites(data.get("suites", []) or [])
    return ids


def bind_requirements() -> dict[str, list[dict[str, str]]]:
    """Map `module::test_fn` -> the requirement IDs declared in its body."""
    bindings: dict[str, list[dict[str, str]]] = {}

    for path in find_test_sources():
        text = path.read_text("utf-8", errors="replace")
        # Only consider test modules; a production comment mentioning a REQ is
        # not a test binding.
        if "#[cfg(test)]" not in text and "#[test]" not in text and "#[tokio::test]" not in text:
            continue

        # Split the file into functions preceded by attributes.
        positions = list(TEST_FN_RE.finditer(text))
        for idx, match in enumerate(positions):
            fn_name = match.group(1)
            start = match.start()
            end = positions[idx + 1].start() if idx + 1 < len(positions) else len(text)

            # Include the attribute block immediately above the fn.
            attr_start = text.rfind("\n\n", 0, start)
            attr_start = 0 if attr_start == -1 else attr_start
            body = text[attr_start:end]

            if "#[test]" not in body and "#[tokio::test]" not in body:
                continue

            # Require an explicit binding marker. A bare `REQ-XXX-NNN` anywhere
            # in the body is not enough: helper functions such as `temp_db` and
            # production functions such as `export_evidence` mention requirement
            # IDs in doc comments, and counting those produced phantom bindings
            # that were never collected.
            #
            # Accepted marker forms, both immediately above the test:
            #     /// covers: REQ-DOM-001, REQ-DOM-002
            #     // covers: REQ-DOM-001
            marker = re.search(
                r"(?://[/!]?|#)\s*(?:covers|verifies|reqs?)\s*:\s*([^\n]*)",
                body,
                re.IGNORECASE,
            )
            if not marker:
                continue
            reqs = sorted(set(REQ_RE.findall(marker.group(1))))
            if not reqs:
                continue

            # Find the INNERMOST enclosing module, not the first one in the
            # file. Taking the first `mod` match attributed every test in a
            # multi-module file (e.g. commands.rs) to the wrong module, which
            # made collected tests look uncollected under a bogus id like
            # "fmt::..." or "default::...".
            module = None
            depth_stack: list[tuple[int, str]] = []
            for m in re.finditer(r"\bmod\s+([a-z0-9_]+)\s*\{", text[:start]):
                # Track brace depth so a closed module is popped.
                open_brace = text.index("{", m.start())
                depth = text.count("{", 0, open_brace) - text.count("}", 0, open_brace)
                while depth_stack and depth_stack[-1][0] >= depth:
                    depth_stack.pop()
                depth_stack.append((depth, m.group(1)))
            if depth_stack:
                module = depth_stack[-1][1]

            # Prefix with the file's own module segment, which the crate root
            # contributes implicitly.
            file_prefix = crate_module_prefix(path)
            segments = [s for s in (file_prefix, module) if s]
            test_id = "::".join([*segments, fn_name]) if segments else fn_name

            for req in reqs:
                bindings.setdefault(req, []).append(
                    {
                        "test_id": test_id,
                        "file": str(path).replace("\\", "/"),
                        "fn": fn_name,
                    }
                )
    return bindings


def parse_results() -> dict[str, str]:
    """test_id -> PASS/FAIL from a real run, across both runners."""
    proc = subprocess.run(
        ["cargo", "test", "--workspace", "--locked"],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    out = proc.stdout + proc.stderr
    results: dict[str, str] = {}
    for line in out.splitlines():
        line = line.strip()
        m = re.match(r"^test (\S+) \.\.\. (ok|FAILED|ignored)$", line)
        if m:
            results[m.group(1)] = {
                "ok": "PASS",
                "FAILED": "FAIL",
                "ignored": "IGNORED",
            }[m.group(2)]

    # E2E outcomes. Without these, an E2E-bound requirement has a collected test
    # and no RESULT, which the caller reports as PARTIAL -- indistinguishable
    # from a test that ran and did not pass. Measured: all four REQ-UI rows sat
    # at PARTIAL with exec=1 pass=0 while their Playwright tests were green.
    report = Path("apps/desktop/e2e-report.json")
    if not report.exists():
        report = Path(".agent/state/e2e-report.json")
    if report.exists():
        try:
            data = json.loads(report.read_text("utf-8", errors="replace"))
        except (json.JSONDecodeError, OSError):
            data = {}

        def walk_suites(suites: list) -> None:
            for suite in suites or []:
                for spec in suite.get("specs", []) or []:
                    title = spec.get("title")
                    if title:
                        results[f"e2e::{title}"] = (
                            "PASS" if spec.get("ok") else "FAIL"
                        )
                walk_suites(suite.get("suites", []) or [])

        walk_suites(data.get("suites", []) or [])
    return results


def all_requirements() -> list[str]:
    reqs: set[str] = set()
    for path in [Path(".agent/execplans"), Path(".agent/specs")]:
        for f in path.glob("*.md"):
            reqs |= set(REQ_RE.findall(f.read_text("utf-8", errors="replace")))
    return sorted(reqs)


def main() -> int:
    check_only = "--check" in sys.argv

    collected = collect_test_ids()
    bindings = bind_requirements()

    # Merge Playwright bindings. A requirement bound by BOTH a Rust test and an
    # E2E test keeps both, so the row reflects every executed acceptance test.
    for req, items in bind_e2e_requirements().items():
        bindings.setdefault(req, []).extend(items)

    collected |= collect_e2e_test_ids()
    results = parse_results()
    requirements = all_requirements()

    rows = []
    for req in requirements:
        bound = bindings.get(req, [])
        executed = [b for b in bound if b["test_id"] in collected]
        passed = [b for b in executed if results.get(b["test_id"]) == "PASS"]
        failed = [b for b in executed if results.get(b["test_id"]) == "FAIL"]

        if not bound:
            status = "NO_BOUND_TEST"
        elif failed:
            status = "FAIL"
        elif not executed:
            status = "BOUND_NOT_COLLECTED"
        elif len(passed) == len(executed):
            status = "PASS"
        else:
            status = "PARTIAL"

        rows.append(
            {
                "requirement_id": req,
                "bound_tests": len(bound),
                "executed_tests": len(executed),
                "passed_tests": len(passed),
                "failed_tests": len(failed),
                "status": status,
                "test_ids": ";".join(sorted({b["test_id"] for b in executed})),
                "test_files": ";".join(sorted({b["file"] for b in executed})),
            }
        )

    if check_only:
        if not OUT.exists():
            print("requirement binding check: FAIL (matrix missing)", file=sys.stderr)
            return 1
        existing = list(csv.DictReader(OUT.open(newline="", encoding="utf-8")))
        if len(existing) != len(rows):
            print(
                f"requirement binding check: FAIL ({len(existing)} rows, "
                f"recomputed {len(rows)})",
                file=sys.stderr,
            )
            return 1
        print(f"requirement binding check: ok ({len(rows)} requirements)")
        return 0

    with OUT.open("w", newline="", encoding="utf-8") as fh:
        w = csv.DictWriter(fh, fieldnames=list(rows[0].keys()), lineterminator="\n")
        w.writeheader()
        w.writerows(rows)

    with LEDGER.open("w", encoding="utf-8") as fh:
        for row in rows:
            for b in bindings.get(row["requirement_id"], []):
                fh.write(
                    json.dumps(
                        {
                            "requirement_id": row["requirement_id"],
                            "test_id": b["test_id"],
                            "file": b["file"],
                            "collected": b["test_id"] in collected,
                            "result": results.get(b["test_id"], "NOT_RUN"),
                        },
                        sort_keys=True,
                    )
                    + "\n"
                )

    tally: dict[str, int] = {}
    for r in rows:
        tally[r["status"]] = tally.get(r["status"], 0) + 1

    print(f"wrote {OUT} ({len(rows)} requirements)")
    print(f"wrote {LEDGER}")
    print(f"  collected tests      = {len(collected)}")
    print(f"  requirements found   = {len(requirements)}")
    print(f"  requirements bound   = {len([r for r in rows if r['bound_tests']])}")
    for k in sorted(tally):
        print(f"  {k}: {tally[k]}")
    unbound = [r["requirement_id"] for r in rows if r["status"] == "NO_BOUND_TEST"]
    if unbound:
        print(f"\n  UNBOUND REQUIREMENTS ({len(unbound)}): {', '.join(unbound)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
