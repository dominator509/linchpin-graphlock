/**
 * Shared UI primitives for the LINCHPIN desktop client.
 *
 * GraphLock context: this package previously contained a single `UI_VERSION`
 * constant and nothing else, so it had no testable behaviour and the JS unit
 * lane could only pass by masking an empty suite (the DOD-006/DOD-007 finding).
 *
 * Rather than invent tests for a constant, this module implements the small
 * amount of genuinely shared, spec-mandated presentation logic:
 *
 *   * `formatConfidentialityBadge` — SPEC-005 makes workspace confidentiality
 *     the default boundary and SPEC-004 requires a persistent badge for it.
 *   * `formatSensitivityBadge` — SPEC-004 requires the Disclosure Firewall's
 *     classification to be visible before an export.
 *   * `assertNonDefinitiveLegalLanguage` — SPEC-004/REQ-UI-003 forbid definitive
 *     legal conclusions in product copy.
 *
 * These are pure functions, which is what makes them testable without a DOM.
 */

export const UI_VERSION = "0.1.0";

/** Confidentiality classification of a workspace (SPEC-005). */
export type Confidentiality = "device-only" | "export-approved" | "shared";

/** Disclosure Firewall classification of a payload (SPEC-002 / SPEC-005). */
export type Sensitivity = "Public" | "Confidential" | "Restricted";

export interface Badge {
  label: string;
  tone: "neutral" | "caution" | "danger";
}

/**
 * Badge for the workspace confidentiality boundary.
 *
 * An unknown confidentiality value returns the most restrictive tone rather
 * than the most permissive: when the boundary is not understood, the safe
 * assumption is that content must not leave the device.
 */
export function formatConfidentialityBadge(value: string): Badge {
  switch (value) {
    case "device-only":
      return { label: "Device-only", tone: "neutral" };
    case "export-approved":
      return { label: "Export approved", tone: "caution" };
    case "shared":
      return { label: "Shared externally", tone: "danger" };
    default:
      return { label: "Boundary unknown", tone: "danger" };
  }
}

/** Badge for the Disclosure Firewall's classification of an export. */
export function formatSensitivityBadge(value: string): Badge {
  switch (value) {
    case "Public":
      return { label: "Public", tone: "neutral" };
    case "Confidential":
      return { label: "Confidential", tone: "caution" };
    case "Restricted":
      return { label: "Restricted — export blocked", tone: "danger" };
    default:
      return { label: "Unclassified — export blocked", tone: "danger" };
  }
}

/** Phrasings that assert a legal conclusion the product cannot support. */
const DEFINITIVE_LEGAL_PHRASES = [
  "guaranteed patent",
  "guaranteed approval",
  "guaranteed grant",
  "legally valid",
  "will be granted",
  "assured patent",
  "patentable as claimed",
  "enforceable",
];

export interface LanguageFinding {
  phrase: string;
  index: number;
}

/**
 * Find definitive legal claims in product copy (REQ-UI-003).
 *
 * Returns every offending phrase with its offset so the caller can report
 * precisely where the copy over-claims. Comparison is case-insensitive because
 * a capitalised variant is the same assertion.
 */
export function assertNonDefinitiveLegalLanguage(
  copy: string,
): LanguageFinding[] {
  const haystack = copy.toLowerCase();
  const findings: LanguageFinding[] = [];
  for (const phrase of DEFINITIVE_LEGAL_PHRASES) {
    let from = 0;
    for (;;) {
      const index = haystack.indexOf(phrase, from);
      if (index === -1) break;
      findings.push({ phrase, index });
      from = index + phrase.length;
    }
  }
  return findings.sort((a, b) => a.index - b.index);
}

/**
 * Truncate text for a badge or summary without splitting a word.
 *
 * Returns the original string when it already fits, so callers can rely on
 * `formatSummary(x) === x` for short input.
 */
export function formatSummary(text: string, maxLength = 64): string {
  if (maxLength <= 0) return "";
  if (text.length <= maxLength) return text;
  const cut = text.slice(0, maxLength);
  const lastSpace = cut.lastIndexOf(" ");
  const body = lastSpace > 0 ? cut.slice(0, lastSpace) : cut;
  return `${body.trimEnd()}…`;
}
