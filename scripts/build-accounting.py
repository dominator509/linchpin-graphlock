#!/usr/bin/env python3
"""Build an honest, per-ID accounting of the 484-capability registry.

GraphLock context (DOD-030): every registry ID needs exactly one final accounted
status with evidence and applicability reasoning; zero IDs missing or
duplicated. DOD-032 requires the exact status taxonomy. DOD-026 forbids
laundering uncertainty into completion.

This script is deliberately conservative. It does NOT synthesize PASS. It reads:

  * .agent/verification/MASTER_TEST_REGISTRY.csv  (the 484 canonical IDs)
  * .agent/verification/APPLICABILITY_MATRIX.csv (the DOD-041 applicability decision)
  * .agent/verification/state/CASE_RESULTS.json   (real executed results, if any)

and emits .agent/verification/reports/COMPLETE_TEST_ACCOUNTING.csv,
.agent/verification/state/TEST_LEDGER.jsonl, and a generated
.agent/verification/state/ACCOUNTING_STATUS.md summary.

TWO DEFECTS FIXED HERE (same class: one blanket string standing in for 484
individual decisions):

  1. Every ID without an executed case was recorded as NOT_RUN_BLOCKED_MATERIAL
     with the reason "Real dependency/harness provisioning is required before a
     truthful PASS or N/A decision can be recorded". For the IDs whose
     applicability decision is NOT_APPLICABLE -- decided per ID from repository
     evidence -- that reason was FALSE: nothing was owed and no provisioning was
     pending. Those rows now carry the taxonomy status NOT_APPLICABLE and the
     matrix's own per-ID evidence, and the digest of the matrix document is
     recorded so the reasoning cannot drift unnoticed.
  2. All unrun rows cited ONE stale hand-written snapshot (ACCOUNTING_STATUS.md,
     measured at an earlier candidate) as their evidence. Rows now cite the per-ID
     source of their decision, and that status document is GENERATED from the
     ledger instead of hand-written.

Usage: python3 scripts/build-accounting.py [--check]
  --check : exit 1 if any ID is missing/duplicated, or if the report is stale.
"""
from __future__ import annotations

import csv
import hashlib
import json
import sys
from pathlib import Path

REGISTRY = Path(".agent/verification/MASTER_TEST_REGISTRY.csv")
MATRIX = Path(".agent/verification/APPLICABILITY_MATRIX.csv")
CASE_RESULTS = Path(".agent/verification/state/CASE_RESULTS.json")
REPORT = Path(".agent/verification/reports/COMPLETE_TEST_ACCOUNTING.csv")
LEDGER = Path(".agent/verification/state/TEST_LEDGER.jsonl")
STATUS_DOC = Path(".agent/verification/state/ACCOUNTING_STATUS.md")


def evidence_digest(path: str) -> str | None:
    """SHA-256 of the evidence document a ledger row cites, None if absent."""
    if not path:
        return None
    p = Path(path)
    if not p.is_file():
        return None
    return hashlib.sha256(p.read_bytes()).hexdigest()


def evidence_size(path: str) -> int | None:
    if not path:
        return None
    p = Path(path)
    return p.stat().st_size if p.is_file() else None


# DOD-032 taxonomy. Nothing outside this set may appear in final_status.
VALID_STATUSES = {
    "PASS",
    "FAIL",
    "ERROR",
    "BLOCKED_PREREQUISITE",
    "BLOCKED_ENVIRONMENT",
    "BLOCKED_CAPABILITY",
    "BLOCKED_CREDENTIALS",
    "BLOCKED_SAFETY",
    "EXTERNAL_REQUIRED",
    "DEFERRED_LONG_RUNNING",
    "NOT_APPLICABLE",
    "PARTIAL",
    "UNVERIFIED",
    "NOT_RUN_BLOCKED_MATERIAL",
}


def load_registry() -> list[dict]:
    with REGISTRY.open(newline="", encoding="utf-8") as fh:
        return list(csv.DictReader(fh))


def load_applicability() -> dict[str, dict]:
    """The DOD-041 applicability decision and its evidence, per registry ID."""
    if not MATRIX.exists():
        raise SystemExit(f"applicability matrix missing: {MATRIX}")
    with MATRIX.open(newline="", encoding="utf-8") as fh:
        return {r["test_id"]: r for r in csv.DictReader(fh)}


def load_case_results() -> dict[str, dict]:
    if not CASE_RESULTS.exists():
        return {}
    try:
        data = json.loads(CASE_RESULTS.read_text("utf-8"))
    except json.JSONDecodeError as exc:
        raise SystemExit(f"case results are not valid JSON: {exc}")
    results = data.get("results", data if isinstance(data, list) else [])
    out: dict[str, dict] = {}
    for row in results:
        tid = row.get("test_id")
        if not tid:
            raise SystemExit("case result missing test_id")
        if tid in out:
            raise SystemExit(f"duplicate case result for {tid}")
        status = row.get("status")
        if status not in VALID_STATUSES:
            raise SystemExit(f"{tid}: status {status!r} outside DOD-032 taxonomy")
        out[tid] = row
    return out


def render_status_doc(ledger_rows: list[dict]) -> str:
    """The readable accounting summary, generated so it cannot go stale.

    It used to be hand-written and was cited as the evidence document for every
    unrun row while being measured at an earlier candidate -- a stale snapshot
    standing behind 473 decisions. It is now derived from the ledger, and
    `--check` fails if the file does not match the ledger exactly.
    """
    tally: dict[str, int] = {}
    for row in ledger_rows:
        tally[row["final_status"]] = tally.get(row["final_status"], 0) + 1
    lines = [
        "# Verification Accounting Status (generated)",
        "",
        "Generated by `scripts/build-accounting.py`. Do not hand-edit: `--check` fails",
        "if this file does not match `.agent/verification/state/TEST_LEDGER.jsonl`.",
        "",
        "Inputs: `MASTER_TEST_REGISTRY.csv` (484 canonical IDs),",
        "`APPLICABILITY_MATRIX.csv` (the per-ID DOD-041 decision and its evidence),",
        "`state/CASE_RESULTS.json` (executed cases).",
        "",
        "## Status tally (all 484 IDs, exactly one status each)",
        "",
        "| Status | IDs |",
        "| --- | --- |",
    ]
    lines += [f"| {status} | {tally[status]} |" for status in sorted(tally)]
    lines += [
        "",
        "A status of `NOT_APPLICABLE` is a decision, not a skipped test: each such row",
        "carries the applicability matrix's own per-ID evidence in the ledger, and the",
        "digest of that matrix document is recorded beside it.",
        "",
        "## IDs that are not NOT_APPLICABLE",
        "",
        "| ID | Status | Evidence | Title |",
        "| --- | --- | --- | --- |",
    ]
    for row in ledger_rows:
        if row["final_status"] == "NOT_APPLICABLE":
            continue
        lines.append(
            f"| {row['test_id']} | {row['final_status']} | `{row['evidence_path']}` | "
            f"{row['title']} |"
        )
    lines += [
        "",
        "## Reproduce",
        "",
        "```",
        "python3 scripts/build-accounting.py",
        "python3 scripts/build-accounting.py --check",
        "```",
        "",
    ]
    return "\n".join(lines)


def main() -> int:
    check_only = "--check" in sys.argv
    rows = load_registry()
    case_results = load_case_results()
    matrix = load_applicability()

    ids = [r["test_id"] for r in rows]
    if len(ids) != len(set(ids)):
        dupes = sorted({i for i in ids if ids.count(i) > 1})
        raise SystemExit(f"registry has duplicate IDs: {dupes}")
    if len(rows) != 484:
        raise SystemExit(f"registry drifted from the canonical 484: found {len(rows)}")

    unknown = sorted(set(case_results) - set(ids))
    if unknown:
        raise SystemExit(f"case results reference IDs not in the registry: {unknown}")

    # COHERENCE GUARD, added after the defect it catches actually happened. Round 51
    # reclassified four IDs from NOT_APPLICABLE to APPLICABLE, but three of those edits
    # left the older "no-cmd" entries in place, and a Python dict literal takes the LAST
    # assignment -- so the matrix kept saying NOT_APPLICABLE while case results for
    # those IDs were written anyway and this script happily reported them as PASS. A
    # solution that inflates the tally by ignoring the applicability decision is worse
    # than a wrong decision, because it hides itself. Now it is a hard failure.
    contradictory = sorted(
        tid
        for tid in case_results
        if matrix.get(tid, {}).get("decision") == "NOT_APPLICABLE"
    )
    if contradictory:
        raise SystemExit(
            "case results exist for IDs the applicability matrix decided NOT_APPLICABLE: "
            f"{contradictory}. Fix the applicability decision (the probe table most often "
            "holds a stale duplicate key, where the LAST assignment wins) or remove the "
            "case result -- the two records may not disagree."
        )

    out_rows, ledger_rows = [], []
    tally: dict[str, int] = {}

    for row in rows:
        tid = row["test_id"]
        case = case_results.get(tid)
        if case is not None:
            status = case["status"]
            evidence = case.get("evidence_path", "")
            reason = case.get("reason", "")
        else:
            # No candidate-specific case result exists for this ID. The truthful
            # status and the reason come from the DOD-041 applicability decision,
            # which was itself made per ID from repository evidence -- NOT from a
            # blanket "material required" sentence (see the module docstring).
            decision = matrix.get(tid, {})
            evidence = MATRIX.as_posix()
            matrix_reason = decision.get("evidence", "no applicability evidence recorded")
            if decision.get("decision") == "NOT_APPLICABLE":
                status = "NOT_APPLICABLE"
                reason = (
                    "no case was executed and none is owed: the DOD-041 applicability "
                    f"matrix decided NOT_APPLICABLE for this ID from repository evidence -- "
                    f"{matrix_reason}"
                )
            else:
                status = "NOT_RUN_BLOCKED_MATERIAL"
                reason = (
                    "the applicability matrix declares this ID APPLICABLE but no "
                    "candidate-specific case result has been recorded for it; the missing "
                    "material is an executed case for an APPLICABLE ID, not a harness or "
                    f"dependency yet to be provisioned. Matrix evidence: {matrix_reason}"
                )
        if status not in VALID_STATUSES:
            raise SystemExit(f"{tid}: status {status!r} outside DOD-032 taxonomy")

        tally[status] = tally.get(status, 0) + 1
        out_rows.append(
            {
                "test_id": tid,
                "final_status": status,
                "evidence_path": evidence,
                "reason": reason,
            }
        )
        ledger_rows.append(
            {
                "test_id": tid,
                "source_group": row["source_group"],
                "kind": row["kind"],
                "title": row["title"],
                "default_stage": row["default_stage"],
                "applicability": row["applicability"],
                "final_status": status,
                "evidence_path": evidence,
                # DOD-025 REQUIRED EVIDENCE: "Evidence index entries and content
                # hashes linked to each result." This previously carried no
                # digest at all, so a ledger row named an evidence document but
                # nothing proved which revision of it was judged.
                "evidence_sha256": evidence_digest(evidence),
                "evidence_bytes": evidence_size(evidence),
            }
        )

    # Invariant (DOD-030): zero missing, zero duplicated, one status each.
    assert len(out_rows) == 484, len(out_rows)
    assert len({r["test_id"] for r in out_rows}) == 484

    if check_only:
        if not REPORT.exists():
            print("accounting check: FAIL (report missing)", file=sys.stderr)
            return 1
        existing = list(
            csv.DictReader(REPORT.open(newline="", encoding="utf-8"))
        )
        if len(existing) != 484:
            print(
                f"accounting check: FAIL (report has {len(existing)} rows)",
                file=sys.stderr,
            )
            return 1

        # Evidence currency for the ledger (DOD-025 / DOD-040). A row that cites
        # an evidence document must carry a digest of the revision that was
        # judged, and that digest must still match the file on disk. Reported per
        # ID rather than counted, so the affected results are named.
        #
        # Skipped under --structure-only, which harness-validate.sh uses: that
        # script is itself an invalidated stage, so a currency check inside it is
        # circular. verify.sh asserts currency once the state has settled.
        if "--structure-only" in sys.argv:
            print("accounting check: ok (484 IDs, one status each, structure only)")
            return 0
        if not LEDGER.exists():
            print("accounting check: FAIL (ledger missing)", file=sys.stderr)
            return 1
        ledger = [
            json.loads(l) for l in LEDGER.read_text("utf-8").splitlines() if l.strip()
        ]
        if len(ledger) != 484:
            print(
                f"accounting check: FAIL (ledger has {len(ledger)} rows)",
                file=sys.stderr,
            )
            return 1
        unhashed = [
            r["test_id"]
            for r in ledger
            if r.get("evidence_path") and not r.get("evidence_sha256")
        ]
        stale = [
            r["test_id"]
            for r in ledger
            if r.get("evidence_sha256")
            and r["evidence_sha256"] != evidence_digest(r.get("evidence_path") or "")
        ]
        if unhashed:
            print(
                f"accounting check: FAIL ({len(unhashed)} ledger rows cite evidence "
                f"with no digest, first: {unhashed[0]})",
                file=sys.stderr,
            )
            return 1
        if stale:
            print(
                f"accounting check: FAIL ({len(stale)} ledger rows cite evidence "
                f"that has changed since they were recorded: {', '.join(stale[:5])}"
                f"{'...' if len(stale) > 5 else ''})",
                file=sys.stderr,
            )
            return 1
        digested = len([r for r in ledger if r.get("evidence_sha256")])
        if not STATUS_DOC.exists():
            print("accounting check: FAIL (status document missing)", file=sys.stderr)
            return 1
        if STATUS_DOC.read_text(encoding="utf-8") != render_status_doc(ledger):
            print(
                "accounting check: FAIL (status document is stale; "
                "run python3 scripts/build-accounting.py)",
                file=sys.stderr,
            )
            return 1
        print(
            f"accounting check: ok (484 IDs, one status each; "
            f"{digested} evidence digests current; status document current)"
        )
        return 0

    REPORT.parent.mkdir(parents=True, exist_ok=True)
    with REPORT.open("w", newline="", encoding="utf-8") as fh:
        writer = csv.DictWriter(
            fh,
            fieldnames=["test_id", "final_status", "evidence_path", "reason"],
            lineterminator="\n",
        )
        writer.writeheader()
        writer.writerows(out_rows)

    with LEDGER.open("w", encoding="utf-8") as fh:
        for entry in ledger_rows:
            fh.write(json.dumps(entry, sort_keys=True) + "\n")

    STATUS_DOC.write_text(render_status_doc(ledger_rows), encoding="utf-8")

    print(f"wrote {REPORT} ({len(out_rows)} rows)")
    print(f"wrote {LEDGER} ({len(ledger_rows)} rows)")
    print(f"wrote {STATUS_DOC}")
    for status in sorted(tally):
        print(f"  {status}: {tally[status]}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
