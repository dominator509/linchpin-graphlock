#!/usr/bin/env python3
"""Architecture drift accounting (EP-010 M3, AGENTS.md section 6).

WHY THIS EXISTS. `ARCHITECTURE.md` section 145 says every node review must ask:

    "does this preserve the five truth boundaries, local confidentiality,
     layer/import law, evidence provenance, high-impact authorization,
     rebuildable derived state, terms-gated provider behavior, deterministic
     filing manifests, and exact-artifact proof? Any 'no' is architecture drift."

Nothing checked it. AGENTS.md section 6 requires architecture drift accounting
before node closure and release, and the repository had no such gate, so the
question was answered by prose in status files.

WHAT IS CHECKED MECHANICALLY (and can therefore fail):
  1. Truth boundaries -- the five statements in ARCHITECTURE.md section 5 must be
     transcribed verbatim into `crates/domain/src/scope.rs`, id for id. A softened
     or missing boundary fails.
  2. Import law -- ARCHITECTURE.md section 3: domain imports no Tauri/SQL/HTTP/OS/
     provider/MCP/UI package; application depends on domain ports and never on a
     concrete adapter; a declared workspace dependency that nothing references is a
     dead edge and fails.
  3. Local confidentiality -- production source must contain no non-loopback URL
     literal outside an explicit allowlist; a shipped default endpoint that is not
     loopback fails.

WHAT IS BOUND RATHER THAN RE-DERIVED (stated so it is not read as more than it is):
the remaining review items are each bound to the executed gate or symbol that
proves them, and this gate fails when a binding disappears. It does not re-run
those gates; their own lanes do that.

SELF-TEST: `--self-test` builds a fixture tree containing one violation of each
mechanical rule and requires every rule to catch its own. A drift gate that has
never failed is not evidence.

Usage:
  python3 scripts/architecture-drift.py            # check and write evidence
  python3 scripts/architecture-drift.py --check    # fail if the record is stale
  python3 scripts/architecture-drift.py --self-test
"""
from __future__ import annotations

import hashlib
import json
import re
import shutil
import sys
import tempfile
from pathlib import Path

EVIDENCE = Path(".agent/evidence/architecture-drift")
OUT_JSON = EVIDENCE / "REPORT.json"
OUT_MD = EVIDENCE / "STATUS.md"

# A literal URL in production source is acceptable when it is loopback (the
# product is local-first) or when it is a bare scheme used for validation rather
# than as a destination. Everything else needs an entry here WITH a reason.
URL_ALLOWLIST: dict[tuple[str, str], str] = {}

# Crates that are adapters in the sense of ARCHITECTURE.md section 3: they
# implement ports and may depend inward. Inward layers must never import them.
ADAPTER_CRATES = {
    "storage",
    "provider_transport",
    "mcp_hub",
    "platform_windows",
    "crash_reporter",
}
INWARD_CRATES = {"domain", "application"}

# ARCHITECTURE.md section 3, bullet 3, as a package-name set: "Domain imports no
# Tauri, SQL, HTTP, OS, provider, MCP, or UI packages."
DOMAIN_FORBIDDEN = ADAPTER_CRATES | {
    "tauri",
    "tauri-build",
    "rusqlite",
    "libsqlite3-sys",
    "sqlx",
    "diesel",
    "reqwest",
    "hyper",
    "ureq",
    "http",
    "http-body",
    "windows",
    "winapi",
    "winreg",
}

PRODUCTION_SOURCE_ROOTS = ["crates", "apps/desktop/src-tauri/src", "packages"]

# Review items this gate binds instead of re-deriving: (item, files that must
# exist, symbols that must appear in a named file, the lane that proves it).
BINDINGS: list[dict] = [
    {
        "item": "evidence provenance",
        "files": [".agent/evidence/EVIDENCE_HASHES.md", "scripts/generate-evidence-index.py"],
        "symbols": [("scripts/generate-evidence-index.py", "DOD_STATUS.jsonl")],
        "lane": "verify.sh: evidence index currency",
    },
    {
        "item": "high-impact authorization",
        "files": ["crates/mcp_hub/src/lib.rs", "crates/domain/src/scope.rs"],
        "symbols": [
            ("crates/mcp_hub/src/lib.rs", "pub fn check_capability"),
            ("crates/mcp_hub/src/lib.rs", "unwrap_or(false)"),
            ("crates/domain/src/scope.rs", "capability grants are explicit and deny by default"),
        ],
        "lane": "cargo test -p mcp_hub (deny-by-default grants; unknown client has no capabilities)",
    },
    {
        "item": "rebuildable derived state",
        "files": [".agent/evidence/recovery-drill/report.json", "crates/storage/Cargo.toml"],
        "symbols": [("crates/storage/Cargo.toml", "tantivy")],
        "lane": "cargo test -p linchpin-desktop --test recovery_drill (canonical state restored and reconciled)",
    },
    {
        "item": "terms-gated provider behavior",
        "files": ["apps/desktop/src-tauri/src/commands.rs"],
        "symbols": [
            ("apps/desktop/src-tauri/src/commands.rs", "pub fn provider_status"),
            ("apps/desktop/src-tauri/src/commands.rs", "terms review required"),
            (
                "apps/desktop/src-tauri/src/commands.rs",
                "fn test_provider_status_local_lane_is_measured_and_others_are_not_configured",
            ),
        ],
        "lane": "cargo test -p linchpin-desktop (provider lanes: local measured, the rest not configured)",
    },
    {
        "item": "deterministic filing manifests",
        "files": ["crates/patent/src/lib.rs"],
        "symbols": [
            ("crates/patent/src/lib.rs", "fn build_manifest"),
            ("crates/patent/src/lib.rs", "fn test_manifest_digest_depends_on_both_inputs"),
        ],
        "lane": "cargo test -p patent",
    },
    {
        "item": "exact-artifact proof",
        "files": [".agent/evidence/artifact-e2e/STATUS.md", "scripts/artifact-e2e.sh"],
        "symbols": [(".agent/verification/state/RUN_MANIFEST.json", "executable_sha256")],
        "lane": "scripts/artifact-e2e.sh via scripts/test-e2e.sh lane 2",
    },
]


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8", errors="replace") if path.exists() else ""


def strip_test_modules(text: str) -> str:
    """Drop everything from the first `#[cfg(test)]` module onwards.

    Test code is not production code: a URL literal in a test fixture is not a
    shipped default, and counting it would make the confidentiality rule noise.
    """
    marker = re.search(r"#\[cfg\(test\)\]", text)
    return text[: marker.start()] if marker else text


def cargo_dependencies(path: Path) -> dict[str, list[str]]:
    """Production vs dev dependencies from a Cargo.toml (production = [dependencies])."""
    sections: dict[str, list[str]] = {"dependencies": [], "dev-dependencies": []}
    current = None
    for raw in read(path).splitlines():
        line = raw.split("#", 1)[0].rstrip()
        if line.startswith("["):
            header = line.strip("[]").strip()
            if header == "dependencies":
                current = "dependencies"
            elif header in ("dev-dependencies", "build-dependencies"):
                current = "dev-dependencies"
            else:
                current = None
            continue
        if current and "=" in line and not line.startswith(" "):
            name = line.split("=", 1)[0].strip()
            if name:
                sections[current].append(name)
    return sections


def check_truth_boundaries(root: Path) -> list[str]:
    findings: list[str] = []
    architecture = read(root / "ARCHITECTURE.md")
    scope = read(root / "crates/domain/src/scope.rs")
    section = architecture.split("## 5. Five truth boundaries", 1)[-1].split("\n## ", 1)[0]
    declared = re.findall(r"^(TB-\d) (.+)$", section, re.M)
    if len(declared) != 5:
        findings.append(f"ARCHITECTURE.md section 5 declares {len(declared)} truth boundaries, expected 5")
    code = re.findall(
        r'id: "(TB-\d)",\s*\n\s*statement: "([^"]+)"',
        scope,
    )
    if len(code) != 5:
        findings.append(f"crates/domain/src/scope.rs declares {len(code)} truth boundaries, expected 5")
    arch_map = {tb: " ".join(text.split()) for tb, text in declared}
    code_map = {tb: " ".join(text.split()) for tb, text in code}

    def normalize(text: str) -> str:
        return " ".join(text.replace("`", "").split()).strip()

    for tb, statement in arch_map.items():
        if tb not in code_map:
            findings.append(f"{tb} is declared in ARCHITECTURE.md but not in scope.rs")
            continue
        if normalize(statement) != normalize(code_map[tb]):
            findings.append(
                f"{tb} wording drifted from ARCHITECTURE.md: architecture={normalize(statement)!r} code={normalize(code_map[tb])!r}"
            )
    for tb in code_map:
        if tb not in arch_map:
            findings.append(f"{tb} exists in scope.rs but not in ARCHITECTURE.md section 5")
    return findings


def check_import_law(root: Path) -> list[str]:
    findings: list[str] = []
    crates = sorted(p for p in (root / "crates").glob("*/Cargo.toml"))
    if not crates:
        findings.append("no crate manifests found under crates/")
        return findings
    names = {path.parent.name: path for path in crates}
    for name, path in names.items():
        deps = cargo_dependencies(path)
        production = set(deps["dependencies"])
        if name in INWARD_CRATES:
            offenders = sorted(production & ADAPTER_CRATES)
            if offenders:
                findings.append(
                    f"{name} depends on concrete adapter crate(s) {offenders} in [dependencies] "
                    "(ARCHITECTURE.md section 3: inward layers never import adapters)"
                )
        if name == "domain":
            offenders = sorted(production & DOMAIN_FORBIDDEN)
            if offenders:
                findings.append(
                    f"domain depends on {offenders}, which ARCHITECTURE.md section 3 forbids "
                    "(no Tauri, SQL, HTTP, OS, provider, MCP or UI packages)"
                )
        # Dead edge: a declared workspace dependency that the crate never names.
        source = "\n".join(
            read(p) for p in sorted((path.parent / "src").rglob("*.rs"))
        )
        for dep in production:
            if dep not in names:
                continue
            if not re.search(rf"\b{re.escape(dep)}\s*::", source):
                findings.append(
                    f"{name} declares workspace dependency '{dep}' in [dependencies] but never references it in src/ "
                    "(dead dependency edge)"
                )
    return findings


def iter_production_sources(root: Path):
    for rel in PRODUCTION_SOURCE_ROOTS:
        base = root / rel
        if not base.exists():
            continue
        for path in sorted(base.rglob("*")):
            if not path.is_file():
                continue
            if any(part in {"target", "node_modules", "dist", "gen"} for part in path.parts):
                continue
            # Integration tests, benches and examples are not production source.
            # Measured false positive before this rule existed: `crates/*/tests/*.rs`
            # URL literals were reported as shipped defaults.
            if any(part in {"tests", "benches", "examples", "fixtures"} for part in path.parts):
                continue
            if path.suffix not in {".rs", ".ts", ".tsx", ".js", ".jsx"}:
                continue
            if ".test." in path.name or ".spec." in path.name:
                continue
            if path.name.endswith(".d.ts"):
                continue
            yield path


URL_RE = re.compile(r"https?://[^\s\"'`)]*")


def check_confidentiality(root: Path) -> list[str]:
    findings: list[str] = []
    for path in iter_production_sources(root):
        rel = str(path.relative_to(root)).replace("\\", "/")
        text = read(path)
        if path.suffix == ".rs":
            text = strip_test_modules(text)
        for match in URL_RE.finditer(text):
            literal = match.group(0)
            host = literal.split("//", 1)[1].split("/", 1)[0]
            if literal in {"http://", "https://"}:
                continue  # a bare scheme, used for validation rather than as a destination
            if host.startswith(("127.0.0.1", "localhost", "[::1]", "::1")):
                continue
            if (rel, literal) in URL_ALLOWLIST:
                continue
            findings.append(
                f"{rel}: non-loopback URL literal {literal!r} in production source "
                "(local-first product: shipped defaults must be loopback)"
            )
    return findings


def check_bindings(root: Path) -> list[str]:
    findings: list[str] = []
    for binding in BINDINGS:
        for rel in binding["files"]:
            if not (root / rel).exists():
                findings.append(f"{binding['item']}: bound evidence {rel} does not exist")
        for rel, symbol in binding["symbols"]:
            if symbol not in read(root / rel):
                findings.append(f"{binding['item']}: symbol {symbol!r} not found in {rel}")
    return findings


def run_checks(root: Path) -> dict:
    results = {
        "truth_boundaries": check_truth_boundaries(root),
        "import_law": check_import_law(root),
        "local_confidentiality": check_confidentiality(root),
        "bindings": check_bindings(root),
    }
    findings = [f"{name}: {detail}" for name, items in results.items() for detail in items]
    return {"checks": results, "findings": findings}


def self_test() -> int:
    """Prove every mechanical rule catches a planted violation (and passes clean)."""
    failures: list[str] = []
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        (root / "ARCHITECTURE.md").write_text(
            "## 5. Five truth boundaries\n"
            "TB-1 Opportunity quality != patentability.\nTB-2 Patentability != FTO.\n"
            "TB-3 Draft != filing.\nTB-4 Patent rights != commercialization.\n"
            "TB-5 AI assistance != human inventorship.\n\n## 6. Next\n",
            encoding="utf-8",
        )
        scope = root / "crates/domain/src"
        scope.mkdir(parents=True)
        # Only four boundaries transcribed: TB-5 is missing.
        (scope / "scope.rs").write_text(
            'pub const TRUTH_BOUNDARIES: [TruthBoundary; 4] = [\n'
            '    TruthBoundary {\n        id: "TB-1",\n        statement: "Opportunity quality != patentability.",\n    },\n'
            '    TruthBoundary {\n        id: "TB-2",\n        statement: "Patentability != FTO.",\n    },\n'
            '    TruthBoundary {\n        id: "TB-3",\n        statement: "Draft != filing.",\n    },\n'
            '    TruthBoundary {\n        id: "TB-4",\n        statement: "Patent rights != commercialization.",\n    },\n];\n',
            encoding="utf-8",
        )
        # A softened boundary as well as a missing one: TB-2 wording changed.
        (scope / "scope.rs").write_text(
            (scope / "scope.rs").read_text(encoding="utf-8").replace(
                '"Patentability != FTO."', '"Patentability is basically FTO."'
            ),
            encoding="utf-8",
        )
        domain_src = root / "crates/domain/src/lib.rs"
        domain_src.write_text("pub fn ok() {}\n", encoding="utf-8")
        (root / "crates/domain/Cargo.toml").write_text(
            '[package]\nname = "domain"\n\n[dependencies]\nreqwest = "0.12"\n', encoding="utf-8"
        )
        app = root / "crates/application"
        (app / "src").mkdir(parents=True)
        (app / "src/lib.rs").write_text("pub fn ok() {}\n", encoding="utf-8")
        (app / "Cargo.toml").write_text(
            '[package]\nname = "application"\n\n[dependencies]\nstorage = { path = "../storage" }\n',
            encoding="utf-8",
        )
        (root / "crates/storage").mkdir(parents=True)
        (root / "crates/storage/Cargo.toml").write_text('[package]\nname = "storage"\n', encoding="utf-8")
        (root / "crates/storage/src").mkdir(parents=True, exist_ok=True)
        (root / "crates/storage/src/lib.rs").write_text("pub fn ok() {}\n", encoding="utf-8")
        # Production source with an external endpoint.
        (app / "src/client.rs").write_text(
            'pub const DEFAULT: &str = "https://api.example.com/v1";\n', encoding="utf-8"
        )

        result = run_checks(root)
        checks = result["checks"]
        if not checks["truth_boundaries"]:
            failures.append("truth-boundary check did not catch a missing and a reworded boundary")
        if not any("reqwest" in f for f in checks["import_law"]):
            failures.append("import-law check did not catch a forbidden domain dependency")
        if not any("application depends on concrete adapter" in f for f in checks["import_law"]):
            failures.append("import-law check did not catch application -> storage")
        if not any("dead dependency edge" in f for f in checks["import_law"]):
            failures.append("import-law check did not catch the dead dependency edge")
        if not any("api.example.com" in f for f in checks["local_confidentiality"]):
            failures.append("confidentiality check did not catch a non-loopback default URL")
        if not checks["bindings"]:
            failures.append("binding check did not catch missing bound evidence")

        # The same rules must pass on a clean fixture: loopback and transcribed text.
        clean = root / "clean"
        shutil.copytree(root / "crates", clean / "crates")
        (clean / "ARCHITECTURE.md").write_text((root / "ARCHITECTURE.md").read_text(encoding="utf-8"), encoding="utf-8")
        (clean / "crates/domain/Cargo.toml").write_text('[package]\nname = "domain"\n', encoding="utf-8")
        (clean / "crates/application/Cargo.toml").write_text('[package]\nname = "application"\n', encoding="utf-8")
        (clean / "crates/application/src/client.rs").write_text(
            'pub const DEFAULT: &str = "http://127.0.0.1:11434";\n', encoding="utf-8"
        )
        (clean / "crates/domain/src/scope.rs").write_text(
            "".join(
                f'    TruthBoundary {{\n        id: "{tb}",\n        statement: "{text}",\n    }},\n'
                for tb, text in [
                    ("TB-1", "Opportunity quality != patentability."),
                    ("TB-2", "Patentability != FTO."),
                    ("TB-3", "Draft != filing."),
                    ("TB-4", "Patent rights != commercialization."),
                    ("TB-5", "AI assistance != human inventorship."),
                ]
            ),
            encoding="utf-8",
        )
        clean_result = run_checks(clean)
        for name in ("truth_boundaries", "import_law", "local_confidentiality"):
            if clean_result["checks"][name]:
                failures.append(f"{name} check reported a finding on a clean fixture: {clean_result['checks'][name]}")

    if failures:
        print("architecture-drift: SELF-TEST FAILED", file=sys.stderr)
        for failure in failures:
            print(f"  {failure}", file=sys.stderr)
        return 1
    print("architecture-drift: self-test ok (every rule caught its planted violation; clean fixture passes)")
    return 0


def digest_of(payload: dict) -> str:
    return hashlib.sha256(json.dumps(payload, sort_keys=True).encode("utf-8")).hexdigest()


def main() -> int:
    if "--self-test" in sys.argv:
        return self_test()

    root = Path(".").resolve()
    result = run_checks(root)
    inputs = {
        rel: hashlib.sha256((root / rel).read_bytes()).hexdigest()
        for rel in [
            "ARCHITECTURE.md",
            "crates/domain/src/scope.rs",
            "crates/application/Cargo.toml",
            "crates/domain/Cargo.toml",
        ]
        if (root / rel).exists()
    }
    record = {
        "gate": "architecture-drift",
        "covers": ["EP-010 M3", "AGENTS.md section 6", "ARCHITECTURE.md section 145"],
        "harness": "scripts/architecture-drift.py",
        "checks": result["checks"],
        "findings": result["findings"],
        "bound_items": [
            {"item": b["item"], "files": b["files"], "lane": b["lane"]} for b in BINDINGS
        ],
        "inputs": inputs,
        "verdict": "PASS" if not result["findings"] else "DRIFT",
    }
    record["digest"] = digest_of({k: v for k, v in record.items() if k != "digest"})

    if "--check" in sys.argv:
        if not OUT_JSON.exists():
            print("architecture-drift: FAIL -- no drift record exists", file=sys.stderr)
            return 1
        existing = json.loads(OUT_JSON.read_text(encoding="utf-8"))
        if existing.get("inputs") != inputs:
            print(
                "architecture-drift: FAIL -- the record is stale against ARCHITECTURE.md, scope.rs or the crate manifests",
                file=sys.stderr,
            )
            print("  remedy: python3 scripts/architecture-drift.py", file=sys.stderr)
            return 1
        print("architecture-drift: record is current")
        return 0

    EVIDENCE.mkdir(parents=True, exist_ok=True)
    OUT_JSON.write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")

    lines = [
        "# Architecture drift accounting",
        "",
        "Generated by `scripts/architecture-drift.py`. Do not hand-edit.",
        "",
        f"- Verdict: **{record['verdict']}**",
        f"- Mechanical checks: {', '.join(sorted(result['checks']))}",
        "",
        "## Checked against the tree, not asserted",
        "",
        "| Rule | Source | Result |",
        "| --- | --- | --- |",
    ]
    for name, items in sorted(result["checks"].items()):
        verdict = "no findings" if not items else f"{len(items)} finding(s)"
        lines.append(f"| {name} | {'ARCHITECTURE.md section 5'} | {verdict} |" if name == "truth_boundaries" else f"| {name} | see harness | {verdict} |")
    lines += ["", "## Bound to executed gates (existence-verified, not re-run here)", ""]
    for binding in record["bound_items"]:
        lines.append(f"- **{binding['item']}** — lane: {binding['lane']}")
        for rel in binding["files"]:
            lines.append(f"  - `{rel}`")
    if result["findings"]:
        lines += ["", "## Findings", ""] + [f"- {finding}" for finding in result["findings"]]
    lines.append("")
    OUT_MD.write_text("\n".join(lines), encoding="utf-8")

    if result["findings"]:
        print("architecture-drift: DRIFT", file=sys.stderr)
        for finding in result["findings"]:
            print(f"  {finding}", file=sys.stderr)
        return 1
    print(
        f"architecture-drift: ok -- {len(result['checks'])} check groups, no drift; "
        f"{len(BINDINGS)} review items bound to executed gates"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
