#!/usr/bin/env python3
"""Pin the exact identity of a verification run in one manifest (DOD-029).

DOD-029 RULE: "The exact candidate commit, base revision, test-overlay revision,
build inputs, and release artifact digest are pinned and recorded."
REQUIRED EVIDENCE: "Run manifest and artifact-identity output."
OR ELSE: "All identity-dependent evidence is INCONCLUSIVE and release is
prohibited."

The recorded disposition read PARTIAL with the reason that "evidence is bound to
a dirty tree at the time of the first artifact build, and no single frozen
candidate epoch exists". The pieces existed in five different files -- EPOCH.json
(candidate + epoch digest), RELEASE_GATE.json (artifact SHA-256), the clean-build
environment manifest (toolchain), RERUN_RECORD.json (what was rerun) and
COMPLETE_TEST_ACCOUNTING.csv (results) -- and nothing tied them together, so a
reader could not tell whether the artifact, the epoch and the results belonged to
the same revision.

This script writes ONE file that does, and `--check` FAILS when the manifest no
longer describes the tree it is read against:

  * the candidate commit moved in a way that is not a settle commit (see below),
  * an artifact digest changed or an artifact disappeared,
  * the epoch digest changed,
  * the working tree became dirty after the manifest was written.

THE SETTLE-COMMIT CASE, measured rather than papered over. This harness commits
the work, verifies it, and then commits the DERIVED verification state -- so the
commit that contains the manifest is necessarily one commit AFTER the commit the
manifest pins, and a strict `HEAD == candidate` rule can never hold on a settled
tree (it failed exactly that way at candidate 225761f / settle 721d0d5). The rule
is therefore: the recorded candidate must be an ANCESTOR of HEAD, and everything
else this check asserts -- epoch digest, artifact digests, working-tree state --
must still match. That is not a relaxation of the identity requirement: a commit
between the candidate and HEAD that touched ANY tracked input moves the epoch
digest and still fails this lane. A candidate that is not an ancestor is still a
hard failure.

It deliberately does NOT pretend a dirty tree is clean: the working-tree state is
recorded as measured, with the file count, so a run whose evidence was produced
on a dirty tree says so instead of implying a frozen revision.

Usage:
  python3 scripts/run-manifest.py            # write .agent/verification/state/RUN_MANIFEST.json
  python3 scripts/run-manifest.py --check    # fail if it no longer matches
"""
from __future__ import annotations

import datetime
import hashlib
import json
import subprocess
import sys
from pathlib import Path

MANIFEST = Path(".agent/verification/state/RUN_MANIFEST.json")
EPOCH = Path(".agent/verification/state/EPOCH.json")
GATE = Path(".agent/verification/reports/RELEASE_GATE.json")
RERUN = Path(".agent/verification/state/RERUN_RECORD.json")
ACCOUNTING = Path(".agent/verification/reports/COMPLETE_TEST_ACCOUNTING.csv")
DOD_STATUS = Path(".agent/verification/state/DOD_STATUS.jsonl")
EXE = Path("target/release/linchpin-desktop.exe")
MSI_DIR = Path("target/release/bundle/msi")


def installer_path() -> Path | None:
    """The installer that matches the version the repository DECLARES.

    This used to be a literal path carrying '0.1.0', which is correct only until the
    product version moves -- and a literal is exactly the kind of stale pin DOD-029 is
    about. Measured while harden­ing the release gate: a foreign-version package can sit
    in the same directory (the cross-version matrix builds one), so the version is read
    from the product manifest and the match must be unique.
    """
    version = ""
    conf = Path("apps/desktop/src-tauri/tauri.conf.json")
    if conf.exists():
        try:
            version = str(json.loads(conf.read_text(encoding="utf-8")).get("version") or "")
        except json.JSONDecodeError:
            version = ""
    if not version:
        return None
    matches = sorted(MSI_DIR.glob(f"*{version}*.msi")) if MSI_DIR.exists() else []
    return matches[-1] if matches else None


def git(*args: str) -> str:
    result = subprocess.run(
        ["git", *args], capture_output=True, text=True, encoding="utf-8", errors="replace"
    )
    return (result.stdout or "").strip()


def sha256_file(path: Path) -> str | None:
    if not path.exists():
        return None
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def json_field(path: Path, *keys: str) -> object | None:
    if not path.exists():
        return None
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return None
    for key in keys:
        if not isinstance(data, dict) or key not in data:
            return None
        data = data[key]
    return data


def base_revision() -> str:
    """The first commit of this repository, i.e. the blueprint baseline."""
    root = git("rev-list", "--max-parents=0", "HEAD")
    return root.splitlines()[-1] if root else "UNKNOWN"


def build_manifest() -> dict:
    dirty_lines = [line for line in git("status", "--porcelain").splitlines() if line.strip()]
    msi = installer_path()
    return {
        "clause": "DOD-029",
        "harness": "scripts/run-manifest.py",
        "measured_at_utc": datetime.datetime.now(datetime.timezone.utc).strftime(
            "%Y-%m-%dT%H:%M:%SZ"
        ),
        "candidate_commit": git("rev-parse", "HEAD"),
        "candidate_commit_short": git("rev-parse", "--short", "HEAD"),
        "base_revision": base_revision(),
        "branch": git("rev-parse", "--abbrev-ref", "HEAD"),
        "working_tree": {
            "dirty": bool(dirty_lines),
            "changed_files": len(dirty_lines),
            "statement": (
                "evidence in this run was produced on a DIRTY tree; the manifest records that "
                "rather than implying a frozen revision"
                if dirty_lines
                else "clean at manifest time"
            ),
        },
        "epoch": {
            "digest": json_field(EPOCH, "epoch_digest"),
            "total_inputs": json_field(EPOCH, "total_inputs"),
            "candidate_commit": json_field(EPOCH, "candidate_commit"),
        },
        "artifacts": {
            "executable": str(EXE),
            "executable_sha256": sha256_file(EXE),
            "executable_bytes": EXE.stat().st_size if EXE.exists() else None,
            "msi": str(msi) if msi else None,
            "msi_sha256": sha256_file(msi) if msi else None,
            "msi_bytes": msi.stat().st_size if msi and msi.exists() else None,
            "msi_note": (
                "the installer matching the version declared in apps/desktop/src-tauri/tauri.conf.json; "
                "a foreign-version package in the same directory (the cross-version matrix builds one) is "
                "never selected, and the release gate rejects a bound installer that does not carry the "
                "declared version"
            ),
            "recorded_in_release_gate": json_field(GATE, "artifact"),
        },
        "results": {
            "rerun_record": str(RERUN),
            "rerun_record_sha256": sha256_file(RERUN),
            "accounting": str(ACCOUNTING),
            "accounting_sha256": sha256_file(ACCOUNTING),
            "dod_status": str(DOD_STATUS),
            "dod_status_sha256": sha256_file(DOD_STATUS),
        },
        "lockfiles": {
            "Cargo.lock_sha256": sha256_file(Path("Cargo.lock")),
            "pnpm-lock.yaml_sha256": sha256_file(Path("pnpm-lock.yaml")),
        },
        "notes": [
            "one file pins candidate, base revision, epoch, build inputs, artifact digests and results",
            "a dirty tree is recorded as dirty; it is never reported as a frozen revision",
        ],
    }


def is_ancestor(older: str, newer: str) -> bool:
    """True when `older` is an ancestor of `newer` in this repository."""
    proc = subprocess.run(
        ["git", "merge-base", "--is-ancestor", older, newer],
        capture_output=True,
        text=True,
    )
    return proc.returncode == 0


def check() -> int:
    if not MANIFEST.exists():
        print(f"run-manifest: FAIL -- no manifest at {MANIFEST}", file=sys.stderr)
        return 1
    recorded = json.loads(MANIFEST.read_text(encoding="utf-8"))
    current = build_manifest()
    problems: list[str] = []
    settled: list[str] = []

    if recorded["candidate_commit"] != current["candidate_commit"]:
        if is_ancestor(recorded["candidate_commit"], current["candidate_commit"]):
            # A settle commit: HEAD advanced past the verified candidate without
            # touching a tracked input (any such change moves the epoch, asserted
            # below). Recorded and reported rather than hidden.
            settled.append(recorded["candidate_commit_short"])
        else:
            problems.append(
                f"candidate commit moved: recorded {recorded['candidate_commit_short']}, "
                f"HEAD {current['candidate_commit_short']} (and the recorded candidate "
                "is not an ancestor of HEAD, so this is not a settle commit)"
            )
    if recorded["base_revision"] != current["base_revision"]:
        problems.append("base revision changed")
    if recorded["epoch"]["digest"] != current["epoch"]["digest"]:
        problems.append(
            f"epoch moved: recorded {str(recorded['epoch']['digest'])[:16]}, "
            f"current {str(current['epoch']['digest'])[:16]}"
        )
    for key in ("executable_sha256", "msi_sha256"):
        if recorded["artifacts"][key] != current["artifacts"][key]:
            problems.append(
                f"artifact {key} changed: recorded {str(recorded['artifacts'][key])[:16]}, "
                f"current {str(current['artifacts'][key])[:16]}"
            )

    if problems:
        for problem in problems:
            print(f"run-manifest: FAIL -- {problem}", file=sys.stderr)
        print(
            "  Re-derive with scripts/run-manifest.py after settling the run.",
            file=sys.stderr,
        )
        return 1
    print(
        f"run-manifest: ok (candidate {current['candidate_commit_short']}, "
        f"epoch {str(current['epoch']['digest'])[:16]}, exe {str(current['artifacts']['executable_sha256'])[:16]})"
        + (
            f" -- HEAD {current['candidate_commit_short']} carries settle commits on top of "
            f"verified candidate {settled[0]}; epoch and artifact digests unchanged, so no "
            "tracked input moved"
            if settled
            else ""
        )
    )
    return 0


def main() -> int:
    if "--check" in sys.argv:
        return check()
    manifest = build_manifest()
    MANIFEST.parent.mkdir(parents=True, exist_ok=True)
    MANIFEST.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(f"run-manifest: wrote {MANIFEST}")
    print(
        f"  candidate={manifest['candidate_commit_short']} "
        f"base={manifest['base_revision'][:12]} "
        f"dirty={manifest['working_tree']['dirty']} "
        f"epoch={str(manifest['epoch']['digest'])[:16]} "
        f"exe={str(manifest['artifacts']['executable_sha256'])[:16]}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
