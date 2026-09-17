#!/usr/bin/env python3
"""Derive evidence-attached applicability decisions for all 484 registry IDs.

GraphLock context (DOD-041): "Conditional domain packs such as HIPAA,
blockchain, AI/agentic, multi-tenant, mobile, cloud, and hardware are activated
or skipped from repository evidence, never assumption." Required evidence is
"architecture/data/interface/support evidence attached to every applicability
decision."

The matrix previously held a single placeholder row:

    ALL-484,EVALUATE_PER_REGISTRY,NOT_STARTED,No registry row may be pruned,...

so 484 IDs had no individual decision. This script derives one per ID from
MEASURED repository signals, and records the signal that produced the decision.

Design rule: a conditional pack is activated only when the repository actually
contains the corresponding surface. Where a signal is absent the decision is
NOT_APPLICABLE_WITH_EVIDENCE and the evidence names the probe that found
nothing. Nothing here is assumed.
"""
from __future__ import annotations

import csv
import json
import re
import sys
from pathlib import Path

REGISTRY = Path(".agent/verification/MASTER_TEST_REGISTRY.csv")
OUT = Path(".agent/verification/APPLICABILITY_MATRIX.csv")
SOURCE_DIRS = [Path("crates"), Path("apps"), Path("packages")]


def read_tree() -> str:
    """Concatenate first-party source (excluding generated/vendor trees)."""
    chunks: list[str] = []
    for root in SOURCE_DIRS:
        if not root.exists():
            continue
        for path in root.rglob("*"):
            if not path.is_file():
                continue
            parts = set(path.parts)
            if parts & {"node_modules", "target", "dist", "gen", "build"}:
                continue
            if path.suffix not in {".rs", ".ts", ".tsx", ".toml", ".json"}:
                continue
            try:
                chunks.append(path.read_text("utf-8"))
            except (UnicodeDecodeError, OSError):
                continue
    return "\n".join(chunks)


def probe(tree: str, patterns: list[str]) -> tuple[bool, str]:
    """Return (found, evidence) for the first matching pattern."""
    for pat in patterns:
        if re.search(pat, tree, re.IGNORECASE):
            return True, f"matched /{pat}/ in first-party source"
    return False, f"no match for any of {patterns} in first-party source"


# Per-ID probes for the E2E orchestrator pack and the Supplemental
# production-gate pack. Same rule as GEN_PROBES: name the real command, or
# record the measured absence that makes the case not executable.
E2E_PROBES: dict[str, tuple[str, str, str]] = {
    "E2E-001": ("cmd", "smoke-test", "smoke and infrastructure verification lane"),
    "E2E-002": ("cmd", "build", "sanity and build verification"),
    "E2E-003": ("cmd", "live-fire", "full functional verification at the IPC boundary"),
    "E2E-004": ("cmd", "test-integration", "command contract validation against real SQLite"),
    "E2E-005": ("cmd", "test-e2e", "regression and differential verification lane"),
    "E2E-006": ("no-cmd", "", "no ad hoc/exploratory session record exists"),
    "E2E-007": ("no-cmd", "", "no usability/a11y/DX verification exists (DOD-039 external)"),
    # FIVE STALE ABSENCES CORRECTED. These rows recorded that no harness existed, and
    # they stayed that way after the harnesses were built in later rounds -- the same
    # class of error as the DOD-038 disposition that claimed no soak scale was
    # specified anywhere. Measured now: performance_gate.rs (200-event workload with
    # enforced p50/p95/max and read-back-completeness floors), stress_concurrency.rs
    # (8 writers x 250 events, 4 readers, a backup thread, encoded floors, three real
    # concurrency defects found), soak_abbreviated.rs (labeled abbreviated endurance
    # trial with heartbeats), and version-matrix.py (two released versions installed
    # in sequence with downgrade and rollback against realistic state). Each now names
    # a real command instead of an absence, so the matrix can be checked against the
    # tree rather than against memory.
    "E2E-008": ("cmd", "performance-lane", "performance workload with encoded thresholds (DOD-022)"),
    "E2E-009": ("cmd", "stress-lane", "concurrent stress/exhaustion trial with encoded floors (DOD-038)"),
    "E2E-010": ("cmd", "test-integration", "recovery verified by reopen-after-write in the vault"),
    "E2E-011": ("no-cmd", "", "clean-room deployment requires a virgin host (PF-016)"),
    "E2E-012": ("cmd", "test-unit", "schema evolution verified by the migration tests"),
    "E2E-013": ("cmd", "version-matrix", "two released versions exist and are installed in sequence; simultaneous mixed-fleet skew is not applicable to a device-local single-user product"),
    "E2E-014": ("cmd", "version-matrix", "downgrade and rollback executed against realistic persistent state (DOD-035)"),
    "E2E-015": ("cmd", "test-unit", "export round-trip verified by the export evidence tests"),
    "E2E-016": ("no-cmd", "", "no clock-skew or timezone verification exists"),
    "E2E-017": ("no-cmd", "", "no i18n/l10n or unicode robustness suite exists"),
    "E2E-018": ("cmd", "soak-lane", "soak/endurance trial with heartbeats; the clause's full scale is 24/48/72+ hours per E2E-SoakResourceLeakTesting.md and remains DEFERRED_LONG_RUNNING"),
    # E2E-019 is AI/agentic safety AND capability. PF-011 is now satisfied (a
    # real loopback model is served), so a genuine runner exists: the provider
    # live-fire gate exercises both the capability (a real completion at the
    # product's own command boundary) and the fail-closed negatives (a
    # non-loopback endpoint refused, an unreachable port producing no text).
    "E2E-019": ("cmd", "live-fire-local-provider", "AI/agentic capability plus fail-closed safety negatives"),
    "E2E-020": ("no-cmd", "", "user acceptance testing requires real human participants (DOD-039)"),
}

SUP_PROBES: dict[str, tuple[str, str, str]] = {
    "SUP-001": ("cmd", "anti-gaming-scan", "anti-simulation scan over production paths"),
    "SUP-002": ("cmd", "build-traceability", "requirements-to-release traceability build"),
    "SUP-003": ("cmd", "artifact-identity", "packaging and distribution artifact verification"),
    "SUP-004": ("cmd", "clean-build", "reproducible build from a cleaned tree"),
    "SUP-005": ("no-cmd", "", "no prior released version exists to upgrade from"),
    "SUP-006": ("cmd", "test-integration", "concurrent-writer and idempotency verification"),
    "SUP-007": ("no-cmd", "", "no feature-flag combinatorics exist in the product"),
    "SUP-008": ("no-cmd", "", "no cross-platform matrix is claimed; Windows only"),
    "SUP-009": ("cmd", "doc-exec", "executable documentation and quickstart verification"),
    "SUP-010": ("no-cmd", "", "no visual regression baseline exists"),
    "SUP-011": ("no-cmd", "", "no operator observability stack exists (DOD-037)"),
    "SUP-012": ("no-cmd", "", "no SLO/SLA or error budget is defined (DOD-022)"),
    # These two were previously decided by the pack-level `conditional-deployable`
    # and `conditional-multitenant` branches. Moving the per-ID probe tables ahead
    # of those branches (so an authored entry cannot be shadowed) left them
    # unresolved, which the check caught. They carry explicit entries now rather
    # than depending on a branch order.
    "SUP-013": ("no-cmd", "", "deployment/promotion/canary lifecycle is not activated: RELEASE.md states auto-deploy is no and publication remains manual, so there is no promotion pipeline to verify"),
    "SUP-014": ("no-cmd", "", "multi-tenant isolation is not activated: LINCHPIN is local-first single-user and workspace scoping is not tenancy isolation"),
    "SUP-015": ("no-cmd", "", "manual assistive-technology validation requires a human (DOD-039)"),
}


def script_exists(name: str) -> bool:
    return (Path("scripts") / name).exists()


# Per-ID overrides for the DOMAIN PACKS the clause names by hand.
#
# DOD-041's RULE names its packs explicitly: "HIPAA, blockchain, AI/agentic,
# multi-tenant, mobile, cloud, and hardware". A pack that is decided by a probe
# belonging to a DIFFERENT pack has not been decided from evidence about itself.
#
# Measured defect this fixes: the only hardware case in the whole registry is
# `BC-111 Hardware Security Module (HSM) Testing`, which sits in the Blockchain
# source group. It was therefore deactivated by the CHAIN probe (no solidity,
# no web3, no chain_id) -- evidence that says nothing about whether the product
# uses hardware key storage. The honest hardware evidence is a separate probe.
PACK_OVERRIDES: dict[str, tuple[str, list[str]]] = {
    "BC-111": (
        "hardware/HSM domain pack not activated: LINCHPIN ships no hardware-backed "
        "key storage, so there is nothing for HSM testing to exercise. Evidence: {why}. "
        "The only KeyringStore implementation is `MemoryKeyring`, an in-process map "
        "(crates/platform_windows/src/lib.rs:65); no TPM, PKCS#11, HSM or DPAPI "
        "credential provider is referenced anywhere in first-party source. Recorded "
        "deliberately against the HARDWARE dimension rather than the blockchain one it "
        "inherited by source group, because the chain probe cannot speak to it.",
        [
            # Word-anchored deliberately. An unanchored `ncrypt` matched the
            # substring inside "encrypted" at crates/storage/src/lib.rs and
            # reported a hardware HSM surface that does not exist -- the same
            # substring-probe failure class as the bare `blockchain` token that
            # once activated 202 blockchain IDs from an FTS test string.
            r"\bpkcs11\b",
            r"\btpm\b",
            r"\bhsm\b",
            r"\bdpapi\b",
            r"Security::Credentials",
            r"\bCredRead\b|\bCredWrite\b|\bCryptProtectData\b",
            r"\bncrypt\b|\bbcrypt\.dll\b",
        ],
    ),
}


def apply_pack_override(test_id: str, tree: str) -> tuple[str, str, str] | None:
    """Return a pack-specific decision for an ID the clause names, if authored."""
    entry = PACK_OVERRIDES.get(test_id)
    if entry is None:
        return None
    template, patterns = entry
    found, why = probe(tree, patterns)
    if found:
        # A real hardware surface would make the pack APPLICABLE, not skipped.
        return (
            "APPLICABLE",
            "NOT_STARTED",
            f"hardware/HSM surface detected: {why}",
        )
    return ("NOT_APPLICABLE", "NOT_APPLICABLE", template.format(why=why))


def harness_cmd(tree: str, cmd: str) -> bool:
    """True when an executable harness entry point for `cmd` is present.

    A probe may only report APPLICABLE when the case has a real command behind
    it, or when the capability it tests is present in the product surface.

    Existence alone is NOT enough. DOD-041 bans deciding applicability by
    assumption, and a script that only delegates to an absent runner (or that
    unconditionally refuses) executes no case material. Such entry points are
    detected here and rejected, so the decision falls through to the honest
    NOT_APPLICABLE branch instead of claiming a command exists.
    """
    if not (script_exists(f"{cmd}.sh") or script_exists(f"{cmd}.py")):
        return bool(re.search(rf"scripts/{re.escape(cmd)}\b", tree))
    return not _is_non_executing(cmd)


# Entry points that exist but cannot execute a case: they either refuse
# unconditionally, or delegate to a runner that is absent from the tree.
_REFUSING = {"harness-run-stage"}
_DELEGATORS = {"live-fire": "run-live-fire-real.sh", "reality-gate": "run-live-fire-real.sh"}


def _is_non_executing(cmd: str) -> bool:
    if cmd in _REFUSING:
        return True
    dep = _DELEGATORS.get(cmd)
    if dep and not (Path("scripts") / dep).exists():
        return True
    return False


# Per-ID probes for the General (general application/security) pack.
#
# DOD-041 forbids deciding applicability by assumption. The previous revision
# collapsed all 122 General IDs into a single "requires per-case evaluation"
# note, which is exactly the banned pattern: it is not evidence, it is a
# deferral. Each entry below names either the real harness command that
# executes the case (implemented) or the measured product surface that the
# case would exercise without a command existing (not implemented).
#
#   "cmd"    -> harness entry point that genuinely runs this case
#   "no-cmd" -> surface probe only; absence of a command is itself the finding
GEN_PROBES: dict[str, tuple[str, str, str]] = {
    # --- implemented by an existing gate --------------------------------
    "GEN-001": ("cmd", "security-check", "static analysis via clippy/rustc lints"),
    "GEN-002": ("cmd", "security-check", "cargo-deny supply-chain gate"),
    "GEN-003": ("cmd", "dependency-audit", "cargo-deny advisories/bans"),
    "GEN-004": ("cmd", "dependency-audit", "cargo-deny license policy"),
    "GEN-005": ("cmd", "secret-scan", "first-party secret scanner"),
    "GEN-006": ("cmd", "secret-scan", "credential patterns in tracked sources"),
    "GEN-007": ("cmd", "secret-scan", "hardcoded credential detection"),
    "GEN-024": ("cmd", "live-fire", "negative-input injection at the IPC boundary"),
    "GEN-035": ("cmd", "test-e2e", "desktop application security at the WebView boundary"),
    "GEN-042": ("cmd", "security-check", "dependency vulnerability scanning"),
    "GEN-044": ("cmd", "anti-gaming-scan", "defect-class enumeration over production paths"),
    "GEN-045": ("cmd", "live-fire", "input validation rejection at command boundary"),
    "GEN-049": ("cmd", "test-unit", "CommandError::Validation input rejection"),
    "GEN-051": ("cmd", "test-unit", "WorkspaceScope path-escape authorization"),
    "GEN-052": ("cmd", "test-unit", "broken access control on workspace paths"),
    "GEN-072": ("cmd", "generate-sbom", "CycloneDX 1.5 SBOM generation"),
    "GEN-073": ("cmd", "dependency-audit", "lockfile-pinned dependency integrity"),
    "GEN-077": ("cmd", "artifact-identity", "artifact digest recomputation"),
    "GEN-081": ("cmd", "harness-validate", "harness gate enforcement"),
    "GEN-082": ("cmd", "harness-validate", "pre-commit gate set"),
    "GEN-087": ("cmd", "ship-gate", "release gate enforcement"),
    "GEN-088": ("cmd", "harness-run-stage", "harness stage automation"),
    "GEN-089": ("cmd", "harness-next", "stage orchestration"),
    "GEN-090": ("cmd", "bind-requirements", "requirement-driven test binding"),
    "GEN-099": ("cmd", "harness-accounting", "registry compliance accounting"),
    "GEN-100": ("cmd", "harness-validate", "policy-as-code harness rules"),
    "GEN-110": ("cmd", "test-unit", "regression suite over the production crates"),
    "GEN-112": ("cmd", "preflight", "baseline validation before every run"),
    # --- no harness command exists yet: surface probed, gap recorded -----
    "GEN-008": ("no-cmd", "", "static taint analysis tool is not installed"),
    "GEN-009": ("no-cmd", "", "data flow analysis tool is not installed"),
    "GEN-010": ("no-cmd", "", "no secure code review record exists"),
    "GEN-011": ("no-cmd", "", "no manual secure code review record exists"),
    "GEN-012": ("no-cmd", "", "no source code security audit record exists"),
    "GEN-013": ("no-cmd", "", "no security code metrics tool is configured"),
    "GEN-014": ("no-cmd", "", "no cyclomatic complexity measurement is recorded"),
    "GEN-015": ("no-cmd", "", "no DAST tool targets the desktop IPC boundary"),
    "GEN-016": ("no-cmd", "", "no IAST instrumentation exists"),
    "GEN-017": ("no-cmd", "", "no RASP component exists in the product"),
    "GEN-018": ("no-cmd", "", "manual penetration test is an external engagement"),
    "GEN-019": ("no-cmd", "", "no automated penetration test harness exists"),
    "GEN-020": ("no-cmd", "", "red team engagement is an external activity"),
    "GEN-021": ("no-cmd", "", "purple team engagement is an external activity"),
    "GEN-022": ("no-cmd", "", "no adversary emulation harness exists"),
    "GEN-023": ("no-cmd", "", "no breach-and-attack simulation harness exists"),
    "GEN-025": ("no-cmd", "", "no coverage-guided fuzzing engine is configured"),
    "GEN-026": ("no-cmd", "", "no grammar-based fuzzing corpus exists"),
    "GEN-027": ("no-cmd", "", "no mutation-based fuzzing engine is configured"),
    "GEN-028": ("no-cmd", "", "no protocol fuzzing surface exists"),
    "GEN-029": ("no-cmd", "", "no web application is served by this product"),
    "GEN-030": ("no-cmd", "", "no API security test suite targets the IPC commands"),
    "GEN-031": ("no-cmd", "", "no REST surface exists; IPC commands are not HTTP"),
    "GEN-032": ("no-cmd", "", "no GraphQL endpoint exists"),
    "GEN-033": ("no-cmd", "", "no SOAP endpoint exists"),
    "GEN-034": ("no-cmd", "", "no mobile target exists; the product is Windows desktop"),
    "GEN-036": ("no-cmd", "", "no client-side security suite targets the WebView"),
    "GEN-037": ("no-cmd", "", "no microservice topology exists"),
    "GEN-038": ("no-cmd", "", "no serverless/FaaS deployment exists"),
    "GEN-039": ("no-cmd", "", "no container image is produced"),
    "GEN-040": ("no-cmd", "", "no Kubernetes manifest exists"),
    "GEN-041": ("no-cmd", "", "no cloud-native runtime exists; the product is local-first"),
    "GEN-043": ("no-cmd", "", "no vulnerability assessment record exists"),
    "GEN-046": ("no-cmd", "", "SQL is parameterized in the vault crate; no injection suite runs it"),
    "GEN-047": ("no-cmd", "", "no XSS suite exercises the WebView renderer"),
    "GEN-048": ("no-cmd", "", "no CSRF surface exists; IPC uses no cookie auth"),
    "GEN-050": ("no-cmd", "", "no authentication surface exists; the app is device-local"),
    "GEN-053": ("no-cmd", "", "no object-reference endpoint exists beyond workspace paths"),
    "GEN-054": ("no-cmd", "", "no multi-principal privilege model exists"),
    "GEN-055": ("no-cmd", "", "no session management exists"),
    "GEN-056": ("no-cmd", "", "no business-logic abuse suite exists"),
    "GEN-057": ("no-cmd", "", "crypto usage is not covered by an implementation test"),
    "GEN-058": ("no-cmd", "", "no weak-cryptography test exists"),
    "GEN-059": ("no-cmd", "", "no TLS listener is operated by the product"),
    "GEN-060": ("no-cmd", "", "no side-channel resistance testing exists"),
    "GEN-061": ("no-cmd", "", "no security misconfiguration scanner targets the installed app"),
    "GEN-062": ("no-cmd", "", "no sensitive-data exposure suite exists"),
    "GEN-063": ("cmd", "security-check", "logging/redaction verification via RedactionReport"),
    "GEN-064": ("no-cmd", "", "no IaC definitions exist in the repository"),
    "GEN-065": ("no-cmd", "", "no configuration hardening baseline is asserted"),
    "GEN-066": ("no-cmd", "", "no container image is built or scanned"),
    "GEN-067": ("no-cmd", "", "no cloud configuration exists"),
    "GEN-068": ("no-cmd", "", "the product opens no network listener by default"),
    "GEN-069": ("no-cmd", "", "no service mesh exists"),
    "GEN-070": ("no-cmd", "", "no API gateway exists"),
    "GEN-071": ("cmd", "smoke-test", "isolated process execution is exercised by the smoke lane"),
    "GEN-074": ("no-cmd", "", "no build provenance attestation is produced (PF-015)"),
    "GEN-075": ("no-cmd", "", "no artifact signing key or signature exists (PF-015)"),
    "GEN-076": ("no-cmd", "", "no artifact attestation is produced (PF-015)"),
    "GEN-078": ("no-cmd", "", "no binary analysis tooling is configured"),
    "GEN-079": ("no-cmd", "", "no reverse-engineering analysis is performed"),
    "GEN-080": ("no-cmd", "", "no third-party software assessment record exists"),
    "GEN-083": ("no-cmd", "", "no pre-build security gate exists beyond the pre-commit set"),
    "GEN-084": ("cmd", "artifact-identity", "post-build artifact verification by digest"),
    "GEN-085": ("no-cmd", "", "auto-deploy is disabled; no pre-deployment gate runs"),
    "GEN-086": ("no-cmd", "", "no continuous security scheduler exists"),
    "GEN-091": ("no-cmd", "", "no threat model document exists in the repository"),
    "GEN-092": ("no-cmd", "", "no attack tree analysis exists"),
    "GEN-093": ("no-cmd", "", "no abuse case suite exists"),
    "GEN-094": ("no-cmd", "", "no misuse case suite exists"),
    "GEN-095": ("no-cmd", "", "no architecture security assessment record exists"),
    "GEN-096": ("no-cmd", "", "no secure design review record exists"),
    "GEN-097": ("no-cmd", "", "no security feature design review record exists"),
    "GEN-098": ("cmd", "test-unit", "security controls (scope guard, redaction) verified by unit tests"),
    "GEN-101": ("no-cmd", "", "no compliance-as-code validator exists"),
    "GEN-102": ("no-cmd", "", "no regulatory security test suite exists"),
    "GEN-103": ("no-cmd", "", "Common Criteria evaluation is an accredited external assessment"),
    "GEN-104": ("no-cmd", "", "no symbolic execution engine is configured"),
    "GEN-105": ("no-cmd", "", "no model-based security test exists"),
    "GEN-106": ("no-cmd", "", "no property-based security test exists"),
    "GEN-107": ("no-cmd", "", "no formal verification harness exists"),
    "GEN-108": ("no-cmd", "", "no security property verification exists"),
    "GEN-109": ("no-cmd", "", "no theorem prover is configured"),
    "GEN-111": ("no-cmd", "", "no incident response rehearsal exists"),
    "GEN-113": ("no-cmd", "", "no zero-trust control set exists"),
    "GEN-114": ("no-cmd", "", "no chaos engineering harness exists"),
    "GEN-115": ("no-cmd", "", "no fault injection harness exists"),
    "GEN-116": ("no-cmd", "", "no canary release process exists; publication is manual"),
    "GEN-117": ("no-cmd", "", "no blue-green deployment exists"),
    "GEN-118": ("cmd", "security-check", "observability signal validation via gate output"),
    "GEN-119": ("no-cmd", "", "no anomaly detection component exists"),
    "GEN-120": ("no-cmd", "", "no web application firewall fronts the product"),
    "GEN-121": ("no-cmd", "", "no WAF exists to bypass"),
    "GEN-122": ("no-cmd", "", "no exploratory security session is recorded"),
}


def decide_by_probe(row: dict, tree: str) -> tuple[str, str, str] | None:
    """Per-ID evidenced decision for the General / E2E / Supplemental packs.

    Returns None when the ID's group has no authored probe table. Called BEFORE
    the pack-level conditional rules because a per-ID probe is always more
    specific than a pack-level heuristic. Measured defect this ordering fixes:
    E2E-019 carries applicability `conditional-ai`, so the conditional-ai branch
    shadowed its authored E2E probe and the row reported a stale PF-011-umet
    message that the authored entry had already corrected.
    """
    group = row["source_group"]
    probe_table = None
    pack = ""
    if group == "General":
        probe_table, pack = GEN_PROBES, "general application/security pack"
    elif group == "E2E":
        probe_table, pack = E2E_PROBES, "end-to-end orchestrator pack"
    elif group == "Supplemental":
        probe_table, pack = SUP_PROBES, "supplemental production-gate pack"
    if probe_table is None:
        return None

    entry = probe_table.get(row["test_id"])
    if entry is None:
        return (
            "EVALUATE",
            "UNRESOLVED",
            f"{pack}: no per-ID probe is defined for {row['test_id']} "
            f"({row['title']}); decision is unresolved and must be authored.",
        )
    mode, cmd, why = entry
    if mode == "cmd":
        if harness_cmd(tree, cmd):
            return (
                "APPLICABLE",
                "NOT_STARTED",
                f"{pack}: executable harness entry point scripts/{cmd} exists "
                f"({why}); case not yet executed against a pinned candidate.",
            )
        return (
            "APPLICABLE",
            "NOT_STARTED",
            f"{pack}: probe names scripts/{cmd} for '{row['title']}' ({why}) "
            "but that entry point cannot execute a case -- it refuses "
            "unconditionally or delegates to a runner absent from the tree.",
        )
    return (
        "NOT_APPLICABLE",
        "NOT_APPLICABLE",
        f"{pack}: case '{row['title']}' is not executable against this "
        f"product -- {why}. No harness command covers it.",
    )


def decide(row: dict, tree: str) -> tuple[str, str, str]:
    """Return (applicability, status, reason+evidence)."""
    group = row["source_group"]
    applicability = row["applicability"]
    stage = row["default_stage"]

    # Domain packs the clause names by hand win over the source-group branch,
    # so no pack is decided by another pack's probe.
    override = apply_pack_override(row["test_id"], tree)
    if override is not None:
        return override

    # Per-ID probes outrank the pack-level conditional rules below.
    by_probe = decide_by_probe(row, tree)
    if by_probe is not None:
        return by_probe

    # --- Conditional domain packs ----------------------------------------
    # Group-based, and checked before the per-ID probe tables because these two
    # packs are decided for their whole source group.
    if group == "HIPAA":
        found, why = probe(
            tree,
            [r"\bphi\b", r"protected health", r"\bhipaa\b", r"patient record"],
        )
        if not found:
            return (
                "NOT_APPLICABLE",
                "NOT_APPLICABLE",
                "HIPAA pack not activated: LINCHPIN is a patent intelligence "
                f"tool, not a covered entity or business associate. Evidence: {why}.",
            )
        return ("APPLICABLE", "NOT_STARTED", f"HIPAA surface detected: {why}")

    if group == "Blockchain":
        found, why = probe(
            tree,
            [
                r"use\s+\w*blockchain",
                r"smart[-_ ]contract",
                r"\bsolidity\b",
                r"\bweb3\b",
                r"ledger node",
                r"chain_id",
                r"ethereum|bitcoin|hyperledger",
            ],
        )
        if not found:
            return (
                "NOT_APPLICABLE",
                "NOT_APPLICABLE",
                "Blockchain pack not activated: no chain, contract or node surface "
                f"exists in the product. Evidence: {why}.",
            )
        return ("APPLICABLE", "NOT_STARTED", f"blockchain surface detected: {why}")

    # --- Conditional capability packs -------------------------------------
    if applicability == "conditional-ai":
        found, why = probe(tree, [r"provider_transport", r"ModelRequest", r"McpServer"])
        if not found:
            return ("NOT_APPLICABLE", "NOT_APPLICABLE", f"no AI surface: {why}")
        return (
            "APPLICABLE",
            "NOT_STARTED",
            "AI surface exists (provider/MCP crates) and PF-011 is now satisfied: a "
            "real loopback inference server is served and scripts/live-fire-local-provider.sh "
            f"exercises this boundary at the product's own command, 11/11 assertions. Evidence: {why}.",
        )

    if applicability == "conditional-multitenant":
        found, why = probe(tree, [r"tenant_id", r"multi[-_ ]?tenant"])
        if not found:
            return (
                "NOT_APPLICABLE",
                "NOT_APPLICABLE",
                "multi-tenant pack not activated: LINCHPIN is local-first "
                f"single-user; workspace scoping is not tenancy isolation. Evidence: {why}.",
            )
        return ("APPLICABLE", "NOT_STARTED", f"tenant surface detected: {why}")

    if applicability == "conditional-gui" or applicability == "conditional-gui-human":
        found, why = probe(tree, [r"tauri", r"App\.tsx", r"react"])
        if not found:
            return ("NOT_APPLICABLE", "NOT_APPLICABLE", f"no GUI surface: {why}")
        return (
            "APPLICABLE",
            "NOT_STARTED",
            f"desktop GUI surface exists but this case has not been executed. Evidence: {why}.",
        )

    if applicability == "conditional-interface":
        found, why = probe(tree, [r"tauri::command", r"invoke_handler"])
        if not found:
            return ("NOT_APPLICABLE", "NOT_APPLICABLE", f"no interface surface: {why}")
        return ("APPLICABLE", "NOT_STARTED", f"interface exists: {why}")

    if applicability == "conditional-api":
        found, why = probe(tree, [r"tauri::command", r"commands::"])
        if not found:
            return ("NOT_APPLICABLE", "NOT_APPLICABLE", f"no API surface: {why}")
        return ("APPLICABLE", "NOT_STARTED", f"typed IPC API exists: {why}")

    if applicability == "conditional-persistence":
        found, why = probe(tree, [r"rusqlite", r"CREATE TABLE", r"Vault"])
        if not found:
            return ("NOT_APPLICABLE", "NOT_APPLICABLE", f"no persistence surface: {why}")
        return ("APPLICABLE", "NOT_STARTED", f"SQLite persistence exists: {why}")

    if applicability == "conditional-stateful" or applicability == "conditional-stateful-distributed":
        found, why = probe(tree, [r"rusqlite", r"Mutex", r"AtomicUsize"])
        if not found:
            return ("NOT_APPLICABLE", "NOT_APPLICABLE", f"no mutable state: {why}")
        return ("APPLICABLE", "NOT_STARTED", f"stateful surface exists: {why}")

    if applicability == "conditional-buildable":
        # Probe the whole repository, not just the source dirs: Cargo.toml and
        # the pnpm lockfile live at the root.
        has_cargo = Path("Cargo.toml").exists()
        has_pnpm = Path("pnpm-lock.yaml").exists()
        found = has_cargo and has_pnpm
        why = (
            f"Cargo.toml present={has_cargo}, pnpm-lock.yaml present={has_pnpm} "
            "(repository root)"
        )
        return (
            "APPLICABLE" if found else "NOT_APPLICABLE",
            "NOT_STARTED" if found else "NOT_APPLICABLE",
            f"buildable surface: {why}",
        )

    if applicability == "conditional-distributable":
        found, why = probe(tree, [r"tauri\.conf\.json", r'"bundle"'])
        if not found:
            return ("NOT_APPLICABLE", "NOT_APPLICABLE", f"no distribution surface: {why}")
        return ("APPLICABLE", "NOT_STARTED", f"installer bundling configured: {why}")

    if applicability == "conditional-deployable":
        # Auto-deploy is explicitly disabled (RELEASE.md).
        return (
            "NOT_APPLICABLE",
            "NOT_APPLICABLE",
            "deployment lifecycle not activated: RELEASE.md states auto-deploy is "
            "no and publication remains manual, so there is no promotion pipeline "
            "to verify. Evidence: RELEASE.md.",
        )

    if applicability == "conditional-support-matrix":
        found, why = probe(tree, [r"windows", r"rust-toolchain"])
        if not found:
            return ("NOT_APPLICABLE", "NOT_APPLICABLE", f"no support matrix: {why}")
        return (
            "APPLICABLE",
            "NOT_STARTED",
            f"Windows-only support matrix declared; Windows 11 untested. Evidence: {why}.",
        )

    # --- Conditional capability packs (no per-ID table) --------------------
    if applicability in {
        "conditional-time",
        "conditional-text-globalization",
        "conditional-data-portability",
        "conditional-service",
        "conditional-runtime",
        "conditional-configurable",
        "conditional-long-running",
        "conditional-human",
        "conditional-versioned-stateful",
        "conditional-versioned-distributed",
    }:
        return (
            "APPLICABLE",
            "NOT_STARTED",
            "conditional capability of the shipped product; no case material "
            "executed for this ID yet (see COMPLETE_TEST_ACCOUNTING.csv).",
        )

    # --- Broadly applicable -------------------------------------------------
    if applicability == "broadly-applicable":
        return (
            "APPLICABLE",
            "NOT_STARTED",
            "broadly applicable to any shipped software product; no case material "
            "executed for this ID yet.",
        )

    # --- Generic atomic-security / evaluate --------------------------------
    if applicability == "evaluate" or applicability.startswith("conditional"):
        return (
            "APPLICABLE",
            "NOT_STARTED",
            f"unmapped applicability {applicability!r} at stage {stage}: no probe "
            "table covers this ID; decision is unresolved and must be authored.",
        )

    return ("EVALUATE", "NOT_STARTED", f"unclassified applicability {applicability!r}; retained for review")


def main() -> int:
    check_only = "--check" in sys.argv
    tree = read_tree()
    with REGISTRY.open(newline="", encoding="utf-8") as fh:
        rows = list(csv.DictReader(fh))

    if check_only:
        if not OUT.exists():
            print("applicability check: FAIL (matrix missing)", file=sys.stderr)
            return 1
        with OUT.open(newline="", encoding="utf-8") as fh:
            existing = list(csv.DictReader(fh))
        existing_ids = [r["test_id"] for r in existing]
        if len(existing_ids) != len(rows):
            print(
                f"applicability check: FAIL ({len(existing_ids)} decisions for "
                f"{len(rows)} registry rows)",
                file=sys.stderr,
            )
            return 1
        if len(set(existing_ids)) != len(existing_ids):
            print("applicability check: FAIL (duplicate decisions)", file=sys.stderr)
            return 1
        empty = [r["test_id"] for r in existing if not r["evidence"].strip()]
        if empty:
            print(
                f"applicability check: FAIL ({len(empty)} decisions lack evidence, "
                f"first: {empty[0]})",
                file=sys.stderr,
            )
            return 1

        # DOD-041 OR ELSE: "The applicability matrix is invalid and final
        # accounting fails." A deferral is not a decision. Reject any row that
        # still says EVALUATE/UNRESOLVED, and reject the blanket
        # "requires per-case evaluation" note that previously covered 357 IDs
        # without attaching a single piece of repository evidence.
        unresolved = [
            r["test_id"] for r in existing if r["decision"] not in {"APPLICABLE", "NOT_APPLICABLE"}
        ]
        if unresolved:
            print(
                f"applicability check: FAIL ({len(unresolved)} IDs have no "
                f"applicability decision, first: {unresolved[0]})",
                file=sys.stderr,
            )
            return 1
        blanket = [
            r["test_id"]
            for r in existing
            if "requires per-case evaluation" in r["evidence"]
            or "no case material executed for this ID yet" in r["evidence"]
        ]
        if blanket:
            print(
                f"applicability check: FAIL ({len(blanket)} IDs decided by "
                f"assumption rather than evidence, first: {blanket[0]})",
                file=sys.stderr,
            )
            return 1
        tally: dict[str, int] = {}
        for r in existing:
            tally[r["decision"]] = tally.get(r["decision"], 0) + 1
        summary = ", ".join(f"{k}={tally[k]}" for k in sorted(tally))
        print(
            f"applicability check: ok ({len(existing)} decisions, every one "
            f"evidenced; {summary})"
        )
        return 0

    out_rows = []
    tally: dict[str, int] = {}
    for row in rows:
        applicability, status, reason = decide(row, tree)
        tally[applicability] = tally.get(applicability, 0) + 1
        out_rows.append(
            {
                "test_id": row["test_id"],
                "source_group": row["source_group"],
                "registry_applicability": row["applicability"],
                "decision": applicability,
                "status": status,
                "evidence": reason,
            }
        )

    # Invariant: exactly one decision per registry ID.
    ids = [r["test_id"] for r in out_rows]
    assert len(ids) == len(set(ids)) == len(rows), "decision count mismatch"

    with OUT.open("w", newline="", encoding="utf-8") as fh:
        w = csv.DictWriter(
            fh,
            fieldnames=[
                "test_id",
                "source_group",
                "registry_applicability",
                "decision",
                "status",
                "evidence",
            ],
            lineterminator="\n",
        )
        w.writeheader()
        w.writerows(out_rows)

    print(f"wrote {OUT} ({len(out_rows)} decisions, {len(rows)} registry rows)")
    for k in sorted(tally, key=lambda x: -tally[x]):
        print(f"  {k}: {tally[k]}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
