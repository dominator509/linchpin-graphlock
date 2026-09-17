#!/usr/bin/env python3
"""Weak-cryptography detection: scan the dependency graph and first-party source (GEN-058).

WHY THIS EXISTS. GEN-058 ("Weak Cryptography Testing") was recorded NOT_APPLICABLE with
"no weak-cryptography test exists" -- true, and closable: nothing in this repository
looked for weak or deprecated primitives anywhere. The cryptographic IMPLEMENTATION is
tested (GEN-057), but using a primitive correctly says nothing about whether a weak one
was pulled in or called.

WHAT IT SCANS, and what it deliberately does not:

  * FIRST-PARTY SOURCE for identifier USE of weak primitives (`md5::`, `Sha1::`,
    `Rc4`, `Ecb`, `"aes-128-ecb"`, ...). The patterns are code-shaped on purpose: this
    repository's own documents discuss these names in prose, and a scan that flagged its
    own documentation would be worthless.
  * LOCKED DEPENDENCY GRAPHS (Cargo.lock, pnpm-lock.yaml) for weak primitives reaching
    the product transitively, which is the common real-world path.
  * It does NOT attempt cryptanalysis, key-strength analysis, or protocol review, and it
    cannot judge an algorithm that is weak for reasons not on its list. The list is
    visible in one place so a reviewer can extend it.

DISCRIMINATION IS PROVEN, not assumed: `--self-test` runs the same scanner over synthetic
inputs that DO contain a weak crate and a weak call and asserts it reports them, and over
a clean synthetic tree and asserts it reports nothing. A gate that has never failed is a
gate nobody can trust, and this one finds nothing in the real tree today.
"""
from __future__ import annotations

import argparse
import json
import pathlib
import re
import sys
import tempfile

ROOT = pathlib.Path(".").resolve()
EVIDENCE = ROOT / ".agent/evidence/weak-crypto"

# One list, in one place, so a reviewer can extend it without reading the code.
WEAK_PRIMITIVES = [
    "md5", "sha1", "sha-1", "des", "3des", "tripledes", "rc4", "rc2", "blowfish",
    "ecb", "md4", "md2", "skipjack", "cast5",
]

# Code-shaped usage: a path, a type, a turbofish, a string literal naming a mode.
SOURCE_PATTERNS = [
    re.compile(r"\b(md5|md4|md2|sha1|rc4|rc2|blowfish|skipjack|cast5)\s*::"),
    re.compile(r"\b(Md5|Md4|Sha1|Rc4|Rc2|Blowfish|Des|TripleDes|Ecb)\b\s*::"),
    re.compile(r"\buse\s+[\w:]*\b(md5|md4|sha1|rc4|rc2|blowfish)\b"),
    re.compile(r"[\"'](?:aes|des|3des|rc4|rc2|bf|blowfish)[\w-]*(?:-|\b)ecb[\"']", re.IGNORECASE),
    re.compile(r"[\"'](?:des|3des|rc4|rc2|md5|sha1)[\"']\s*\b", re.IGNORECASE),
]

SOURCE_SUFFIXES = {".rs", ".ts", ".tsx", ".mjs", ".js", ".py", ".sh"}
SKIP_PARTS = {"node_modules", "target", "dist", ".git", "gen", "build"}

# Documented exceptions. Every entry needs a path fragment and a reason, and the reason
# is the point: an exception that cannot be justified in a sentence should not exist.
ALLOWLIST: list[dict[str, str]] = [
    {
        "path": "scripts/weak-crypto-scan.py",
        "reason": (
            "the scanner must name the primitives it looks for and must build a synthetic "
            "weak fixture to prove it discriminates; scanning itself without this exception "
            "flags its own pattern list and self-test string, which is a false positive and "
            "would make the gate permanently red"
        ),
    },
]


def scan_source(root: pathlib.Path) -> list[dict]:
    findings: list[dict] = []
    for base in ("crates", "apps", "packages", "scripts"):
        for path in (root / base).rglob("*"):
            if not path.is_file() or path.suffix not in SOURCE_SUFFIXES:
                continue
            if SKIP_PARTS & set(path.parts):
                continue
            rel = path.relative_to(root).as_posix()
            if any(entry["path"] in rel for entry in ALLOWLIST):
                continue
            try:
                text = path.read_text("utf-8")
            except (UnicodeDecodeError, OSError):
                continue
            for number, line in enumerate(text.splitlines(), 1):
                stripped = line.strip()
                if stripped.startswith("//") or stripped.startswith("#") or stripped.startswith("*"):
                    continue
                for pattern in SOURCE_PATTERNS:
                    if pattern.search(line):
                        findings.append({"kind": "source", "path": rel, "line": number,
                                         "text": stripped[:160], "pattern": pattern.pattern})
                        break
    return findings


def scan_cargo_lock(root: pathlib.Path) -> list[dict]:
    path = root / "Cargo.lock"
    if not path.exists():
        return []
    findings = []
    text = path.read_text("utf-8", errors="replace")
    for match in re.finditer(r'name\s*=\s*"([^"]+)"', text):
        name = match.group(1)
        # `-sys` crates and their transitive C libraries keep the same weak names; the
        # comparison is on the whole name so `des` does not match `desktop`.
        if name.lower() in {"md5", "md4", "md2", "sha1", "sha-1", "des", "rc4", "rc2",
                            "blowfish", "skipjack", "cast5", "tripledes", "des-sys"}:
            line = text[: match.start()].count("\n") + 1
            findings.append({"kind": "cargo-lock", "path": "Cargo.lock", "line": line,
                             "text": f'name = "{name}"'})
    return findings


def scan_pnpm_lock(root: pathlib.Path) -> list[dict]:
    path = root / "pnpm-lock.yaml"
    if not path.exists():
        return []
    findings = []
    text = path.read_text("utf-8", errors="replace")
    for number, line in enumerate(text.splitlines(), 1):
        # Package keys look like "  /md5@2.3.0:" or "  md5@2.3.0:".
        match = re.match(r"\s*/?([@\w.\-]+)@[\d]", line)
        if not match:
            continue
        package = match.group(1).lstrip("/").split("/")[-1].lower()
        if package in {"md5", "md4", "md2", "sha1", "sha-1", "des", "rc4", "rc2",
                       "blowfish", "skipjack", "cast5"}:
            findings.append({"kind": "pnpm-lock", "path": "pnpm-lock.yaml", "line": number,
                             "text": line.strip()[:160]})
    return findings


def scan(root: pathlib.Path) -> list[dict]:
    return scan_source(root) + scan_cargo_lock(root) + scan_pnpm_lock(root)


def self_test() -> int:
    """Prove the scanner detects what it claims to detect, and stays quiet otherwise."""
    with tempfile.TemporaryDirectory() as tmp:
        dirty = pathlib.Path(tmp) / "dirty"
        (dirty / "crates" / "demo" / "src").mkdir(parents=True)
        (dirty / "Cargo.lock").write_text(
            '[[package]]\nname = "md5"\nversion = "0.7.0"\n', encoding="utf-8")
        (dirty / "pnpm-lock.yaml").write_text("  /rc4@0.1.0:\n    resolution: {}\n", encoding="utf-8")
        (dirty / "crates/demo/src/lib.rs").write_text(
            "pub fn weak(data: &[u8]) -> String {\n    let d = md5::compute(data);\n"
            "    format!(\"{:x}\", d)\n}\n", encoding="utf-8")
        dirty_findings = scan(dirty)

        clean = pathlib.Path(tmp) / "clean"
        (clean / "crates" / "demo" / "src").mkdir(parents=True)
        (clean / "Cargo.lock").write_text(
            '[[package]]\nname = "sha2"\nversion = "0.10.0"\n', encoding="utf-8")
        (clean / "crates/demo/src/lib.rs").write_text(
            "pub fn strong(data: &[u8]) -> Vec<u8> {\n    // sha2 is the approved family\n"
            "    sha2::Sha256::digest(data).to_vec()\n}\n", encoding="utf-8")
        clean_findings = scan(clean)

        kinds = {f["kind"] for f in dirty_findings}
        ok = (
            {"cargo-lock", "pnpm-lock", "source"} <= kinds
            and not clean_findings
        )
        print(f"weak-crypto self-test: dirty tree -> {len(dirty_findings)} finding(s) in {sorted(kinds)}")
        print(f"weak-crypto self-test: clean tree -> {len(clean_findings)} finding(s)")
        if not ok:
            print("weak-crypto self-test: FAIL -- the scanner did not discriminate", file=sys.stderr)
            return 1
        print("weak-crypto self-test: ok (detects weak crates in both lockfiles and a weak call in source)")
        return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true", help="prove the scanner discriminates")
    parser.add_argument("--check", action="store_true", help="fail if the recorded report is stale")
    args = parser.parse_args()

    if args.self_test:
        return self_test()

    findings = scan(ROOT)
    report = {
        "gate": "weak-crypto-scan",
        "covers": ["GEN-058", "DOD-021"],
        "harness": "scripts/weak-crypto-scan.py",
        "primitives_watched": WEAK_PRIMITIVES,
        "scanned": ["first-party source (crates, apps, packages, scripts)",
                    "Cargo.lock", "pnpm-lock.yaml"],
        "allowlist": ALLOWLIST,
        "findings": findings,
        "verdict": "PASS" if not findings else "FAIL",
        "limits": (
            "detects named weak primitives in code use and in the locked graphs; it does not "
            "perform cryptanalysis, judge key strength or mode misuse beyond the named patterns, "
            "and cannot see a primitive that is weak for a reason not on its list"
        ),
    }
    if args.check:
        recorded = EVIDENCE / "report.json"
        if not recorded.exists():
            print("weak-crypto check: FAIL -- no recorded report", file=sys.stderr)
            return 1
        previous = json.loads(recorded.read_text(encoding="utf-8"))
        if previous.get("findings") != findings:
            print("weak-crypto check: FAIL -- findings changed since the report was written; "
                  "rerun python3 scripts/weak-crypto-scan.py", file=sys.stderr)
            return 1
        print(f"weak-crypto check: ok ({len(findings)} finding(s), unchanged)")
        return 0

    EVIDENCE.mkdir(parents=True, exist_ok=True)
    (EVIDENCE / "report.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    lines = [
        "# Weak-cryptography scan (GEN-058)",
        "",
        "Generated by `scripts/weak-crypto-scan.py`. Do not hand-edit.",
        "",
        f"- Verdict: **{report['verdict']}**",
        f"- Findings: **{len(findings)}**",
        f"- Primitives watched: {', '.join(WEAK_PRIMITIVES)}",
        f"- Scanned: {', '.join(report['scanned'])}",
        "",
        "## Findings",
        "",
    ]
    if findings:
        lines += ["| Kind | Location | Text |", "| --- | --- | --- |"]
        lines += [f"| {f['kind']} | `{f['path']}:{f['line']}` | `{f['text'][:100]}` |" for f in findings]
    else:
        lines.append("None: no weak primitive appears in first-party code use or in either locked graph.")
    lines += [
        "",
        "## Limits, stated rather than implied",
        "",
        report["limits"],
        "",
        "## Discrimination",
        "",
        "`python3 scripts/weak-crypto-scan.py --self-test` runs the same scanner over",
        "synthetic inputs that DO contain a weak crate in each lockfile and a weak call in",
        "source, and asserts it reports all three, then over a clean tree and asserts it",
        "reports nothing. A gate that finds nothing today has to prove it can find",
        "something.",
        "",
        "## Reproduce",
        "",
        "```",
        "python3 scripts/weak-crypto-scan.py",
        "python3 scripts/weak-crypto-scan.py --self-test",
        "python3 scripts/weak-crypto-scan.py --check",
        "```",
        "",
    ]
    (EVIDENCE / "STATUS.md").write_text("\n".join(lines), encoding="utf-8")
    if findings:
        print(f"weak-crypto-scan: FAIL -- {len(findings)} weak primitive(s) found", file=sys.stderr)
        for finding in findings[:10]:
            print(f"  {finding['path']}:{finding['line']} {finding['text'][:100]}", file=sys.stderr)
        return 1
    print("weak-crypto-scan: ok (no weak primitive in code use or in either locked graph)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
