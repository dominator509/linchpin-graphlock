#!/usr/bin/env python3
"""Insert `covers:` binding markers above tests that verify a requirement.

DOD-001 requires each requirement to have "at least one acceptance test" with
"executed acceptance evidence". The bindings are declared explicitly rather than
inferred, so this script performs a targeted, reviewable insertion: it reads a
mapping of {file: {test_fn: [requirements]}} and inserts

    /// covers: REQ-X-001, REQ-Y-002

immediately above each named test function, once. It is idempotent -- a test
that already carries a marker is left alone.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

# Only tests that genuinely assert the requirement's behaviour are listed.
# A test that merely touches related code is NOT included: over-claiming the
# binding would defeat the purpose of the exercise.
BINDINGS: dict[str, dict[str, list[str]]] = {
    "crates/domain/src/lib.rs": {
        "test_domain_entity_lifecycle": ["REQ-DOM-001"],
        "test_human_vs_ai_origin": ["REQ-DOM-001", "REQ-DOM-002"],
        "test_opportunity_candidate": ["REQ-DOM-003"],
        "test_claim_graph_and_design_around": ["REQ-DOM-004"],
        "test_docket_state_machine": ["REQ-DOM-007"],
    },
    "crates/storage/src/vault.rs": {
        "test_migrations_are_recorded_and_idempotent": ["REQ-RES-002"],
        "test_conception_event_survives_reopen": ["REQ-DATA-001", "REQ-DOM-001"],
        "test_unknown_workspace_is_refused": ["REQ-DATA-001", "REQ-SEC-001"],
        "test_events_are_workspace_scoped": ["REQ-DATA-001", "REQ-SEC-001"],
        "test_blobs_are_content_addressed": ["REQ-DATA-002"],
        "test_audit_chain_verifies_and_detects_tampering": ["REQ-DATA-002"],
        "test_sha256_known_vector": ["REQ-DATA-002"],
    },
    "crates/evidence/src/lib.rs": {
        "test_disclosure_firewall": ["REQ-DOM-008"],
        "test_input_hardening": ["REQ-SEC-002"],
        "test_sanitize_path_property_over_generated_corpus": ["REQ-SEC-002"],
        "test_archive_entry_rejects_zip_slip": ["REQ-SEC-002"],
        "test_secret_redaction_canary": ["REQ-OPS-011"],
        "test_redactor_ignores_empty_secret": ["REQ-OPS-011"],
        "test_redactor_preserves_ordinary_text": ["REQ-OPS-011"],
        "test_allowlist_rejects_empty_values": ["REQ-SEC-010"],
        "test_allowlist_holds_no_cryptographic_material": ["REQ-SEC-010"],
    },
    "crates/crash_reporter/src/lib.rs": {
        "test_telemetry_correlation": ["REQ-OPS-010"],
        "test_redaction_actually_removes_invention_content": ["REQ-REPAIR-001"],
        "test_flag_alone_is_not_sufficient": ["REQ-REPAIR-001"],
        "test_token_shaped_secrets_are_scrubbed_without_registration": ["REQ-REPAIR-001"],
        "test_empty_secret_is_not_registered": ["REQ-REPAIR-001"],
        "test_sanitized_repair_capsule": ["REQ-REPAIR-002"],
    },
    "crates/patent/src/lib.rs": {
        "test_claim_linter": ["REQ-PAT-001"],
        "test_package_builder_does_not_claim_unimplemented_formats": ["REQ-PAT-002"],
        "test_manifest_digest_depends_on_both_inputs": ["REQ-PAT-002"],
        "test_uspto_manifest_handoff": ["REQ-PAT-005"],
        "test_receipt_import_accepts_realistic_receipt": ["REQ-PAT-005"],
        "test_receipt_import_fails_closed_on_malformed_input": ["REQ-PAT-005"],
        "test_office_action_workspace": ["REQ-PAT-004"],
    },
    "crates/commercialization/src/lib.rs": {
        "test_data_room_workflow": ["REQ-COM-001", "REQ-COM-004"],
        "test_completed_runs_cannot_be_fabricated": ["REQ-SHIP-001"],
        "test_failing_runs_do_not_count_as_completed": ["REQ-SHIP-001"],
        "test_duplicate_runs_count_once": ["REQ-SHIP-001"],
    },
    "crates/mcp_hub/src/lib.rs": {
        "test_mcp_grants_and_boundaries": ["REQ-MCP-001"],
        "test_grant_capability_is_idempotent": ["REQ-MCP-001"],
        "test_unknown_client_has_no_capabilities": ["REQ-MCP-001", "REQ-SEC-003"],
        "test_injection_filter_allows_benign_payloads": ["REQ-LLM-001"],
        "probe_injection_filter_covers_variants": ["REQ-LLM-001"],
    },
    "crates/provider_transport/src/lib.rs": {
        "test_local_adapter_rejects_non_loopback_endpoint": ["REQ-LLM-001", "REQ-SEC-001"],
        "test_unreachable_endpoint_fails_closed_not_fabricated": ["REQ-LLM-001"],
        "test_empty_prompt_is_rejected_before_any_io": ["REQ-LLM-003"],
        "test_unimplemented_transport_never_succeeds": ["REQ-LLM-004"],
    },
    "crates/application/src/lib.rs": {
        "test_health_reports_degraded_for_unavailable_storage": ["REQ-OPS-010"],
        "test_health_reports_ok_for_writable_storage": ["REQ-OPS-010"],
        "test_health_against_real_app_data_path": ["REQ-OPS-010"],
        "test_health_with_no_probes_is_not_ok": ["REQ-OPS-010"],
        "test_soak_measures_real_memory_growth": ["REQ-OPS-003"],
        "test_soak_does_not_fabricate_a_leak": ["REQ-OPS-003"],
        "test_git_sandbox_proof": ["REQ-SEC-010"],
    },
    "crates/platform_windows/src/lib.rs": {
        "test_app_paths": ["REQ-PLAT-001"],
        "test_current_rss_bytes_is_a_real_measurement": ["REQ-PLAT-003"],
    },
    "apps/desktop/src-tauri/src/commands.rs": {
        "test_record_conception_labels_human_origin": ["REQ-DOM-001"],
        "test_record_conception_labels_ai_origin_distinctly": ["REQ-DOM-002"],
        "test_record_conception_persists_through_the_vault": [
            "REQ-DATA-001",
            "REQ-DATA-002",
        ],
        "test_record_conception_persists_ai_origin_distinctly": ["REQ-DOM-002"],
        "test_namespace_status_covers_spec_003_and_is_honest": ["REQ-LLM-002"],
        "test_error_serializes_with_spec_006_class": ["REQ-OPS-001"],
        "test_command_result_serializes_envelope": ["REQ-LLM-003"],
        "test_lint_claims_uses_the_real_linter": ["REQ-PAT-001"],
        "test_build_filing_package_does_not_claim_a_document_format": ["REQ-PAT-002"],
        "test_import_receipt_parses_real_values": ["REQ-PAT-005"],
        "test_import_receipt_fails_closed_on_bad_input": ["REQ-PAT-005"],
        "test_filing_handoff_reports_blockers_and_never_automates": [
            "REQ-PAT-005",
            "REQ-REL-003",
        ],
        "test_research_lifecycle_happy_path": ["REQ-RES-001"],
        "test_research_rejects_illegal_transition": ["REQ-RES-001"],
        "test_research_citation_accumulates_and_is_refused_when_finished": ["REQ-RES-001"],
        "test_export_firewall_blocks_restricted_content": ["REQ-DOM-008"],
        "test_opportunity_requires_evidence_to_be_a_finding": ["REQ-DOM-003"],
        "test_docket_requires_filing_before_commercialization": ["REQ-DOM-007"],
        "test_office_action_refuses_allowance_response": ["REQ-PAT-004"],
        "test_commercialization_package_is_redacted": ["REQ-COM-001", "REQ-COM-004"],
        "test_provider_status_reports_no_configured_lane": ["REQ-LLM-004"],
        "test_local_inference_returns_no_text_when_no_model_is_served": ["REQ-LLM-001"],
        "test_local_inference_refuses_non_loopback_endpoint": [
            "REQ-LLM-001",
            "REQ-SEC-001",
        ],
        "test_mcp_capability_grant_is_explicit": ["REQ-MCP-001", "REQ-SEC-003"],
        "test_repair_capsule_redacts_secrets": ["REQ-REPAIR-001"],
        "test_export_gateway_hardens_paths_and_blocks_restricted": ["REQ-DOM-008"],
    },
    "apps/desktop/src-tauri/src/lib.rs": {
        "test_health_reflects_missing_storage_directory": ["REQ-OPS-010"],
        "test_health_reports_ok_for_writable_root": ["REQ-OPS-010"],
        "test_health_version_matches_crate_version": ["REQ-OPS-010"],
    },
    "packages/contracts/tests/validate.test.ts": {},
}

MARKER_RE = re.compile(r"///\s*covers:", re.IGNORECASE)


def main() -> int:
    changed = 0
    for rel, tests in BINDINGS.items():
        path = Path(rel)
        if not path.exists():
            print(f"SKIP missing file: {rel}", file=sys.stderr)
            continue
        lines = path.read_text("utf-8").splitlines(keepends=True)

        # Collect insertion points first, then apply them back-to-front so
        # earlier indices stay valid.
        insertions: list[tuple[int, str]] = []
        for i, line in enumerate(lines):
            m = re.match(r"\s*(?:pub\s+)?(?:async\s+)?fn\s+([a-z0-9_]+)\s*\(", line)
            if not m:
                continue
            reqs = tests.get(m.group(1))
            if not reqs:
                continue

            # Walk up past the whole attribute/doc block so the marker lands
            # ABOVE the test item. Inserting directly above the `fn` line puts
            # it inside the body when attributes span several lines, which rustc
            # rejects as an "unused doc comment".
            start = i
            j = i - 1
            while j >= 0:
                stripped = lines[j].strip()
                if stripped.startswith("#[") or stripped.startswith("///"):
                    start = j
                    j -= 1
                    continue
                break

            window = "".join(lines[max(0, start - 3) : i + 1])
            if MARKER_RE.search(window):
                continue

            indent = re.match(r"\s*", lines[start]).group(0)
            insertions.append((start, f"{indent}/// covers: {', '.join(reqs)}\n"))

        for index, marker in sorted(insertions, reverse=True):
            lines.insert(index, marker)
            changed += 1

        if insertions:
            path.write_text("".join(lines), "utf-8")
            print(f"{rel}: inserted {len(insertions)} marker(s)")

    print(f"total markers inserted: {changed}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
