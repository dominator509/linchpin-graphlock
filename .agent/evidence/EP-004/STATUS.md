EP-004 Candidate Epoch: EP-004-candidate-001
Status: DONE_VERIFIED

## Execution Summary
- Implemented ProviderTransport trait and LocalModelTransport adapter in crates/provider_transport.
- Implemented McpGrant capability policy enforcement and McpHubServer in crates/mcp_hub.
- Verified provider transport capabilities and local model execution invariants.

## Gate Results & Evidence
- Cargo Unit Tests: PASS (cargo test --workspace --exclude linchpin-desktop).
- TypeScript Typecheck: PASS (pnpm -r typecheck).
- TypeScript Linting: PASS (pnpm -r lint).
- TypeScript Unit Tests: PASS (pnpm -r test:unit).
- Desktop Build: PASS (pnpm --filter @linchpin/desktop build).
- Anti-Gaming Scan: PASS (python3 scripts/anti-gaming-scan.py .).
