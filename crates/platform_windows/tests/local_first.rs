//! Local-first / offline acceptance tests.
//!
//! REQ-PLAT-002 and REQ-REL-002 are runtime claims: "the local-first core
//! remains functional with a local model and cached/user evidence, with no
//! mandatory network dependency for core workflows", and "offline/local-model
//! proof". A claim about what the product does when it runs cannot be
//! established by reading source, so these tests assert on the report produced
//! by an actual run of the packaged executable.
//!
//! The run is driven by `apps/desktop/e2e-local-provider.mjs` inside
//! `scripts/live-fire-local-provider.sh`, which launches the artifact over CDP,
//! invokes the product's own commands, and MEASURES the process's network
//! egress instead of inferring it from configuration.
//!
//! The test is deliberately not vacuous when the report is missing: an absent
//! report fails, naming the gate to run. A bound acceptance test that passes
//! because its evidence does not exist is the failure mode DOD-006 and DOD-024
//! exist to catch.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate lives two levels below the repository root")
        .to_path_buf()
}

/// Check titles the report MUST contain for the requirement to be covered.
///
/// Asserting only "every check passed" would pass vacuously if the local-first
/// checks were deleted from the harness, so the required titles are named here
/// as well. Removing one breaks this test.
const REQUIRED_CHECKS: [&str; 8] = [
    "core: resolved configuration is reported (REQ-PLAT-002)",
    "core: durable evidence written with no network (REQ-PLAT-002)",
    "core: disclosure screening runs on-device (REQ-PLAT-002)",
    "core: docket deadline resolves from a local ruleset (REQ-PLAT-002)",
    "core: valuation range computed on-device (REQ-PLAT-002)",
    "egress sampler observes connections (positive control, REQ-REL-002)",
    "core: no non-loopback egress during core workflows (REQ-PLAT-002, REQ-REL-002)",
    "core: user evidence persists to a device-local vault file (REQ-PLAT-002)",
];

fn report() -> serde_json::Value {
    // Written beside the human-readable STATUS.md by
    // apps/desktop/e2e-local-provider.mjs. An earlier revision of this test
    // looked in .agent/state/ and failed on a report that was written somewhere
    // else -- which is what the panic message is for.
    let path = repo_root().join(".agent/evidence/local-provider/report.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e}\nThe local-first proof has not been run for this \
             candidate. Run `sh scripts/live-fire-local-provider.sh` (it requires a \
             served loopback model, PF-011).",
            path.display()
        )
    });
    serde_json::from_str(&text).expect("local-first report is valid JSON")
}

/// covers: REQ-PLAT-002, REQ-REL-002
/// Local-first core workflows must work against a local model with no
/// non-loopback network dependency, and the claim must rest on a real run.
#[test]
fn test_local_first_core_works_with_a_local_model_and_no_network() {
    let data = report();

    let total = data["total"].as_u64().unwrap_or(0);
    let passed = data["passed"].as_u64().unwrap_or(0);
    assert!(total > 0, "the report contains no assertions");
    assert_eq!(
        passed, total,
        "the local-first run has failing assertions; see \
         .agent/evidence/local-provider/STATUS.md"
    );

    let checks = data["checks"]
        .as_array()
        .expect("the report carries a checks array");
    let titles: Vec<&str> = checks.iter().filter_map(|c| c["name"].as_str()).collect();

    // Re-derive the pass/fail verdict from the individual checks instead of
    // trusting the counters. Mutation-proven gap this closes: flipping one
    // check's `ok` to false while leaving `passed == total` intact passed the
    // earlier version, because only the counters were inspected and only a few
    // named checks were asserted. Every check must now report ok.
    let failed: Vec<&str> = checks
        .iter()
        .filter(|c| c["ok"] != true)
        .filter_map(|c| c["name"].as_str())
        .collect();
    assert!(
        failed.is_empty(),
        "the run contains failing assertions: {failed:?}"
    );

    for required in REQUIRED_CHECKS {
        assert!(
            titles.contains(&required),
            "required local-first assertion missing from the run: {required:?}"
        );
    }

    // The egress assertion is the substantive one: the product reached its
    // local model without opening a single non-loopback connection.
    let egress = checks
        .iter()
        .find(|c| {
            c["name"]
                .as_str()
                .map(|n| n.starts_with("core: no non-loopback egress"))
                .unwrap_or(false)
        })
        .expect("egress assertion present");
    assert_eq!(
        egress["ok"], true,
        "non-loopback egress was observed: {}",
        egress["observed"]
    );

    // Re-derive the verdict from the OBSERVED addresses instead of trusting the
    // harness's boolean. Without this, the assertion above only proves the
    // harness said "ok": a filter that silently accepted external addresses
    // would still report ok, and this test would pass. Parsing the evidence
    // makes the check independent of the harness's own judgement.
    let observed = egress["observed"].as_str().unwrap_or("");
    let addr_list = observed.rsplit(':').next().unwrap_or("");
    let addrs: Vec<&str> = addr_list
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && *s != "none")
        .collect();
    let not_loopback: Vec<&&str> = addrs
        .iter()
        .filter(|a| !(a.starts_with("127.") || **a == "::1" || **a == "0.0.0.0" || **a == "::"))
        .collect();
    assert!(
        not_loopback.is_empty(),
        "re-derived from the observed evidence, these are not loopback: \
         {not_loopback:?} (raw: {observed:?})"
    );
    // Positive control: a sample that saw nothing proves nothing.
    assert!(
        addrs.iter().any(|a| a.starts_with("127.") || *a == "::1"),
        "the egress sample observed no loopback endpoint, so its silence is \
         uninformative (raw: {observed:?})"
    );

    // The run must be bound to a real artifact, not to a source tree.
    let digest = data["executable_sha256"].as_str().unwrap_or("");
    assert_eq!(digest.len(), 64, "report must pin the executable digest");
    let exe = data["executable"].as_str().unwrap_or("");
    assert!(
        exe.ends_with(".exe"),
        "the proof must run the packaged executable, found {exe:?}"
    );
}

/// covers: REQ-PLAT-002
/// The offline proof must have run against a real local model, not an unset one.
#[test]
fn test_local_first_proof_names_a_real_local_model_and_endpoint() {
    let data = report();
    let endpoint = data["endpoint"].as_str().unwrap_or("");
    let model = data["model"].as_str().unwrap_or("");

    assert!(
        endpoint.contains("127.0.0.1") || endpoint.contains("localhost"),
        "the proof must target a loopback endpoint, found {endpoint:?}"
    );
    assert!(
        !model.is_empty() && model != "unset",
        "the proof must name the model it used, found {model:?}"
    );
}
