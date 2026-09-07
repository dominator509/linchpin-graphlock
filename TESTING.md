# Testing

## Test layers
Unit: pure domain/scoring/state transitions/parser primitives. Integration: real SQLCipher, files, search indexes, local model, MCP process and sandbox external services. Contract: provider/data-source/MCP schemas with positive/negative/cancellation/version mismatch. E2E: packaged desktop path through persistence and independent readback. Artifact smoke: exact installer/app digest. Security/fuzz/property/mutation/performance/accessibility/soak/chaos according to registry applicability.

## Test-double zone
Mocks/fakes are allowed only in narrowly declared unit tests for deterministic branch isolation. They are forbidden as final proof of provider, patent data, persistence, MCP, installer, update, filing-export, Git repair or OS credential behavior. Final integration proof uses real local equivalents or authorized sandbox/service accounts.

## Required product-specific tests
- Human conception cannot be silently relabeled from AI suggestion.
- A research conclusion without evidence IDs fails material-claim validation.
- LINCHPIN scoring never emits a patentability certainty percentage.
- Claim export fails when required limitation support is absent.
- Post-filing new-matter insertion path fails closed pending review.
- Disclosure Firewall blocks public export of Confidential content without filing receipt/override.
- `FILED` state cannot be set by a model/UI action; only verified receipt import transition.
- Provider adapter never returns/exposes raw first-party OAuth material.
- Gemini terms-gated adapter is disabled absent accepted ADR.
- External provider failure does not corrupt local checkpoint and local provider can continue eligible work.
- Malicious source/MCP prompt cannot alter tool/egress policy.
- Crash capsule redaction mutation demonstrates a planted secret/invention phrase is removed.
- Restart restores canonical workflow/checkpoint/deadlines.
- Installer round-trip preserves vault and exact-artifact smoke.

## Fixtures
Synthetic inventions and public-domain/open test documents only. Fixture workspaces carry an unmistakable TEST marker and live outside production paths. Every integration test creates and cleans a unique workspace ID.

## Flaky tests
A flaky required test is a defect. Identify and fix nondeterminism or remove the behavior by requirement/ADR; never retry-until-green.

## Collection guard and mutation
Each suite declares expected minimum IDs. Zero/low collection fails. For every critical invariant, introduce a controlled mutation of production behavior in a test worktree and prove the relevant test turns red before closure.

## Exact artifact
Final smoke/E2E/cleanroom/update/rollback tests record SHA-256 of the exact installed candidate and bind evidence to that digest.

## Harness
The 484-ID registry under `.agent/verification/MASTER_TEST_REGISTRY.csv` is never pruned. All statuses and applicability are reconciled at V-021.
