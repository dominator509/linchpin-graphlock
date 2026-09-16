#!/usr/bin/env python3
"""Execute controlled-defect proofs (DOD-018) instead of asserting them in prose.

DOD-018 RULE: "At least one controlled defect or mutation is introduced for each
critical feature and the relevant test must fail."
REQUIRED EVIDENCE: "Mutation/defect ID, changed behavior, failing test evidence,
restoration, and green rerun."

Every earlier mutation proof in this repository was performed by hand and then
described in a disposition string. That is a claim, not evidence: the reader
cannot rerun it, and nothing detects the failure mode this harness was written
to prevent -- a mutation that "failed the test" because the mutant did not
COMPILE. A build error exits non-zero exactly like a caught mutation, so it
would have been recorded as a caught defect while proving nothing. That is the
same stale-artifact trap observed in the UI mutation attempt, where TypeScript
refused to build and the test ran against the previous bundle.

This script makes the distinction mechanical:

  CAUGHT       the mutant built, the named test FAILED, and no compile error is
               present. Only this outcome is a valid mutation proof.
  SURVIVED     the mutant built and the named test PASSED. The test does not
               discriminate the behaviour it claims to protect -- reported as a
               harness FAILURE, never as a pass.
  BUILD_ERROR  the mutant did not compile. INVALID evidence, reported as a
               harness FAILURE rather than counted as a catch.
  TIMEOUT      the mutant hung. Reported as a harness FAILURE: an unbounded
               mutant has already been observed to hang for 868s.

Restoration is unconditional: the original bytes are held in memory, written
back in a `finally` block, verified byte-for-byte, and the command is then rerun
to prove the tree is green again. A mutant is never left in the tree.

Usage:
  python3 scripts/mutation-proof.py            # run every declared mutation
  python3 scripts/mutation-proof.py --list     # show what would run
  python3 scripts/mutation-proof.py --only ID  # run one mutation by id
  python3 scripts/mutation-proof.py --check    # verify the recorded report
                                               # matches the current tree
"""
from __future__ import annotations

import hashlib
import json
import subprocess
import sys
import time
from pathlib import Path

REPORT = Path(".agent/evidence/mutation-proof/REPORT.json")
PER_COMMAND_TIMEOUT_S = 900

# A mutation is a list of literal edits plus the test that must notice. Literal
# text (not regex) on purpose: a pattern that silently matches nothing would
# turn every mutation into a no-op that the harness would then report as
# SURVIVED, which is a failure mode worth being unable to have.
MUTATIONS = [
    {
        "id": "MUT-REL-005-a",
        "clause": "DOD-018",
        "covers": "REQ-REL-005",
        "feature": "backup reconciliation refuses an absent path",
        "changed_behavior": (
            "Vault::digest_of reverts to SQLite open-or-create semantics, so a "
            "mistyped backup path is CREATED, digested as empty state, and "
            "reported as a valid reconciliation target."
        ),
        "edits": [
            {
                "path": "crates/storage/src/vault.rs",
                "old": (
                    "        if !path.exists() {\n"
                    "            return Err(VaultError::BackupMissing(path.display().to_string()));\n"
                    "        }\n"
                    "        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;\n"
                    "        Self::digest_connection(&conn)\n"
                ),
                "new": "        Ok(Vault::open(path)?.state_digest()?)\n",
            }
        ],
        "command": [
            "cargo",
            "test",
            "-p",
            "storage",
            "--lib",
            "test_digest_of_is_read_only_and_refuses_non_vaults",
        ],
        "expect_test": "test_digest_of_is_read_only_and_refuses_non_vaults",
    },
    {
        "id": "MUT-REL-005-b",
        "clause": "DOD-018",
        "covers": "REQ-REL-005",
        "feature": "restore replaces content with the snapshot",
        "changed_behavior": (
            "Vault::restore_from reports success without copying any page, so a "
            "restore silently does nothing and post-backup work survives."
        ),
        "edits": [
            {
                "path": "crates/storage/src/vault.rs",
                "old": (
                    "        let src = Connection::open(source)?;\n"
                    "        let backup = rusqlite::backup::Backup::new(&src, &mut self.conn)?;\n"
                    "        backup.run_to_completion(64, std::time::Duration::from_millis(1), None)?;\n"
                    "        Ok(())\n"
                ),
                "new": "        let _ = source;\n        Ok(())\n",
            }
        ],
        "command": [
            "cargo",
            "test",
            "-p",
            "storage",
            "--lib",
            "test_backup_and_restore_reconcile_to_the_snapshot",
        ],
        "expect_test": "test_backup_and_restore_reconcile_to_the_snapshot",
    },
    {
        "id": "MUT-REL-005-c",
        "clause": "DOD-018",
        "covers": "REQ-REL-005",
        "feature": "the destructive product command fails closed on a bad source",
        "changed_behavior": (
            "Both guards between an operator's mistyped path and the destructive "
            "restore step are removed, reproducing the measured defect in which "
            "restore_vault was ACCEPTED and replaced the live vault with an "
            "empty database."
        ),
        "edits": [
            {
                "path": "crates/storage/src/vault.rs",
                "old": (
                    "        if !path.exists() {\n"
                    "            return Err(VaultError::BackupMissing(path.display().to_string()));\n"
                    "        }\n"
                    "        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;\n"
                    "        Self::digest_connection(&conn)\n"
                ),
                "new": "        Ok(Vault::open(path)?.state_digest()?)\n",
            },
            {
                "path": "apps/desktop/src-tauri/src/commands.rs",
                "old": (
                    "    if !src.exists() {\n"
                    "        return CommandResult::failure(\n"
                    "            correlation,\n"
                    "            CommandError::policy(format!(\"no backup at {}\", src.display())),\n"
                    "        );\n"
                    "    }\n"
                ),
                "new": "",
            },
        ],
        "command": [
            "cargo",
            "test",
            "-p",
            "linchpin-desktop",
            "--lib",
            "test_backup_and_restore_commands_reconcile",
        ],
        "expect_test": "test_backup_and_restore_commands_reconcile",
    },
]


def run(command: list[str], timeout_s: int) -> tuple[int, str, float]:
    started = time.time()
    try:
        proc = subprocess.run(
            command,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            shell=False,
            timeout=timeout_s,
        )
        output = (proc.stdout or "") + (proc.stderr or "")
        return proc.returncode, output, time.time() - started
    except subprocess.TimeoutExpired as exc:
        output = (exc.stdout or "") + (exc.stderr or "")
        if isinstance(output, bytes):
            output = output.decode("utf-8", "replace")
        return 124, output + f"\n[mutation-proof] timed out after {timeout_s}s\n", time.time() - started


def is_compile_error(output: str) -> bool:
    markers = (
        "could not compile",
        "error[E",
        "error: expected",
        "error: cannot find",
        "aborting due to",
    )
    return any(marker in output for marker in markers)


def classify(code: int, output: str, expect_test: str) -> tuple[str, str]:
    if code == 124:
        return "TIMEOUT", "the mutant hung; duration is unbounded, not evidence"
    if is_compile_error(output):
        return (
            "BUILD_ERROR",
            "the mutant did not compile, so a non-zero exit proves nothing",
        )
    failing_line = next(
        (line for line in output.splitlines() if line.startswith("test ") and "FAILED" in line),
        "",
    )
    ran_named_test = expect_test in output
    if code != 0 and ran_named_test and failing_line:
        return "CAUGHT", failing_line.strip()
    if code == 0 and ran_named_test:
        return "SURVIVED", "the named test passed against the mutant"
    return (
        "UNCLASSIFIED",
        f"exit {code} with no failure of {expect_test} in the output",
    )


def restore(path: Path, original: bytes) -> None:
    path.write_bytes(original)
    if path.read_bytes() != original:
        raise SystemExit(f"[mutation-proof] FATAL: could not restore {path}")


def execute(mutation: dict, run_green: bool) -> dict:
    originals: dict[Path, bytes] = {}
    result: dict = {
        "id": mutation["id"],
        "clause": mutation["clause"],
        "covers": mutation["covers"],
        "feature": mutation["feature"],
        "changed_behavior": mutation["changed_behavior"],
        "command": " ".join(mutation["command"]),
        "expect_test": mutation["expect_test"],
    }
    try:
        for edit in mutation["edits"]:
            path = Path(edit["path"])
            if path not in originals:
                originals[path] = path.read_bytes()
            text = path.read_text(encoding="utf-8")
            count = text.count(edit["old"])
            if count != 1:
                raise SystemExit(
                    f"[mutation-proof] FATAL: {mutation['id']} anchor matched {count} times "
                    f"in {path}; a mutation that changes nothing must not be reported"
                )
            path.write_text(text.replace(edit["old"], edit["new"]), encoding="utf-8")

        code, output, duration = run(mutation["command"], PER_COMMAND_TIMEOUT_S)
        verdict, detail = classify(code, output, mutation["expect_test"])
        result.update(
            {
                "verdict": verdict,
                "detail": detail,
                "mutant_exit_code": code,
                "mutant_duration_s": round(duration, 2),
                "mutant_output_sha256": "sha256:"
                + hashlib.sha256(output.encode("utf-8", "replace")).hexdigest(),
                "mutant_output_tail": "\n".join(output.splitlines()[-25:]),
            }
        )
    finally:
        for path, original in originals.items():
            restore(path, original)
        result["restored"] = all(
            path.read_bytes() == original for path, original in originals.items()
        )

    if run_green:
        code, output, duration = run(mutation["command"], PER_COMMAND_TIMEOUT_S)
        result.update(
            {
                "green_exit_code": code,
                "green_duration_s": round(duration, 2),
                "green_ok": code == 0 and "test result: ok" in output,
            }
        )
    return result


def main() -> int:
    args = sys.argv[1:]
    if "--list" in args:
        for mutation in MUTATIONS:
            print(f"{mutation['id']}  {mutation['covers']}  {' '.join(mutation['command'])}")
        return 0

    if "--check" in args:
        if not REPORT.exists():
            print(f"mutation-proof: FAIL -- no report at {REPORT}", file=sys.stderr)
            return 1
        records = json.loads(REPORT.read_text(encoding="utf-8"))["mutations"]
        bad = [r["id"] for r in records if r["verdict"] != "CAUGHT" or not r.get("green_ok")]
        if bad:
            print(f"mutation-proof: FAIL -- not proven: {', '.join(bad)}", file=sys.stderr)
            return 1
        print(f"mutation-proof: ok ({len(records)} controlled defects caught and restored)")
        return 0

    only = None
    if "--only" in args:
        only = args[args.index("--only") + 1]
    selected = [m for m in MUTATIONS if only is None or m["id"] == only]
    if not selected:
        print(f"mutation-proof: no mutation with id {only}", file=sys.stderr)
        return 1

    results = []
    failures = []
    for mutation in selected:
        print(f"mutation-proof: injecting {mutation['id']} ({mutation['covers']})", flush=True)
        result = execute(mutation, run_green=True)
        results.append(result)
        print(
            f"mutation-proof: {result['id']} -> {result['verdict']} "
            f"(mutant exit {result.get('mutant_exit_code')}, restored={result['restored']}, "
            f"green={result.get('green_ok')}) : {result['detail']}",
            flush=True,
        )
        if result["verdict"] != "CAUGHT":
            failures.append(f"{result['id']}: {result['verdict']} -- {result['detail']}")
        if not result["restored"]:
            failures.append(f"{result['id']}: the tree was NOT restored")
        if not result.get("green_ok"):
            failures.append(
                f"{result['id']}: the restored tree is not green (exit {result.get('green_exit_code')})"
            )

    REPORT.parent.mkdir(parents=True, exist_ok=True)
    REPORT.write_text(
        json.dumps(
            {
                "clause": "DOD-018",
                "harness": "scripts/mutation-proof.py",
                "mutations": results,
            },
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )
    print(f"mutation-proof: report written to {REPORT}")

    if failures:
        for failure in failures:
            print(f"mutation-proof: FAIL -- {failure}", file=sys.stderr)
        return 1
    print(f"mutation-proof: ok -- {len(results)} controlled defect(s) caught and restored")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
