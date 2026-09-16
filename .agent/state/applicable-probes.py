#!/usr/bin/env python3
"""Architecture probes that ground the NOT_RUN decisions for four registry IDs.

WHY THIS EXISTS. Four APPLICABLE registry IDs cannot be executed here because the
material they test does not exist in this architecture (authorization surface,
sandbox isolation, pre-commit hooks, hardware key store). Saying "material is
required" without NAMING the material is the blanket excuse this round exists to
remove, and naming it from memory would be an unverified claim. These probes
measure the absence (and, as positive controls, the presence of the nearest real
control) from the repository itself, so each NOT_RUN row can cite a measurement.

Every probe prints a machine-checkable `key: value` line plus its hits, so the
evidence documents can assert on the measured value instead of restating prose.
"""
import pathlib
import re
import sys

ROOT = pathlib.Path(".").resolve()
FIRST_PARTY = ["crates", "apps", "packages"]
SKIP_DIRS = {"target", "node_modules", "dist", ".git", "gen", "build"}


def sources() -> list[pathlib.Path]:
    out = []
    for base in FIRST_PARTY:
        for path in (ROOT / base).rglob("*"):
            if path.suffix not in {".rs", ".ts", ".tsx", ".js", ".mjs"}:
                continue
            if any(part in SKIP_DIRS for part in path.parts):
                continue
            out.append(path)
    return sorted(out)


def probe(name: str, pattern: str, limit: int = 8) -> list[str]:
    rx = re.compile(pattern, re.IGNORECASE)
    hits = []
    for path in sources():
        try:
            text = path.read_text(encoding="utf-8")
        except (UnicodeDecodeError, OSError):
            continue
        for num, line in enumerate(text.splitlines(), 1):
            if rx.search(line):
                rel = path.relative_to(ROOT).as_posix()
                hits.append(f"  {rel}:{num}: {line.strip()[:110]}")
    print(f"{name}_hits: {len(hits)}")
    for hit in hits[:limit]:
        print(hit)
    if len(hits) > limit:
        print(f"  ... {len(hits) - limit} more")
    return hits


def main() -> int:
    print("=== probe: authorization / identity surface (first-party source)")
    probe("auth_surface", r"\b(fn\s+)?(login|logout|authenticate|authorization_header|session_token|is_admin|role_based_access)\b")
    print("=== probe: nearest real access control (positive control)")
    probe("mcp_capability_grant", r"check_mcp_capability|capability_granted|CapabilityDenied")
    print("=== probe: disclosure firewall (positive control)")
    probe("disclosure_firewall", r"disclosure")

    print("=== probe: sandbox / isolation primitives")
    probe("sandbox_primitive", r"\b(CreateJobObject|AppContainer|sandbox|seccomp|namespace_isolation)\b")
    for name in ["Dockerfile", "docker-compose.yml", "docker-compose.yaml", ".devcontainer", "k8s", "kustomization.yaml"]:
        print(f"sandbox_artifact[{name}]: {(ROOT / name).exists()}")

    print("=== probe: git hooks and hook frameworks")
    hooks = ROOT / ".git/hooks"
    active = []
    if hooks.is_dir():
        active = sorted(p.name for p in hooks.iterdir() if p.is_file() and not p.name.endswith(".sample"))
    print(f"git_hooks_active: {len(active)} {active}")
    for name in [".husky", "lefthook.yml", ".pre-commit-config.yaml", ".pre-commit-config.yml"]:
        print(f"hook_framework[{name}]: {(ROOT / name).exists()}")
    pkg = ROOT / "package.json"
    hooks_field = "husky" in pkg.read_text(encoding="utf-8") if pkg.exists() else False
    print(f"root_package_husky_field: {hooks_field}")

    print("=== probe: hardware key store / HSM surface")
    probe("hardware_key_store", r"\b(dpapi|ncrypt|CryptAcquireContext|pkcs11|hsm|tpm2?|cng)\b")
    print("keyring_impl: " + ", ".join(
        p.relative_to(ROOT).as_posix() for p in sources() if "keyring" in p.name.lower()
    ))

    print("=== probe: loopback-only transport guard (positive control)")
    probe("loopback_guard", r"loopback")

    print("=== probe: hosted CI configuration")
    for name in [
        ".github/workflows",
        ".gitlab-ci.yml",
        "azure-pipelines.yml",
        "Jenkinsfile",
        ".circleci/config.yml",
        ".woodpecker.yml",
    ]:
        print(f"ci_config[{name}]: {(ROOT / name).exists()}")

    print("=== probe: executed-test inventory by concern")
    # Every test name below is printed from the source that defines it, so an
    # evidence document cannot cite a test that does not exist. Execution of these
    # tests is recorded separately by the coverage gate and the collection guard.
    rust_tests: dict[str, list[str]] = {}
    js_tests: list[str] = []
    for path in sources():
        try:
            text = path.read_text(encoding="utf-8")
        except (UnicodeDecodeError, OSError):
            continue
        rel = path.relative_to(ROOT).as_posix()
        for num, line in enumerate(text.splitlines(), 1):
            m = re.match(r"\s*(?:async\s+)?fn\s+([a-z0-9_]+)\s*\(", line)
            if m and path.suffix == ".rs":
                rust_tests.setdefault(m.group(1), []).append(f"{rel}:{num}")
            m = re.match(r"\s*(?:test|it)\(\s*[\"'](.+?)[\"']", line)
            if m and path.suffix in {".ts", ".tsx", ".mjs", ".js"}:
                js_tests.append(f"{rel}:{num}: {m.group(1)[:90]}")
    for concern, words in {
        "redaction": ["redact", "secret", "credential"],
        "traversal": ["traversal", "sanitize", "archive"],
        "validation": ["reject", "invalid", "empty", "unknown"],
        "idempotency": ["idempot", "replay", "keyed", "duplicate"],
        "concurrency": ["concurrent", "locked", "busy"],
        "migration": ["migration", "schema"],
        "diagnostics": ["diagnostic", "alert", "trace", "health"],
        "backup_restore": ["backup", "restore", "quarantine", "recovery"],
        "mcp": ["mcp", "capability", "disclosure"],
        "provider": ["provider", "loopback", "inference"],
        "supply_chain": ["sbom", "license", "deny", "lock"],
        "truth_boundary": ["claim", "boundary", "scope"],
    }.items():
        hits = sorted(
            f"{name} @ {loc[0]}" for name, loc in rust_tests.items()
            if any(w in name for w in words)
        )
        print(f"concern[{concern}]_tests: {len(hits)}")
        for hit in hits[:6]:
            print(f"  {hit}")
    print(f"js_tests_declared: {len(js_tests)}")
    for line in js_tests[:10]:
        print(f"  {line}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
