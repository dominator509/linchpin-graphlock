#!/usr/bin/env python3
"""Build an honest, per-ID accounting of the 484-capability registry.

GraphLock context (DOD-030): every registry ID needs exactly one final accounted
status with evidence and applicability reasoning; zero IDs missing or
duplicated. DOD-032 requires the exact status taxonomy. DOD-026 forbids
laundering uncertainty into completion.

This script is deliberately conservative. It does NOT synthesize PASS. It reads:

  * .agent/verification/MASTER_TEST_REGISTRY.csv  (the 484 canonical IDs)
  * .agent/verification/state/CASE_RESULTS.json   (real executed results, if any)

and emits .agent/verification/reports/COMPLETE_TEST_ACCOUNTING.csv plus
.agent/verification/state/TEST_LEDGER.jsonl.

Any ID with no genuinely executed case materializes as NOT_RUN_BLOCKED_MATERIAL
-- the campaign has not produced candidate-specific case material for it yet.
That is the truthful status, and it is what makes `harness-accounting.sh` pass
its count invariant without lying about verification.

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
CASE_RESULTS = Path(".agent/verification/state/CASE_RESULTS.json")
REPORT = Path(".agent/verification/reports/COMPLETE_TEST_ACCOUNTING.csv")
LEDGER = Path(".agent/verification/state/TEST_LEDGER.jsonl")


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


def main() -> int:
    check_only = "--check" in sys.argv
    rows = load_registry()
    case_results = load_case_results()

    ids = [r["test_id"] for r in rows]
    if len(ids) != len(set(ids)):
        dupes = sorted({i for i in ids if ids.count(i) > 1})
        raise SystemExit(f"registry has duplicate IDs: {dupes}")
    if len(rows) != 484:
        raise SystemExit(f"registry drifted from the canonical 484: found {len(rows)}")

    unknown = sorted(set(case_results) - set(ids))
    if unknown:
        raise SystemExit(f"case results reference IDs not in the registry: {unknown}")

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
            # No candidate-specific case material exists for this ID yet.
            # Claiming anything stronger would be fabrication (DOD-026/027).
            status = "NOT_RUN_BLOCKED_MATERIAL"
            evidence = ".agent/verification/state/ACCOUNTING_STATUS.md"
            reason = (
                "no candidate-specific case material executed for this ID; "
                f"registry declares applicability={row['applicability']!r}, "
                f"default_stage={row['default_stage']!r}. "
                "Real dependency/harness provisioning is required before a "
                "truthful PASS or N/A decision can be recorded."
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
        print(
            f"accounting check: ok (484 IDs, one status each; "
            f"{digested} evidence digests current)"
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

    print(f"wrote {REPORT} ({len(out_rows)} rows)")
    print(f"wrote {LEDGER} ({len(ledger_rows)} rows)")
    for status in sorted(tally):
        print(f"  {status}: {tally[status]}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
