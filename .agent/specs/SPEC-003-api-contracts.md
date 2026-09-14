# SPEC-003 API / IPC Contracts

REQ-LLM-001 Tauri commands are versioned application use cases, not generic SQL/file/shell primitives. No route accepts raw executable shell text from model output.

REQ-LLM-002 Contract namespaces: workspace, conception, opportunity, research, evidence, patent, filing, docket, prosecution, commercialization, provider, mcp, incident, export.

REQ-LLM-003 Every command accepts workspace-scoped typed input and returns typed result/error with correlation ID.

REQ-LLM-004 Provider JobEnvelope/JobResult are JSON-Schema versioned.

REQ-MCP-001 MCP grants are JSON-Schema versioned and explicit by server/client/workspace/capability/expiry.
