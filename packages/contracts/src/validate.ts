/**
 * Runtime contracts for LINCHPIN IPC payloads.
 *
 * `SPEC-003` requires that "Provider JobEnvelope/JobResult and MCP grants are
 * JSON-Schema versioned" and that "every command accepts workspace-scoped typed
 * input". TypeScript interfaces are erased at runtime, so they cannot validate a
 * payload that arrives over IPC — and a payload crossing the Tauri boundary is
 * untrusted input from the webview's perspective.
 *
 * GraphLock context: before this module the contracts package exported only
 * `interface` declarations with no runtime code, and the three JS packages
 * contained zero test files. This adds the validation the spec asks for, in the
 * one place where it is genuinely needed rather than inventing tests for a
 * type-only file.
 *
 * Design constraints:
 *   * dependency-free, so no schema library is introduced (AGENTS.md §10).
 *   * validation fails closed and reports *which* field failed, because a
 *     generic "invalid payload" is not actionable for an operator.
 */

/** Schema version carried by every versioned envelope. */
export const CONTRACT_SCHEMA_VERSION = 1;

export interface JobEnvelope {
  jobId: string;
  payload: Record<string, unknown>;
}

export interface SystemHealth {
  status: string;
  version: string;
  storage_ok: boolean;
}

/** A single validation finding. `path` locates the offending field. */
export interface ValidationIssue {
  path: string;
  message: string;
}

export type ValidationResult<T> =
  { ok: true; value: T } | { ok: false; issues: ValidationIssue[] };

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isNonEmptyString(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0;
}

/**
 * Validate a JobEnvelope received over IPC.
 *
 * Rejects unknown schema versions rather than assuming forward compatibility:
 * a payload from a newer producer may carry semantics this build cannot honour,
 * and silently accepting it is how a versioning scheme stops being one.
 */
export function validateJobEnvelope(
  input: unknown,
): ValidationResult<JobEnvelope> {
  const issues: ValidationIssue[] = [];

  if (!isPlainObject(input)) {
    return {
      ok: false,
      issues: [{ path: "$", message: "expected an object" }],
    };
  }

  const version = input.schemaVersion;
  if (version === undefined) {
    issues.push({ path: "schemaVersion", message: "is required" });
  } else if (version !== CONTRACT_SCHEMA_VERSION) {
    issues.push({
      path: "schemaVersion",
      message: `unsupported version ${String(version)}; this build accepts ${CONTRACT_SCHEMA_VERSION}`,
    });
  }

  if (!isNonEmptyString(input.jobId)) {
    issues.push({ path: "jobId", message: "must be a non-empty string" });
  }

  if (!isPlainObject(input.payload)) {
    issues.push({ path: "payload", message: "must be an object" });
  }

  if (issues.length > 0) return { ok: false, issues };

  return {
    ok: true,
    value: {
      jobId: input.jobId as string,
      payload: input.payload as Record<string, unknown>,
    },
  };
}

/**
 * Validate a SystemHealth payload.
 *
 * `status` must be one of the values the backend can actually produce. A health
 * payload that says anything else is not a state this UI knows how to render,
 * so it is rejected rather than displayed.
 */
export function validateSystemHealth(
  input: unknown,
): ValidationResult<SystemHealth> {
  const issues: ValidationIssue[] = [];

  if (!isPlainObject(input)) {
    return {
      ok: false,
      issues: [{ path: "$", message: "expected an object" }],
    };
  }

  if (input.status !== "OK" && input.status !== "DEGRADED") {
    issues.push({
      path: "status",
      message: `must be "OK" or "DEGRADED", got ${JSON.stringify(input.status)}`,
    });
  }

  if (!isNonEmptyString(input.version)) {
    issues.push({ path: "version", message: "must be a non-empty string" });
  }

  if (typeof input.storage_ok !== "boolean") {
    issues.push({ path: "storage_ok", message: "must be a boolean" });
  }

  if (issues.length > 0) return { ok: false, issues };

  return {
    ok: true,
    value: {
      status: input.status as string,
      version: input.version as string,
      storage_ok: input.storage_ok as boolean,
    },
  };
}

/**
 * Validate an MCP grant.
 *
 * `SPEC-005` requires grants to be explicit by
 * server/client/workspace/capability/expiry and that "models cannot
 * self-approve". An approval flag must therefore be present and exactly true;
 * a truthy string must not be accepted, because that is how a payload from a
 * model could approve itself.
 */
export interface McpGrant {
  serverId: string;
  clientId: string;
  workspaceId: string;
  capability: string;
  expiresAt: string;
  humanApproved: boolean;
}

export function validateMcpGrant(input: unknown): ValidationResult<McpGrant> {
  const issues: ValidationIssue[] = [];

  if (!isPlainObject(input)) {
    return {
      ok: false,
      issues: [{ path: "$", message: "expected an object" }],
    };
  }

  for (const field of [
    "serverId",
    "clientId",
    "workspaceId",
    "capability",
  ] as const) {
    if (!isNonEmptyString(input[field])) {
      issues.push({ path: field, message: "must be a non-empty string" });
    }
  }

  if (!isNonEmptyString(input.expiresAt)) {
    issues.push({ path: "expiresAt", message: "must be a non-empty string" });
  }

  if (input.humanApproved !== true) {
    issues.push({
      path: "humanApproved",
      message: "must be exactly true; models cannot self-approve",
    });
  }

  if (issues.length > 0) return { ok: false, issues };

  return {
    ok: true,
    value: {
      serverId: input.serverId as string,
      clientId: input.clientId as string,
      workspaceId: input.workspaceId as string,
      capability: input.capability as string,
      expiresAt: input.expiresAt as string,
      humanApproved: true,
    },
  };
}

/** True when a string contains characters that could be executed as shell. */
export function containsShellMetacharacters(value: string): boolean {
  return /[;&|`$><\n\r]|\$\(/.test(value);
}
