# MCP Integration Specification

LINCHPIN is both an MCP client and a least-privilege MCP server. Bootstrap target is the MCP 2026-07-28 specification; EP-000 re-verifies the current published spec/SDK and pins exact versions.

## Client
- Servers are registered per workspace with explicit transport, executable/origin, trust tier and capability grant.
- Discovery never grants execution. Each tool/resource/prompt is independently allow/deny scoped.
- High-impact tools (network publishing, outbound communication, Git write, file mutation outside workspace, payments, legal submission) require policy plus human approval.
- Server output is untrusted data and cannot inject GraphLock commands or provider credentials.

## Server
Expose versioned, narrow domain capabilities such as `research.search`, `evidence.read`, `claims.analyze`, `draft.export`, `docket.read`, `commercialization.package`, and `incident.export_repair_capsule`. Do not expose raw SQL, unrestricted shell, arbitrary filesystem write, signing, payment or Patent Center submit.

## Security
Authenticate remote transports, bind local transports to loopback/stdio as appropriate, enforce workspace authorization, log correlation IDs and grant decisions, cap payload/time/output sizes, and require confirmation for sensitive egress. OAuth/session material owned by provider CLIs is never surfaced through MCP.
