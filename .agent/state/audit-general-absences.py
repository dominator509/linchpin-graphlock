#!/usr/bin/env python3
"""Round 51 audit: which General-pack absence claims are still true?

Read-only. For each candidate it prints the registry title and probes the tree for
the capability the row says does not exist, so a reclassification can cite measured
evidence rather than an impression.
"""
import csv
import pathlib
import re

ROOT = pathlib.Path(".").resolve()
SKIP = {"node_modules", "target", "dist", ".git", "gen", "build"}


def tree_text() -> str:
    chunks = []
    for base in ("crates", "apps", "packages", "scripts"):
        for path in (ROOT / base).rglob("*"):
            if not path.is_file() or path.suffix not in {".rs", ".ts", ".tsx", ".mjs", ".py", ".sh"}:
                continue
            if SKIP & set(path.parts):
                continue
            try:
                chunks.append(path.read_text("utf-8"))
            except (UnicodeDecodeError, OSError):
                continue
    return "\n".join(chunks)


TREE = tree_text()

CANDIDATES: dict[str, list[tuple[str, str]]] = {
    "GEN-008": [("taint", r"\btaint\b")],
    "GEN-009": [("dataflow", r"data[- ]?flow|dataflow")],
    "GEN-013": [("security metrics", r"security[_ ]metric|metric.*security")],
    "GEN-014": [("complexity", r"cyclomatic|complexity")],
    "GEN-025": [("coverage-guided fuzz", r"coverage-guided|libfuzzer|cargo-fuzz|afl")],
    "GEN-026": [("grammar corpus", r"grammar|corpus")],
    "GEN-027": [("mutation fuzz", r"mutation|mutat")],
    "GEN-028": [("protocol fuzz", r"protocol.*fuzz|fuzz.*protocol")],
    "GEN-030": [("api security suite", r"api[_ ]security|ipc.*security|security.*ipc")],
    "GEN-036": [("client-side security", r"webview.*security|client-side|csp|content-security")],
    "GEN-043": [("vuln assessment", r"vulnerab")],
    "GEN-057": [("crypto test", r"crypto|encrypt|signature|hmac|sha256")],
    "GEN-058": [("weak crypto", r"weak.*crypto|crypto.*weak|deprecated.*cipher")],
    "GEN-065": [("config hardening", r"hardening|hardened|config.*baseline")],
    "GEN-083": [("pre-build gate", r"pre-build|prebuild|before_?build")],
    "GEN-091": [("threat model", r"threat[_ ]model|threatmap|threat_map")],
    "GEN-111": [("incident response", r"incident|postmortem|post-mortem|rehearsal")],
    "GEN-114": [("chaos", r"chaos")],
    "GEN-115": [("fault injection", r"fault[_ ]inject|fault_class|total_loss|corruption")],
    "GEN-119": [("anomaly detection", r"anomaly|anomal")],
}

rows = {r["test_id"]: r for r in csv.DictReader((ROOT / ".agent/verification/MASTER_TEST_REGISTRY.csv").open(encoding="utf-8"))}
for test_id, patterns in CANDIDATES.items():
    row = rows[test_id]
    print(f"## {test_id} | {row['title']} | applicability={row['applicability']}")
    for label, pattern in patterns:
        hits = [m.start() for m in re.finditer(pattern, TREE, re.IGNORECASE)]
        first = ""
        if hits:
            snippet = TREE[max(0, hits[0] - 60) : hits[0] + 80].replace("\n", " ")
            first = f" first@{hits[0]}: ...{snippet.strip()[:150]}..."
        print(f"   {label}: {len(hits)} match(es){first}")
