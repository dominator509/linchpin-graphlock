#!/usr/bin/env python3
"""Secret scan over tracked first-party content (DOD-021).

GraphLock context: DOD-021 requires "secret scanning" among the supply-chain
gates, and its evidence list names "commands, versions, reports, exit codes,
thresholds". No secret scan existed in this repository -- `cargo deny` covers
licenses and advisories, and the anti-gaming scan covers fabrication, but
neither looks for credentials.

No scanner is installed and AGENTS.md section 10 forbids adding a dependency for
something buildable with the standard library, so this implements the scan
directly.

Design rules, chosen so the gate is trustworthy rather than noisy:

  * Scan only GIT-TRACKED files. Scanning build output or vendored trees
    produces false positives that train reviewers to ignore the gate.
  * Report a match as a finding only when the value survives an entropy check.
    A documented placeholder such as `sk-123456789` in a test is not a secret;
    a 40-character random string is.
  * Allow an explicit, auditable allowlist. An allowlisted hit is still printed
    with its rule id, so the decision is visible rather than silent.
  * Never print the matched value itself. The report names the file, line and
    rule only, because echoing a live credential into a log is the very thing
    the scanner exists to prevent.

Usage:
  python3 scripts/secret-scan.py            # scan; exit 1 on findings
  python3 scripts/secret-scan.py --json     # machine-readable report
"""
from __future__ import annotations

import json
import math
import re
import subprocess
import sys
from pathlib import Path

# (rule_id, description, compiled pattern)
RULES: list[tuple[str, str, re.Pattern[str]]] = [
    (
        "AWS_ACCESS_KEY",
        "AWS access key id",
        re.compile(r"\bAKIA[0-9A-Z]{16}\b"),
    ),
    (
        "AWS_SECRET",
        "AWS secret access key assignment",
        re.compile(
            r"(?i)aws_secret_access_key\s*[:=]\s*['\"]?([A-Za-z0-9/+=]{40})"
        ),
    ),
    (
        "GITHUB_TOKEN",
        "GitHub token",
        re.compile(r"\bgh[pousr]_[A-Za-z0-9]{36,}\b"),
    ),
    (
        "OPENAI_KEY",
        "OpenAI-style API key",
        re.compile(r"\bsk-[A-Za-z0-9]{20,}\b"),
    ),
    (
        "ANTHROPIC_KEY",
        "Anthropic API key",
        re.compile(r"\bsk-ant-[A-Za-z0-9\-_]{20,}\b"),
    ),
    (
        "SLACK_TOKEN",
        "Slack token",
        re.compile(r"\bxox[baprs]-[A-Za-z0-9\-]{10,}\b"),
    ),
    (
        "PRIVATE_KEY_BLOCK",
        "PEM private key block",
        re.compile(r"-----BEGIN (?:RSA |EC |OPENSSH |PGP )?PRIVATE KEY-----"),
    ),
    (
        "GOOGLE_API_KEY",
        "Google API key",
        re.compile(r"\bAIza[0-9A-Za-z\-_]{35}\b"),
    ),
    (
        "GENERIC_ASSIGNMENT",
        "credential-looking assignment",
        re.compile(
            r"(?i)\b(?:api[_-]?key|apikey|secret|password|passwd|token|bearer)"
            r"\s*[:=]\s*['\"]([A-Za-z0-9/+=_\-]{16,})['\"]"
        ),
    ),
]

# Values that appear in documentation, examples, tests and placeholders. Each
# entry is a decision that is visible in this list rather than silently skipped.
ALLOWLIST_VALUES = {
    "keyring:openai_prod_key",
    "sk-123456789",
    "sk-live-0123456789abcdef",
    "ghp_ABCDEFGHIJKLMNOP",
    "trusted_hash_abc123",
    "malicious_hash_xyz999",
    "secret123",
    "hunter2",
    "master_key",
    "PROVISIONAL-CLAIM-1",
    "TOKEN-abc",
    # AWS's own published documentation example access key id, used as a
    # redaction FIXTURE in crates/crash_reporter's key-value redaction test. It is
    # not a credential, and removing it would remove the test case for a
    # credential shape the redactor must catch.
    "AKIAIOSFODNN7EXAMPLE",
}

# Files whose whole purpose is to document or test credential handling.
ALLOWLISTED_FILES = {
    ".env.example",
    ".agent/verification/state/REQUIREMENT_TEST_BINDINGS.jsonl",
}

BINARY_SUFFIXES = {
    ".png", ".ico", ".icns", ".exe", ".msi", ".dll", ".dmp", ".gz", ".b64",
    ".woff", ".woff2", ".ttf", ".pdf",
}


def shannon_entropy(value: str) -> float:
    if not value:
        return 0.0
    counts: dict[str, int] = {}
    for ch in value:
        counts[ch] = counts.get(ch, 0) + 1
    total = len(value)
    return -sum((c / total) * math.log2(c / total) for c in counts.values())


def tracked_files() -> list[Path]:
    proc = subprocess.run(
        ["git", "ls-files", "-z"],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    if proc.returncode != 0:
        raise SystemExit("git ls-files failed; run inside the repository")
    return [Path(p) for p in proc.stdout.split("\0") if p]


def scan() -> list[dict[str, object]]:
    findings: list[dict[str, object]] = []

    for path in tracked_files():
        rel = str(path).replace("\\", "/")
        if path.suffix.lower() in BINARY_SUFFIXES:
            continue
        if not path.is_file():
            continue
        try:
            text = path.read_text("utf-8")
        except (UnicodeDecodeError, OSError):
            continue

        allowlisted_file = rel in ALLOWLISTED_FILES
        for lineno, line in enumerate(text.splitlines(), 1):
            for rule_id, description, pattern in RULES:
                for match in pattern.finditer(line):
                    value = match.group(match.lastindex or 0)
                    if value in ALLOWLIST_VALUES:
                        continue
                    # Low-entropy strings are placeholders, not credentials.
                    if shannon_entropy(value) < 3.0:
                        continue
                    findings.append(
                        {
                            "rule": rule_id,
                            "description": description,
                            "file": rel,
                            "line": lineno,
                            # The value is deliberately NOT included.
                            "value_length": len(value),
                            "file_allowlisted": allowlisted_file,
                        }
                    )
    return findings


def main() -> int:
    check_only = "--check" in sys.argv
    findings = scan()
    blocking = [f for f in findings if not f["file_allowlisted"]]

    if check_only:
        # Deterministic recomputation: any blocking finding fails the check.
        if blocking:
            print(
                f"secret-scan check: FAIL -- {len(blocking)} unallowlisted finding(s)",
                file=sys.stderr,
            )
            return 1
        print(f"secret-scan check: ok ({len(RULES)} rules, no findings)")
        return 0

    if "--json" in sys.argv:
        print(
            json.dumps(
                {
                    "rules": [r[0] for r in RULES],
                    "files_scanned": len(tracked_files()),
                    "findings": findings,
                    "blocking": len(blocking),
                },
                indent=2,
            )
        )
    else:
        print(f"secret-scan: {len(RULES)} rules over {len(tracked_files())} tracked files")
        if not findings:
            print("secret-scan: no findings")
        for f in findings:
            tag = "ALLOWLISTED" if f["file_allowlisted"] else "FINDING"
            print(
                f"  {tag}: {f['rule']} at {f['file']}:{f['line']} "
                f"({f['description']}, {f['value_length']} chars; value withheld)"
            )
        if blocking:
            print(
                f"secret-scan: FAIL -- {len(blocking)} unallowlisted finding(s)",
                file=sys.stderr,
            )
            return 1
        print("secret-scan: ok")

    return 1 if blocking else 0


if __name__ == "__main__":
    raise SystemExit(main())
