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

  * every release-scoped clause in DOD_REGISTRY.csv is accounted (all 42 clauses
    carry "release" in their scope; the set is read from the registry, not listed
    here, because a hand list silently omitted the unresolved clauses);
  * a clause that is FAIL, PARTIAL, ERROR, BLOCKED_*, UNVERIFIED, NOT_APPLICABLE
    or MISSING blocks GO outright;
  * a clause that is EXTERNAL_REQUIRED or DEFERRED_LONG_RUNNING yields at most
    CONDITIONAL_EXTERNAL_GATES, never GO;
  * missing artifact identity or incomplete 484 accounting yields INCONCLUSIVE;
  * any unaccounted registry ID yields INCONCLUSIVE.

Usage:
  python3 scripts/ship-gate.py            # compute and write RELEASE_GATE.json
  python3 scripts/ship-gate.py --check    # verify the file matches a fresh run
"""
from __future__ import annotations

import csv
import hashlib
import json
import subprocess
import sys
from pathlib import Path

DOD_STATUS = Path(".agent/verification/state/DOD_STATUS.jsonl")
DOD_REGISTRY = Path(".agent/verification/DOD_REGISTRY.csv")
ACCOUNTING = Path(".agent/verification/reports/COMPLETE_TEST_ACCOUNTING.csv")
MANIFEST = Path(".agent/verification/state/RUN_MANIFEST.json")
EPOCH = Path(".agent/verification/state/EPOCH.json")
GATE = Path(".agent/verification/reports/RELEASE_GATE.json")
MSI_DIR = Path("target/release/bundle/msi")
EXE = Path("target/release/linchpin-desktop.exe")

VALID_VERDICTS = {"GO", "NO_GO", "CONDITIONAL_EXTERNAL_GATES", "INCONCLUSIVE"}


def is_ancestor(older: str, newer: str) -> bool:
    """True when `older` is an ancestor of `newer` in this repository."""
    proc = subprocess.run(
        ["git", "merge-base", "--is-ancestor", older, newer],
        capture_output=True,
        text=True,
    )
    return proc.returncode == 0


def load_release_clauses() -> list[str]:
    """Every clause the pack scopes to release, READ FROM THE REGISTRY.

    MEASURED DEFECT THIS REPLACES: this used to be a hand-written list of 17
    clauses (MANDATORY_FOR_GO). The pack scopes ALL 42 clauses to release --
    "release" appears in every `scope` row of DOD_REGISTRY.csv -- and the hand list
    silently omitted exactly the clauses that were NOT PASS: DOD-014 (PARTIAL),
    DOD-034 (EXTERNAL_REQUIRED), DOD-035 (FAIL), DOD-038 (PARTIAL) and DOD-039
    (EXTERNAL_REQUIRED). The gate therefore reported ONE blocking clause
    (DOD-001=PARTIAL) while four clauses were unresolved, and a reader could believe
    the only thing between this candidate and GO was requirement traceability. A
    gate whose blocker list can be shortened by editing the gate is not a gate, so
    the clause set is derived from the registry and any clause missing from it is a
    hard failure rather than a silent omission.
    """
    if not DOD_REGISTRY.exists():
        raise SystemExit(f"ship-gate: DOD registry missing: {DOD_REGISTRY}")
    with DOD_REGISTRY.open(newline="", encoding="utf-8") as fh:
        rows = list(csv.DictReader(fh))
    clauses = [r["dod_id"] for r in rows if "release" in r.get("scope", "").lower()]
    if len(clauses) != len(rows):
        raise SystemExit(
            "ship-gate: the registry has clauses that are not release-scoped; the "
            "verdict rule below assumes every accounted clause applies at release"
        )
    if not clauses:
        raise SystemExit("ship-gate: no release-scoped clauses found in the registry")
    return clauses


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
    # READ FROM THE AUTHORITATIVE IDENTITY FILES, not from CASE_RESULTS.json.
    # MEASURED DEFECT THIS REPLACES: the gate used to take the candidate from
    # CASE_RESULTS.json's header, which had been frozen at '1bca002' while the
    # actual settled candidate was c317684 -- so RELEASE_GATE.json named a commit
    # that was not the one whose artifact digests and epoch it recorded, which is
    # exactly the drift DOD-029 forbids. RUN_MANIFEST.json is the pinned identity
    # (enforced by its own verify.sh lane) and EPOCH.json is what the invalidation
    # graph is computed against; both are read here, and they must agree.
    candidate = "UNKNOWN"
    candidate_source = "none"
    if MANIFEST.exists():
        candidate = json.loads(MANIFEST.read_text("utf-8")).get("candidate_commit", "UNKNOWN")
        candidate_source = "RUN_MANIFEST.json"
    if EPOCH.exists():
        epoch_candidate = str(json.loads(EPOCH.read_text("utf-8")).get("candidate_commit") or "")
        same = bool(epoch_candidate) and (
            epoch_candidate == candidate
            or epoch_candidate.startswith(candidate[:7])
            or candidate.startswith(epoch_candidate[:7])
        )
        if candidate == "UNKNOWN" and epoch_candidate:
            candidate = epoch_candidate
            candidate_source = "EPOCH.json"
        elif epoch_candidate and not same:
            # EPOCH.json names HEAD; RUN_MANIFEST.json names the commit whose tree
            # was verified. After a settle commit the two legitimately differ by
            # commits that changed only derived verification state -- the same
            # relation run-manifest.py --check allows. MEASURED: rejecting that
            # outright made this gate report INCONCLUSIVE during a settle rerun
            # ("recorded NO_GO, recomputed INCONCLUSIVE") and failed V-000 for a
            # state that was in fact consistent. The relation is VERIFIED, not
            # assumed: the pinned candidate must be an ancestor of the epoch
            # candidate, and a genuine divergence is still INCONCLUSIVE.
            if is_ancestor(candidate, epoch_candidate):
                artifact["settle_commits_after_candidate"] = True
            else:
                reasons.append(
                    f"candidate identity disagrees: RUN_MANIFEST.json says {candidate[:7]}, "
                    f"EPOCH.json says {epoch_candidate[:7]} (and the pinned candidate is "
                    "not an ancestor of the epoch candidate)"
                )
    artifact["candidate_commit"] = candidate
    artifact["candidate_source"] = candidate_source
    # The pinned candidate is the commit whose tree was verified; after a settle
    # commit HEAD is ahead of it without changing a tracked input. Recorded here so
    # a reader of RELEASE_GATE.json can tell "the artifact and results belong to
    # this commit" from "HEAD is that commit".
    head = subprocess.run(
        ["git", "rev-parse", "HEAD"], capture_output=True, text=True
    ).stdout.strip()
    if head:
        artifact["head_commit"] = head
        artifact["candidate_is_head"] = head == candidate
        if head != candidate:
            artifact["candidate_is_ancestor_of_head"] = is_ancestor(candidate, head)
            if not artifact["candidate_is_ancestor_of_head"]:
                reasons.append(
                    f"pinned candidate {candidate[:7]} is not an ancestor of HEAD {head[:7]}"
                )

    # --- DoD clause dispositions -----------------------------------------
    dod = load_dod()
    if len(dod) != 42:
        reasons.append(f"DoD status incomplete: {len(dod)}/42 clauses")

    release_clauses = load_release_clauses()
    for clause in release_clauses:
        status = dod.get(clause, "MISSING")
        if status == "PASS":
            continue
        if status in {"EXTERNAL_REQUIRED", "DEFERRED_LONG_RUNNING"}:
            # An unavailable outside participant or an unfinished long-duration
            # trial: it conditions the release rather than failing it, so it caps
            # the verdict at CONDITIONAL_EXTERNAL_GATES instead of blocking.
            external.append(f"{clause}={status}")
        else:
            # FAIL, PARTIAL, ERROR, BLOCKED_*, UNVERIFIED, NOT_APPLICABLE and
            # MISSING all block: work that could have been done was not, or a
            # clause carries no disposition at all.
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
        "mandatory_clauses": sorted(release_clauses),
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
