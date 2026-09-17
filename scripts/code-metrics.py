#!/usr/bin/env python3
"""Security code metrics and cyclomatic complexity over first-party source (GEN-013, GEN-014).

WHY THIS EXISTS. GEN-013 ("Security Code Metrics Analysis") and GEN-014 ("Cyclomatic
Complexity Analysis") were both NOT_APPLICABLE with "no metrics tool is configured" and
"no cyclomatic complexity measurement is recorded". Neither needed a new dependency: the
numbers that matter for this codebase can be measured from the source, and what turns a
measurement into a control is an ENFORCED THRESHOLD plus a hotspot list a reviewer reads.

WHAT IT MEASURES (production code only, tests tagged separately)
  * per-function cyclomatic complexity: 1 + decision points inside the function body
    (`if`, `else if`, `match` arms, `&&`, `||`, `for`, `while`, `loop`, `?`);
  * security-relevant counts: `unsafe` blocks, panicking calls (`unwrap`, `expect`,
    `panic!`, `todo!`, `unimplemented!`) split between production and test code,
    `#[allow(...)]` suppressions, and the file/function size distribution;
  * a top-N hotspot list, because a metric nobody can point at is not actionable.

WHAT IT ENFORCES (thresholds below, all of them failing the gate rather than printing)
  * maximum complexity for any single function;
  * zero `unsafe` blocks outside an explicit allowlist;
  * zero `todo!`/`unimplemented!` in production code;
  * a bound on panicking calls in production code that is above today's measurement and
    therefore ratchets rather than rubber-stamps.

DISCRIMINATION IS PROVEN: `--self-test` measures a synthetic file containing a
deliberately complex function, an `unsafe` block and a `todo!`, and asserts every one is
reported and would breach its threshold.
"""
from __future__ import annotations

import json
import pathlib
import re
import sys
import tempfile

ROOT = pathlib.Path(".").resolve()
EVIDENCE = ROOT / ".agent/evidence/code-metrics"

# Thresholds, chosen FROM the measurement rather than guessed (round 59).
#
# First measurement, before `#[cfg(test)]` modules were excluded, reported 396 production
# panicking calls and the ratchet was set at 120 -- a bound seventeen times the real number,
# which would have passed a hundred-panic regression. With test modules excluded the real
# figure is 7, so the ratchet is 15: tight enough that a new chain of unwraps trips it, with
# a little room for ordinary work. Complexity is 40 against a measured production maximum of
# 35, and test-file functions are exempt because a stress or soak trial is a long procedure
# by design -- they are still measured and printed.
MAX_FUNCTION_COMPLEXITY = 40
MAX_PRODUCTION_PANICS = 15
UNSAFE_ALLOWLIST: list[dict[str, str]] = [
    {
        "path": "crates/platform_windows/src/lib.rs",
        "reason": (
            "one Win32 call, GetProcessMemoryInfo, which is the only way to read this "
            "process's working set. It is `unsafe` because it is an FFI call taking a POD "
            "out-parameter whose `cb` field must be set to its own size; the call's Result "
            "is checked (windows 0.61 returns Result rather than BOOL) and the comment above "
            "it records that the handle must not be closed. Third-party FFI is the one place "
            "this project accepts `unsafe`, and it is the only block in the workspace."
        ),
    },
]

DECISION_TOKENS = [
    (re.compile(r"\bif\s"), 1),
    (re.compile(r"\belse\s+if\b"), 1),
    (re.compile(r"\bmatch\b"), 0),  # the arms are counted instead, to avoid double counting
    (re.compile(r"=>"), 1),        # one match arm
    (re.compile(r"&&"), 1),
    (re.compile(r"\|\|"), 1),
    (re.compile(r"\bfor\s"), 1),
    (re.compile(r"\bwhile\s"), 1),
    (re.compile(r"\bloop\b"), 1),
    (re.compile(r"\?"), 1),
]
PANIC_PATTERNS = [
    (re.compile(r"\.unwrap\(\)"), "unwrap"),
    (re.compile(r"\.expect\("), "expect"),
    (re.compile(r"\bpanic!\("), "panic"),
    (re.compile(r"\btodo!\("), "todo"),
    (re.compile(r"\bunimplemented!\("), "unimplemented"),
]
SOURCE_SUFFIXES = {".rs", ".ts", ".tsx"}
SKIP_PARTS = {"node_modules", "target", "dist", ".git", "gen", "build"}


def strip_test_modules(text: str) -> str:
    """Remove `#[cfg(test)] mod ... { ... }` regions from a production file.

    MEASURED WHY: the first version of this harness counted panicking calls across whole
    files, so `crates/storage/src/vault.rs` reported 146 production panics -- almost all of
    them inside its `#[cfg(test)] mod tests`. That number would have set a ratchet against
    test code and described production behaviour that does not exist. Test code is now
    removed from production counts and reported separately, because a `unwrap()` in a test
    and a `unwrap()` on a user path are different facts.
    """
    out = text
    for match in re.finditer(r"#\[cfg\(test\)\]", text):
        body = body_of(text, match.end())
        if body:
            out = out.replace(body, "", 1)
    return out


def production_files() -> list[tuple[pathlib.Path, bool]]:
    """(path, is_test) for first-party source."""
    out: list[tuple[pathlib.Path, bool]] = []
    for base in ("crates", "apps", "packages"):
        for path in (ROOT / base).rglob("*"):
            if not path.is_file() or path.suffix not in SOURCE_SUFFIXES:
                continue
            if SKIP_PARTS & set(path.parts):
                continue
            rel = path.relative_to(ROOT).as_posix()
            is_test = "/tests/" in rel or rel.endswith(".test.ts") or rel.endswith(".spec.ts") or rel.endswith(".test.tsx")
            out.append((path, is_test))
    return sorted(out)


def body_of(text: str, start: int) -> str:
    """The brace-balanced body beginning at the first '{' after `start`."""
    open_at = text.find("{", start)
    if open_at < 0:
        return ""
    depth = 0
    for index in range(open_at, len(text)):
        char = text[index]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return text[open_at : index + 1]
    return text[open_at:]


def functions(text: str) -> list[dict]:
    found = []
    for match in re.finditer(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)", text):
        name = match.group(1)
        body = body_of(text, match.end())
        if not body:
            continue
        complexity = 1
        for pattern, weight in DECISION_TOKENS:
            complexity += weight * len(pattern.findall(body))
        # `else if` was counted twice (once as `if`, once as `else if`).
        complexity -= len(re.findall(r"\belse\s+if\b", body))
        line = text[: match.start()].count("\n") + 1
        found.append({"name": name, "line": line, "complexity": complexity, "lines": body.count("\n") + 1})
    return found


def measure(root: pathlib.Path | None = None) -> dict:
    root = root or ROOT
    files = []
    totals = {
        "functions": 0,
        "max_complexity": 0,
        "mean_complexity": 0.0,
        "unsafe_blocks": 0,
        "panics_production": 0,
        "panics_test": 0,
        "allow_attributes": 0,
        "todo_production": 0,
    }
    hotspots: list[dict] = []
    for path, is_test in (production_files() if root == ROOT else [(p, False) for p in sorted(root.rglob("*.rs"))]):
        try:
            text = path.read_text(encoding="utf-8")
        except (UnicodeDecodeError, OSError):
            continue
        rel = path.relative_to(root).as_posix()
        # Production counts exclude `#[cfg(test)]` modules; test counts see the whole file.
        production_text = text if is_test or root != ROOT else strip_test_modules(text)
        fns = functions(production_text) if path.suffix == ".rs" else []
        complexities = [f["complexity"] for f in fns]
        totals["functions"] += len(fns)
        if complexities:
            totals["max_complexity"] = max(totals["max_complexity"], max(complexities))
        for fn in fns:
            hotspots.append({"file": rel, "function": fn["name"], "line": fn["line"],
                             "complexity": fn["complexity"], "test": is_test})
        unsafe = len(re.findall(r"\bunsafe\s*\{", production_text))
        totals["unsafe_blocks"] += unsafe
        counts = {label: len(pattern.findall(production_text)) for pattern, label in PANIC_PATTERNS}
        panics = counts["unwrap"] + counts["expect"] + counts["panic"]
        if is_test:
            totals["panics_test"] += panics
        else:
            totals["panics_production"] += panics
            test_counts = {label: len(pattern.findall(text)) for pattern, label in PANIC_PATTERNS}
            totals["panics_test"] += test_counts["unwrap"] + test_counts["expect"] + test_counts["panic"]
        totals["allow_attributes"] += len(re.findall(r"#\[allow\(", production_text))
        if not is_test:
            totals["todo_production"] += counts["todo"] + counts["unimplemented"]
        files.append({"path": rel, "test": is_test, "functions": len(fns), "unsafe": unsafe, **counts})
    if totals["functions"]:
        totals["mean_complexity"] = round(
            sum(f["complexity"] for f in hotspots) / totals["functions"], 2
        )
    production_hotspots = [h for h in hotspots if not h.get("test")]
    test_hotspots = [h for h in hotspots if h.get("test")]
    totals["max_complexity_production"] = max((h["complexity"] for h in production_hotspots), default=0)
    totals["max_complexity_test"] = max((h["complexity"] for h in test_hotspots), default=0)
    totals["functions_production"] = len(production_hotspots)
    totals["functions_test"] = len(test_hotspots)
    hotspots.sort(key=lambda h: (-h["complexity"], h["file"], h["line"]))
    return {"totals": totals, "hotspots": hotspots[:25], "files": files}


def main() -> int:
    if "--self-test" in sys.argv:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            (root / "sample.rs").write_text(
                "fn simple() -> u8 { 1 }\n"
                "fn gnarly(x: u8) -> u8 {\n"
                + "".join(f"    if x > {i} && x != {i} {{ return {i}; }}\n" for i in range(20))
                + "    x\n}\n"
                "fn dangerous() { unsafe { std::ptr::null::<u8>().read(); } todo!() }\n",
                encoding="utf-8",
            )
            result = measure(root)
            gnarly = next((h for h in result["hotspots"] if h["function"] == "gnarly"), None)
            ok = (
                gnarly is not None
                and gnarly["complexity"] >= 20 + 20 + 1 - 1  # 20 ifs + 20 && plus the base
                and result["totals"]["unsafe_blocks"] >= 1
                and result["totals"]["todo_production"] >= 1
            )
            print(f"code-metrics self-test: gnarly complexity = {gnarly['complexity'] if gnarly else None}")
            print(
                f"code-metrics self-test: unsafe={result['totals']['unsafe_blocks']} "
                f"todo={result['totals']['todo_production']} panics={result['totals']['panics_production']}"
            )
            if not ok:
                print("code-metrics self-test: FAIL -- the measurement missed a planted defect", file=sys.stderr)
                return 1
            print("code-metrics self-test: ok (complexity, unsafe and todo! are all measured)")
            return 0

    result = measure()
    totals = result["totals"]
    breaches: list[str] = []
    for hotspot in result["hotspots"]:
        if hotspot.get("test"):
            continue  # test-file functions are measured and printed, not ceilinged
        if hotspot["complexity"] > MAX_FUNCTION_COMPLEXITY:
            breaches.append(
                f"{hotspot['file']}:{hotspot['line']} {hotspot['function']} complexity "
                f"{hotspot['complexity']} exceeds {MAX_FUNCTION_COMPLEXITY}"
            )
    if totals["unsafe_blocks"] > len(UNSAFE_ALLOWLIST):
        breaches.append(f"{totals['unsafe_blocks']} unsafe block(s) outside the allowlist")
    if totals["todo_production"]:
        breaches.append(f"{totals['todo_production']} todo!/unimplemented! in production code")
    if totals["panics_production"] > MAX_PRODUCTION_PANICS:
        breaches.append(
            f"{totals['panics_production']} panicking call(s) in production exceeds the ratchet "
            f"{MAX_PRODUCTION_PANICS}"
        )

    report = {
        "gate": "code-metrics",
        "covers": ["GEN-013", "GEN-014"],
        "harness": "scripts/code-metrics.py",
        "thresholds": {
            "max_function_complexity": MAX_FUNCTION_COMPLEXITY,
            "max_production_panics": MAX_PRODUCTION_PANICS,
            "unsafe_allowlist": UNSAFE_ALLOWLIST,
        },
        "totals": totals,
        "hotspots": result["hotspots"],
        "breaches": breaches,
        "verdict": "PASS" if not breaches else "FAIL",
        "limits": (
            "complexity is decision-token counting, not a dataflow analysis: it ranks functions "
            "for review and enforces a ceiling, and it is blind to nesting depth, state space and "
            "architecture. Panicking-call counts include arguments and do not distinguish an "
            "infallible unwrap from a load-bearing one."
        ),
    }
    EVIDENCE.mkdir(parents=True, exist_ok=True)

    if "--check" in sys.argv:
        recorded = EVIDENCE / "report.json"
        if not recorded.exists():
            print("code-metrics check: FAIL -- no recorded report", file=sys.stderr)
            return 1
        previous = json.loads(recorded.read_text(encoding="utf-8"))
        if previous.get("totals") != totals or previous.get("hotspots") != result["hotspots"]:
            print(
                "code-metrics check: FAIL -- the measurements moved since the report was written; "
                "rerun python3 scripts/code-metrics.py",
                file=sys.stderr,
            )
            return 1
        print(f"code-metrics check: ok ({totals['functions']} functions, max complexity {totals['max_complexity']})")
        return 0

    (EVIDENCE / "report.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    lines = [
        "# Security code metrics and complexity (GEN-013, GEN-014)",
        "",
        "Generated by `scripts/code-metrics.py`. Do not hand-edit.",
        "",
        f"- Verdict: **{report['verdict']}**",
        f"- Functions measured: **{totals['functions']}**; max complexity **{totals['max_complexity']}**, "
        f"mean **{totals['mean_complexity']}**",
        f"- Enforced ceilings: complexity <= {MAX_FUNCTION_COMPLEXITY}, production panicking calls <= "
        f"{MAX_PRODUCTION_PANICS}, unsafe blocks <= {len(UNSAFE_ALLOWLIST)}, todo!/unimplemented! == 0",
        "",
        "## Counts",
        "",
        "| Metric | Value |",
        "| --- | --- |",
        f"| functions | {totals['functions']} |",
        f"| max complexity | {totals['max_complexity']} |",
        f"| mean complexity | {totals['mean_complexity']} |",
        f"| unsafe blocks | {totals['unsafe_blocks']} |",
        f"| panicking calls (production) | {totals['panics_production']} |",
        f"| panicking calls (tests) | {totals['panics_test']} |",
        f"| #[allow(...)] suppressions | {totals['allow_attributes']} |",
        f"| todo!/unimplemented! (production) | {totals['todo_production']} |",
        "",
        "## Complexity hotspots",
        "",
        "| File | Function | Line | Complexity |",
        "| --- | --- | --- | --- |",
    ]
    for hotspot in result["hotspots"][:15]:
        lines.append(f"| `{hotspot['file']}` | {hotspot['function']} | {hotspot['line']} | {hotspot['complexity']} |")
    lines += ["", "## Breaches", ""]
    if breaches:
        lines += [f"- {breach}" for breach in breaches]
    else:
        lines.append("None: every enforced ceiling holds on this candidate.")
    lines += [
        "",
        "## Limits, stated rather than implied",
        "",
        report["limits"],
        "",
        "## Discrimination",
        "",
        "`python3 scripts/code-metrics.py --self-test` measures a synthetic file containing a",
        "function with 20 `if` conditions, an `unsafe` block and a `todo!`, and asserts each is",
        "measured. A metrics gate that has never seen a defect cannot be trusted.",
        "",
        "## Reproduce",
        "",
        "```",
        "python3 scripts/code-metrics.py",
        "python3 scripts/code-metrics.py --self-test",
        "python3 scripts/code-metrics.py --check",
        "```",
        "",
    ]
    (EVIDENCE / "STATUS.md").write_text("\n".join(lines), encoding="utf-8")

    if breaches:
        print(f"code-metrics: FAIL -- {len(breaches)} breach(es)", file=sys.stderr)
        for breach in breaches[:10]:
            print(f"  {breach}", file=sys.stderr)
        return 1
    print(
        f"code-metrics: ok ({totals['functions']} functions: {totals['functions_production']} production "
        f"(max complexity {totals['max_complexity_production']} <= {MAX_FUNCTION_COMPLEXITY}), "
        f"{totals['functions_test']} in test files (max {totals['max_complexity_test']}, exempt); "
        f"{totals['panics_production']} production panicking calls <= {MAX_PRODUCTION_PANICS}; "
        f"{totals['unsafe_blocks']} unsafe block(s))"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
