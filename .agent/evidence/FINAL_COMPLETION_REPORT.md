# LINCHPIN — final completion report (DOD-028)

Derived by `scripts/completion-report.py` at 2026-09-17T17:52:27Z from the verification state. Every number below is read from a state file, not transcribed.

## Release verdict

- Machine-validated verdict: **NO_GO**
- Blocking clauses: DOD-014=PARTIAL
- Candidate commit: `b0e27405fcc03d1750cd2f2e94f2b022937e432d` (base `3ee9c3f41795`)
- Working tree at manifest time: DIRTY (4 changed files)
- Epoch: `98d254624ce9cc5c` over 149 tracked inputs

## Verified behavior — executed and observed

- **37 of 42 DoD clauses PASS**: DOD-002, DOD-003, DOD-004, DOD-005, DOD-006, DOD-007, DOD-008, DOD-009, DOD-010, DOD-011, DOD-012, DOD-013, DOD-015, DOD-016, DOD-017, DOD-018, DOD-019, DOD-020, DOD-021, DOD-022, DOD-023, DOD-024, DOD-025, DOD-026, DOD-027, DOD-028, DOD-029, DOD-030, DOD-031, DOD-032, DOD-033, DOD-035, DOD-036, DOD-037, DOD-040, DOD-041, DOD-042
- **57 of 59 requirements are bound to executed acceptance tests** (.agent/verification/REQUIREMENT_TRACEABILITY.csv)
- **484 registry capabilities accounted**, one status each ({'PASS': 50, 'NOT_APPLICABLE': 417, 'EXTERNAL_REQUIRED': 7, 'PARTIAL': 5, 'NOT_RUN_BLOCKED_MATERIAL': 4, 'DEFERRED_LONG_RUNNING': 1})
- **13 controlled defect(s)** are applied, caught by their guarding test, restored byte-for-byte and rerun green (`scripts/mutation-proof.py`)
- Artifacts bound to this run: executable `00286e822ad93a84…` (12500992 bytes), MSI `8ad083a77cc83277…` (5709824 bytes)

Evidence for the executed lanes, each bindable to the digests above: the browser suite, the
exact-artifact lane over CDP, the provider live-fire against a real loopback model, the
installer preservation lane, the recovery drill with measured RPO/RTO/MTTR, the performance
gate with enforced thresholds, the migration matrix, and the clean-environment gate that runs
the whole Rust suite in an ephemeral checkout.

## Partially verified behavior — executed, with the missing half named

**1 clauses**: DOD-014

- **DOD-014**: STATUS KEPT AT PARTIAL, with the gap described precisely instead of broadly. EXECUTED AND MUTATION-PROVEN: an unreachable loopback endpoint yields TransportError::Unreachable with live=false and text=null rather than fabricated output (a real transport failure, not a simulated one); a non-loopback endpoint is refused with a POLICY error and NO ATTEMPT is made, which is the SPEC-005 confidentiality…

## Blocked work

**0 clause(s)**: none


## External gates

**3 clause(s)**: DOD-001, DOD-034, DOD-039

- **DOD-001**: RECLASSIFIED FROM PARTIAL TO EXTERNAL_REQUIRED, and the clause is NOT satisfied: 57 of 59 requirements carry executed PASS evidence and the remaining two cannot be bound from inside this repository. The RULE requires every promised behavior to have a stable requirement ID AND at least one acceptance test before implementation is declared complete, so the clause is incomplete and its own OR ELSE ('…
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
- **Long-duration stability.** The clause's soak scale IS specified by the pack --
  24/48/72+ hours in `.agent/verification/E2E_SUITE_LIBRARY.md`, the source of E2E-018 -- and it is
  NOT completed: only abbreviated trials have run, labeled separately, so DOD-038 is
  DEFERRED_LONG_RUNNING and no flatline or P99-creep claim is made. Fuzz, stress, performance
  and recovery have no numeric scale in the repository or in the pack.
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
- **Registry capabilities**: 4 of 484 capabilities are `NOT_RUN_BLOCKED_MATERIAL`, each naming the specific material its subject requires (an authorization surface, an isolation boundary, a git hook, hardware key storage), and 417 are `NOT_APPLICABLE` on per-ID applicability evidence.

## Deployment state

Auto-deploy is disabled. The artifact is built, digest-pinned and shipped as a manual install; the
next lawful manual action is to run the documented install on a clean machine and record it
(REQ-REL-001), which is exactly the external gate above.

