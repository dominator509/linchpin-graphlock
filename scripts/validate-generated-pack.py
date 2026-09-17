#!/usr/bin/env python3
"""GraphLock v3.1 generated-pack validator.

This is intentionally conservative. It validates pack shape and evidence discipline;
it does not prove the product works by itself.
"""
from __future__ import annotations
import csv, json, os, re, sys
from pathlib import Path

root = Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
errors: list[str] = []
warnings: list[str] = []

def err(msg: str) -> None:
    errors.append(msg)

def exists(rel: str) -> Path:
    p = root / rel
    if not p.exists():
        err(f"missing required file: {rel}")
    return p

required = [
    "AGENTS.md", "COMMANDS.md", ".agent/GRAPH.md", ".agent/LOOPS.md",
    ".agent/DONE_LAW.md", ".agent/verification/FUNCTIONAL_PROOF_MATRIX.csv",
    "scripts/ledger.sh", "scripts/graph-next.sh"
]
for rel in required:
    exists(rel)

# Duplicate probe keys. MEASURED DEFECT this guards: round 51 reclassified four
# registry rows from NOT_APPLICABLE to APPLICABLE but left three of the older "no-cmd"
# entries in the same dict literal, and Python keeps the LAST assignment -- so the
# matrix silently kept the old decision while case results for those IDs were written
# anyway. A duplicate key in a probe table is never intentional, and the failure mode
# is invisible in the rendered matrix, so it is an error here.
builder = root / "scripts/build-applicability.py"
if builder.exists():
    text = builder.read_text("utf-8", errors="replace")
    key_pattern = re.compile(r'"([A-Z]+-\d+)": \(')
    for table in ("GEN_PROBES", "E2E_PROBES", "SUP_PROBES"):
        start = text.find(f"{table}: dict")
        if start < 0:
            continue
        positions = [text.find(f"{other}: dict", start + 1) for other in
                     ("GEN_PROBES", "E2E_PROBES", "SUP_PROBES")]
        end = min([p for p in positions if p > 0] + [len(text)])
        keys = key_pattern.findall(text[start:end])
        duplicates = sorted({key for key in keys if keys.count(key) > 1})
        if duplicates:
            err(f"{table} declares duplicate keys (last assignment silently wins): {duplicates}")


# Placeholder residue.
# The skip predicate must test whether ANY component is an excluded directory
# (`not any(part in SKIP_DIRS ...)`). The previous form used
# `any(part not in SKIP_DIRS ...)`, which is true for every absolute path
# (drive, "dev", ... are never in the set), so nothing was ever skipped: the
# scan walked target/, node_modules/ and dist/, taking ~54s and reporting
# thousands of false "placeholder residue" hits from generated JSON/build
# files. Component matching is also relative to the scan root.
SKIP_DIRS = {".git", "node_modules", ".venv", "target", "dist", "build", ".next", ".cache"}
for p in root.rglob("*"):
    if not p.is_file():
        continue
    try:
        rel_parts = set(p.relative_to(root).parts)
    except ValueError:
        continue
    if rel_parts & SKIP_DIRS:
        continue
    try:
        if p.stat().st_size >= 5_000_000:
            continue
        text = p.read_text("utf-8")
    except (UnicodeDecodeError, OSError):
        continue
    # Placeholder residue detection.
    #
    # The original heuristic flagged any `{{` or `}}`, which produces false
    # positives on legitimate sources: JSX inline styles (`style={{...}}`) and
    # generated JSON schema documents. Refine to marker-shaped residue that
    # actually signals an unfinished template, while keeping the check live for
    # prose files (which are where template placeholders really occur).
    if p.suffix in {".md", ".txt", ".rst"}:
        if ("{" * 2) in text or ("}" * 2) in text:
            err(f"placeholder residue in {p.relative_to(root)}")
    elif p.suffix in {".rs", ".ts", ".tsx", ".js", ".jsx", ".py", ".sh"}:
        # Only flag brace runs that stand alone as a token, not JSX `={{...}}`
        # or Rust struct literals. This validator and the anti-gaming scanner
        # necessarily contain brace literals for their own matching logic.
        if p.name in {"validate-generated-pack.py", "anti-gaming-scan.py"}:
            continue
        for m in re.finditer(r"(?<![={(\s])\{\{\{?(?![})\s])", text):
            line = text.count("\n", 0, m.start()) + 1
            err(
                f"placeholder residue in {p.relative_to(root)}:{line} "
                f"({m.group(0)!r})"
            )
    if p.name not in {"validate-generated-pack.py", "anti-gaming-scan.py"} and re.search(r"\b(rest omitted|similar to above|and so on|TODO pass|coming soon)\b", text, re.I):
        warnings.append(f"possible incomplete prose/code in {p.relative_to(root)}")

# Graph parse, cycles, and dependency sanity.
graph = root/".agent/GRAPH.md"
if graph.exists():
    text = graph.read_text("utf-8", errors="replace")
    m = re.search(r"GRAPH-TABLE-BEGIN\n(.*?)\nGRAPH-TABLE-END", text, re.S)
    if not m:
        err(".agent/GRAPH.md missing GRAPH-TABLE block")
    else:
        deps: dict[str, list[str]] = {}
        for lineno, line in enumerate(m.group(1).splitlines(), 1):
            mm = re.fullmatch(r"NODE\s+(\S+)\s+DEPS\s+(.+)", line.strip())
            if not mm:
                err(f"malformed graph line {lineno}: {line}")
                continue
            nid, dep_s = mm.group(1), mm.group(2)
            if nid in deps: err(f"duplicate graph node {nid}")
            deps[nid] = [] if dep_s == "-" else dep_s.split(",")
        for nid, ds in deps.items():
            for d in ds:
                if d not in deps:
                    err(f"{nid} depends on missing node {d}")
        visiting: set[str] = set(); visited: set[str] = set()
        def dfs(n: str) -> None:
            if n in visiting:
                err(f"cycle detected at {n}"); return
            if n in visited: return
            visiting.add(n)
            for d in deps.get(n, []): dfs(d)
            visiting.remove(n); visited.add(n)
        for n in list(deps): dfs(n)

# Functional matrix.
fpm = root/".agent/verification/FUNCTIONAL_PROOF_MATRIX.csv"
if fpm.exists():
    required_cols = ["requirement_id","user_outcome","entrypoint_ui_or_api","command_or_route","code_path","data_written","data_read_back","worker_or_async_effect","authz_rule","negative_case","restart_persistence_case","concurrency_case","e2e_test_id","artifact_digest","evidence_path","status"]
    with fpm.open(newline='', encoding='utf-8') as fh:
        rdr = csv.DictReader(fh)
        missing = [c for c in required_cols if c not in (rdr.fieldnames or [])]
        if missing: err(f"FUNCTIONAL_PROOF_MATRIX missing columns: {missing}")
        rows = list(rdr)
    if not rows:
        err("FUNCTIONAL_PROOF_MATRIX has no rows")
    for idx, row in enumerate(rows, 2):
        if row.get("status") == "DONE_VERIFIED":
            for c in ["requirement_id","entrypoint_ui_or_api","code_path","negative_case","e2e_test_id","evidence_path"]:
                if not row.get(c): err(f"FUNCTIONAL_PROOF_MATRIX row {idx} DONE_VERIFIED missing {c}")

# Command references.
commands = root/"COMMANDS.md"
if commands.exists():
    txt = commands.read_text('utf-8', errors='replace')
    if "validate-generated-pack.py" not in txt:
        warnings.append("COMMANDS.md does not mention validate-generated-pack.py")
    if "anti-gaming" not in txt.lower():
        warnings.append("COMMANDS.md does not mention anti-gaming scan/review")

# Ledger closure consistency when present.
ledger = root/".agent/state/LEDGER.md"
if ledger.exists():
    txt = ledger.read_text('utf-8', errors='replace')
    for node in sorted(set(re.findall(r"\|\s*(EP-\d{3,})\s*\|\s*CLOSED_BLOCKED\s*\|", txt))):
        rec = root/f".agent/blocked/{node}.blocked.json"
        if not rec.exists():
            err(f"CLOSED_BLOCKED ledger event missing blocked record {rec.relative_to(root)}")
    for node in sorted(set(re.findall(r"\|\s*(EP-\d{3,})\s*\|\s*NODE_DONE\s*\|", txt))):
        review = root/f".agent/evidence/{node}/anti_gaming_review.json"
        if not review.exists():
            warnings.append(f"NODE_DONE lacks anti_gaming_review.json for {node}")
        else:
            try:
                data = json.loads(review.read_text('utf-8'))
                if data.get('verdict') != 'PASS': err(f"anti_gaming_review for {node} is not PASS")
            except Exception as exc:
                err(f"cannot parse anti_gaming_review for {node}: {exc}")

if errors:
    print("generated pack validation: failed")
    for e in errors: print(f"ERROR: {e}")
    for w in warnings: print(f"WARN: {w}")
    sys.exit(1)
print("generated pack validation: ok")
for w in warnings: print(f"WARN: {w}")
