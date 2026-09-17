#!/usr/bin/env python3
"""Per-advisory assessment of the shipped dependency graph (GEN-043).

WHY THIS EXISTS. GEN-043 ("Vulnerability Assessment") was recorded NOT_APPLICABLE with
"no vulnerability assessment record exists". GEN-042 scans for advisories and GEN-021
discloses them; neither ASSESSES one. A scan that prints "2 moderate" and moves on is a
finding without a decision, and the difference matters: an advisory in a dev-only test
runner is not the same risk as one in a shipped parser, and nothing in this repository
said which was which in a form a gate could check.

WHAT THIS DOES
  1. Runs the two advisory scanners (`cargo deny --format json check advisories`,
     `pnpm audit --json`) and normalizes what they find: ecosystem, advisory id, package,
     severity, title, and the dependency PATHS the npm scanner reports.
  2. Reads the authored register `.agent/verification/state/ADVISORY_ASSESSMENTS.json`,
     which carries one entry per advisory: decision, rationale, reachability statement,
     owner, and the observed paths it was assessed against.
  3. FAILS when an advisory found by a scanner has no entry (a finding without a
     decision), when an entry is malformed (decision outside the set, no rationale, no
     reachability statement, no owner), when an ACCEPT entry's observed paths disagree
     with what the scanner reported (a stale assessment of a moved dependency), or when an
     entry claims an advisory that no longer appears AND still claims it is accepted
     rather than RESOLVED.
  4. Renders `.agent/evidence/advisory-assessment/STATUS.md` and writes report.json.

DECISIONS ARE NOT OPINIONS HERE: an entry must say where the vulnerable package sits in
the graph, which is the fact that decides whether the product is exposed.
"""
from __future__ import annotations

import json
import pathlib
import shutil
import subprocess
import sys

ROOT = pathlib.Path(".").resolve()
EVIDENCE = ROOT / ".agent/evidence/advisory-assessment"
REGISTER = ROOT / ".agent/verification/state/ADVISORY_ASSESSMENTS.json"
DECISIONS = {"FIX", "MITIGATE", "ACCEPT", "NOT_REACHABLE", "RESOLVED"}


def run(argv: list[str]) -> tuple[int, str]:
    # Windows resolves pnpm only through its .cmd shim, so argv[0] is looked up on PATH
    # before it is executed; measured, calling "pnpm" directly raises
    # FileNotFoundError: [WinError 2], which would have been reported as a scanner failure
    # rather than as a harness bug.
    executable = shutil.which(argv[0]) or shutil.which(f"{argv[0]}.cmd")
    if executable is None:
        return 127, f"{argv[0]} is not on PATH"
    proc = subprocess.run(
        [executable, *argv[1:]], cwd=ROOT, capture_output=True, text=True, errors="replace"
    )
    return proc.returncode, (proc.stdout or "") + (proc.stderr or "")


def rust_advisories() -> tuple[list[dict], str]:
    code, out = run(["cargo", "deny", "--format", "json", "check", "advisories"])
    advisories: list[dict] = []
    note = "cargo-deny reported no advisories" if code == 0 else f"cargo-deny exited {code}"
    for line in out.splitlines():
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            record = json.loads(line)
        except json.JSONDecodeError:
            continue
        if record.get("type") != "diagnostic":
            continue
        fields = record.get("fields", {})
        advisory = fields.get("advisory") or {}
        if not advisory:
            continue
        advisories.append(
            {
                "ecosystem": "cargo",
                "id": str(advisory.get("id") or advisory.get("package") or "unknown"),
                "package": str(advisory.get("package") or "unknown"),
                "severity": str(advisory.get("severity") or "unknown"),
                "title": str(advisory.get("title") or "")[:200],
                "paths": [],
            }
        )
    return advisories, note


def npm_advisories() -> tuple[list[dict], str]:
    code, out = run(["pnpm", "audit", "--json"])
    advisories: list[dict] = []
    note = f"pnpm audit exited {code}"
    start = out.find("{")
    if start < 0:
        return advisories, note
    try:
        payload = json.loads(out[start:])
    except json.JSONDecodeError:
        return advisories, note
    for advisory_id, entry in (payload.get("advisories") or {}).items():
        paths: list[str] = []
        versions: list[str] = []
        for finding in entry.get("findings") or []:
            paths.extend(finding.get("paths") or [])
            # MEASURED DEFECT, fixed here: this used to record the finding's VERSION
            # as if it were the package name, so the register said
            # `"package": "3.2.7"` for an advisory in vitest. The package name is
            # `module_name`; the version is one attribute of the finding.
            versions.append(str(finding.get("version") or "unknown"))
        package = str(entry.get("module_name") or entry.get("module") or "unknown")
        advisories.append(
            {
                "ecosystem": "npm",
                "id": f"npm:{advisory_id}",
                "package": package,
                "installed_versions": sorted(set(versions)),
                "severity": str(entry.get("severity") or "unknown"),
                "title": str(entry.get("title") or "")[:200],
                "paths": sorted(set(paths)),
            }
        )
    return advisories, note


def load_register() -> dict:
    if not REGISTER.exists():
        raise SystemExit(
            f"advisory-assessment: FAIL -- no register at {REGISTER}. Every advisory needs a "
            "recorded decision before this capability can be claimed."
        )
    return json.loads(REGISTER.read_text(encoding="utf-8"))


def main() -> int:
    check_only = "--check" in sys.argv
    rust, rust_note = rust_advisories()
    npm, npm_note = npm_advisories()
    found = {a["id"]: a for a in rust + npm}

    register = load_register()
    entries = {entry["id"]: entry for entry in register.get("assessments", [])}

    problems: list[str] = []
    for advisory_id, advisory in sorted(found.items()):
        entry = entries.get(advisory_id)
        if entry is None:
            problems.append(
                f"{advisory_id} ({advisory['ecosystem']} {advisory['package']}, "
                f"{advisory['severity']}) has NO recorded assessment"
            )
            continue
        if entry.get("decision") not in DECISIONS:
            problems.append(f"{advisory_id}: decision {entry.get('decision')!r} outside {sorted(DECISIONS)}")
        # RESOLVED means "no scanner reports this any more"; using it for a LIVE advisory
        # is how a finding gets filed as closed without being assessed. MEASURED: the first
        # version of this register marked a live advisory RESOLVED and this gate accepted
        # it, because the check only ran in the other direction (entries not found must be
        # RESOLVED) and never asked whether a found entry may be RESOLVED.
        if entry.get("decision") == "RESOLVED":
            problems.append(
                f"{advisory_id}: is reported by a scanner but its assessment says RESOLVED; "
                "a live finding needs FIX, MITIGATE, ACCEPT or NOT_REACHABLE"
            )
        # Rationale and reachability must be real sentences; the owner is a name, so a
        # length threshold there would reject "product owner" for being short.
        for field, minimum in (("rationale", 40), ("reachability", 40), ("owner", 3)):
            value = str(entry.get(field, "")).strip()
            if len(value) < minimum:
                problems.append(f"{advisory_id}: {field} is missing or too short to be a decision")
        if entry.get("decision") == "ACCEPT":
            recorded_paths = sorted(set(entry.get("observed_paths") or []))
            if recorded_paths != advisory["paths"]:
                problems.append(
                    f"{advisory_id}: the assessment was made against paths {recorded_paths} but the "
                    f"scanner now reports {advisory['paths']} -- the dependency moved, so the "
                    "assessment is stale"
                )
        # MEASURED DEFECT, fixed here: nothing compared the PACKAGE NAME the register
        # records with the package the scanner names. The npm normalizer used to write
        # the finding's version into that field, so two advisories were registered as
        # `"package": "3.2.7"` -- an assessment about a version number, not a package --
        # and this gate accepted it, because it only read reachability, paths and the
        # decision. An assessment must name the package it is about.
        recorded_package = str(entry.get("package", "")).strip()
        if recorded_package != advisory["package"]:
            problems.append(
                f"{advisory_id}: the register records package {recorded_package!r} but the scanner "
                f"reports {advisory['package']!r}"
            )
    for advisory_id, entry in sorted(entries.items()):
        if advisory_id not in found and entry.get("decision") != "RESOLVED":
            problems.append(
                f"{advisory_id}: recorded as {entry.get('decision')} but no scanner reports it any "
                "more; mark it RESOLVED so the register does not carry decisions about nothing"
            )

    report = {
        "gate": "advisory-assessment",
        "covers": ["GEN-043", "DOD-021"],
        "harness": "scripts/assess-advisories.py",
        "scanners": {"cargo-deny": rust_note, "pnpm-audit": npm_note},
        "found": sorted(found.values(), key=lambda a: a["id"]),
        "assessments": register.get("assessments", []),
        "problems": problems,
        "verdict": "PASS" if not problems else "FAIL",
    }

    if check_only:
        recorded = EVIDENCE / "report.json"
        if not recorded.exists():
            print("advisory-assessment check: FAIL -- no recorded report", file=sys.stderr)
            return 1
        previous = json.loads(recorded.read_text(encoding="utf-8"))
        if previous.get("found") != report["found"]:
            print(
                "advisory-assessment check: FAIL -- the advisory set changed since the report was "
                "written; re-assess with python3 scripts/assess-advisories.py",
                file=sys.stderr,
            )
            return 1
        if problems:
            for problem in problems:
                print(f"  {problem}", file=sys.stderr)
            print("advisory-assessment check: FAIL", file=sys.stderr)
            return 1
        print(f"advisory-assessment check: ok ({len(found)} advisory(ies), every one assessed)")
        return 0

    EVIDENCE.mkdir(parents=True, exist_ok=True)
    (EVIDENCE / "report.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")

    lines = [
        "# Advisory assessment (GEN-043)",
        "",
        "Generated by `scripts/assess-advisories.py` from the two scanners and the authored",
        "register `.agent/verification/state/ADVISORY_ASSESSMENTS.json`. Do not hand-edit.",
        "",
        f"- Verdict: **{report['verdict']}**",
        f"- Scan: {rust_note}; {npm_note}",
        f"- Advisories found: **{len(found)}**; assessed: **{len(entries)}**",
        "",
        "## Advisories and decisions",
        "",
        "| Advisory | Ecosystem | Package | Severity | Decision | Reachability |",
        "| --- | --- | --- | --- | --- | --- |",
    ]
    for advisory in sorted(found.values(), key=lambda a: a["id"]):
        entry = entries.get(advisory["id"], {})
        lines.append(
            f"| {advisory['id']} | {advisory['ecosystem']} | {advisory['package']} | "
            f"{advisory['severity']} | {entry.get('decision', 'UNASSESSED')} | "
            f"{str(entry.get('reachability', ''))[:160]} |"
        )
    if not found:
        lines.append("| — | — | — | — | — | no advisory reported by either scanner |")
    lines += ["", "## Rationale per advisory", ""]
    for advisory in sorted(found.values(), key=lambda a: a["id"]):
        entry = entries.get(advisory["id"], {})
        lines += [
            f"### {advisory['id']} — {advisory['title'] or advisory['package']}",
            "",
            f"- Decision: **{entry.get('decision', 'UNASSESSED')}** (owner: {entry.get('owner', 'unrecorded')})",
            f"- Reachability: {entry.get('reachability', 'unrecorded')}",
            f"- Rationale: {entry.get('rationale', 'unrecorded')}",
            f"- Observed paths at assessment: {', '.join(entry.get('observed_paths', [])) or 'none reported'}",
            "",
        ]
    lines += [
        "## What this gate refuses",
        "",
        "It fails when a scanner reports an advisory with no decision, when a decision lacks a",
        "rationale, reachability statement or owner, when an ACCEPT assessment was made against",
        "different dependency paths than the scanner now reports (the dependency moved), and when",
        "the register still holds a decision about an advisory nothing reports any more.",
        "",
        "## Reproduce",
        "",
        "```",
        "python3 scripts/assess-advisories.py",
        "python3 scripts/assess-advisories.py --check",
        "```",
        "",
    ]
    (EVIDENCE / "STATUS.md").write_text("\n".join(lines), encoding="utf-8")

    if problems:
        print(f"advisory-assessment: FAIL -- {len(problems)} problem(s)", file=sys.stderr)
        for problem in problems:
            print(f"  {problem}", file=sys.stderr)
        return 1
    print(f"advisory-assessment: ok ({len(found)} advisory(ies), every one assessed)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
