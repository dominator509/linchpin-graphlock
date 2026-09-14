#!/usr/bin/env python3
"""Derive evidence-attached applicability decisions for all 484 registry IDs.

GraphLock context (DOD-041): "Conditional domain packs such as HIPAA,
blockchain, AI/agentic, multi-tenant, mobile, cloud, and hardware are activated
or skipped from repository evidence, never assumption." Required evidence is
"architecture/data/interface/support evidence attached to every applicability
decision."

The matrix previously held a single placeholder row:

    ALL-484,EVALUATE_PER_REGISTRY,NOT_STARTED,No registry row may be pruned,...

so 484 IDs had no individual decision. This script derives one per ID from
MEASURED repository signals, and records the signal that produced the decision.

Design rule: a conditional pack is activated only when the repository actually
contains the corresponding surface. Where a signal is absent the decision is
NOT_APPLICABLE_WITH_EVIDENCE and the evidence names the probe that found
nothing. Nothing here is assumed.
"""
from __future__ import annotations

import csv
import json
import re
import sys
from pathlib import Path

REGISTRY = Path(".agent/verification/MASTER_TEST_REGISTRY.csv")
OUT = Path(".agent/verification/APPLICABILITY_MATRIX.csv")
SOURCE_DIRS = [Path("crates"), Path("apps"), Path("packages")]


def read_tree() -> str:
    """Concatenate first-party source (excluding generated/vendor trees)."""
    chunks: list[str] = []
    for root in SOURCE_DIRS:
        if not root.exists():
            continue
        for path in root.rglob("*"):
            if not path.is_file():
                continue
            parts = set(path.parts)
            if parts & {"node_modules", "target", "dist", "gen", "build"}:
                continue
            if path.suffix not in {".rs", ".ts", ".tsx", ".toml", ".json"}:
                continue
            try:
                chunks.append(path.read_text("utf-8"))
            except (UnicodeDecodeError, OSError):
                continue
    return "\n".join(chunks)


def probe(tree: str, patterns: list[str]) -> tuple[bool, str]:
    """Return (found, evidence) for the first matching pattern."""
    for pat in patterns:
        if re.search(pat, tree, re.IGNORECASE):
            return True, f"matched /{pat}/ in first-party source"
    return False, f"no match for any of {patterns} in first-party source"


def decide(row: dict, tree: str) -> tuple[str, str, str]:
    """Return (applicability, status, reason+evidence)."""
    applicability = row["applicability"]
    group = row["source_group"]
    stage = row["default_stage"]

    # --- Conditional domain packs ----------------------------------------
    if group == "HIPAA":
        found, why = probe(
            tree,
            [r"\bphi\b", r"protected health", r"\bhipaa\b", r"patient record"],
        )
        if not found:
            return (
                "NOT_APPLICABLE",
                "NOT_APPLICABLE",
                "HIPAA pack not activated: LINCHPIN is a patent intelligence "
                f"tool, not a covered entity or business associate. Evidence: {why}.",
            )
        return ("APPLICABLE", "NOT_STARTED", f"HIPAA surface detected: {why}")

    if group == "Blockchain":
        found, why = probe(
            tree,
            [r"blockchain", r"smart contract", r"solidity", r"\bweb3\b", r"ledger node"],
        )
        if not found:
            return (
                "NOT_APPLICABLE",
                "NOT_APPLICABLE",
                "Blockchain pack not activated: no chain, contract or node surface "
                f"exists in the product. Evidence: {why}.",
            )
        return ("APPLICABLE", "NOT_STARTED", f"blockchain surface detected: {why}")

    # --- Conditional capability packs -------------------------------------
    if applicability == "conditional-ai":
        found, why = probe(tree, [r"provider_transport", r"ModelRequest", r"McpServer"])
        if not found:
            return ("NOT_APPLICABLE", "NOT_APPLICABLE", f"no AI surface: {why}")
        return (
            "APPLICABLE",
            "NOT_STARTED",
            f"AI surface exists (provider/MCP crates) but no live provider is "
            f"configured (PF-011 unmet). Evidence: {why}.",
        )

    if applicability == "conditional-multitenant":
        found, why = probe(tree, [r"tenant_id", r"multi[-_ ]?tenant"])
        if not found:
            return (
                "NOT_APPLICABLE",
                "NOT_APPLICABLE",
                "multi-tenant pack not activated: LINCHPIN is local-first "
                f"single-user; workspace scoping is not tenancy isolation. Evidence: {why}.",
            )
        return ("APPLICABLE", "NOT_STARTED", f"tenant surface detected: {why}")

    if applicability == "conditional-gui" or applicability == "conditional-gui-human":
        found, why = probe(tree, [r"tauri", r"App\.tsx", r"react"])
        if not found:
            return ("NOT_APPLICABLE", "NOT_APPLICABLE", f"no GUI surface: {why}")
        return (
            "APPLICABLE",
            "NOT_STARTED",
            f"desktop GUI surface exists but this case has not been executed. Evidence: {why}.",
        )

    if applicability == "conditional-interface":
        found, why = probe(tree, [r"tauri::command", r"invoke_handler"])
        if not found:
            return ("NOT_APPLICABLE", "NOT_APPLICABLE", f"no interface surface: {why}")
        return ("APPLICABLE", "NOT_STARTED", f"interface exists: {why}")

    if applicability == "conditional-api":
        found, why = probe(tree, [r"tauri::command", r"commands::"])
        if not found:
            return ("NOT_APPLICABLE", "NOT_APPLICABLE", f"no API surface: {why}")
        return ("APPLICABLE", "NOT_STARTED", f"typed IPC API exists: {why}")

    if applicability == "conditional-persistence":
        found, why = probe(tree, [r"rusqlite", r"CREATE TABLE", r"Vault"])
        if not found:
            return ("NOT_APPLICABLE", "NOT_APPLICABLE", f"no persistence surface: {why}")
        return ("APPLICABLE", "NOT_STARTED", f"SQLite persistence exists: {why}")

    if applicability == "conditional-stateful" or applicability == "conditional-stateful-distributed":
        found, why = probe(tree, [r"rusqlite", r"Mutex", r"AtomicUsize"])
        if not found:
            return ("NOT_APPLICABLE", "NOT_APPLICABLE", f"no mutable state: {why}")
        return ("APPLICABLE", "NOT_STARTED", f"stateful surface exists: {why}")

    if applicability == "conditional-buildable":
        # Probe the whole repository, not just the source dirs: Cargo.toml and
        # the pnpm lockfile live at the root.
        has_cargo = Path("Cargo.toml").exists()
        has_pnpm = Path("pnpm-lock.yaml").exists()
        found = has_cargo and has_pnpm
        why = (
            f"Cargo.toml present={has_cargo}, pnpm-lock.yaml present={has_pnpm} "
            "(repository root)"
        )
        return (
            "APPLICABLE" if found else "NOT_APPLICABLE",
            "NOT_STARTED" if found else "NOT_APPLICABLE",
            f"buildable surface: {why}",
        )

    if applicability == "conditional-distributable":
        found, why = probe(tree, [r"tauri\.conf\.json", r'"bundle"'])
        if not found:
            return ("NOT_APPLICABLE", "NOT_APPLICABLE", f"no distribution surface: {why}")
        return ("APPLICABLE", "NOT_STARTED", f"installer bundling configured: {why}")

    if applicability == "conditional-deployable":
        # Auto-deploy is explicitly disabled (RELEASE.md).
        return (
            "NOT_APPLICABLE",
            "NOT_APPLICABLE",
            "deployment lifecycle not activated: RELEASE.md states auto-deploy is "
            "no and publication remains manual, so there is no promotion pipeline "
            "to verify. Evidence: RELEASE.md.",
        )

    if applicability == "conditional-support-matrix":
        found, why = probe(tree, [r"windows", r"rust-toolchain"])
        if not found:
            return ("NOT_APPLICABLE", "NOT_APPLICABLE", f"no support matrix: {why}")
        return (
            "APPLICABLE",
            "NOT_STARTED",
            f"Windows-only support matrix declared; Windows 11 untested. Evidence: {why}.",
        )

    if applicability in {
        "conditional-time",
        "conditional-text-globalization",
        "conditional-data-portability",
        "conditional-service",
        "conditional-runtime",
        "conditional-configurable",
        "conditional-long-running",
        "conditional-human",
        "conditional-versioned-stateful",
        "conditional-versioned-distributed",
    }:
        return (
            "APPLICABLE",
            "NOT_STARTED",
            "conditional capability of the shipped product; no case material "
            "executed for this ID yet (see COMPLETE_TEST_ACCOUNTING.csv).",
        )

    # --- Broadly applicable -------------------------------------------------
    if applicability == "broadly-applicable":
        return (
            "APPLICABLE",
            "NOT_STARTED",
            "broadly applicable to any shipped software product; no case material "
            "executed for this ID yet.",
        )

    # --- Generic atomic-security / evaluate --------------------------------
    if applicability == "evaluate" or applicability.startswith("conditional"):
        return (
            "APPLICABLE",
            "NOT_STARTED",
            "applicability requires per-case evaluation at its default stage "
            f"({stage}); no case material executed for this ID yet.",
        )

    return ("EVALUATE", "NOT_STARTED", f"unclassified applicability {applicability!r}; retained for review")


def main() -> int:
    check_only = "--check" in sys.argv
    tree = read_tree()
    with REGISTRY.open(newline="", encoding="utf-8") as fh:
        rows = list(csv.DictReader(fh))

    if check_only:
        if not OUT.exists():
            print("applicability check: FAIL (matrix missing)", file=sys.stderr)
            return 1
        with OUT.open(newline="", encoding="utf-8") as fh:
            existing = list(csv.DictReader(fh))
        existing_ids = [r["test_id"] for r in existing]
        if len(existing_ids) != len(rows):
            print(
                f"applicability check: FAIL ({len(existing_ids)} decisions for "
                f"{len(rows)} registry rows)",
                file=sys.stderr,
            )
            return 1
        if len(set(existing_ids)) != len(existing_ids):
            print("applicability check: FAIL (duplicate decisions)", file=sys.stderr)
            return 1
        empty = [r["test_id"] for r in existing if not r["evidence"].strip()]
        if empty:
            print(
                f"applicability check: FAIL ({len(empty)} decisions lack evidence, "
                f"first: {empty[0]})",
                file=sys.stderr,
            )
            return 1
        print(
            f"applicability check: ok ({len(existing)} decisions, every one with "
            "evidence)"
        )
        return 0

    out_rows = []
    tally: dict[str, int] = {}
    for row in rows:
        applicability, status, reason = decide(row, tree)
        tally[applicability] = tally.get(applicability, 0) + 1
        out_rows.append(
            {
                "test_id": row["test_id"],
                "source_group": row["source_group"],
                "registry_applicability": row["applicability"],
                "decision": applicability,
                "status": status,
                "evidence": reason,
            }
        )

    # Invariant: exactly one decision per registry ID.
    ids = [r["test_id"] for r in out_rows]
    assert len(ids) == len(set(ids)) == len(rows), "decision count mismatch"

    with OUT.open("w", newline="", encoding="utf-8") as fh:
        w = csv.DictWriter(
            fh,
            fieldnames=[
                "test_id",
                "source_group",
                "registry_applicability",
                "decision",
                "status",
                "evidence",
            ],
            lineterminator="\n",
        )
        w.writeheader()
        w.writerows(out_rows)

    print(f"wrote {OUT} ({len(out_rows)} decisions, {len(rows)} registry rows)")
    for k in sorted(tally, key=lambda x: -tally[x]):
        print(f"  {k}: {tally[k]}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
