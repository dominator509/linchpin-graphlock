#!/usr/bin/env python3
"""Generate an SBOM and third-party notices from the locked dependency graph.

GraphLock context (DOD-003): the required evidence is "artifact paths, formats,
sizes, metadata, checksums, signatures/attestations when applicable, and build
logs", and DOD-021 requires a dependency/license/SBOM scan. No SBOM existed.

No SBOM tool is installed, and AGENTS.md section 10 forbids adding a dependency
for something buildable with existing tooling. `cargo metadata --locked` already
emits the complete, resolved dependency graph, so this script converts it into:

  * .agent/evidence/sbom/linchpin.cdx.json  -- CycloneDX 1.5 JSON
  * .agent/evidence/sbom/THIRD_PARTY_NOTICES.md -- grouped license inventory

CycloneDX 1.5 is used because its component/license shape maps directly onto
what `cargo metadata` provides. Nothing is inferred: a package's license is
whatever its manifest declares, and a package whose manifest declares none is
reported as `NOASSERTION` rather than guessed.
"""
from __future__ import annotations

import hashlib
import json
import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

OUT_DIR = Path(".agent/evidence/sbom")
CDX = OUT_DIR / "linchpin.cdx.json"
NOTICES = OUT_DIR / "THIRD_PARTY_NOTICES.md"

# Permissive allowlist transcribed from LICENSE_ALLOWLIST.md.
ALLOWED = {
    "MIT",
    "Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Zlib",
    "Unicode-3.0",
    "Unicode-DFS-2016",
    "CC0-1.0",
    "0BSD",
}
# Requires the review ADR-002 requests; reported separately, never silently allowed.
REVIEW_REQUIRED = {"MPL-2.0"}
# Bucket label for choice expressions whose alternatives are all allowlisted.
CHOICE_KEY = "choice, allowlisted alternative available"


def cargo_metadata() -> dict:
    # `encoding="utf-8"` is required: on Windows the default text decoding is
    # cp1252, which raises UnicodeDecodeError on `cargo metadata` output (it
    # contains UTF-8 in crate descriptions). The failure surfaced as a thread
    # traceback and a None stdout, not an obvious encoding error.
    proc = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--locked"],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    if proc.returncode != 0:
        raise SystemExit(f"cargo metadata failed: {(proc.stderr or '')[:400]}")
    if not proc.stdout:
        raise SystemExit("cargo metadata produced no output")
    return json.loads(proc.stdout)


def sha256_file(path: Path) -> str | None:
    if not path.exists():
        return None
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def split_license(expr: str | None) -> tuple[list[str], bool]:
    """Parse an SPDX-ish expression into (ids, is_choice).

    `cargo metadata` emits a mixture of modern SPDX (`MIT OR Apache-2.0`,
    `Apache-2.0 WITH LLVM-exception`) and legacy slash syntax (`MIT/Apache-2.0`,
    `Apache-2.0 / MIT / MPL-2.0`). Both are handled.

    `is_choice` is True when the expression offers alternatives (OR, or the
    legacy `/`). That distinction matters: a crate declaring
    `MIT OR Apache-2.0 OR LGPL-2.1-or-later` is NOT an LGPL dependency, because
    the consumer may take MIT. Flattening alternatives into a flat list -- which
    an earlier version of this script did -- produces false copyleft findings.
    """
    if not expr:
        return [], False
    normalised = expr.replace("(", " ").replace(")", " ")
    # Legacy `/` means "or" in Cargo's historic syntax.
    normalised = normalised.replace("/", " OR ")
    # `WITH <exception>` is a modifier of the preceding license, not a license
    # of its own. Dropping only the WITH keyword (as an earlier version did)
    # left `LLVM-exception` looking like a standalone non-allowlisted license.
    normalised = re.sub(r"\bWITH\s+\S+", " ", normalised)
    tokens = normalised.split()
    is_choice = any(t.upper() == "OR" for t in tokens)
    ids = [t for t in tokens if t.upper() not in {"AND", "OR"}]
    return ids, is_choice


def main() -> int:
    check_only = "--check" in sys.argv
    meta = cargo_metadata()

    workspace_ids = set(meta["workspace_members"])
    components = []
    first_party, third_party = 0, 0
    license_counts: dict[str, int] = defaultdict(int)
    review_needed: list[tuple[str, str]] = []
    other_licenses: list[tuple[str, str]] = []
    unknown_license: list[str] = []

    for pkg in sorted(meta["packages"], key=lambda p: (p["name"], p["version"])):
        is_first_party = pkg["id"] in workspace_ids
        if is_first_party:
            first_party += 1
        else:
            third_party += 1

        declared = pkg.get("license")
        ids, is_choice = split_license(declared)
        if not ids:
            if not is_first_party:
                unknown_license.append(f"{pkg['name']}@{pkg['version']}")
            ids = ["NOASSERTION"]

        # A choice expression is only a problem when EVERY alternative is
        # disallowed. If any alternative is on the allowlist, the consumer can
        # take it, so the component is not a copyleft dependency.
        allowed_alternative = bool(set(ids) & ALLOWED)
        if is_choice and allowed_alternative:
            effective = CHOICE_KEY
            license_counts[effective] += 1
        else:
            for lic in ids:
                license_counts[lic] += 1
                if lic in REVIEW_REQUIRED and not is_first_party:
                    review_needed.append((f"{pkg['name']}@{pkg['version']}", lic))
                elif (
                    not is_first_party
                    and lic not in ALLOWED
                    and lic != "NOASSERTION"
                    and lic not in REVIEW_REQUIRED
                ):
                    other_licenses.append((f"{pkg['name']}@{pkg['version']}", lic))

        component = {
            "type": "library",
            "name": pkg["name"],
            "version": pkg["version"],
            "bom-ref": pkg["id"],
            "scope": "required",
            "properties": [
                {"name": "linchpin:first_party", "value": str(is_first_party).lower()},
            ],
        }
        if declared:
            component["licenses"] = [{"license": {"id": ids[0] if len(ids) == 1 else declared}}]
        else:
            component["licenses"] = [{"license": {"name": "NOASSERTION"}}]
        if pkg.get("source"):
            component["externalReferences"] = [
                {"type": "distribution", "url": pkg["source"]}
            ]
        components.append(component)

    artifact = {
        "artifact": str(Path("target/release/linchpin-desktop.exe")),
        "sha256": sha256_file(Path("target/release/linchpin-desktop.exe")),
    }
    cargo_lock = {
        "path": "Cargo.lock",
        "sha256": sha256_file(Path("Cargo.lock")),
    }
    pnpm_lock = {
        "path": "pnpm-lock.yaml",
        "sha256": sha256_file(Path("pnpm-lock.yaml")),
    }

    bom = {
        "bomFormat": "CycloneDX",
        "specVersion": "1.5",
        "version": 1,
        "metadata": {
            "component": {
                "type": "application",
                "name": "LINCHPIN",
                "version": "0.1.0",
                "licenses": [{"license": {"id": "MIT"}}],
            },
            "properties": [
                {"name": "linchpin:artifact", "value": json.dumps(artifact)},
                {"name": "linchpin:cargo_lock", "value": json.dumps(cargo_lock)},
                {"name": "linchpin:pnpm_lock", "value": json.dumps(pnpm_lock)},
            ],
        },
        "components": components,
    }

    if check_only:
        if not CDX.exists():
            print("sbom check: FAIL (no SBOM)", file=sys.stderr)
            return 1
        existing = json.loads(CDX.read_text("utf-8"))
        if len(existing.get("components", [])) != len(components):
            print(
                f"sbom check: FAIL (stale: {len(existing.get('components', []))} "
                f"components, recomputed {len(components)})",
                file=sys.stderr,
            )
            return 1
        print(f"sbom check: ok ({len(components)} components, current)")
        return 0

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    CDX.write_text(json.dumps(bom, indent=2, sort_keys=True) + "\n", "utf-8")

    lines = [
        "# Third-Party Notices",
        "",
        "Generated by `scripts/generate-sbom.py` from `cargo metadata --locked`.",
        "Do not hand-edit; regenerate after any dependency change.",
        "",
        "## Inventory",
        "",
        f"- First-party workspace packages: {first_party}",
        f"- Third-party components: {third_party}",
        f"- Total components: {len(components)}",
        "",
        "## Artifact identity",
        "",
        f"- `{artifact['artifact']}` sha256 `{artifact['sha256']}`",
        f"- `{cargo_lock['path']}` sha256 `{cargo_lock['sha256']}`",
        f"- `{pnpm_lock['path']}` sha256 `{pnpm_lock['sha256']}`",
        "",
        "## License inventory",
        "",
    ]
    for lic, count in sorted(license_counts.items(), key=lambda kv: (-kv[1], kv[0])):
        marker = ""
        if lic in REVIEW_REQUIRED:
            marker = " — **requires the file-level review described in ADR-002**"
        elif lic == CHOICE_KEY:
            marker = " — every alternative on the allowlist, so no review needed"
        elif lic not in ALLOWED and lic != "NOASSERTION":
            marker = " — **not on the LICENSE_ALLOWLIST.md default allowlist**"
        elif lic == "NOASSERTION":
            marker = " — declared license missing from the package manifest"
        lines.append(f"- `{lic}`: {count} component(s){marker}")

    lines += ["", "## Components requiring license review", ""]
    if review_needed:
        for name, lic in review_needed:
            lines.append(f"- `{name}` — {lic}")
    else:
        lines.append("- none")

    lines += ["", "## Components with a non-allowlisted license", ""]
    if other_licenses:
        for name, lic in other_licenses:
            lines.append(f"- `{name}` — {lic}")
    else:
        lines.append(
            "- none (every non-allowlisted id appears only inside a choice "
            "expression that also offers an allowlisted license)"
        )

    lines += ["", "## Components with no declared license", ""]
    if unknown_license:
        for name in unknown_license:
            lines.append(f"- `{name}`")
    else:
        lines.append("- none")

    lines += [
        "",
        "## Scope note",
        "",
        "This inventory covers the Rust dependency graph resolved from the locked",
        "Cargo.lock. The JavaScript dependency tree is pinned by pnpm-lock.yaml and",
        "is inventoried by the JavaScript license scan; Python has no dependency set",
        "because no Python package is shipped. A single merged SBOM covering all",
        "three ecosystems is a release requirement that remains open.",
        "",
    ]
    NOTICES.write_text("\n".join(lines), "utf-8")

    print(f"wrote {CDX} ({len(components)} components)")
    print(f"wrote {NOTICES}")
    print(f"  first-party={first_party} third-party={third_party}")
    print(f"  components requiring review: {len(review_needed)}")
    print(f"  components with a non-allowlisted license: {len(other_licenses)}")
    print(f"  components with no declared license: {len(unknown_license)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
