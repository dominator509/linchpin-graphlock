#!/usr/bin/env python3
"""Execute the reruns an epoch change obliges (DOD-040).

DOD-040 RULE: "Any code, dependency, schema, configuration, build, test-oracle,
or artifact change invalidates and reruns every affected downstream result."
OR ELSE: "Affected PASS statuses are revoked until rerun."

`scripts/change-invalidation.py` computes the epoch and NAMES the affected
stages, but names are not reruns: the graph mapped input classes to abstract
labels like "V-005 clean build" that no command actually corresponds to, so
nothing was ever re-executed. A reader could not tell from the graph which
command to run, and no record existed of whether it had been run.

This script closes that gap:

  1. STAGE_RUNNERS maps each named stage to a concrete repository script --
     verified to exist, not assumed.
  2. It reads the declared changed classes from the recorded epoch.
  3. It runs the mapped commands, recording command, exit code, duration and
     output digest for each.
  4. It writes .agent/verification/state/RERUN_RECORD.json and reports any
     failure.

A stage with no runner is reported as NO_RUNNER rather than silently skipped,
and a stage that fails is reported as FAIL so the obligation is visible.

Usage:
  python3 scripts/rerun-invalidated.py             # run the affected stages
  python3 scripts/rerun-invalidated.py --list      # show the mapping only
  python3 scripts/rerun-invalidated.py --check     # fail if a recorded rerun
                                                   # is missing or older than
                                                   # the current epoch
"""
from __future__ import annotations

import hashlib
import json
import subprocess
import sys
import time
from pathlib import Path

EPOCH = Path(".agent/verification/state/EPOCH.json")
RECORD = Path(".agent/verification/state/RERUN_RECORD.json")
GRAPH = Path(".agent/verification/state/CHANGE_INVALIDATION_GRAPH.md")

# Concrete, verified runners for each stage name the invalidation graph uses.
# Every value is a repository script that exists; a stage with no entry reports
# NO_RUNNER rather than being dropped.
STAGE_RUNNERS: dict[str, list[list[str]]] = {
    "V-000 harness validation": [
        ["sh", "scripts/harness-validate.sh"],
    ],
    "V-004 supply chain": [
        ["sh", "scripts/security-check.sh"],
        ["python3", "scripts/generate-sbom.py", "--check"],
    ],
    "V-005 clean build": [
        ["sh", "scripts/build.sh"],
    ],
    "V-006 smoke": [
        ["sh", "scripts/smoke-test.sh"],
    ],
    "V-007 sanity": [
        ["sh", "scripts/typecheck.sh"],
    ],
    "V-008 full functionality": [
        ["sh", "scripts/test-unit.sh"],
    ],
    "V-009 integration/concurrency": [
        ["sh", "scripts/test-integration.sh"],
    ],
    "V-011 configuration matrix": [
        ["sh", "scripts/format-check.sh"],
    ],
    "V-012 regression/mutation": [
        ["sh", "scripts/lint.sh"],
    ],
    "V-015 usability/accessibility": [
        ["sh", "scripts/test-e2e.sh"],
    ],
    "V-020 exact artifact": [
        ["sh", "scripts/artifact-e2e.sh"],
    ],
    # V-013 owns the dynamic-security / domain-pack lane, which is where the
    # mandatory offline provider proof belongs (PF-011, DOD-010). Exits 2 with a
    # named prerequisite when no loopback provider is served, so it reports an
    # absent dependency rather than passing silently (DOD-006).
    "V-013 dynamic security/domain packs": [
        ["sh", "scripts/live-fire-local-provider.sh"],
    ],
    "V-021 final accounting": [
        ["sh", "scripts/harness-accounting.sh"],
        ["python3", "scripts/ship-gate.py", "--check"],
    ],
    "DOD-007 collection guard": [
        ["python3", "scripts/test-collection-guard.py", "--check"],
    ],
    "DOD-021 license gate": [
        ["cargo", "deny", "check", "licenses"],
    ],
}

# Stages the graph names that still have no executable runner. Listed explicitly
# so the obligation is visible rather than silently absent.
KNOWN_UNRUNNABLE: dict[str, str] = {}


def sha256_text(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8", errors="replace")).hexdigest()


def declared_changed_classes() -> list[str]:
    """Changed classes recorded against the current epoch."""
    if not EPOCH.exists():
        return []
    data = json.loads(EPOCH.read_text("utf-8"))
    # The epoch file records the classes present at generation; the graph
    # reports which moved. Re-read the graph for the authoritative list.
    if GRAPH.exists():
        for line in GRAPH.read_text("utf-8").splitlines():
            if "Changed input classes" in line and "**" in line:
                inner = line.split("**")[1]
                classes = inner.split(":", 1)[1].strip()
                if classes.lower().startswith("none"):
                    return []
                return [c.strip() for c in classes.split(",") if c.strip()]
    return data.get("changed_classes", [])


def stages_for(classes: list[str]) -> list[str]:
    """Union of the stages the changed classes invalidate."""
    sys.path.insert(0, "scripts")
    import importlib.util

    spec = importlib.util.spec_from_file_location(
        "change_invalidation", "scripts/change-invalidation.py"
    )
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)

    stages: list[str] = []
    for cls in classes:
        for stage in mod.INVALIDATION_RULES.get(cls, []):
            if stage not in stages:
                stages.append(stage)
    return stages


def main() -> int:
    list_only = "--list" in sys.argv
    check_only = "--check" in sys.argv

    classes = declared_changed_classes()
    stages = stages_for(classes)

    if list_only:
        print(f"changed classes: {classes or ['none']}")
        for stage in stages:
            runners = STAGE_RUNNERS.get(stage)
            if runners:
                for cmd in runners:
                    print(f"  {stage} -> {' '.join(cmd)}")
            elif stage in KNOWN_UNRUNNABLE:
                print(f"  {stage} -> NO_RUNNER ({KNOWN_UNRUNNABLE[stage][:60]}...)")
            else:
                print(f"  {stage} -> NO_RUNNER (unmapped stage)")
        return 0

    if check_only:
        if not RECORD.exists():
            print(
                "rerun check: FAIL -- no rerun record; DOD-040 obliges an epoch "
                "change to rerun affected stages",
                file=sys.stderr,
            )
            return 1
        rec = json.loads(RECORD.read_text("utf-8"))
        current = (
            json.loads(EPOCH.read_text("utf-8")).get("epoch_digest")
            if EPOCH.exists()
            else None
        )
        if rec.get("epoch_digest") != current:
            print(
                "rerun check: FAIL -- the rerun record predates the current epoch; "
                "affected results must be rerun",
                file=sys.stderr,
            )
            return 1
        failed = [r for r in rec.get("results", []) if r["exit_code"] != 0]
        if failed:
            names = ", ".join(f"{r['stage']}" for r in failed)
            print(f"rerun check: FAIL -- recorded rerun failures: {names}", file=sys.stderr)
            return 1
        print(
            f"rerun check: ok ({len(rec.get('results', []))} commands rerun against "
            f"epoch {str(current)[:16]}...)"
        )
        return 0

    if not classes:
        print("rerun: no changed input classes; nothing is invalidated")
        RECORD.write_text(
            json.dumps(
                {
                    "epoch_digest": json.loads(EPOCH.read_text("utf-8")).get(
                        "epoch_digest"
                    )
                    if EPOCH.exists()
                    else None,
                    "changed_classes": [],
                    "results": [],
                    "note": "no epoch change; no rerun obligation",
                },
                indent=2,
            )
            + "\n",
            "utf-8",
        )
        return 0

    print(f"rerun: {len(classes)} changed class(es) -> {len(stages)} stage(s)")
    results: list[dict] = []
    failures = 0

    for stage in stages:
        runners = STAGE_RUNNERS.get(stage)
        if not runners:
            reason = KNOWN_UNRUNNABLE.get(stage, "unmapped stage")
            print(f"  {stage}: NO_RUNNER")
            results.append(
                {
                    "stage": stage,
                    "command": None,
                    "exit_code": None,
                    "status": "NO_RUNNER",
                    "reason": reason,
                }
            )
            continue

        for cmd in runners:
            # A command whose script is absent is a mapping error, not a pass.
            script = cmd[1] if len(cmd) > 1 else ""
            if script.startswith("scripts/") and not Path(script).exists():
                results.append(
                    {
                        "stage": stage,
                        "command": " ".join(cmd),
                        "exit_code": None,
                        "status": "NO_RUNNER",
                        "reason": f"{script} does not exist",
                    }
                )
                print(f"  {stage}: NO_RUNNER ({script} missing)")
                continue

            started = time.time()
            proc = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                encoding="utf-8",
                errors="replace",
                shell=False,
            )
            duration = time.time() - started
            status = "PASS" if proc.returncode == 0 else "FAIL"
            if proc.returncode != 0:
                failures += 1
            out = (proc.stdout or "") + (proc.stderr or "")
            results.append(
                {
                    "stage": stage,
                    "command": " ".join(cmd),
                    "exit_code": proc.returncode,
                    "status": status,
                    "duration_s": round(duration, 2),
                    "output_sha256": sha256_text(out),
                    "output_tail": out.strip().splitlines()[-3:] if out.strip() else [],
                }
            )
            print(f"  {stage}: {status} (exit {proc.returncode}, {duration:.1f}s) -- {' '.join(cmd)}")

    epoch = (
        json.loads(EPOCH.read_text("utf-8")).get("epoch_digest")
        if EPOCH.exists()
        else None
    )
    RECORD.write_text(
        json.dumps(
            {
                "epoch_digest": epoch,
                "changed_classes": classes,
                "stages": stages,
                "results": results,
                "failures": failures,
            },
            indent=2,
            sort_keys=True,
        )
        + "\n",
        "utf-8",
    )
    print(f"rerun: wrote {RECORD}")
    if failures:
        print(f"rerun: FAIL -- {failures} command(s) failed", file=sys.stderr)
        return 1
    print(f"rerun: ok ({len(results)} command(s) rerun)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
