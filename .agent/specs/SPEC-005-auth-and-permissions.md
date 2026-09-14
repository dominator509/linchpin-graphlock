# SPEC-005 Auth and Permissions

REQ-SEC-001 V1 trusts the logged-in local OS user plus optional app lock. Workspace confidentiality is the default boundary: invention content does not leave the device unless an egress policy explicitly authorizes it.

REQ-SEC-002 Provider auth belongs to the provider executable. Workspace egress policies govern external provider/data calls.

REQ-SEC-003 MCP grants are explicit by server/client/workspace/capability/expiry.

REQ-SEC-010 HIGH_IMPACT capabilities: FILE_EXTERNAL, PAY_EXTERNAL, PUBLISH_PUBLIC, SEND_OUTREACH, MUTATE_GIT, EXPORT_CONFIDENTIAL, CHANGE_EGRESS_POLICY. Each requires local approval; models cannot self-approve.
