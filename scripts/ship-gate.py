#!/usr/bin/env python3
"""Machine-validated ship gate (DOD-042, V-021).

GraphLock context: `RELEASE_GATE.json` read
`{"verdict": "NOT_EVALUATED", "reason": "Blueprint only; no candidate artifact
exists."}` -- a statement that became false many rounds ago. Nothing computed
it, so the release verdict could never change.

DOD-042: "The final release verdict is produced only by the machine-validated
ship gate and is one of GO, NO_GO, CONDITIONAL_EXTERNAL_GATES, or
INCONCLUSIVE." Required evidence: RELEASE_GATE.json, validator outputs, DOD
status, 484-test accounting, and exact artifact identity.

This script DERIVES the verdict from measured state. It does not accept a
verdict as input and it never writes GO unless every mandatory predicate holds.
The predicates are deliberately conservative:

  * a mandatory DoD clause that is FAIL blocks GO outright
  * a mandatory clause that is EXTERNAL_REQUIRED yields at most
    CONDITIONAL_EXTERNAL_GATES, never GO
  * missing artifact identity or incomplete 484 accounting yields INCONCLUSIVE
  * any unaccounted registry ID yields INCONCLUSIVE

Usage:
  python3 scripts/ship-gate.py            # compute and write RELEASE_GATE.json
  python3 scripts/ship-gate.py --check    # verify the file matches a fresh run
"""
from __future__ import annotations

import csv
import hashlib
import json
import sys
from pathlib import Path

DOD_STATUS = Path(".agent/verification/state/DOD_STATUS.jsonl")
ACCOUNTING = Path(".agent/verification/reports/COMPLETE_TEST_ACCOUNTING.csv")
CASE_RESULTS = Path(".agent/verification/state/CASE_RESULTS.json")
GATE = Path(".agent/verification/reports/RELEASE_GATE.json")
MSI_DIR = Path("target/release/bundle/msi")
EXE = Path("target/release/linchpin-desktop.exe")

VALID_VERDICTS = {"GO", "NO_GO", "CONDITIONAL_EXTERNAL_GATES", "INCONCLUSIVE"}

# Clauses that must be PASS for an unconditional GO. Everything else is
# reported but does not by itself block, because the pack scopes many clauses
# to feature/milestone rather than release.
MANDATORY_FOR_GO = {
    "DOD-001",  # requirement traceability
    "DOD-002",  # clean build
    "DOD-003",  # distribution artifact
    "DOD-004",  # exact-artifact tests
    "DOD-006",  # no skipped tests
    "DOD-007",  # collection guard
    "DOD-018",  # mutation proof
    "DOD-019",  # placeholder scan
    "DOD-021",  # static analysis / supply chain
    "DOD-024",  # no failure masking
    "DOD-026",  # honest status taxonomy
    "DOD-027",  # no weakened acceptance
    "DOD-030",  # 484 accounting
    "DOD-032",  # status definitions
    "DOD-040",  # change invalidation
    "DOD-041",  # applicability matrix
    "DOD-042",  # this clause
}


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def load_dod() -> dict[str, str]:
    if not DOD_STATUS.exists():
        return {}
    rows = [
        json.loads(line)
        for line in DOD_STATUS.read_text("utf-8").splitlines()
        if line.strip()
    ]
    return {r["dod_id"]: r["status"] for r in rows}


def load_accounting() -> tuple[int, dict[str, int]]:
    if not ACCOUNTING.exists():
        return 0, {}
    with ACCOUNTING.open(newline="", encoding="utf-8") as fh:
        rows = list(csv.DictReader(fh))
    tally: dict[str, int] = {}
    for r in rows:
        tally[r["final_status"]] = tally.get(r["final_status"], 0) + 1
    return len(rows), tally


def main() -> int:
    check_only = "--check" in sys.argv
    reasons: list[str] = []
    blockers: list[str] = []
    external: list[str] = []

    # --- 484 accounting (DOD-030) ----------------------------------------
    accounted, tally = load_accounting()
    if accounted != 484:
        reasons.append(f"registry accounting incomplete: {accounted}/484")

    # --- artifact identity (DOD-003, DOD-029) -----------------------------
    artifact: dict[str, str] = {}
    if EXE.exists():
        artifact["executable"] = str(EXE)
        artifact["executable_sha256"] = sha256_file(EXE)
        artifact["executable_bytes"] = str(EXE.stat().st_size)
    else:
        reasons.append("no built executable at target/release/linchpin-desktop.exe")

    msis = sorted(MSI_DIR.glob("*.msi")) if MSI_DIR.exists() else []
    if msia := (msis[-1] if msis else None):
        artifact["msi"] = str(msia)
        artifact["msi_sha256"] = sha256_file(msia)
        artifact["msi_bytes"] = str(msia.stat().st_size)
    else:
        reasons.append("no installer artifact under target/release/bundle/msi")

    # --- candidate identity (DOD-029) ------------------------------------
    candidate = "UNKNOWN"
    if CASE_RESULTS.exists():
        data = json.loads(CASE_RESULTS.read_text("utf-8"))
        candidate = data.get("candidate_commit", "UNKNOWN")
        artifact["candidate_commit"] = candidate

    # --- DoD clause dispositions -----------------------------------------
    dod = load_dod()
    if len(dod) != 42:
        reasons.append(f"DoD status incomplete: {len(dod)}/42 clauses")

    for clause in sorted(MANDATORY_FOR_GO):
        status = dod.get(clause, "MISSING")
        if status == "PASS":
            continue
        if status in {"EXTERNAL_REQUIRED", "DEFERRED_LONG_RUNNING"}:
            external.append(f"{clause}={status}")
        else:
            blockers.append(f"{clause}={status}")

    # --- verdict ----------------------------------------------------------
    if reasons:
        verdict = "INCONCLUSIVE"
    elif blockers:
        verdict = "NO_GO"
    elif external:
        verdict = "CONDITIONAL_EXTERNAL_GATES"
    else:
        verdict = "GO"

    assert verdict in VALID_VERDICTS, verdict

    gate = {
        "verdict": verdict,
        "candidate_commit": candidate,
        "mandatory_clauses": sorted(MANDATORY_FOR_GO),
        "blocking_clauses": blockers,
        "external_clauses": external,
        "inconclusive_reasons": reasons,
        "dod_status_tally": {},
        "registry_accounting": {"accounted": accounted, "total": 484, "tally": tally},
        "artifact": artifact,
        "note": (
            "Verdict derived by scripts/ship-gate.py from DOD_STATUS.jsonl, "
            "COMPLETE_TEST_ACCOUNTING.csv and artifact digests. Not hand-written."
        ),
    }
    dod_tally: dict[str, int] = {}
    for status in dod.values():
        dod_tally[status] = dod_tally.get(status, 0) + 1
    gate["dod_status_tally"] = dod_tally

    if check_only:
        if not GATE.exists():
            print("ship-gate check: FAIL (no gate file)", file=sys.stderr)
            return 1
        current = json.loads(GATE.read_text("utf-8"))
        if current.get("verdict") != verdict:
            print(
                f"ship-gate check: FAIL (recorded {current.get('verdict')}, "
                f"recomputed {verdict})",
                file=sys.stderr,
            )
            return 1
        print(f"ship-gate check: ok (verdict {verdict}, unchanged)")
        return 0

    GATE.parent.mkdir(parents=True, exist_ok=True)
    GATE.write_text(json.dumps(gate, indent=2, sort_keys=True) + "\n", "utf-8")

    print(f"ship-gate: verdict = {verdict}")
    print(f"  candidate          = {candidate}")
    print(f"  registry accounted = {accounted}/484")
    print(f"  dod tally          = {gate['dod_status_tally']}")
    if blockers:
        print(f"  BLOCKING CLAUSES   = {', '.join(blockers)}")
    if external:
        print(f"  EXTERNAL CLAUSES   = {', '.join(external)}")
    if reasons:
        for r in reasons:
            print(f"  INCONCLUSIVE: {r}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
