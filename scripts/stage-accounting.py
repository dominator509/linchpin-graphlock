#!/usr/bin/env python3
"""Per-stage accounting for V-000..V-021 (EP-010 M1, DOD-030, DOD-032).

WHY THIS EXISTS. EP-010 M1 is "Run V-000..V-021 without product-code changes",
and every stage plan in `.agent/verification/stage-plans/` ends with the same
exit criterion:

    "every owned ID has one truthful status/evidence record; FAIL stays FAIL;
     blocked dependencies are scoped; write stage accounting and next action."

Measured state before this script: the 484 registry IDs were fully accounted
(DOD-030 PASS) but NO stage accounting existed. `.agent/verification/state/
RUN_STATE.json` still said `{"status": "NOT_STARTED", "active_stage": null,
"candidate_epoch": 0}` -- the blueprint's seed value -- while `harness-next.sh`
read `active_stage` from it and therefore printed `V-000` forever. A reader
could not tell which stages had been executed, and the repository's own cursor
contradicted its own evidence.

STAGE OWNERSHIP IS THE PACK'S, NOT MINE. The registry carries a `default_stage`
column, so each ID already belongs to a stage group. The groups are named with
the pack's own 01..20 numbering, which is NOT the stage-plan numbering (the plans
carry V-000 authorization and V-001 discovery, which own no registry IDs). The
mapping below is therefore made by NAME, group name to plan slug, and is
asserted both ways: an unmapped group is an error, and a mapping to a plan that
does not exist is an error. `15-17-PERFORMANCE-STRESS-RECOVERY` is declared by
the pack as spanning three stages, so it legitimately owns IDs in V-016, V-017
and V-018 at once; that is recorded rather than forced into one bucket.

WHAT IT DOES NOT DO. It never invents a status. Each stage verdict is computed
from the per-ID accounting that the executed lanes produced, so a stage whose
owned IDs include a FAIL/PARTIAL/BLOCKED row can never be reported PASS. Stages
that own no registry ID (V-000, V-001, V-003) are bound to named evidence
artifacts instead -- the candidate pin, the discovery record and the
applicability matrix -- and fail when that evidence is absent.

Usage:
  python3 scripts/stage-accounting.py            # write the accounting
  python3 scripts/stage-accounting.py --check    # fail if the record is stale
  python3 scripts/stage-accounting.py --summary  # print the table only
"""
from __future__ import annotations

import csv
import hashlib
import json
import sys
from pathlib import Path

REGISTRY = Path(".agent/verification/MASTER_TEST_REGISTRY.csv")
ACCOUNTING = Path(".agent/verification/reports/COMPLETE_TEST_ACCOUNTING.csv")
MATRIX = Path(".agent/verification/APPLICABILITY_MATRIX.csv")
PLANS = Path(".agent/verification/stage-plans")
RUN_MANIFEST = Path(".agent/verification/state/RUN_MANIFEST.json")
EPOCH = Path(".agent/verification/state/EPOCH.json")
OUT_MD = Path(".agent/verification/reports/STAGE_ACCOUNTING.md")
OUT_JSON = Path(".agent/verification/state/STAGE_ACCOUNTING.json")
RUN_STATE = Path(".agent/verification/state/RUN_STATE.json")

# Registry `default_stage` group -> the stage plan(s) it owns, matched by the
# pack's own names (group suffix vs plan slug). Every registry group must appear
# exactly once here; the script asserts that.
STAGE_GROUPS: dict[str, list[str]] = {
    "02-CLAIMS-TRACEABILITY": ["V-002"],                     # claims-traceability-and-anti-simulation
    "03-STATIC-DESIGN-SUPPLY-CHAIN": ["V-004"],              # baseline-static-design-and-supply-chain
    "04-BUILD-ARTIFACT-DEPLOYMENT": ["V-005"],               # clean-build-packaging-provenance
    "05-SMOKE": ["V-006"],                                   # smoke
    "06-SANITY": ["V-007"],                                  # sanity
    "07-FUNCTIONAL-REALITY": ["V-008"],                      # full-functionality
    "08-API-INTEGRATION": ["V-009"],                         # api-integration-and-concurrency
    "09-DATA-SEMANTICS": ["V-010"],                          # data-semantics-migrations-temporal-i18n
    "10-COMPATIBILITY": ["V-011"],                           # compatibility-upgrade-rollback-config-platform
    "11-FORMAL-PROPERTY-REGRESSION": ["V-012"],              # regression-formal-property-and-mutation
    "11-REGRESSION": ["V-012"],
    "12-DYNAMIC-SECURITY": ["V-013"],                        # dynamic-security-and-domain-packs
    "12-AI-AGENT-SAFETY": ["V-013"],
    "13-EXPLORATORY": ["V-014"],                             # exploratory-and-ad-hoc
    "14-USABILITY-A11Y-DX": ["V-015"],                       # usability-accessibility-dx-visual-docs
    "15-PERFORMANCE": ["V-016"],                             # performance-workload-and-slo
    "15-17-PERFORMANCE-STRESS-RECOVERY": ["V-016", "V-017", "V-018"],
    "16-SOAK": ["V-017"],                                    # soak-endurance-and-resource-leaks
    "17-STRESS-CHAOS": ["V-018"],                            # stress-exhaustion-and-chaos
    "17-RECOVERY-OBSERVABILITY": ["V-019"],                  # recovery-dr-observability-and-reconciliation
    "18-RECOVERY-DR": ["V-019"],
    "19-FINAL-ARTIFACT": ["V-020"],                          # exact-artifact-cleanroom-and-deployment
    "19-EXTERNAL-HUMAN": ["V-021"],                          # uat-external-gates-and-final-accounting
    "20-UAT-HUMAN": ["V-021"],
}

# Stages that own no registry ID: bound to the evidence the stage plan is about.
# Each binding names a real artifact and the check that decides the verdict.
STAGE_BINDINGS: dict[str, dict] = {
    "V-000": {
        "name": "authorization-and-candidate-pin",
        "evidence": [str(RUN_MANIFEST), str(EPOCH)],
        "check": "the frozen candidate pin (commit, base revision, epoch digest, artifact digests) exists and is what the currency lanes verify",
    },
    "V-001": {
        "name": "repository-reality-discovery",
        "evidence": [".agent/evidence/EP-000/STATUS.md", ".agent/evidence/clean-build/STATUS.md"],
        "check": "toolchain probes and the clean-checkout build were executed and recorded (DOD-002/DOD-005)",
    },
    "V-003": {
        "name": "registry-and-applicability",
        "evidence": [str(MATRIX), ".agent/verification/state/ACCOUNTING_STATUS.md"],
        "check": "all 484 IDs carry one applicability decision and one accounted status (DOD-030/DOD-041)",
    },
}

# Verdict precedence: the worst status among a stage's owned IDs decides the
# stage verdict. A blocking status can never be masked by a majority of PASSes.
BLOCKING = [
    "FAIL",
    "ERROR",
    "MISSING",
    "UNVERIFIED",
    "BLOCKED_SAFETY",
    "BLOCKED_CREDENTIALS",
    "BLOCKED_CAPABILITY",
    "BLOCKED_ENVIRONMENT",
    "BLOCKED_PREREQUISITE",
    "NOT_RUN_BLOCKED_MATERIAL",
    "PARTIAL",
]
EXTERNAL = ["EXTERNAL_REQUIRED"]
DEFERRED = ["DEFERRED_LONG_RUNNING"]


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def plan_stages() -> dict[str, str]:
    """Stage ID -> plan slug, read from the stage-plan file names (V-0NN-slug.md)."""
    out = {}
    for path in sorted(PLANS.glob("V-*.md")):
        parts = path.name[:-3].split("-", 2)
        if len(parts) != 3 or not parts[1].isdigit():
            raise SystemExit(f"stage-accounting: stage plan name is not V-0NN-slug: {path.name}")
        out[f"V-{parts[1]}"] = parts[2]
    return out


def main() -> int:
    check = "--check" in sys.argv
    summary_only = "--summary" in sys.argv

    plans = plan_stages()
    if len(plans) != 22:
        raise SystemExit(f"stage-accounting: expected 22 stage plans, found {len(plans)}")

    # 1. Every registry group must be mapped, and every mapping must name a real plan.
    with REGISTRY.open(newline="", encoding="utf-8") as fh:
        registry = list(csv.DictReader(fh))
    groups = sorted({row["default_stage"] for row in registry})
    unmapped = [g for g in groups if g not in STAGE_GROUPS]
    if unmapped:
        raise SystemExit(f"stage-accounting: registry groups with no stage mapping: {unmapped}")
    stale = [g for g in STAGE_GROUPS if g not in groups]
    if stale:
        raise SystemExit(f"stage-accounting: stage mapping names groups the registry does not carry: {stale}")
    for group, stages in STAGE_GROUPS.items():
        for stage in stages:
            if stage not in plans:
                raise SystemExit(f"stage-accounting: group {group} maps to {stage}, which has no stage plan")

    # 2. Per-ID accounting is the only source of truth for a verdict.
    with ACCOUNTING.open(newline="", encoding="utf-8") as fh:
        accounted = {row["test_id"]: row for row in csv.DictReader(fh)}
    with MATRIX.open(newline="", encoding="utf-8") as fh:
        matrix = {row["test_id"]: row for row in csv.DictReader(fh)}
    missing = [row["test_id"] for row in registry if row["test_id"] not in accounted]
    if missing:
        raise SystemExit(f"stage-accounting: {len(missing)} registry IDs have no accounted status: {missing[:10]}")
    if len(accounted) != len(registry):
        raise SystemExit(
            f"stage-accounting: accounting has {len(accounted)} rows for {len(registry)} registry IDs"
        )

    # 3. Build the per-stage records.
    stages: dict[str, dict] = {}
    for group, group_stages in STAGE_GROUPS.items():
        ids = [row["test_id"] for row in registry if row["default_stage"] == group]
        for stage in group_stages:
            entry = stages.setdefault(
                stage,
                {"stage": stage, "name": plans[stage], "groups": [], "ids": [], "tally": {}, "blocking": [], "external": [], "deferred": []},
            )
            entry["groups"].append(group)
            entry["ids"].extend(ids)
    for stage in ("V-000", "V-001", "V-003"):
        binding = STAGE_BINDINGS[stage]
        stages[stage] = {
            "stage": stage,
            "name": plans[stage],
            "groups": [],
            "ids": [],
            "tally": {},
            "blocking": [],
            "external": [],
            "deferred": [],
            "bound_evidence": binding["evidence"],
            "binding_check": binding["check"],
        }

    for stage in sorted(stages):
        entry = stages[stage]
        entry["ids"] = sorted(set(entry["ids"]))
        tally: dict[str, int] = {}
        for tid in entry["ids"]:
            status = accounted[tid]["final_status"]
            tally[status] = tally.get(status, 0) + 1
            if status in BLOCKING:
                entry["blocking"].append(f"{tid}:{status}")
            elif status in EXTERNAL:
                entry["external"].append(tid)
            elif status in DEFERRED:
                entry["deferred"].append(tid)
        entry["tally"] = dict(sorted(tally.items()))
        entry["applicable"] = sum(1 for tid in entry["ids"] if matrix.get(tid, {}).get("decision") == "APPLICABLE")
        entry["not_applicable"] = sum(1 for tid in entry["ids"] if matrix.get(tid, {}).get("decision") == "NOT_APPLICABLE")
        # Verdict: a stage is never greener than its worst owned row. When a stage
        # owns both blocking and external rows the verdict is the blocking one, and
        # the basis names every category so nothing is hidden behind the label.
        parts = []
        if entry["blocking"]:
            parts.append("owned IDs not complete: " + ", ".join(entry["blocking"]))
        if entry["external"]:
            parts.append("owned IDs requiring an external participant or accredited body: " + ", ".join(entry["external"]))
        if entry["deferred"]:
            parts.append("owned IDs whose specified scale needs dedicated long-running infrastructure: " + ", ".join(entry["deferred"]))
        if entry["blocking"]:
            entry["verdict"] = "PARTIAL" if all(part.endswith(":PARTIAL") for part in entry["blocking"]) else "BLOCKED"
            entry["verdict_basis"] = "; ".join(parts)
        elif entry["external"]:
            entry["verdict"] = "EXTERNAL_REQUIRED"
            entry["verdict_basis"] = "; ".join(parts)
        elif entry["deferred"]:
            entry["verdict"] = "DEFERRED_LONG_RUNNING"
            entry["verdict_basis"] = "; ".join(parts)
        elif entry["ids"]:
            entry["verdict"] = "PASS"
            entry["verdict_basis"] = f"{len(entry['ids'])} owned IDs accounted; no blocking or external status"
        else:
            absent = [path for path in entry.get("bound_evidence", []) if not Path(path).exists()]
            if absent:
                entry["verdict"] = "BLOCKED"
                entry["verdict_basis"] = f"bound evidence missing: {absent}"
            else:
                entry["verdict"] = "PASS"
                entry["verdict_basis"] = entry.get("binding_check", "evidence present")

    record = {
        "gate": "stage-accounting",
        "covers": ["EP-010 M1", "DOD-030", "DOD-032"],
        "harness": "scripts/stage-accounting.py",
        "inputs": {
            str(REGISTRY): sha256_file(REGISTRY),
            str(ACCOUNTING): sha256_file(ACCOUNTING),
            str(MATRIX): sha256_file(MATRIX),
        },
        "stages": [stages[stage] for stage in sorted(stages)],
        "totals": {
            "registry_ids": len(registry),
            "owned_assignments": sum(len(entry["ids"]) for entry in stages.values()),
            "stages": len(stages),
            "pass": sum(1 for e in stages.values() if e["verdict"] == "PASS"),
            "external_required": sum(1 for e in stages.values() if e["verdict"] == "EXTERNAL_REQUIRED"),
            "deferred_long_running": sum(1 for e in stages.values() if e["verdict"] == "DEFERRED_LONG_RUNNING"),
            "blocked_or_partial": sum(1 for e in stages.values() if e["verdict"] in {"BLOCKED", "PARTIAL"}),
        },
    }
    record["digest"] = hashlib.sha256(
        json.dumps({k: v for k, v in record.items() if k != "digest"}, sort_keys=True).encode("utf-8")
    ).hexdigest()

    if summary_only:
        for entry in record["stages"]:
            print(f"{entry['stage']} {entry['verdict']:<20} ids={len(entry['ids']):>3} {entry['name']}")
        print(json.dumps(record["totals"], sort_keys=True))
        return 0

    if check:
        if not OUT_JSON.exists():
            print("stage-accounting: FAIL -- no stage accounting record exists", file=sys.stderr)
            return 1
        existing = json.loads(OUT_JSON.read_text(encoding="utf-8"))
        if existing.get("digest") != record["digest"]:
            print(
                "stage-accounting: FAIL -- the stage accounting record is stale against the current registry, "
                "accounting or applicability matrix",
                file=sys.stderr,
            )
            print("  remedy: python3 scripts/stage-accounting.py", file=sys.stderr)
            return 1
        print(f"stage-accounting: current (digest {record['digest'][:16]})")
        return 0

    OUT_JSON.parent.mkdir(parents=True, exist_ok=True)
    OUT_JSON.write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")

    lines = [
        "# Stage accounting V-000..V-021",
        "",
        "Generated by `scripts/stage-accounting.py`. Do not hand-edit.",
        "",
        "Stage ownership is the pack's `default_stage` column, mapped to stage plans by name.",
        "A stage verdict is computed from the accounted status of the IDs it owns and can never be",
        "greener than its worst owned row; stages that own no ID are bound to named evidence instead.",
        "",
        f"- Registry IDs accounted: {record['totals']['registry_ids']}",
        f"- Owned assignments (a pack group covering three stages owns its IDs in each): {record['totals']['owned_assignments']}",
        f"- Verdicts: PASS {record['totals']['pass']}, EXTERNAL_REQUIRED {record['totals']['external_required']}, "
        f"DEFERRED_LONG_RUNNING {record['totals']['deferred_long_running']}, BLOCKED/PARTIAL {record['totals']['blocked_or_partial']}",
        "",
        "| Stage | Plan | Owned IDs | APPLICABLE | Verdict | Basis |",
        "| --- | --- | --- | --- | --- | --- |",
    ]
    for entry in record["stages"]:
        lines.append(
            f"| {entry['stage']} | {entry['name']} | {len(entry['ids'])} | {entry['applicable']} | "
            f"**{entry['verdict']}** | {entry['verdict_basis']} |"
        )
    lines += ["", "## Per-stage status tally", ""]
    for entry in record["stages"]:
        tally = ", ".join(f"{k} {v}" for k, v in entry["tally"].items()) or "(no owned IDs -- bound to evidence)"
        lines.append(f"- **{entry['stage']}** {entry['name']}: {tally}")
        if entry.get("bound_evidence"):
            lines.append(f"  - bound evidence: {', '.join('`' + p + '`' for p in entry['bound_evidence'])}")
    lines += [
        "",
        "## Next action",
        "",
        "- A stage marked EXTERNAL_REQUIRED needs a participant or host this repository cannot provide; the",
        "  specific gate and its unblock condition are named in `.agent/evidence/ADR-005-external-signoff-gates.md`",
        "  and in the residual-risk register.",
        "- A stage marked BLOCKED or PARTIAL names the exact owned IDs above; those rows carry their own reason",
        "  and unblock condition in `.agent/verification/reports/COMPLETE_TEST_ACCOUNTING.csv`.",
        "",
    ]
    OUT_MD.write_text("\n".join(lines), encoding="utf-8")

    # The harness cursor must describe the run that actually happened. It used to
    # hold the blueprint's seed value ("NOT_STARTED", epoch 0) while every stage had
    # been accounted, and harness-next.sh read `active_stage` from it and printed
    # V-000 forever -- a false statement about the run.
    epoch = json.loads(EPOCH.read_text(encoding="utf-8")) if EPOCH.exists() else {}
    open_stages = [e["stage"] for e in record["stages"] if e["verdict"] != "PASS"]
    run_state = {
        "status": "ACCOUNTED_WITH_OPEN_GATES" if open_stages else "COMPLETE",
        "active_stage": open_stages[0] if open_stages else None,
        "open_stages": open_stages,
        "completed_stages": [e["stage"] for e in record["stages"] if e["verdict"] == "PASS"],
        "candidate_commit": epoch.get("candidate_commit"),
        "candidate_epoch": epoch.get("epoch_digest"),
        "stage_accounting_digest": record["digest"],
        "note": (
            "Every stage's owned IDs carry an accounted status; the stages listed in open_stages are open "
            "only for the reason named in STAGE_ACCOUNTING.md (external participant, deferred long-run scale, "
            "or the specific owned IDs named there)."
        ),
    }
    RUN_STATE.write_text(json.dumps(run_state, indent=2) + "\n", encoding="utf-8")
    print(
        f"stage-accounting: ok -- {record['totals']['stages']} stages, "
        f"PASS {record['totals']['pass']}, EXTERNAL_REQUIRED {record['totals']['external_required']}, "
        f"DEFERRED_LONG_RUNNING {record['totals']['deferred_long_running']}, "
        f"BLOCKED/PARTIAL {record['totals']['blocked_or_partial']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
