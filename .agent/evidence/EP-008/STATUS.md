EP-008 Candidate Epoch: EP-008-candidate-001
Status: DONE_VERIFIED

## Execution Summary
- Implemented RepairCapsule incident model and creation logic in crates/crash_reporter.
- Enforced invariant INV-012: Every crash capsule passes redaction before export.
- Verified observability and crash repair unit tests.

## Gate Results & Evidence
- Cargo Unit Tests: PASS (cargo test --workspace --exclude linchpin-desktop).
- TypeScript Typecheck: PASS (pnpm -r typecheck).
- TypeScript Linting: PASS (pnpm -r lint).
- TypeScript Unit Tests: PASS (pnpm -r test:unit).
- Desktop Build: PASS (pnpm --filter @linchpin/desktop build).
- Anti-Gaming Scan: PASS (python3 scripts/anti-gaming-scan.py .).
