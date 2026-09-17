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

PRODUCT_MANIFESTS = [
    Path("apps/desktop/src-tauri/tauri.conf.json"),
    Path("apps/desktop/src-tauri/Cargo.toml"),
]


def declared_product_version() -> str:
    """The version the repository DECLARES for the product, or "" when unreadable.

    Used to reject a bound installer that carries some other version's name -- the
    cross-version matrix legitimately builds a v0.2.0 package from this same tree
    with the manifests temporarily bumped, and such a package must never be mistaken
    for the release artifact.
    """
    conf = PRODUCT_MANIFESTS[0]
    if conf.exists():
        try:
            return str(json.loads(conf.read_text("utf-8")).get("version") or "")
        except json.JSONDecodeError:
            pass
    cargo = PRODUCT_MANIFESTS[1]
    if cargo.exists():
        for line in cargo.read_text("utf-8").splitlines():
            if line.startswith("version") and '"' in line:
                return line.split('"')[1]
    return ""


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
    # STRUCTURE-ONLY mode exists for the one caller that runs MID-SETTLE.
    #
    # MEASURED: `sh scripts/harness-validate.sh` runs as stage V-000, before the settle
    # path refreshes the derived state, so the recorded RELEASE_GATE.json legitimately
    # describes the pre-rerun state and an equality check there is unsatisfiable -- it
    # failed with "ship-gate check: FAIL (recorded INCONCLUSIVE, recomputed
    # CONDITIONAL_EXTERNAL_GATES)" for a state that was simply not refreshed yet. That
    # is the same circularity the other derived artifacts already solve with
    # `--structure-only`. The STRICT equality check moved to `scripts/verify.sh`, which
    # runs after the refresh, so nothing was weakened: structure is checked where the
    # comparison cannot hold, and identity where it can.
    structure_only = "--structure-only" in sys.argv
    reasons: list[str] = []
    blockers: list[str] = []
    external: list[str] = []

    # --- 484 accounting (DOD-030) ----------------------------------------
    accounted, tally = load_accounting()
    if accounted != 484:
        reasons.append(f"registry accounting incomplete: {accounted}/484")

    # --- artifact identity (DOD-003, DOD-029) -----------------------------
    artifact: dict[str, str] = {}
    manifest_data = json.loads(MANIFEST.read_text("utf-8")) if MANIFEST.exists() else {}
    recorded_artifacts = manifest_data.get("artifacts") or {}
    if EXE.exists():
        artifact["executable"] = str(EXE)
        artifact["executable_sha256"] = sha256_file(EXE)
        artifact["executable_bytes"] = str(EXE.stat().st_size)
        if recorded_artifacts.get("executable_sha256") not in (None, artifact["executable_sha256"]):
            reasons.append(
                f"the pinned executable hashes {artifact['executable_sha256'][:16]} but RUN_MANIFEST.json "
                f"recorded {str(recorded_artifacts['executable_sha256'])[:16]}; the artifact moved under the identity"
            )
    else:
        reasons.append("no built executable at target/release/linchpin-desktop.exe")
    # Bind to the installer the identity files PIN, and verify it, instead of
    # selecting one by glob.
    #
    # MEASURED DEFECT THIS REPLACES: this used `sorted(MSI_DIR.glob("*.msi"))[-1]`.
    # The cross-version matrix builds a v0.2.0 package through the production build
    # path (with the version manifests temporarily bumped and then restored) and left
    # it in target/release/bundle/msi, so the lexicographically-last file won and the
    # release gate bound its identity to a TEST artifact of a version the repository
    # does not declare. Lexicographic order is not even version order (0.9.0 sorts
    # after 0.10.0), so the selection was wrong in principle as well as in fact.
    pinned = recorded_artifacts.get("msi")
    if pinned:
        msi_path = Path(pinned)
        if not msi_path.exists():
            reasons.append(f"the pinned installer {pinned} does not exist")
        else:
            digest = sha256_file(msi_path)
            recorded = recorded_artifacts.get("msi_sha256")
            artifact["msi"] = str(msi_path)
            artifact["msi_sha256"] = digest
            artifact["msi_bytes"] = str(msi_path.stat().st_size)
            if recorded and recorded != digest:
                reasons.append(
                    f"the pinned installer {pinned} hashes {digest[:16]} but RUN_MANIFEST.json "
                    f"recorded {str(recorded)[:16]}; the artifact moved under the identity"
                )
            declared = declared_product_version()
            if declared and declared not in msi_path.name:
                reasons.append(
                    f"the pinned installer {msi_path.name} does not carry the declared product "
                    f"version {declared}"
                )
    else:
        reasons.append("RUN_MANIFEST.json does not pin an installer artifact")

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
            # EPOCH.json names the commit the invalidation graph was computed at;
            # RUN_MANIFEST.json names the commit whose tree was verified. They may
            # legitimately differ by commits that changed only derived verification
            # state, in EITHER order -- MEASURED: a fix committed after the epoch was
            # recorded left EPOCH at 04fd659 and the manifest at 1c32527, and the
            # one-directional check called that a disagreement. The relation is
            # VERIFIED, not assumed: the two must be related by ancestry (no divergent
            # identity), and a genuine fork is still INCONCLUSIVE.
            if is_ancestor(candidate, epoch_candidate) or is_ancestor(epoch_candidate, candidate):
                artifact["settle_commits_after_candidate"] = True
            else:
                reasons.append(
                    f"candidate identity disagrees: RUN_MANIFEST.json says {candidate[:7]}, "
                    f"EPOCH.json says {epoch_candidate[:7]} (and the two are not related by "
                    "ancestry, so they describe different work)"
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
        if structure_only:
            # Structure of BOTH: the recomputed verdict must be lawful and complete,
            # and the recorded file must still carry a lawful verdict and the fields a
            # reader depends on. Values are not compared here; verify.sh does that.
            required = {"verdict", "blocking_clauses", "external_clauses", "artifact", "registry_accounting"}
            missing = sorted(required - set(current))
            if missing:
                print(f"ship-gate check: FAIL (recorded gate is missing {missing})", file=sys.stderr)
                return 1
            if current.get("verdict") not in VALID_VERDICTS:
                print(
                    f"ship-gate check: FAIL (recorded verdict {current.get('verdict')!r} is not one of "
                    f"{sorted(VALID_VERDICTS)})",
                    file=sys.stderr,
                )
                return 1
            if len(dod) != 42 or accounted != 484:
                print(
                    f"ship-gate check: FAIL (structure: {len(dod)}/42 clauses, {accounted}/484 IDs)",
                    file=sys.stderr,
                )
                return 1
            print(
                f"ship-gate check: ok (structure only; recorded {current.get('verdict')}, "
                f"recomputed {verdict}; identity is verified after the settle refresh)"
            )
            return 0
        if current.get("verdict") != verdict:
            print(
                f"ship-gate check: FAIL (recorded {current.get('verdict')}, "
                f"recomputed {verdict})",
                file=sys.stderr,
            )
            return 1
        # Comparing only the VERDICT was not enough. MEASURED: replacing the pinned
        # installer with different bytes added the reason "the artifact moved under
        # the identity" while the verdict stayed INCONCLUSIVE, so --check passed and
        # the recorded gate kept describing an artifact that no longer existed. The
        # identity fields and the reason set are compared too, so a same-verdict
        # change still fails.
        for field in ("artifact", "blocking_clauses", "external_clauses", "inconclusive_reasons", "dod_status_tally", "registry_accounting"):
            if current.get(field) != gate.get(field):
                print(
                    f"ship-gate check: FAIL (the recorded {field} no longer matches the "
                    f"recomputed one; re-derive with python3 scripts/ship-gate.py)",
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
