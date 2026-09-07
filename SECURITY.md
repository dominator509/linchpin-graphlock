# LINCHPIN Security

## Security goals
Protect invention confidentiality and integrity; prevent accidental disclosure; isolate untrusted research/MCP/provider content; keep credentials with their owner; make high-impact actions human-authorized; preserve forensically useful but sanitized failure evidence; ship verifiable signed artifacts.

## Data classifications
`CONFIDENTIAL_DEVICE_ONLY`, `CONFIDENTIAL_PROVIDER_ALLOWED`, `NON_CONFIDENTIAL`, `PUBLIC_SAFE`, `SECRET_CREDENTIAL`. Default project content is `CONFIDENTIAL_DEVICE_ONLY`.

## Threat model
Threats include malicious retrieved documents/prompt injection; hostile MCP server/tool descriptions; provider process compromise; poisoned prior art; path traversal/archive bombs/malformed DOCX/PDF/ZIP; key theft; log/crash exfiltration; supply-chain compromise; malicious update; accidental public export; unauthorized Git mutation; incorrect deadline/form rule; cross-workspace data leakage; local untrusted user access.

## Controls
- SQLCipher canonical DB; content vault authenticated encryption; keys in Windows Credential Manager.
- Separate per-workspace data/evidence identifiers and explicit egress policy.
- No raw consumer session cookie/OAuth-token harvesting. Official provider executable owns auth.
- Network allowlist by adapter; no arbitrary provider-generated URLs.
- Retrieved content is data, never trusted instructions. Tool calls are schema/policy checked.
- MCP grants are capability-scoped and workspace-scoped; high-impact operations require local approval.
- CSP and Tauri capability allowlist; no broad shell plugin exposure to React.
- IPC commands validate workspace authorization, path canonicalization, payload size and schema.
- Archives/documents have decompression/recursion/size limits; parsers are fuzzed.
- Logs default to identifiers/hashes, not invention bodies. Crash capsule redactor blocks secrets and Confidential text unless the user explicitly elects a local-only full capsule.
- Signed release/update artifacts; update manifest pinning and rollback path.
- Dependency/SBOM/license gates before each release.

## Patent/disclosure-specific security
The Disclosure Firewall is a security control. An export/publish/outreach operation that contains `CONFIDENTIAL_*` data fails closed unless a qualifying filing receipt exists or the user completes a warning/override event. "Patent pending" labels are available only after a recorded qualifying filing event; a draft/provisional plan does not set that state.

## Production-data rule
Automated tests never run against a user's real invention vault. Release E2E uses disposable encrypted fixture workspaces. Any migration against real user data is backup-first, transactional where possible, rollback-tested, and never destructive without explicit user action.

## External legal/financial boundary
LLM-generated legal strategy, filing decisions, fee/entity determinations, inventorship conclusions, FTO opinions, valuation and investment claims are advisory drafts. Material uncertainty surfaces a counsel/official-resource review flag. The application may explain official guidance but cannot impersonate attorney sign-off.

## Security gate
Release requires static security checks, dependency/license audits, secret scan, fuzz/corpus results for untrusted parsers, MCP prompt-injection tests, egress-policy tests, redaction mutation tests, encrypted backup/restore proof, signed-artifact verification and external review when the release profile requires it.
