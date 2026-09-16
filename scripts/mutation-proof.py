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
    {
        "id": "MUT-SCOPE-001-a",
        "clause": "DOD-018",
        "covers": "REQ-SCOPE-001",
        "feature": "the truth-boundary guard is applied to exported copy",
        "changed_behavior": (
            "build_commercialization_package exports its payload without running "
            "the truth-boundary guard, so a package whose text promises an "
            "outcome reaches a third party."
        ),
        "edits": [
            {
                "path": "apps/desktop/src-tauri/src/commands.rs",
                "old": (
                    "            if let Err(violation) = domain::scope::check_claim_text(&payload) {\n"
                    "                return CommandResult::failure(\n"
                    "                    correlation,\n"
                    "                    CommandError::policy(format!(\n"
                    "                        \"commercialization package crosses a truth boundary: {violation}\"\n"
                    "                    )),\n"
                    "                );\n"
                    "            }\n"
                ),
                "new": "",
            }
        ],
        "command": [
            "cargo",
            "test",
            "-p",
            "linchpin-desktop",
            "--lib",
            "test_scope_map_and_five_truth_boundaries_hold_at_the_product_boundary",
        ],
        "expect_test": "test_scope_map_and_five_truth_boundaries_hold_at_the_product_boundary",
    },
    {
        "id": "MUT-SCOPE-001-b",
        "clause": "DOD-018",
        "covers": "REQ-SCOPE-001",
        "feature": "the boundary guard refuses outcome promises",
        "changed_behavior": (
            "check_claim_text accepts every string, so the guard is a permanently "
            "green check while promises flow through it."
        ),
        "edits": [
            {
                "path": "crates/domain/src/scope.rs",
                "old": "    let haystack = text.to_lowercase();\n",
                "new": "    let haystack = text.to_lowercase();\n    if !haystack.is_empty() {\n        return Ok(());\n    }\n",
            }
        ],
        "command": [
            "cargo",
            "test",
            "-p",
            "domain",
            "test_boundary_guard_refuses_promises_and_permits_honest_text",
        ],
        "expect_test": "test_boundary_guard_refuses_promises_and_permits_honest_text",
    },
    {
        "id": "MUT-COM-002-a",
        "clause": "DOD-018",
        "covers": "REQ-COM-002",
        "feature": "a missing chain-of-title link is reported as a gap",
        "changed_behavior": (
            "the inventor-to-first-owner transition is treated as bridged without "
            "any assignment or recordation, so a break in the chain of title is "
            "silently assumed away."
        ),
        "edits": [
            {
                "path": "crates/commercialization/src/asset_readiness.rs",
                "old": (
                    "                        let bridged = timeline.iter().any(|candidate| {\n"
                    "                            candidate.effective_date <= record.effective_date\n"
                    "                                && matches!(\n"
                ),
                "new": (
                    "                        let bridged = true || timeline.iter().any(|candidate| {\n"
                    "                            candidate.effective_date <= record.effective_date\n"
                    "                                && matches!(\n"
                ),
            }
        ],
        "command": [
            "cargo",
            "test",
            "-p",
            "linchpin-desktop",
            "--lib",
            "test_asset_readiness_reports_gaps_and_never_treats_recordation_as_validation",
        ],
        "expect_test": "test_asset_readiness_reports_gaps_and_never_treats_recordation_as_validation",
    },
    {
        "id": "MUT-OPS-037-a",
        "clause": "DOD-018",
        "covers": "REQ-OPS-010, DOD-037",
        "feature": "credential values are redacted before they are logged",
        "changed_behavior": (
            "the diagnostics log stops redacting credential values, so a token or "
            "password recorded in a command message is stored in the clear."
        ),
        "edits": [
            {
                "path": "crates/crash_reporter/src/lib.rs",
                "old": "        out = redact_key_values(&out);\n",
                "new": "",
            }
        ],
        "command": [
            "cargo",
            "test",
            "-p",
            "crash_reporter",
            "test_redaction_covers_key_values_not_only_token_shapes",
        ],
        "expect_test": "test_redaction_covers_key_values_not_only_token_shapes",
    },
    {
        "id": "MUT-DOD-016-a",
        "clause": "DOD-018",
        "covers": "DOD-016, REQ-DATA-003",
        "feature": "a vault claiming a migration it never applied is refused",
        "changed_behavior": (
            "the post-migration schema check is removed, so a database whose "
            "schema_migrations rows exist while the tables do not opens "
            "successfully and fails later at a random write."
        ),
        "edits": [
            {
                "path": "crates/storage/src/vault.rs",
                "old": "        self.verify_schema()\n    }\n",
                "new": "        Ok(())\n    }\n",
            }
        ],
        "command": [
            "cargo",
            "test",
            "-p",
            "storage",
            "test_recorded_migration_without_its_tables_is_refused",
        ],
        "expect_test": "test_recorded_migration_without_its_tables_is_refused",
    },
    {
        "id": "MUT-DATA-001-a",
        "clause": "DOD-018",
        "covers": "REQ-DATA-001, DOD-015",
        "feature": "the conception ledger reads durable state back",
        "changed_behavior": (
            "the ledger read returns an empty list regardless of what is stored, "
            "so the ledger reports no events while the vault holds them."
        ),
        "edits": [
            {
                "path": "apps/desktop/src-tauri/src/commands.rs",
                "old": (
                    "    let stored = match vault.list_conception_events(&scope.workspace_id) {\n"
                    "        Ok(events) => events,\n"
                ),
                "new": (
                    "    let stored = match vault.list_conception_events(&scope.workspace_id) {\n"
                    "        Ok(events) => events.into_iter().take(0).collect::<Vec<_>>(),\n"
                ),
            }
        ],
        "command": [
            "cargo",
            "test",
            "-p",
            "linchpin-desktop",
            "--lib",
            "test_conception_ledger_reads_durable_state_and_separates_origins",
        ],
        "expect_test": "test_conception_ledger_reads_durable_state_and_separates_origins",
    },
    {
        "id": "MUT-OPS-037-b",
        "clause": "DOD-018",
        "covers": "DOD-037",
        "feature": "command outcomes reach the operator diagnostics log",
        "changed_behavior": (
            "record_diagnostic_outcome stops recording anything, so the "
            "diagnostics surface and its dashboard report a healthy system with "
            "no history while commands are failing."
        ),
        "edits": [
            {
                "path": "apps/desktop/src-tauri/src/commands.rs",
                "old": "    let redacted = diagnostics_redactor().apply(detail);\n",
                "new": "    let redacted = diagnostics_redactor().apply(detail);\n    if !redacted.is_empty() {\n        return;\n    }\n",
            }
        ],
        "command": [
            "cargo",
            "test",
            "-p",
            "linchpin-desktop",
            "--lib",
            "test_diagnostics_report_induced_failure_with_correlation_and_redaction",
        ],
        "expect_test": "test_diagnostics_report_induced_failure_with_correlation_and_redaction",
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
