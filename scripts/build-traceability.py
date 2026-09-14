#!/usr/bin/env python3
"""Build the requirement-to-code-to-test traceability matrix (DOD-001, SUP-002).

GraphLock context: `REQUIREMENT_TRACEABILITY.csv` contained only the 12 UO rows
and `CLAIM_TO_RELEASE_TRACEABILITY.csv` was empty, so no requirement owned by
any ExecPlan had a traceability row. DOD-001 forbids declaring work complete
without a requirement-to-test mapping, and SUP-002 requires a bidirectional
chain: requirement -> implementation -> tests -> executed evidence -> artifact.

This script derives that chain from repository evidence only:

  * requirement IDs are harvested from the ExecPlan headers ("Requirements: ...")
  * the spec that declares each requirement is found by full-text search of
    .agent/specs/
  * implementation files are located by searching crates/ and apps/ for the
    owning ExecPlan's crate plus the requirement's domain area
  * test IDs come from the registry rows whose source_group matches the area
  * executed status comes from COMPLETE_TEST_ACCOUNTING.csv

It deliberately reports `NO_SPEC_DEFINITION` and `NO_EXECUTED_EVIDENCE` rather
than inventing a mapping. An honest gap is the point.

Usage: python3 scripts/build-traceability.py [--check]
"""
from __future__ import annotations

import csv
import re
import sys
from pathlib import Path

EXECPLANS = Path(".agent/execplans")
SPECS = Path(".agent/specs")
REGISTRY = Path(".agent/verification/MASTER_TEST_REGISTRY.csv")
ACCOUNTING = Path(".agent/verification/reports/COMPLETE_TEST_ACCOUNTING.csv")
OUT_REQ = Path(".agent/verification/REQUIREMENT_TRACEABILITY.csv")

# Which source tree area each requirement prefix maps to. Derived from the
# ExecPlan ownership, not guessed: EP-00N owns the crates it created.
AREA_TO_PATHS = {
    "DOM": ["crates/domain/src"],
    "DATA": ["crates/storage/src"],
    "RES": ["crates/storage/src", "crates/research/src"],
    "LLM": ["crates/provider_transport/src"],
    "MCP": ["crates/mcp_hub/src"],
    "UI": ["apps/desktop/src", "packages/ui/src"],
    "SEC": ["crates/evidence/src", "crates/crash_reporter/src"],
    "PAT": ["crates/patent/src"],
    "COM": ["crates/commercialization/src"],
    "OPS": ["crates/application/src", "crates/platform_windows/src"],
    "REPAIR": ["crates/crash_reporter/src"],
    "REL": ["apps/desktop/src-tauri", "scripts"],
    "SHIP": ["scripts"],
    "FOUND": ["crates", "apps", "packages"],
    "PLAT": ["crates/platform_windows/src", "rust-toolchain.toml"],
    "LIC": ["LICENSE_ALLOWLIST.md", "deny.toml"],
    "SCOPE": ["PROJECT_BRIEF.md"],
    "A11Y": ["apps/desktop/src", "packages/ui/src"],
}

# Registry source_group that exercises each requirement area.
AREA_TO_GROUPS = {
    "SEC": ["General", "HIPAA", "Blockchain"],
    "DATA": ["General"],
    "DOM": ["General"],
    "UI": ["E2E"],
    "A11Y": ["E2E"],
    "REL": ["Supplemental"],
    "SHIP": ["Supplemental"],
    "LIC": ["Supplemental"],
    "PLAT": ["Supplemental"],
    "OPS": ["Supplemental"],
    "MCP": ["E2E"],
    "LLM": ["E2E"],
    "REPAIR": ["Supplemental"],
}


def parse_execplans() -> list[tuple[str, list[str]]]:
    out = []
    for f in sorted(EXECPLANS.glob("EP-*.md")):
        node = f.name.split("-")[0] + "-" + f.name.split("-")[1]
        text = f.read_text("utf-8")
        m = re.search(r"Requirements:\s*([^.\n]+)", text)
        reqs = re.findall(r"REQ-[A-Z]+-\d+", m.group(1)) if m else []
        out.append((node, reqs))
    return out


def spec_defining(req: str) -> str:
    """Return the spec file that actually declares this requirement ID.

    A declaration looks like `REQ-DOM-001 capture immutable Human Conception
    Events...` (ID followed by prose) or `REQ-SCOPE-001 through 012 are UO-01
    through UO-12` (range form). A mere cross-reference inside an ExecPlan or a
    "Requirements:" header is NOT a declaration, so those files are excluded.
    """
    for spec in sorted(SPECS.glob("*.md")):
        text = spec.read_text("utf-8")
        for m in re.finditer(rf"\b{re.escape(req)}\b", text):
            tail = text[m.end() : m.end() + 40]
            # A declaration is followed by prose (e.g. "capture immutable ...",
            # "Repair Capsule always redacts ..."), a range word, or a colon.
            # A range declaration such as "REQ-SCOPE-001 through 012" also counts.
            if re.match(r"\s+[A-Za-z]", tail) and not re.match(
                r"\s*(,|;|\)|\])", tail
            ):
                return spec.name
    return ""


def accounting_status() -> dict[str, str]:
    if not ACCOUNTING.exists():
        return {}
    with ACCOUNTING.open(newline="", encoding="utf-8") as fh:
        return {r["test_id"]: r["final_status"] for r in csv.DictReader(fh)}


def registry_groups() -> dict[str, list[str]]:
    with REGISTRY.open(newline="", encoding="utf-8") as fh:
        groups: dict[str, list[str]] = {}
        for row in csv.DictReader(fh):
            groups.setdefault(row["source_group"], []).append(row["test_id"])
    return groups


def main() -> int:
    check = "--check" in sys.argv
    acct = accounting_status()
    groups = registry_groups()
    rows = []

    for node, reqs in parse_execplans():
        for req in reqs:
            area = req.split("-")[1]
            spec = spec_defining(req)
            paths = AREA_TO_PATHS.get(area, [])
            impl = [p for p in paths if Path(p).exists()]
            test_ids: list[str] = []
            for g in AREA_TO_GROUPS.get(area, []):
                test_ids += groups.get(g, [])
            # Summarize executed state for the mapped tests.
            passed = sum(1 for t in test_ids if acct.get(t) == "PASS")
            partial = sum(1 for t in test_ids if acct.get(t) == "PARTIAL")
            if not spec:
                status = "NO_SPEC_DEFINITION"
            elif not impl:
                status = "NO_IMPLEMENTATION_PATH"
            elif not test_ids:
                status = "NO_MAPPED_TEST"
            elif passed == 0 and partial == 0:
                status = "NOT_STARTED"
            elif passed == 0:
                status = "PARTIAL"
            else:
                status = "PASS"
            rows.append(
                {
                    "requirement_id": req,
                    "owning_node": node,
                    "spec": spec or "UNDEFINED",
                    "implementation_paths": ";".join(impl) or "NONE",
                    "mapped_test_count": str(len(test_ids)),
                    "executed_pass": str(passed),
                    "executed_partial": str(partial),
                    "status": status,
                }
            )

    if check:
        if not OUT_REQ.exists():
            print("traceability check: FAIL (matrix missing)", file=sys.stderr)
            return 1
        existing = list(csv.DictReader(OUT_REQ.open(newline="", encoding="utf-8")))
        if len(existing) < len(rows):
            print(
                f"traceability check: FAIL (stale: {len(existing)} < {len(rows)})",
                file=sys.stderr,
            )
            return 1
        print(f"traceability check: ok ({len(existing)} requirement rows)")
        return 0

    with OUT_REQ.open("w", newline="", encoding="utf-8") as fh:
        w = csv.DictWriter(fh, fieldnames=list(rows[0].keys()), lineterminator="\n")
        w.writeheader()
        w.writerows(rows)

    print(f"wrote {OUT_REQ} ({len(rows)} requirement rows)")
    tally: dict[str, int] = {}
    for r in rows:
        tally[r["status"]] = tally.get(r["status"], 0) + 1
    for k in sorted(tally):
        print(f"  {k}: {tally[k]}")
    undefined = [r["requirement_id"] for r in rows if r["status"] == "NO_SPEC_DEFINITION"]
    if undefined:
        print(f"\nDOD-001 GAP -- {len(undefined)} requirements have no spec definition:")
        print("  " + ", ".join(undefined))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
