#!/usr/bin/env python3
"""Mechanical audit of EVERY applicability absence claim (DOD-041).

DOD-041 RULE: conditional domain packs "are activated or skipped from repository
evidence, never assumption". Rounds 50 and 51 corrected nine rows whose absence
claim had become false, by hand. This script re-measures the whole set mechanically
so the remaining ~120 skips rest on a probe rather than on prose that may have aged:

  * it reads the no-cmd entries out of scripts/build-applicability.py,
  * derives distinctive keywords from each claim,
  * searches the FIRST-PARTY tree for those keywords, EXCLUDING the applicability
    script itself (whose text contains every claim, which is exactly how a keyword
    search produces false positives),
  * prints every row with hits and the first few file:line locations, so a human can
    decide whether the claim is stale or the hit is incidental.

It writes .agent/evidence/applicability-absence-audit.json + .md as the record.

MEASURED LIMITATION, stated because it decides how this may be used: on the current
tree the probe reports a signal for 84 of 97 claims, because keywords taken from claim
text ("static", "code", "data", "endpoint", "windows") appear everywhere in a Rust
codebase. It is therefore NOT a decision tool and is deliberately NOT wired into
verify.sh: it cannot tell a stale claim from an incidental word match, and treating its
output as evidence would repeat the regex-probe failure this repository already recorded
("false results in both directions"). Its only legitimate use is to list rows worth
reading, one at a time, with the row's own harness named -- which is how the nine
corrections in rounds 50-51 and the four in round 51 were actually made.
"""
from __future__ import annotations

import json
import pathlib
import re
import sys

ROOT = pathlib.Path(".").resolve()
BUILDER = "scripts/build-applicability.py"
EVIDENCE = ROOT / ".agent/evidence"
SKIP_PARTS = {"node_modules", "target", "dist", ".git", "gen", "build"}

# Words that appear in almost every claim and carry no signal.
STOP = {
    "no", "not", "exists", "exist", "is", "are", "the", "a", "an", "and", "or", "of",
    "in", "this", "repository", "product", "configured", "record", "suite", "harness",
    "tool", "targets", "target", "system", "security", "test", "testing", "tests",
    "configured", "installed", "available", "beyond", "set", "surface", "component",
    "mechanism", "process", "practice", "review", "manual", "external", "activity",
    "engagement", "environment", "automated", "automation", "validation", "verified",
    "verification", "analysis", "checking", "checks", "run", "runs", "exists,",
    "missing", "absent", "none", "performed", "produced", "recorded", "asserted",
}


def sources() -> list[pathlib.Path]:
    out: list[pathlib.Path] = []
    for base in ("crates", "apps", "packages", "scripts"):
        for path in (ROOT / base).rglob("*"):
            if not path.is_file() or path.suffix not in {".rs", ".ts", ".tsx", ".mjs", ".py", ".sh", ".toml", ".json"}:
                continue
            if SKIP_PARTS & set(path.parts):
                continue
            rel = path.relative_to(ROOT).as_posix()
            if rel == BUILDER:
                continue
            out.append(path)
    return out


def keywords(claim: str) -> list[str]:
    words = re.findall(r"[a-zA-Z][a-zA-Z0-9_.\-]{3,}", claim.lower())
    seen: list[str] = []
    for word in words:
        stem = word.strip(".,")
        if stem in STOP or stem.isdigit():
            continue
        if stem not in seen:
            seen.append(stem)
    return seen[:6]


def claims() -> list[tuple[str, str]]:
    text = (ROOT / BUILDER).read_text(encoding="utf-8")
    found: list[tuple[str, str]] = []
    for table in ("GEN_PROBES", "E2E_PROBES", "SUP_PROBES"):
        start = text.find(f"{table}: dict")
        if start < 0:
            continue
        end = len(text)
        for other in ("GEN_PROBES", "E2E_PROBES", "SUP_PROBES"):
            if other == table:
                continue
            pos = text.find(f"{other}: dict", start + 1)
            if pos > 0:
                end = min(end, pos)
        block = text[start:end]
        for match in re.finditer(r'"([A-Z]+-\d+)": \("no-cmd", "", "([^"]*)"\)', block):
            found.append((match.group(1), match.group(2)))
    return found


def main() -> int:
    files = sources()
    texts = {}
    for path in files:
        try:
            texts[path.relative_to(ROOT).as_posix()] = path.read_text("utf-8", errors="replace")
        except OSError:
            continue

    rows = []
    for test_id, claim in claims():
        kws = keywords(claim)
        hits: list[str] = []
        for rel, text in texts.items():
            lowered = text.lower()
            for word in kws:
                for match in re.finditer(re.escape(word), lowered):
                    line = lowered[: match.start()].count("\n") + 1
                    hits.append(f"{rel}:{line}:{word}")
                    break
        rows.append(
            {
                "test_id": test_id,
                "claim": claim,
                "keywords": kws,
                "hit_count": len(hits),
                "hits": hits[:12],
            }
        )

    signals = [r for r in rows if r["hit_count"] > 0]
    clean = [r for r in rows if r["hit_count"] == 0]
    report = {
        "gate": "applicability-absence-audit",
        "covers": ["DOD-041"],
        "harness": "scripts/audit-applicability-absences.py",
        "totals": {
            "claims": len(rows),
            "with_signal": len(signals),
            "with_no_signal": len(clean),
        },
        "note": (
            "A signal means at least one keyword from the claim appears in first-party "
            "source OUTSIDE the applicability builder. It is a REVIEW trigger, not proof "
            "that the claim is false: keywords like 'fuzz' legitimately appear in code "
            "that documents the absence. Each signal must be read before a row moves."
        ),
        "signals": signals,
        "no_signal": [{"test_id": r["test_id"], "claim": r["claim"]} for r in clean],
    }
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    (EVIDENCE / "applicability-absence-audit.json").write_text(
        json.dumps(report, indent=2) + "\n", encoding="utf-8"
    )

    lines = [
        "# Applicability absence audit (DOD-041)",
        "",
        "Generated by `scripts/audit-applicability-absences.py`. Do not hand-edit.",
        "",
        f"- Absence claims read from all three probe tables: **{len(rows)}**",
        f"- Claims with at least one keyword signal in first-party source: **{len(signals)}**",
        f"- Claims with no signal at all: **{len(clean)}**",
        "",
        "A signal is a REVIEW trigger, not proof the claim is false: a keyword such as",
        "'fuzz' legitimately appears in code that documents the absence. Every signal",
        "listed below must be read before that row's decision changes.",
        "",
        "## Claims with a signal",
        "",
        "| ID | Claim | Keywords | First hits |",
        "| --- | --- | --- | --- |",
    ]
    for row in signals:
        lines.append(
            f"| {row['test_id']} | {row['claim'][:90]} | {', '.join(row['keywords'])} | "
            f"{'; '.join(row['hits'][:3])} |"
        )
    lines += ["", "## Claims with no signal (absence supported by this probe)", ""]
    for row in clean:
        lines.append(f"- **{row['test_id']}** — {row['claim'][:140]}")
    lines += [
        "",
        "## Reproduce",
        "",
        "```",
        "python3 scripts/audit-applicability-absences.py",
        "```",
        "",
    ]
    (EVIDENCE / "applicability-absence-audit.md").write_text("\n".join(lines) + "\n", encoding="utf-8")

    print(
        f"applicability-absence-audit: {len(rows)} claims, {len(signals)} with a signal, "
        f"{len(clean)} with none"
    )
    for row in signals[:40]:
        print(f"  SIGNAL {row['test_id']}: {row['claim'][:80]} -> {row['hits'][:2]}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
