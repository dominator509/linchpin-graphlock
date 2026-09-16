//! Performance gate: encoded thresholds against a defined workload (DOD-022).
//!
//! DOD-022 RULE: "Performance, resource, cost, and SLO requirements are encoded
//! as automated pass/fail thresholds against a defined workload and
//! environment."
//! REQUIRED EVIDENCE: "Workload model, environment, samples, percentiles,
//! resource metrics, thresholds, and verdict."
//! OR ELSE: "The nonfunctional claim is UNVERIFIED and a mandatory SLO failure
//! is NO_GO."
//!
//! The recorded disposition read FAIL because no threshold existed anywhere: the
//! only timing numbers in the repository were observations inside the recovery
//! drill's report, which nothing enforced. An observation is not a gate.
//!
//! WHAT IS DEFINED HERE
//!   * WORKLOAD: `WRITES` conception events written one at a time through the
//!     production command `record_conception`, each with a runtime canary, into a
//!     fresh file-backed vault; then every event read back through an independent
//!     connection.
//!   * THRESHOLDS: p50, p95 and worst-case write latency, total write wall clock,
//!     read latency, and the resulting vault size. Each FAILS the gate.
//!   * ENVIRONMENT: recorded in the report (OS, architecture, profile) rather
//!     than assumed.
//!
//! WHAT THIS IS NOT, STATED PLAINLY
//!   * The measured profile is `test` (debug) unless the suite is run in release.
//!     The numbers are therefore **regression bounds on this host**, not published
//!     service levels: they exist to fail a change that makes the vault an order
//!     of magnitude slower, not to promise a latency to a customer.
//!   * It is a single machine, single process, local-disk workload. No
//!     concurrency, no network, no multi-user capacity claim.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use linchpin_desktop_lib::commands::{record_conception, WorkspaceScope};
use storage::vault::Vault;

/// Defined workload: events written one at a time through the product command.
const WRITES: usize = 200;

// --- encoded thresholds ---------------------------------------------------
/// Median write latency. Fail above this.
const P50_WRITE_MS: f64 = 50.0;
/// 95th percentile write latency. Fail above this.
const P95_WRITE_MS: f64 = 150.0;
/// Worst single write. Fail above this.
const MAX_WRITE_MS: f64 = 500.0;
/// Total wall clock for the whole write workload.
const TOTAL_WRITE_MS: f64 = 30_000.0;
/// Reading the whole workspace back through a second connection.
const TOTAL_READ_MS: f64 = 2_000.0;
/// A 200-event vault must not exceed this. Catches accidental quadratic growth.
const MAX_VAULT_BYTES: u64 = 32 * 1024 * 1024;

fn unique_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("linchpin-perf-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

fn millis(duration: Duration) -> f64 {
    (duration.as_secs_f64() * 1000.0 * 100.0).round() / 100.0
}

fn percentile(sorted: &[f64], fraction: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let index = ((sorted.len() as f64 - 1.0) * fraction).round() as usize;
    sorted[index.min(sorted.len() - 1)]
}

fn profile() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}

fn environment() -> serde_json::Value {
    serde_json::json!({
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "profile": profile(),
        "note": "single machine, single process, local disk; thresholds are regression bounds on this environment, not published service levels",
    })
}

#[test]
fn performance_thresholds_hold_for_the_defined_workload() {
    let dir = unique_dir();
    let vault_path = dir.join("linchpin-vault.db");
    let scope = WorkspaceScope {
        workspace_id: "ws-perf".to_string(),
    };
    let run = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    // --- write workload ---------------------------------------------------
    let write_started = Instant::now();
    let mut samples: Vec<f64> = Vec::with_capacity(WRITES);
    for index in 0..WRITES {
        let content = format!("perf-canary-{run}-{index}");
        let started = Instant::now();
        let outcome = record_conception(&scope, &content, true, Some(&vault_path));
        let elapsed = millis(started.elapsed());
        assert!(outcome.ok, "write {index} failed: {:?}", outcome.error);
        assert!(
            outcome.value.as_ref().is_some_and(|v| v.persisted),
            "write {index} was not persisted"
        );
        samples.push(elapsed);
    }
    let total_write_ms = millis(write_started.elapsed());

    let mut sorted = samples.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p50 = percentile(&sorted, 0.50);
    let p95 = percentile(&sorted, 0.95);
    let worst = sorted.last().copied().unwrap_or(0.0);

    // --- read workload, through an independent connection -----------------
    let read_started = Instant::now();
    let read_back = Vault::open(&vault_path)
        .expect("reopen vault")
        .list_conception_events("ws-perf")
        .expect("list events");
    let total_read_ms = millis(read_started.elapsed());
    assert_eq!(
        read_back.len(),
        WRITES,
        "the read workload did not observe every write"
    );

    let vault_bytes = std::fs::metadata(&vault_path).map(|m| m.len()).unwrap_or(0);

    let report = serde_json::json!({
        "gate": "performance",
        "covers": ["DOD-022"],
        "harness": "apps/desktop/src-tauri/tests/performance_gate.rs",
        "workload": {
            "operation": "record_conception (production command) then list_conception_events through a second connection",
            "events": WRITES,
            "run_canary": format!("perf-canary-{run}"),
        },
        "environment": environment(),
        "samples_ms": {
            "count": samples.len(),
            "min": sorted.first().copied().unwrap_or(0.0),
            "p50": p50,
            "p95": p95,
            "max": worst,
            "total_write_ms": total_write_ms,
            "total_read_ms": total_read_ms,
        },
        "resources": { "vault_bytes": vault_bytes },
        "thresholds": {
            "p50_write_ms": P50_WRITE_MS,
            "p95_write_ms": P95_WRITE_MS,
            "max_write_ms": MAX_WRITE_MS,
            "total_write_ms": TOTAL_WRITE_MS,
            "total_read_ms": TOTAL_READ_MS,
            "max_vault_bytes": MAX_VAULT_BYTES,
        },
        "verdict": if p50 <= P50_WRITE_MS
            && p95 <= P95_WRITE_MS
            && worst <= MAX_WRITE_MS
            && total_write_ms <= TOTAL_WRITE_MS
            && total_read_ms <= TOTAL_READ_MS
            && vault_bytes <= MAX_VAULT_BYTES
        { "PASS" } else { "FAIL" },
        "limits": [
            "single machine, single process, no concurrency",
            "debug profile unless the suite runs in release",
            "thresholds are regression bounds, not customer-facing SLOs",
        ],
    });

    let evidence =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.agent/evidence/performance");
    assert!(
        evidence
            .parent()
            .is_some_and(|p| p.ends_with(".agent/evidence")),
        "the evidence path escaped the repository: {}",
        evidence.display()
    );
    std::fs::create_dir_all(&evidence).expect("create evidence dir");
    std::fs::write(
        evidence.join("report.json"),
        serde_json::to_string_pretty(&report).expect("serialize") + "\n",
    )
    .expect("write report");

    // --- the gate itself --------------------------------------------------
    assert!(
        p50 <= P50_WRITE_MS,
        "p50 write {p50}ms exceeds the {P50_WRITE_MS}ms threshold"
    );
    assert!(
        p95 <= P95_WRITE_MS,
        "p95 write {p95}ms exceeds the {P95_WRITE_MS}ms threshold"
    );
    assert!(
        worst <= MAX_WRITE_MS,
        "worst write {worst}ms exceeds the {MAX_WRITE_MS}ms threshold"
    );
    assert!(
        total_write_ms <= TOTAL_WRITE_MS,
        "{WRITES} writes took {total_write_ms}ms, over the {TOTAL_WRITE_MS}ms threshold"
    );
    assert!(
        total_read_ms <= TOTAL_READ_MS,
        "reading {WRITES} events took {total_read_ms}ms, over the {TOTAL_READ_MS}ms threshold"
    );
    assert!(
        vault_bytes <= MAX_VAULT_BYTES,
        "the vault reached {vault_bytes} bytes for {WRITES} events, over the {MAX_VAULT_BYTES}-byte threshold"
    );

    println!(
        "performance: {} writes p50 {p50}ms p95 {p95}ms max {worst}ms total {total_write_ms}ms, \
         read {total_read_ms}ms, vault {vault_bytes} bytes, profile {}",
        WRITES,
        profile()
    );

    std::fs::remove_dir_all(&dir).ok();
}
