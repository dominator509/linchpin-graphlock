# Definition of Done

### DOD-001 [task,feature,milestone,node,release]
RULE: Every promised behavior has a stable requirement ID and at least one acceptance test before implementation is declared complete.
BECAUSE: Without stable identity, scope drifts and tests cannot prove the promised behavior.
REQUIRED EVIDENCE: Requirement-to-test traceability row plus executed acceptance evidence.
OR ELSE: The item is INCOMPLETE and NODE_DONE/GO is prohibited until the mapping exists and passes.

### DOD-002 [node,release]
RULE: The repository builds from a clean checkout using the committed frozen/locked dependency files and declared toolchain.
BECAUSE: Developer caches and floating dependencies can hide undeclared state and non-reproducible builds.
REQUIRED EVIDENCE: Clean-environment install/build logs, lockfile digest, tool versions, exit codes, and build sentinel.
OR ELSE: Record FAIL for the clean-build gate; dependent runtime tests may be BLOCKED_PREREQUISITE, but independent tests continue.

### DOD-003 [feature,node,release]
RULE: The production distribution artifact is created successfully in every supported format.
BECAUSE: Source code is not what users install or operators deploy.
REQUIRED EVIDENCE: Artifact paths, formats, sizes, metadata, checksums, signatures/attestations when applicable, and build logs.
OR ELSE: The release is NO_GO and artifact-dependent tests remain BLOCKED_PREREQUISITE.

### DOD-004 [feature,node,release]
RULE: Final smoke and E2E tests run against the exact production artifact digest, not merely source or a development server.
BECAUSE: Packaging can omit files, alter configuration, or introduce behavior not visible in source-level tests.
REQUIRED EVIDENCE: Artifact digest bound to test environment, commands, logs, and observed outcomes.
OR ELSE: Artifact-level behavior is UNVERIFIED and completion is prohibited.

### DOD-005 [milestone,node,release]
RULE: Required tests execute in an ephemeral clean environment or a documented persistent environment created from a known baseline.
BECAUSE: Hidden developer state, caches, databases, and environment variables can create false greens.
REQUIRED EVIDENCE: Environment manifest, image/VM digest, cache policy, provisioning logs, and teardown proof.
OR ELSE: Results are ERROR or INCONCLUSIVE and cannot satisfy completion.

### DOD-006 [task,feature,milestone,node,release]
RULE: No required test is skipped, disabled, ignored, pending, quarantined, or xfailed without an approved requirement-scoped waiver.
BECAUSE: Skipped work creates silent blind spots while allowing green CI.
REQUIRED EVIDENCE: Test-runner collection/skip report and waiver IDs with owner, expiry, rationale, and compensating evidence.
OR ELSE: The affected requirement is INCOMPLETE and the release is NO_GO unless policy explicitly permits the time-bounded waiver.

### DOD-007 [milestone,node,release]
RULE: The harness fails when zero tests or fewer than the expected manifest are collected.
BECAUSE: Many runners exit zero for empty, misconfigured, or partially discovered suites.
REQUIRED EVIDENCE: Expected-test manifest/count, collected IDs/count, and test-collection-guard output.
OR ELSE: The run is ERROR; no pass result from that suite is valid.

### DOD-008 [feature,milestone,node,release]
RULE: Unit tests verify semantic results, boundaries, invalid inputs, error behavior, state transitions, and invariants for critical logic.
BECAUSE: Line execution alone does not prove correctness of meaning or failure behavior.
REQUIRED EVIDENCE: Mapped unit tests, assertions, boundary partitions, invariant list, coverage, and mutation sensitivity.
OR ELSE: The feature remains PARTIAL/UNVERIFIED even if compilation and happy-path tests pass.

### DOD-009 [feature,node,release]
RULE: Integration tests use production-type databases, queues, caches, storage, brokers, and other required dependency classes.
BECAUSE: In-memory substitutes frequently differ in transactions, locking, serialization, consistency, and failure modes.
REQUIRED EVIDENCE: Ephemeral production-type service versions, configuration, migrations, test commands, and independently observed state.
OR ELSE: Integration claims are PARTIAL or SIMULATED and cannot support production readiness.

### DOD-010 [feature,node,release]
RULE: Mocks may support isolation tests but may not be the sole proof of a claimed integration or production feature.
BECAUSE: A mock proves the test author's expectation, not the real dependency or wiring.
REQUIRED EVIDENCE: At least one real/sandbox dependency execution at the final acceptance boundary.
OR ELSE: The feature is SIMULATED/UNVERIFIED and the claim-to-release traceability gate fails.

### DOD-011 [feature,node,release]
RULE: Black-box acceptance tests exercise only public user-facing interfaces and do not call private internals to force success.
BECAUSE: Users and external systems cannot rely on internal shortcuts.
REQUIRED EVIDENCE: HTTP/UI/CLI/SDK/event entry-point evidence plus public outputs and side effects.
OR ELSE: The test is implementation-coupled and cannot satisfy end-to-end acceptance.

### DOD-012 [feature,node,release]
RULE: Important side effects are independently verified through a second connection, client, provider API, database query, event consumer, or durable artifact.
BECAUSE: The component under test can falsely report success without performing the effect.
REQUIRED EVIDENCE: Primary response plus independent observation with correlation/canary ID.
OR ELSE: The side-effect claim is UNVERIFIED and any success response is insufficient.

### DOD-013 [feature,node,release]
RULE: Runtime-generated unpredictable canary data is used for critical black-box proofs.
BECAUSE: Static examples can be hard-coded, cached, or accidentally satisfied by canned responses.
REQUIRED EVIDENCE: Seed/source, generated canary, propagation trace, and final independent observation.
OR ELSE: The proof is susceptible to fabrication and does not satisfy Functional Reality.

### DOD-014 [feature,node,release]
RULE: Wrong credentials, revoked permissions, expired tokens, and unavailable dependencies cause accurate fail-closed behavior rather than simulated success.
BECAUSE: Production systems must communicate real dependency/auth failures and preserve integrity.
REQUIRED EVIDENCE: Negative live-fire logs, status/error contracts, no-side-effect proof, and recovery behavior.
OR ELSE: The feature FAILS resilience/security acceptance and release is blocked when critical.

### DOD-015 [feature,node,release]
RULE: Persistent state survives full process and container restart and is readable by the supported runtime.
BECAUSE: Memory-only or process-local success can masquerade as durable implementation.
REQUIRED EVIDENCE: Pre-restart state hash, hard restart evidence, post-restart independent read, and reconciliation.
OR ELSE: The persistence claim is FALSE/SIMULATED and completion is prohibited.

### DOD-016 [feature,node,release]
RULE: Required migrations work from an empty database and every supported prior released schema with logical data preservation.
BECAUSE: Fresh installs and upgrades exercise different paths and failures can strand customers.
REQUIRED EVIDENCE: Baseline schema/data hashes, migration logs, post-migration invariants, retry/rollback evidence, and supported-version matrix.
OR ELSE: The release is NO_GO for affected install/upgrade paths.

### DOD-017 [feature,node,release]
RULE: Duplicate, retried, reordered, delayed, and concurrent operations preserve idempotency, integrity, and the documented delivery semantics.
BECAUSE: Distributed systems naturally redeliver and race; naive handling duplicates or loses side effects.
REQUIRED EVIDENCE: Idempotency keys, concurrent traces, commit/ack fault cases, final reconciliation, and invariant results.
OR ELSE: The feature FAILS data-integrity acceptance and cannot ship when state-changing.

### DOD-018 [feature,node,release]
RULE: At least one controlled defect or mutation is introduced for each critical feature and the relevant test must fail.
BECAUSE: A permanently green test may not observe the behavior it claims to protect.
REQUIRED EVIDENCE: Mutation/defect ID, changed behavior, failing test evidence, restoration, and green rerun.
OR ELSE: The test is non-discriminating; its pass cannot prove the feature.

### DOD-019 [feature,milestone,node,release]
RULE: Placeholder, stub, fake, demo, simulation, no-op, dead-route, hard-coded-success, and unfinished-code scans run against all production paths.
BECAUSE: AI-generated and scaffolded repositories often appear complete while returning fabricated behavior.
REQUIRED EVIDENCE: Lexical scan, structural trace, reachable-path analysis, allowlist decisions, and findings.
OR ELSE: Any unexplained hit is a release blocker or the affected claim is explicitly marked incomplete.

### DOD-020 [feature,node,release]
RULE: Production mode never selects mock, fake, demo, sample, or in-memory adapters for features represented as production-ready unless that adapter is the documented production architecture.
BECAUSE: Environment switches can silently route real users into simulated behavior.
REQUIRED EVIDENCE: Production configuration resolution, dependency injection graph, runtime adapter identity, and live-fire evidence.
OR ELSE: The feature is SIMULATED and the release is NO_GO.

### DOD-021 [milestone,node,release]
RULE: Applicable formatting, linting, static analysis, type checking, secret scanning, dependency/license/SBOM scanning, IaC/container checks, and security tests pass under enforced thresholds.
BECAUSE: These gates catch classes of defects before runtime and protect the supply chain.
REQUIRED EVIDENCE: Commands, versions, reports, exit codes, thresholds, and approved time-bounded waivers.
OR ELSE: The gate records FAIL or unresolved risk; silent continuation is prohibited.

### DOD-022 [feature,node,release]
RULE: Performance, resource, cost, and SLO requirements are encoded as automated pass/fail thresholds against a defined workload and environment.
BECAUSE: Unmeasured claims such as fast, scalable, or cheap cannot be verified or regression-gated.
REQUIRED EVIDENCE: Workload model, environment, samples, percentiles, resource metrics, thresholds, and verdict.
OR ELSE: The nonfunctional claim is UNVERIFIED and a mandatory SLO failure is NO_GO.

### DOD-023 [node,release]
RULE: README commands, examples, quickstarts, install, upgrade, deployment, rollback, and operator instructions are executed exactly as published in clean environments.
BECAUSE: Documentation drift creates hidden operator knowledge and failed customer installs.
REQUIRED EVIDENCE: Extracted command manifest, clean execution logs, expected/actual outputs, and corrected documentation.
OR ELSE: Documentation is defective and the affected support/install claim cannot pass.

### DOD-024 [task,feature,milestone,node,release]
RULE: No test or scanner failure is hidden by continue-on-error, ignored exit codes, unconditional success, swallowed exceptions, all-retry policies, filtered output, or baseline auto-acceptance.
BECAUSE: Failure masking converts real defects into fraudulent green status.
REQUIRED EVIDENCE: CI/script review, raw exit codes, first-failure logs, status-transition audit, and no-mask checks.
OR ELSE: The run is INVALID/ERROR and all dependent pass claims are revoked.

### DOD-025 [milestone,node,release]
RULE: Raw commands, tool versions, exit codes, logs, reports, traces, seeds, environment fingerprints, candidate SHA, artifact hashes, and evidence digests are preserved.
BECAUSE: A claim that cannot be reproduced or tied to an exact identity cannot be trusted.
REQUIRED EVIDENCE: Evidence index entries and content hashes linked to each result.
OR ELSE: The result is INCONCLUSIVE and cannot satisfy a release gate.

### DOD-026 [task,feature,milestone,node,release]
RULE: Any unmet condition is reported as incomplete, partial, simulated, blocked, experimental, external-required, deferred, failed, errored, or unverified using the exact taxonomy.
BECAUSE: Honest status prevents uncertainty from being laundered into completion.
REQUIRED EVIDENCE: Status record, rationale, evidence, dependency edge, and next action.
OR ELSE: DONE/GO is prohibited and any contrary narrative is a fabrication defect.

### DOD-027 [task,feature,milestone,node,release]
RULE: An incomplete implementation may not be replaced with a fabricated success response, and acceptance criteria may not be weakened merely to obtain a pass.
BECAUSE: Changing the oracle or faking output hides the defect instead of solving it.
REQUIRED EVIDENCE: Diff review, requirement history, gate hash, mutation/live-fire proof, and decision log.
OR ELSE: Revert the manipulation, record a critical process finding, and keep the item incomplete.

### DOD-028 [task,feature,milestone,node,release]
RULE: The final completion report distinguishes verified behavior, partially verified behavior, unverified assumptions, blocked work, external gates, accepted risks, and every remaining limitation.
BECAUSE: Readers need to know exactly what evidence supports each claim.
REQUIRED EVIDENCE: Structured final report and complete accounting linked to requirements/tests/evidence.
OR ELSE: The report is invalid and completion cannot be declared.

### DOD-029 [node,release]
RULE: The exact candidate commit, base revision, test-overlay revision, build inputs, and release artifact digest are pinned and recorded.
BECAUSE: Evidence can otherwise drift across unidentifiable code and artifacts.
REQUIRED EVIDENCE: Run manifest and artifact-identity output.
OR ELSE: All identity-dependent evidence is INCONCLUSIVE and release is prohibited.

### DOD-030 [node,release]
RULE: All 484 canonical test IDs receive an individual applicability decision and final status; zero IDs are missing or duplicated.
BECAUSE: Exhaustive testing requires exhaustive accounting, not silent omission.
REQUIRED EVIDENCE: Registry snapshot, applicability matrix, test ledger, validator report, and final totals invariant.
OR ELSE: Harness validation fails and the project cannot be declared production-ready.

### DOD-031 [node,release]
RULE: A failed prerequisite blocks only tests with explicit dependency edges; independent tests continue to completion.
BECAUSE: Blanket blocking hides defects and confuses product failures with environment limitations.
REQUIRED EVIDENCE: Dependency blocker graph, per-test blocker fields, continuation logs, and blanket-block validator.
OR ELSE: The accounting is invalid, the campaign remains IN_PROGRESS/ERROR, and final verdict cannot be trusted.

### DOD-032 [node,release]
RULE: Every FAIL, ERROR, BLOCKED_PREREQUISITE, BLOCKED_ENVIRONMENT, BLOCKED_CAPABILITY, BLOCKED_CREDENTIALS, BLOCKED_SAFETY, EXTERNAL_REQUIRED, and DEFERRED_LONG_RUNNING result uses the precise definition and required fields.
BECAUSE: Different causes require different remediation and release decisions.
REQUIRED EVIDENCE: Status-transition audit and schema validation.
OR ELSE: Misclassified rows are rejected and must be corrected before final accounting.

### DOD-033 [node,release]
RULE: The harness non-invasively discovers canonical bootstrap and provisions declared test infrastructure when the adapter can do so.
BECAUSE: Missing PostgreSQL, queues, browsers, or build ordering is often a harness setup problem rather than a product capability gap.
REQUIRED EVIDENCE: Repository evidence for commands/versions plus provisioning, health, migration, and teardown logs.
OR ELSE: The attempt is ERROR, not BLOCKED_CAPABILITY or candidate FAIL, until the harness setup is corrected.

### DOD-034 [release]
RULE: A virgin clean room installs and boots the exact final artifact using only public documentation and declared prerequisites, then completes the golden path.
BECAUSE: Developer-machine bias and hidden dependencies make otherwise green software undistributable.
REQUIRED EVIDENCE: Zero-state proof, documented prerequisite log, artifact transfer/digest, install/boot/golden-path evidence.
OR ELSE: The distribution is NO_GO and documentation/install findings remain open.

### DOD-035 [release]
RULE: Every claimed upgrade, downgrade, rollback, version-skew, mixed-fleet, and compatibility path is executed with realistic persistent state.
BECAUSE: Releases fail in transitions even when each version works alone.
REQUIRED EVIDENCE: Compatibility matrix, old/new artifacts, state hashes, queue/event evidence, and rollback outcome.
OR ELSE: Unsupported or failed claimed paths block release or must be explicitly removed from support claims.

### DOD-036 [release]
RULE: Backup, restore, disaster recovery, hard-failure recovery, RPO, RTO, and MTTR claims are executed against reconciled state where applicable.
BECAUSE: Restarting a process is not recovery if transactions are lost, duplicated, or corrupted.
REQUIRED EVIDENCE: Pre-disaster hash, fault injection, restore/recovery logs, post-state reconciliation, and measured objectives.
OR ELSE: Recovery readiness FAILS and stateful production release is NO_GO when mandatory.

### DOD-037 [feature,node,release]
RULE: Health, readiness, logs, metrics, traces, alerts, dashboards, and correlation IDs truthfully describe critical workflows and induced failures without leaking secrets.
BECAUSE: Operators cannot run or recover a system whose telemetry lies or lacks causality.
REQUIRED EVIDENCE: Known-failure injection mapped to signals, alert lifecycle, trace/log correlation, and redaction evidence.
OR ELSE: Observability is FAIL/INCOMPLETE and mandatory production operations gate fails.

### DOD-038 [release]
RULE: Required soak, endurance, fuzz, performance, stress, and recovery durations/workloads are completed at their specified scale; abbreviated trials are labeled separately.
BECAUSE: Short samples cannot prove long-duration stability or representative capacity.
REQUIRED EVIDENCE: Start/end timestamps, continuous heartbeats, telemetry, corpus/workload, interruptions, and full-duration report.
OR ELSE: Status is DEFERRED_LONG_RUNNING, EXTERNAL_REQUIRED, PARTIAL, or FAIL -- never PASS for the full requirement.

### DOD-039 [release]
RULE: Human UAT, manual assistive-technology validation, legal/compliance review, physical hardware/HSM work, and accredited assessment are signed only by the required real participants.
BECAUSE: AI or automated tooling cannot impersonate business acceptance, lived accessibility use, hardware evidence, or professional certification.
REQUIRED EVIDENCE: Named authorized sign-off, scope, date, scenarios/evidence, and unresolved findings.
OR ELSE: Status remains EXTERNAL_REQUIRED and GO is prohibited when the gate is mandatory.

### DOD-040 [milestone,node,release]
RULE: Any code, dependency, schema, configuration, build, test-oracle, or artifact change invalidates and reruns every affected downstream result.
BECAUSE: Old evidence does not prove a changed candidate.
REQUIRED EVIDENCE: Change invalidation graph, prior/new epoch IDs, rerun list, and current evidence hashes.
OR ELSE: Affected PASS statuses are revoked until rerun.

### DOD-041 [feature,node,release]
RULE: Conditional domain packs such as HIPAA, blockchain, AI/agentic, multi-tenant, mobile, cloud, and hardware are activated or skipped from repository evidence, never assumption.
BECAUSE: Running irrelevant tests wastes effort while skipping relevant domain risk creates dangerous blind spots.
REQUIRED EVIDENCE: Architecture/data/interface/support evidence attached to every applicability decision.
OR ELSE: The applicability matrix is invalid and final accounting fails.

### DOD-042 [release]
RULE: The final release verdict is produced only by the machine-validated ship gate and is one of GO, NO_GO, CONDITIONAL_EXTERNAL_GATES, or INCONCLUSIVE.
BECAUSE: Free-form completion language can obscure release blockers and external dependencies.
REQUIRED EVIDENCE: RELEASE_GATE.json, validator outputs, DOD status, 484-test accounting, and exact artifact identity.
OR ELSE: No production-ready tag or deployment is allowed; any contradictory claim is invalid.

Node closure additionally requires real production-path live-fire, independent readback, negative/mutation proof, architecture drift report, anti-gaming PASS, and exact evidence hashes. CLOSED_BLOCKED never counts as completion.
