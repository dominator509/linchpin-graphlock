#!/usr/bin/env python3
"""Compute the change-invalidation graph and evidence digests (DOD-040).

GraphLock context (DOD-040): "Any code, dependency, schema, configuration,
build, test-oracle, or artifact change invalidates and reruns every affected
downstream result." Required evidence: "Change invalidation graph, prior/new
epoch IDs, rerun list, and current evidence hashes." OR ELSE: "Affected PASS
statuses are revoked until rerun."

`.agent/verification/state/CHANGE_INVALIDATION_GRAPH.md` previously read
"Exact mapping is populated during implementation" -- i.e. it was never
populated, so no invalidation was being tracked at all.

This script derives the graph from the repository and emits:

  * .agent/verification/state/CHANGE_INVALIDATION_GRAPH.md  (human view)
  * .agent/verification/state/EPOCH.json                    (epoch identity)

Epoch identity is the digest of every tracked input that can invalidate
evidence: source trees, manifests, lockfiles, config, gates and test oracles.
When that digest changes, the epoch changes, and any PASS recorded against the
previous epoch is stale by definition.

Usage:
  python3 scripts/change-invalidation.py            # recompute and write
  python3 scripts/change-invalidation.py --check    # fail if epoch moved
"""
from __future__ import annotations

import hashlib
import json
import subprocess
import sys
from pathlib import Path

OUT_MD = Path(".agent/verification/state/CHANGE_INVALIDATION_GRAPH.md")
EPOCH = Path(".agent/verification/state/EPOCH.json")

# Which evidence classes each input class invalidates when it changes.
# Derived from the stage plan names under .agent/verification/stage-plans/.
INVALIDATION_RULES: dict[str, list[str]] = {
    "rust-source": [
        "V-005 clean build",
        "V-006 smoke",
        "V-007 sanity",
        "V-008 full functionality",
        "V-009 integration/concurrency",
        "V-012 regression/mutation",
    ],
    "rust-manifest": [
        "V-004 supply chain",
        "V-005 clean build",
        "V-020 exact artifact",
    ],
    "js-source": [
        "V-005 clean build",
        "V-008 full functionality",
        "V-015 usability/accessibility",
    ],
    "js-manifest": ["V-004 supply chain", "V-005 clean build"],
    "lockfile": ["V-004 supply chain", "V-005 clean build", "DOD-021 license gate"],
    # A gate-script change must rerun the gates themselves: editing a checker
    # can make it pass vacuously, so the harness and the build/test lanes are
    # all suspect until re-executed. The earlier value here was the prose
    # "all stages that invoke the changed gate", which maps to no command and
    # was therefore never actually run.
    "gate-script": [
        "V-000 harness validation",
        "V-005 clean build",
        "V-008 full functionality",
        "V-009 integration/concurrency",
    ],
    # A test-oracle change invalidates whatever the oracle guards. The registry
    # and manifest are the oracles for the whole suite, so the suite is rerun.
    "test-oracle": [
        "V-008 full functionality",
        "V-012 regression/mutation",
        "DOD-007 collection guard",
    ],
    "config": ["V-005 clean build", "V-011 configuration matrix"],
    "artifact-inputs": ["V-020 exact artifact", "V-021 final accounting"],
}

INPUT_GLOBS: dict[str, list[str]] = {
    "rust-source": ["crates/**/*.rs", "apps/desktop/src-tauri/src/**/*.rs"],
    "rust-manifest": [
        "Cargo.toml",
        "crates/*/Cargo.toml",
        "apps/desktop/src-tauri/Cargo.toml",
        "rust-toolchain.toml",
        "deny.toml",
    ],
    "js-source": [
        "apps/desktop/src/**/*.ts",
        "apps/desktop/src/**/*.tsx",
        "packages/*/src/**/*.ts",
        "apps/desktop/e2e/**/*.ts",
        "apps/desktop/tests/**/*.ts",
        "packages/*/tests/**/*.ts",
    ],
    "js-manifest": [
        "package.json",
        "apps/desktop/package.json",
        "packages/*/package.json",
        "apps/desktop/tsconfig.json",
        "packages/*/tsconfig.json",
    ],
    "lockfile": ["Cargo.lock", "pnpm-lock.yaml"],
    "gate-script": ["scripts/*.sh", "scripts/*.py"],
    "test-oracle": [
        ".agent/verification/state/TEST_COLLECTION_MANIFEST.json",
        ".agent/verification/MASTER_TEST_REGISTRY.csv",
        ".agent/verification/DOD_REGISTRY.csv",
    ],
    "config": [
        "apps/desktop/src-tauri/tauri.conf.json",
        ".gitattributes",
        ".prettierignore",
        "apps/desktop/vitest.config.ts",
        "apps/desktop/playwright.config.ts",
    ],
    # The ARTIFACT class deliberately does NOT hash build output.
    #
    # Hashing target/** made the epoch churn on every build, and every build was
    # itself a rerun obligation, so the check could never be satisfied: measured,
    # the epoch moved on each pass and RERUN_RECORD was permanently stale. That
    # is a self-defeating design, not a strict one.
    #
    # What actually invalidates downstream results is the SOURCE of the artifact,
    # not the bytes produced from it. The artifact's identity is pinned
    # separately and precisely by scripts/ship-gate.py (executable and MSI
    # SHA-256) and by .agent/evidence/artifact-e2e/STATUS.md, which binds
    # assertions to the exact digest. Those are the right places to detect a
    # changed artifact; the epoch tracks what would PRODUCE one.
    "artifact-inputs": [
        "apps/desktop/src-tauri/tauri.conf.json",
        "apps/desktop/src-tauri/Cargo.toml",
        "Cargo.toml",
    ],
}

EXCLUDED_PARTS = {"node_modules", "target", "dist", "build", ".git", "__pycache__"}


def digest_inputs() -> tuple[dict[str, dict[str, object]], str]:
    """Return per-class file digests and the overall epoch digest."""
    classes: dict[str, dict[str, object]] = {}
    overall = hashlib.sha256()

    for name, patterns in INPUT_GLOBS.items():
        files: dict[str, str] = {}
        for pattern in patterns:
            for path in sorted(Path(".").glob(pattern)):
                if not path.is_file():
                    continue
                if set(path.parts) & EXCLUDED_PARTS:
                    continue
                h = hashlib.sha256()
                with path.open("rb") as fh:
                    for chunk in iter(lambda: fh.read(1 << 20), b""):
                        h.update(chunk)
                files[str(path)] = h.hexdigest()
        classes[name] = {
            "files": files,
            "count": len(files),
            "rules": INVALIDATION_RULES.get(name, []),
        }
        for rel, digest in files.items():
            overall.update(f"{name}:{rel}:{digest}".encode())

    return classes, overall.hexdigest()


def git_head() -> str:
    return subprocess.run(
        ["git", "rev-parse", "--short", "HEAD"],
        capture_output=True,
        text=True,
        check=False,
    ).stdout.strip() or "UNKNOWN"


def main() -> int:
    check_only = "--check" in sys.argv
    classes, epoch = digest_inputs()
    head = git_head()

    prior = None
    if EPOCH.exists():
        try:
            prior = json.loads(EPOCH.read_text("utf-8"))
        except json.JSONDecodeError:
            prior = None

    changed_classes: list[str] = []
    if prior:
        for name, info in classes.items():
            if prior.get("classes", {}).get(name, {}).get("files") != info["files"]:
                changed_classes.append(name)

    if check_only:
        if not EPOCH.exists():
            print("change-invalidation check: FAIL (no epoch recorded)", file=sys.stderr)
            return 1
        recorded = prior.get("epoch_digest") if prior else None
        if recorded != epoch:
            print(
                "change-invalidation check: FAIL -- the epoch moved since it was "
                f"recorded (recorded {recorded}, computed {epoch}). Re-run "
                "scripts/change-invalidation.py and rerun the invalidated stages.",
                file=sys.stderr,
            )
            return 1
        print(
            f"change-invalidation check: ok (epoch {epoch[:16]}..., "
            f"{sum(c['count'] for c in classes.values())} inputs)"
        )
        return 0

    total = sum(int(c["count"]) for c in classes.values())
    lines = [
        "# Change Invalidation Graph",
        "",
        "Generated by `scripts/change-invalidation.py`. Do not hand-edit.",
        "",
        "DOD-040: any code, dependency, schema, configuration, build, test-oracle",
        "or artifact change invalidates and reruns every affected downstream",
        "result. Evidence recorded against a previous epoch is stale by",
        "definition and its PASS status is revoked until rerun.",
        "",
        "## Epoch identity",
        "",
        f"- Current epoch digest: `{epoch}`",
        f"- Candidate commit at generation: `{head}`",
        f"- Total tracked inputs: {total}",
    ]
    if prior:
        lines.append(f"- Previous epoch digest: `{prior.get('epoch_digest')}`")
        if changed_classes:
            lines.append(
                f"- **Changed input classes since that epoch: "
                f"{', '.join(changed_classes)}**"
            )
        else:
            lines.append("- Changed input classes since that epoch: none")
    else:
        lines.append("- Previous epoch digest: none (first recorded epoch)")

    lines += ["", "## Input classes and what a change invalidates", ""]
    for name, info in sorted(classes.items()):
        lines.append(f"### `{name}` — {info['count']} file(s)")
        lines.append("")
        rules = info["rules"] or ["unmapped"]
        for rule in rules:  # type: ignore[union-attr]
            lines.append(f"- invalidates: {rule}")
        lines.append("")
        files = info["files"]  # type: ignore[assignment]
        for rel, digest in sorted(files.items())[:5]:  # type: ignore[union-attr]
            lines.append(f"  - `{rel}` `{digest[:16]}…`")
        if len(files) > 5:  # type: ignore[arg-type]
            lines.append(f"  - … and {len(files) - 5} more")  # type: ignore[arg-type]
        lines.append("")

    lines += [
        "## Rerun obligations on epoch change",
        "",
        "When the epoch digest changes, the following must be rerun before any",
        "PASS recorded against the prior epoch may be reinstated:",
        "",
    ]
    for name in sorted(classes):
        for rule in INVALIDATION_RULES.get(name, []):
            lines.append(f"- (`{name}` changed) → rerun {rule}")
    lines += [
        "",
        "## Honest limitation",
        "",
        "This graph maps input classes to affected stages. It does NOT execute the",
        "reruns, and the stage entries above are stage *names* from",
        "`.agent/verification/stage-plans/`, several of which have no executable",
        "runner yet. Where a stage cannot be run, the correct status is",
        "BLOCKED_PREREQUISITE or NOT_RUN_BLOCKED_MATERIAL, never PASS.",
        "",
    ]

    OUT_MD.write_text("\n".join(lines), "utf-8")
    EPOCH.write_text(
        json.dumps(
            {
                "epoch_digest": epoch,
                "candidate_commit": head,
                "total_inputs": total,
                "classes": {
                    k: {
                        "count": v["count"],
                        "files": v["files"],
                        "rules": v["rules"],
                    }
                    for k, v in classes.items()
                },
            },
            indent=2,
            sort_keys=True,
        )
        + "\n",
        "utf-8",
    )
    print(f"wrote {OUT_MD}")
    print(f"wrote {EPOCH}")
    print(f"  epoch={epoch[:32]}... inputs={total}")
    if changed_classes:
        print(f"  CHANGED CLASSES: {', '.join(changed_classes)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
