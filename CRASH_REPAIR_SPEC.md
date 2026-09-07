# Crash / Repair Capsule Specification

## Capture
Rust panic hooks, Tauri/WebView crash signals and sidecar failures emit a local incident with app version, commit/artifact digest, OS build, correlation IDs, bounded recent structured logs, stack/minidump references, active workflow state and deterministic reproduction hints when known.

## Redaction before export
Default-deny export. Strip patent draft bodies, unpublished invention text, secrets/tokens, personally identifying fields, filesystem usernames and unrelated workspace content. Replace sensitive fields with typed redaction markers and include a redaction manifest. The user previews the exact payload.

## Repair Capsule
A signed/hash-manifested directory contains `incident.json`, sanitized logs, stack/minidump metadata, environment/tool versions, failing requirement IDs, last known commands, relevant test output, reproduction script when safe, and expected/observed behavior. It never claims a root cause unless proven.

## Agentic repair flow
1. User approves export and target coding system/repository.
2. Adapter opens a repair task/branch through an approved MCP or Git provider path.
3. Agent must reproduce failure, add a failing regression test, trace architecture, implement minimal real fix, execute applicable GraphLock gates, and attach evidence.
4. PR creation is optional and approval-gated. No auto-merge, release or production deployment.
5. Fixed candidate must prove crash absence plus regression negative/readback/restart behavior before incident closure.
