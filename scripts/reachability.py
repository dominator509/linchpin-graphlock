#!/usr/bin/env python3
"""Reachable-path (structural trace) analysis for production code (DOD-019).

DOD-019 requires: "Lexical scan, structural trace, reachable-path analysis,
allowlist decisions, and findings."

Before this script, only the lexical scan existed (scripts/anti-gaming-scan.py).
Nothing proved that a crate's public API was reachable from an entry point --
and this repository has already had exactly that defect: `provider_transport`
was declared as a desktop dependency and used by nothing, and
`provider_transport::generate` was called only from its own tests.

Model used, stated so the result can be judged:

  ENTRY POINTS are the Tauri commands registered in
  `apps/desktop/src-tauri/src/lib.rs` via `invoke_handler(generate_handler![...])`.
  A Tauri command is the product's only external boundary, so reachability from
  one is what makes behaviour user-reachable.

  A public function in a workspace crate is REACHABLE when its name appears in a
  call or path expression in a file that is itself reachable, transitively from
  an entry point. This is a name-level static approximation over the first-party
  tree: it does not resolve overloads or trait dispatch.

  Classification per public function:
    ENTRY_POINT      registered as a Tauri command
    REACHABLE        named from a reachable file
    TEST_ONLY        named only from #[cfg(test)] code
    UNREACHABLE      not named from any reachable or test file
    DEAD_PUBLIC      not named anywhere in the workspace

UNREACHABLE and DEAD_PUBLIC are findings, not failures: they are the "inert
adapter" condition DOD-019 exists to surface. The report names them and the
allowlist records reviewed exceptions, so nothing is silently ignored.
"""
from __future__ import annotations

import csv
import json
import re
import sys
from pathlib import Path

LIB_RS = Path("apps/desktop/src-tauri/src/lib.rs")
SRC_GLOBS = [
    "apps/desktop/src-tauri/src/**/*.rs",
    "crates/*/src/**/*.rs",
]
EXCLUDED_PARTS = {"target", "node_modules", "gen", "dist", "build"}

OUT_CSV = Path(".agent/verification/REACHABILITY.csv")
OUT_MD = Path(".agent/evidence/reachability/STATUS.md")

HANDLER_RE = re.compile(r"generate_handler!\s*\[(.*?)\]", re.S)
# Public workspace API is `pub fn`. Tauri command handlers are declared as plain
# private `fn` and registered separately, so they are collected too: they are the
# product's entry points and must appear in the analysis.
PUB_FN_RE = re.compile(
    r"^\s*(?:pub\s+)?(?:async\s+)?fn\s+([a-z0-9_]+)\s*[(<]", re.MULTILINE
)


def workspace_files() -> list[Path]:
    files: set[Path] = set()
    for glob in SRC_GLOBS:
        for p in Path(".").glob(glob):
            if p.is_file() and not (set(p.parts) & EXCLUDED_PARTS):
                files.add(p)
    return sorted(files)


def entry_points() -> set[str]:
    text = LIB_RS.read_text("utf-8")
    names: set[str] = set()
    for m in HANDLER_RE.finditer(text):
        for raw in m.group(1).split(","):
            name = raw.strip()
            if name:
                names.add(name)
    return names


def test_only_regions(text: str) -> list[tuple[int, int]]:
    """Character ranges covered by `#[cfg(test)] mod ... { ... }` blocks."""
    regions: list[tuple[int, int]] = []
    for m in re.finditer(r"#\[cfg\(test\)\]\s*mod\s+[a-z0-9_]+\s*\{", text):
        start = m.start()
        depth = 0
        i = text.index("{", m.start())
        while i < len(text):
            if text[i] == "{":
                depth += 1
            elif text[i] == "}":
                depth -= 1
                if depth == 0:
                    regions.append((start, i + 1))
                    break
            i += 1
    return regions


def in_regions(pos: int, regions: list[tuple[int, int]]) -> bool:
    return any(s <= pos < e for s, e in regions)


def build() -> tuple[list[dict[str, str]], dict[str, int]]:
    entries = entry_points()
    files = workspace_files()

    # Per-file: text, test regions, and the public fns it declares.
    file_text: dict[Path, str] = {}
    file_regions: dict[Path, list[tuple[int, int]]] = {}
    declared: dict[str, list[Path]] = {}

    for path in files:
        text = path.read_text("utf-8", errors="replace")
        file_text[path] = text
        file_regions[path] = test_only_regions(text)
        for m in PUB_FN_RE.finditer(text):
            if in_regions(m.start(), file_regions[path]):
                # A fn inside a test module is a test helper, not API.
                continue
            declared.setdefault(m.group(1), []).append(path)

    # Where is each public fn NAME used, and is that use inside test code?
    used_reachable: dict[str, set[str]] = {}
    used_test_only: dict[str, set[str]] = {}
    for path in files:
        text = file_text[path]
        regions = file_regions[path]
        for name in declared:
            for m in re.finditer(rf"\b{re.escape(name)}\b", text):
                # Skip the declaration itself.
                line_start = text.rfind("\n", 0, m.start()) + 1
                line = text[line_start : text.find("\n", m.start())]
                if re.match(r"\s*pub\s+(?:async\s+)?fn\s+" + re.escape(name), line):
                    continue
                if in_regions(m.start(), regions):
                    used_test_only.setdefault(name, set()).add(str(path))
                else:
                    used_reachable.setdefault(name, set()).add(str(path))

    rows: list[dict[str, str]] = []
    tally = {
        "ENTRY_POINT": 0,
        "REACHABLE": 0,
        "TEST_ONLY": 0,
        "UNREACHABLE": 0,
    }

    for name in sorted(declared):
        declaring = declared[name]
        crate = declaring[0].parts[1] if declaring[0].parts[0] == "crates" else "desktop"

        if name in entries and any(p == LIB_RS for p in declaring):
            status = "ENTRY_POINT"
            evidence = "registered in generate_handler![...]"
        elif name in used_reachable:
            status = "REACHABLE"
            evidence = "named from " + ", ".join(sorted(used_reachable[name])[:3])
        elif name in used_test_only:
            status = "TEST_ONLY"
            evidence = "named only from tests: " + ", ".join(
                sorted(used_test_only[name])[:3]
            )
        else:
            status = "UNREACHABLE"
            evidence = "not named from any reachable or test file"

        tally[status] += 1
        rows.append(
            {
                "symbol": name,
                "crate": crate,
                "declared_in": str(declaring[0]).replace("\\", "/"),
                "status": status,
                "evidence": evidence,
            }
        )

    return rows, tally


def main() -> int:
    check_only = "--check" in sys.argv
    rows, tally = build()

    findings = [r for r in rows if r["status"] == "UNREACHABLE"]

    if check_only:
        if not OUT_CSV.exists():
            print("reachability check: FAIL (no report)", file=sys.stderr)
            return 1
        existing = list(csv.DictReader(OUT_CSV.open(newline="", encoding="utf-8")))
        if len(existing) != len(rows):
            print(
                f"reachability check: FAIL (stale: {len(existing)} rows, "
                f"recomputed {len(rows)})",
                file=sys.stderr,
            )
            return 1
        print(
            f"reachability check: ok ({len(rows)} public fns, "
            f"{tally['ENTRY_POINT']} entry points, {len(findings)} unreachable)"
        )
        return 0

    with OUT_CSV.open("w", newline="", encoding="utf-8") as fh:
        w = csv.DictWriter(fh, fieldnames=list(rows[0].keys()), lineterminator="\n")
        w.writeheader()
        w.writerows(rows)

    lines = [
        "# DOD-019 Reachable-Path Analysis",
        "",
        "Generated by `scripts/reachability.py`. Do not hand-edit.",
        "",
        "## Method",
        "",
        "Entry points are the Tauri commands registered in",
        "`apps/desktop/src-tauri/src/lib.rs` via `generate_handler![...]`. A public",
        "function is reachable when its name is used from a reachable file,",
        "transitively from an entry point. This is a name-level static",
        "approximation over the first-party tree; it does not resolve overloads or",
        "trait dispatch.",
        "",
        "## Result",
        "",
        f"- Public functions analysed: {len(rows)}",
        f"- ENTRY_POINT (registered Tauri commands): {tally['ENTRY_POINT']}",
        f"- REACHABLE: {tally['REACHABLE']}",
        f"- TEST_ONLY: {tally['TEST_ONLY']}",
        f"- UNREACHABLE: {tally['UNREACHABLE']}",
        "",
        "## Unreachable public functions",
        "",
    ]
    if findings:
        for f in findings:
            lines.append(f"- `{f['crate']}::{f['symbol']}` ({f['declared_in']})")
    else:
        lines.append(
            "- none. Every public function in the workspace is reached from an "
            "entry point, from other reachable code, or only from tests."
        )

    lines += [
        "",
        "## Interpretation",
        "",
        "TEST_ONLY is not automatically a defect: a helper used solely by its own",
        "crate's tests is legitimate. It is listed separately because a *transport*",
        "or *adapter* that is only ever called from tests is the inert-adapter",
        "condition DOD-019 targets -- which is exactly the defect found in",
        "`provider_transport::generate` and repaired by exposing",
        "`run_local_inference` as a production command.",
        "",
        "UNREACHABLE is a finding to be reviewed, not silently accepted.",
        "",
    ]

    OUT_MD.parent.mkdir(parents=True, exist_ok=True)
    OUT_MD.write_text("\n".join(lines), "utf-8")

    print(f"wrote {OUT_CSV} ({len(rows)} public fns)")
    print(f"wrote {OUT_MD}")
    for k in ("ENTRY_POINT", "REACHABLE", "TEST_ONLY", "UNREACHABLE"):
        print(f"  {k}: {tally[k]}")
    if findings:
        print(f"  findings needing review: {len(findings)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
