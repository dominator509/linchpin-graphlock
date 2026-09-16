# LINCHPIN — final completion report (DOD-028)

Derived by `scripts/completion-report.py` at 2026-09-16T14:08:38Z from the verification state. Every number below is read from a state file, not transcribed.

## Release verdict

- Machine-validated verdict: **NO_GO**
- Blocking clauses: DOD-001=PARTIAL
- Candidate commit: `c8e62468609b0d5dad4694b98ddfaf0ff80936fe` (base `3ee9c3f41795`)
- Working tree at manifest time: DIRTY (18 changed files)
- Epoch: `a684a5673767ec2d` over 127 tracked inputs

## Verified behavior — executed and observed

- **33 of 42 DoD clauses PASS**: DOD-002, DOD-003, DOD-004, DOD-005, DOD-006, DOD-007, DOD-009, DOD-010, DOD-011, DOD-012, DOD-013, DOD-015, DOD-016, DOD-018, DOD-019, DOD-020, DOD-021, DOD-022, DOD-024, DOD-025, DOD-026, DOD-027, DOD-028, DOD-029, DOD-030, DOD-031, DOD-032, DOD-033, DOD-036, DOD-037, DOD-040, DOD-041, DOD-042
- **57 of 59 requirements are bound to executed acceptance tests** (.agent/verification/REQUIREMENT_TRACEABILITY.csv)
- **484 registry capabilities accounted**, one status each ({'NOT_RUN_BLOCKED_MATERIAL': 473, 'PARTIAL': 10, 'EXTERNAL_REQUIRED': 1})
- **10 controlled defect(s)** are applied, caught by their guarding test, restored byte-for-byte and rerun green (`scripts/mutation-proof.py`)
- Artifacts bound to this run: executable `03dce1f863074ea5…` (12486656 bytes), MSI `164f371cdbc10373…` (5705728 bytes)

Evidence for the executed lanes, each bindable to the digests above: the browser suite, the
exact-artifact lane over CDP, the provider live-fire against a real loopback model, the
installer preservation lane, the recovery drill with measured RPO/RTO/MTTR, the performance
gate with enforced thresholds, the migration matrix, and the clean-environment gate that runs
the whole Rust suite in an ephemeral checkout.

## Partially verified behavior — executed, with the missing half named

**6 clauses**: DOD-001, DOD-008, DOD-014, DOD-017, DOD-023, DOD-038

- **DOD-001**: RE-MEASURED THIS ROUND, because the previous revision of this disposition described a state that no longer exists: 35 of 59 requirements carried executed PASS evidence, and 24 were unbound. Measured now by scripts/bind-requirements.py: 222 collected tests, 59 requirements found, **57 bound to an executed PASS**, 2 unbound. The RULE requires every promised behavior to have a stable requirement ID A…
- **DOD-008** — Not PASS: Unit tests cover boundaries and invalid inputs for domain, evidence, crash_reporter and provider_transport, with mutation proofs for three of them. Not PASS: no coverage measurement and most registry IDs are unexecuted.…
- **DOD-014** — Not PASS: Negative cases exist and were mutation-proven: unreachable endpoint yields TransportError::Unreachable rather than fabricated output, and non-loopback endpoints are refused. Not PASS: no wrong-credential or revoked-permission live-fire against a real provider.…
- **DOD-017** — Not PASS: STALE NOTE CORRECTED. The recorded reason claimed no concurrency run exists -- measured, test_concurrent_writers_preserve_every_successful_insert runs 4 concurrent writers x 25 inserts against file-backed SQLite in WAL mode and asserts the final row count equals the number of successful inserts, proving no silent loss. The engine enforces the primary key, and a failed insert is verified not to mut…
- **DOD-023** — Not PASS: EXECUTED, not assumed, and the counts now live in the cited evidence file rather than in this sentence -- an earlier revision of this disposition carried '22 executed, 3 FAILED, 1 skipped, 0 missing paths' while the measured numbers had moved on, which is the drift DOD-040 exists to catch. scripts/doc-exec.py extracts every command and referenced file path from the operator-facing documents, runs …
- **DOD-038**: PART OF THE CLAUSE IS NOW EXECUTED, AND THE REST IS NOT -- the status moves from FAIL to PARTIAL for that reason, not to PASS. The clause requires 'soak, endurance, fuzz, performance, stress, and recovery durations/workloads' be 'completed at their specified scale', and its OR ELSE is explicit that an abbreviated trial 'is labeled separately' and is 'never PASS for the full requirement'. EXECUTED:…

## Blocked work

**1 clause(s)**: DOD-035

- **DOD-035**: The update and rollback MECHANISM is now executed; the compatibility MATRIX cannot be, because only one version exists. EXECUTED this round by scripts/vault-preservation-e2e.sh, wired as lane 3 of scripts/test-e2e.sh, against the real MSI and the product's real app-data path: install -> seed realistic persistent state (a real SQLite database at the product's real vault filename with an unpredictable per-run canary row, plus a content-addressed blob) -> UPDATE (install over existing) -> UNINSTALL…

## External gates

**2 clause(s)**: DOD-034, DOD-039

- **DOD-034**: A virgin clean room is still required and still absent; this clause cannot be satisfied on a developer host, because the host carries the full Rust/Node/WebView2 toolchain and hidden prerequisites cannot be excluded. ONE PART OF THE BLOCKER IS NOW RESOLVED BY DECISION: PF-016 asked for clean Win10 AND Win11 reference targets, and the owner scoped the supported clean-room matrix to Windows 10 or hi…
- **DOD-039**: CONFIRMED EXTERNAL_REQUIRED by explicit human decision (ADR-005, .agent/evidence/ADR-005-external-signoff-gates.md), not merely left unrun. Human UAT, manual assistive-technology validation, legal review and accredited assessment require named real participants; PF-017 (human UAT / manual AT validators) and PF-018 (patent-workflow independent reviewer) are HUMAN_EXTERNAL and unmet. No authorized v…

## Unverified assumptions

Stated as assumptions rather than results, because no measurement in this run supports them:

- **Cross-machine reproducibility.** Every build, test and artifact in this run was produced on one
  Windows 10 host with one toolchain installation. The clean-environment gate proves a clean CHECKOUT
  is sufficient; it does not prove a different machine reproduces the artifact digest.
- **Behaviour of the unwired provider lanes** (OpenAI, Anthropic, xAI). They report `Unimplemented`
  and `configured=false` in the packaged build, and no call was ever made to a real remote provider.
- **Production data volumes.** The performance gate's workload is 200 events in a local vault; the
  ledger read returns all events for a workspace with no pagination, and that has not been exercised
  at scale.
- **Long-duration stability.** No soak, endurance or fuzz campaign at any specified scale has run.
- **Dated behaviour of external legal sources.** Deadline rules are exercised against the locally
  stored ruleset; no live USPTO source was queried.

## Accepted risks

- **Unsigned release** (ADR-003): no code-signing certificate exists on the host; the unsigned status,
  the expected SmartScreen/AV warnings and the digest-based verification procedure are disclosed in
  RELEASE.md and DEPLOYMENT.md. A digest proves integrity against an out-of-band value, not authorship.
- **MPL-2.0 dependencies** (ADR-002): five crates under MPL-2.0 are used unmodified; the file-level
  review is recorded and the licence is allowlisted in both the gate and the SBOM generator.
- **Two moderate dev-only advisories** (GHSA-82fw-gwwq-j7x9 in `vitest`/`@vitest/mocker`): below the
  enforced `--audit-level high`, not embedded in any shipped artifact, and disclosed in COMMANDS.md.
- **No off-device replication**: RPO after the last backup is unbounded, and the recovery drill asserts
  that work committed after a backup is lost rather than describing the backup as continuous protection.
- **Local paths appear in error messages** by design; secrets never do, and the redaction path is
  asserted by tests and mutation proof.

## Remaining limitations

- **REQ-REL-001 / REQ-REL-004 unbound** — REQ-REL-001, REQ-REL-004: a signed virgin clean room and human accessibility/UAT validators are external participants, not
  automatable work.
- **Update/rollback across versions is unexecuted** because only one version exists; the lanes install
  and remove the same v0.1.0 MSI, and same-version reinstall is weaker evidence than a cross-version
  upgrade.
- **No operator dashboard**: diagnostics are reachable over IPC and rendered nowhere.
- **Registry capabilities**: 473 of 484 remain `NOT_RUN_BLOCKED_MATERIAL` — they require material
  (credentials, external services, human evaluators) that this environment does not hold.

## Deployment state

Auto-deploy is disabled. The artifact is built, digest-pinned and shipped as a manual install; the
next lawful manual action is to run the documented install on a clean machine and record it
(REQ-REL-001), which is exactly the external gate above.

