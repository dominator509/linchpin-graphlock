import { describe, expect, it } from "vitest";
import {
  CONTRACT_SCHEMA_VERSION,
  containsShellMetacharacters,
  validateJobEnvelope,
  validateMcpGrant,
  validateSystemHealth,
} from "../src/validate";

/**
 * Runtime contract tests for SPEC-003 / SPEC-005 payload validation.
 *
 * These were the first unit tests in this repository: all three JS packages
 * previously contained zero test files, and `pnpm -r test:unit` reported success
 * only because vitest ran with --passWithNoTests. That flag is now removed, so
 * an empty suite fails (DOD-006/DOD-007 finding).
 */

describe("validateJobEnvelope", () => {
  it("accepts a well-formed envelope", () => {
    const result = validateJobEnvelope({
      schemaVersion: CONTRACT_SCHEMA_VERSION,
      jobId: "job-1",
      payload: { prompt: "summarize" },
    });
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.value.jobId).toBe("job-1");
      expect(result.value.payload).toEqual({ prompt: "summarize" });
    }
  });

  it("rejects a missing schema version", () => {
    const result = validateJobEnvelope({ jobId: "j", payload: {} });
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.issues.some((i) => i.path === "schemaVersion")).toBe(true);
    }
  });

  it("rejects an unsupported schema version rather than assuming compatibility", () => {
    const result = validateJobEnvelope({
      schemaVersion: 999,
      jobId: "j",
      payload: {},
    });
    expect(result.ok).toBe(false);
    if (!result.ok) {
      const issue = result.issues.find((i) => i.path === "schemaVersion");
      expect(issue?.message).toContain("unsupported version 999");
    }
  });

  it("rejects an empty or whitespace-only jobId", () => {
    for (const jobId of ["", "   "]) {
      const result = validateJobEnvelope({
        schemaVersion: CONTRACT_SCHEMA_VERSION,
        jobId,
        payload: {},
      });
      expect(result.ok, `jobId ${JSON.stringify(jobId)} was accepted`).toBe(
        false,
      );
    }
  });

  it("rejects a non-object payload, including arrays and null", () => {
    for (const payload of [[], null, "text", 42]) {
      const result = validateJobEnvelope({
        schemaVersion: CONTRACT_SCHEMA_VERSION,
        jobId: "j",
        payload,
      });
      expect(result.ok, `payload ${JSON.stringify(payload)} was accepted`).toBe(
        false,
      );
    }
  });

  it("rejects non-objects and reports the root path", () => {
    for (const input of [null, undefined, "str", 7, []]) {
      const result = validateJobEnvelope(input);
      expect(result.ok).toBe(false);
      if (!result.ok) expect(result.issues[0].path).toBe("$");
    }
  });
});

describe("validateSystemHealth", () => {
  it("accepts the two real backend statuses", () => {
    for (const status of ["OK", "DEGRADED"]) {
      const result = validateSystemHealth({
        status,
        version: "0.1.0",
        storage_ok: status === "OK",
      });
      expect(result.ok, `status ${status} was rejected`).toBe(true);
    }
  });

  it("rejects an unknown status", () => {
    const result = validateSystemHealth({
      status: "FINE",
      version: "0.1.0",
      storage_ok: true,
    });
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.issues[0].path).toBe("status");
      expect(result.issues[0].message).toContain("DEGRADED");
    }
  });

  it("requires storage_ok to be a real boolean", () => {
    const result = validateSystemHealth({
      status: "OK",
      version: "0.1.0",
      storage_ok: "true",
    });
    expect(result.ok).toBe(false);
  });

  it("rejects an empty version", () => {
    const result = validateSystemHealth({
      status: "OK",
      version: "",
      storage_ok: true,
    });
    expect(result.ok).toBe(false);
  });
});

describe("validateMcpGrant", () => {
  const valid = {
    serverId: "srv",
    clientId: "cli",
    workspaceId: "ws",
    capability: "ReadVault",
    expiresAt: "2026-12-31T00:00:00Z",
    humanApproved: true,
  };

  it("accepts a fully specified grant", () => {
    expect(validateMcpGrant(valid).ok).toBe(true);
  });

  it("requires every scoping field", () => {
    for (const field of [
      "serverId",
      "clientId",
      "workspaceId",
      "capability",
      "expiresAt",
    ] as const) {
      const broken: Record<string, unknown> = { ...valid };
      delete broken[field];
      const result = validateMcpGrant(broken);
      expect(result.ok, `missing ${field} was accepted`).toBe(false);
      if (!result.ok) {
        expect(result.issues.some((i) => i.path === field)).toBe(true);
      }
    }
  });

  it("refuses a truthy-but-not-true approval, so a model cannot self-approve", () => {
    for (const value of ["true", 1, {}, []]) {
      const result = validateMcpGrant({ ...valid, humanApproved: value });
      expect(
        result.ok,
        `humanApproved=${JSON.stringify(value)} was accepted`,
      ).toBe(false);
    }
  });

  it("refuses an absent approval", () => {
    const broken: Record<string, unknown> = { ...valid };
    delete broken.humanApproved;
    expect(validateMcpGrant(broken).ok).toBe(false);
  });
});

describe("containsShellMetacharacters", () => {
  it("flags command-substitution and chaining characters", () => {
    const dangerous = [
      "a; rm -rf /",
      "a && b",
      "a | b",
      "`id`",
      "$(whoami)",
      "a > file",
      "a\nb",
    ];
    for (const value of dangerous) {
      expect(
        containsShellMetacharacters(value),
        `${JSON.stringify(value)} was not flagged`,
      ).toBe(true);
    }
  });

  it("does not flag ordinary prose", () => {
    const safe = [
      "A self-sealing graphene valve",
      "US 12/345,678",
      "claims 1-10",
      "50% improvement",
      "",
    ];
    for (const value of safe) {
      expect(
        containsShellMetacharacters(value),
        `${JSON.stringify(value)} was wrongly flagged`,
      ).toBe(false);
    }
  });
});
