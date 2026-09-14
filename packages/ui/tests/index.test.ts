import { describe, expect, it } from "vitest";
import {
  assertNonDefinitiveLegalLanguage,
  formatConfidentialityBadge,
  formatSensitivityBadge,
  formatSummary,
  UI_VERSION,
} from "../src/index";

describe("formatConfidentialityBadge", () => {
  it("maps the known confidentiality states", () => {
    expect(formatConfidentialityBadge("device-only").label).toBe("Device-only");
    expect(formatConfidentialityBadge("export-approved").tone).toBe("caution");
    expect(formatConfidentialityBadge("shared").tone).toBe("danger");
  });

  it("fails closed on an unknown state rather than assuming it is safe", () => {
    for (const unknown of ["", "public", "DEVICE-ONLY", "whatever"]) {
      const badge = formatConfidentialityBadge(unknown);
      expect(badge.tone, `${JSON.stringify(unknown)} did not fail closed`).toBe(
        "danger",
      );
    }
  });
});

describe("formatSensitivityBadge", () => {
  it("marks Restricted as export-blocked", () => {
    const badge = formatSensitivityBadge("Restricted");
    expect(badge.tone).toBe("danger");
    expect(badge.label).toContain("blocked");
  });

  it("fails closed on an unclassified sensitivity", () => {
    // An unrecognised classification must not render as Public.
    const badge = formatSensitivityBadge("Secret");
    expect(badge.tone).toBe("danger");
    expect(badge.label).not.toContain("Public");
  });

  it("distinguishes Confidential from Public", () => {
    expect(formatSensitivityBadge("Public").tone).toBe("neutral");
    expect(formatSensitivityBadge("Confidential").tone).toBe("caution");
  });
});

describe("assertNonDefinitiveLegalLanguage", () => {
  it("finds prohibited definitive claims, case-insensitively", () => {
    const findings = assertNonDefinitiveLegalLanguage(
      "This is a GUARANTEED PATENT and the claims are Legally Valid.",
    );
    const phrases = findings.map((f) => f.phrase);
    expect(phrases).toContain("guaranteed patent");
    expect(phrases).toContain("legally valid");
  });

  it("reports every occurrence with an offset", () => {
    const findings = assertNonDefinitiveLegalLanguage(
      "enforceable today, enforceable tomorrow",
    );
    expect(findings).toHaveLength(2);
    expect(findings[0].index).toBeLessThan(findings[1].index);
  });

  it("returns nothing for compliant copy", () => {
    const compliant = [
      "Screen the idea for prior art.",
      "Draft claims for human review.",
      "Evidence coverage is incomplete.",
      "Uncertainty is high.",
    ];
    for (const copy of compliant) {
      expect(
        assertNonDefinitiveLegalLanguage(copy),
        `compliant copy was flagged: ${copy}`,
      ).toEqual([]);
    }
  });

  it("returns findings ordered by position", () => {
    const findings = assertNonDefinitiveLegalLanguage(
      "Weaker: will be granted. Stronger: enforceable.",
    );
    const indexes = findings.map((f) => f.index);
    expect([...indexes].sort((a, b) => a - b)).toEqual(indexes);
  });
});

describe("formatSummary", () => {
  it("returns short text unchanged", () => {
    expect(formatSummary("short", 64)).toBe("short");
  });

  it("truncates at a word boundary and marks the elision", () => {
    const result = formatSummary("alpha beta gamma delta", 12);
    expect(result.endsWith("…")).toBe(true);
    expect(result.length).toBeLessThanOrEqual(13);
    expect(result).not.toContain("gamm…");
  });

  it("handles degenerate maxima without throwing", () => {
    expect(formatSummary("anything", 0)).toBe("");
    expect(formatSummary("anything", -5)).toBe("");
  });
});

describe("UI_VERSION", () => {
  it("is a semantic version string", () => {
    expect(UI_VERSION).toMatch(/^\d+\.\d+\.\d+$/);
  });
});
