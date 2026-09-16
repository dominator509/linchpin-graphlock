//! Executed recovery drill with MEASURED objectives (DOD-036, REQ-REL-005).
//!
//! DOD-036 RULE: "Backup, restore, disaster recovery, hard-failure recovery,
//! RPO, RTO, and MTTR claims are executed against reconciled state where
//! applicable."
//! REQUIRED EVIDENCE: "Pre-disaster hash, fault injection, restore/recovery
//! logs, post-state reconciliation, and measured objectives."
//! OR ELSE: "Recovery readiness FAILS and stateful production release is NO_GO
//! when mandatory."
//!
//! The disposition for this clause previously said, correctly, that no
//! fault injection and no measured RPO/RTO/MTTR existed. Prose could not fix
//! that: an unmeasured objective is the thing the clause forbids. This test
//! therefore runs the drill and records what it measured, including the parts
//! that are unflattering.
//!
//! WHAT IS REAL HERE
//!   * state is written through the production command `record_conception`
//!     (the same function the Tauri IPC handler calls), not by poking tables;
//!   * faults are injected at the filesystem, which is where a disk failure
//!     actually happens: total loss of the vault file, and in-place corruption
//!     of a live vault;
//!   * recovery runs through the production command `restore_vault`;
//!   * the effect is read back through an INDEPENDENT connection and compared
//!     against the digest captured before the disaster (reconciliation).
//!
//! WHAT IS NOT PROVEN, STATED RATHER THAN IMPLIED
//!   * this is a single-machine, single-process drill with a small vault on
//!     local disk; it does not measure recovery across machines, across
//!     versions, or at production data volumes;
//!   * there is no off-device replication, so RPO after the LAST BACKUP is
//!     unbounded -- device loss loses everything committed since that backup.
//!     The measured RPO below is the RPO of the restore itself (zero loss of
//!     backed-up state), and the drill asserts that work committed after the
//!     backup IS lost, so nobody can read this report as continuous protection;
//!   * RTO/MTTR are the wall-clock time of the recovery command on this
//!     machine. They are not a service-level guarantee for other hardware.
//!
//! The report is written to `.agent/evidence/recovery-drill/report.json` so the
//! numbers exist as evidence rather than as a claim in a disposition string.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use linchpin_desktop_lib::commands::{
    backup_vault, record_conception, restore_vault, WorkspaceScope,
};
use storage::vault::Vault;

/// Events committed before the backup in each iteration: the workload.
const SEEDED_EVENTS: usize = 25;
/// Iterations. Each is a full disaster/recovery cycle with a fresh vault.
const ITERATIONS: usize = 5;
/// Automated threshold (DOD-022): a recovery that takes longer than this fails
/// the drill instead of being reported as an observation.
const RTO_THRESHOLD: Duration = Duration::from_secs(10);

fn unique_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!(
        "linchpin-recovery-drill-{label}-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

/// An unpredictable per-run marker, so a recovered database that merely LOOKS
/// right cannot satisfy the drill (DOD-013).
fn canary(iteration: usize) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("canary-{iteration}-{nanos}-{:x}", std::process::id())
}

struct Sample {
    iteration: usize,
    fault: &'static str,
    backup_ms: f64,
    rto_ms: f64,
    mttr_ms: f64,
    fault_detected: bool,
    reconciled: bool,
    destination_recreated: bool,
    quarantined: bool,
    lost_after_backup: usize,
    canary_events_recovered: usize,
    digest_before: String,
    digest_after: String,
}

fn sidecar(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "vault.db".to_string());
    name.push_str(suffix);
    path.with_file_name(name)
}

fn strip_sidecars(vault_path: &Path) {
    for suffix in ["-wal", "-shm"] {
        std::fs::remove_file(sidecar(vault_path, suffix)).ok();
    }
}

fn millis(duration: Duration) -> f64 {
    (duration.as_secs_f64() * 1000.0 * 100.0).round() / 100.0
}

fn run_iteration(iteration: usize, fault: &'static str) -> Sample {
    let dir = unique_dir(&format!("iter{iteration}"));
    let vault_path = dir.join("linchpin-vault.db");
    let backup_path = dir.join("linchpin-vault.backup.db");
    let scope = WorkspaceScope {
        workspace_id: format!("ws-drill-{iteration}"),
    };
    let marker = canary(iteration);

    // ---- pre-disaster state, written through the production command ----
    for index in 0..SEEDED_EVENTS {
        let content = if index == 0 {
            format!("{marker} inception disclosure")
        } else {
            format!("{marker} conception event {index}")
        };
        let outcome = record_conception(&scope, &content, true, Some(&vault_path));
        assert!(
            outcome.ok,
            "iteration {iteration}: record_conception failed: {:?}",
            outcome.error
        );
        assert!(
            outcome.value.as_ref().is_some_and(|v| v.persisted),
            "iteration {iteration}: event {index} was not persisted"
        );
    }

    // ---- backup, timed ----
    let backup_started = Instant::now();
    let backup = backup_vault(&scope, backup_path.to_str().unwrap(), &vault_path);
    let backup_ms = millis(backup_started.elapsed());
    assert!(
        backup.ok,
        "iteration {iteration}: backup failed: {:?}",
        backup.error
    );
    let digest_before = backup.value.unwrap().state_digest;

    // Work committed AFTER the backup. The drill asserts this is LOST, which is
    // what makes the RPO statement honest: the backup is a point in time, not
    // continuous protection.
    let after_backup = record_conception(
        &scope,
        &format!("{marker} work after the backup"),
        true,
        Some(&vault_path),
    );
    assert!(
        after_backup.ok,
        "iteration {iteration}: post-backup write failed"
    );

    // ---- fault injection ----
    strip_sidecars(&vault_path);
    match fault {
        "total_loss" => std::fs::remove_file(&vault_path).expect("remove vault"),
        "corruption" => {
            let mut bytes = std::fs::read(&vault_path).expect("read vault");
            for byte in bytes.iter_mut().take(4096) {
                *byte = 0x41;
            }
            std::fs::write(&vault_path, bytes).expect("corrupt vault");
        }
        other => panic!("unknown fault class {other}"),
    }
    // Detection: the fault must be OBSERVED, not assumed, or the drill would be
    // vacuous. The two classes are observed differently, and the difference is
    // itself a product fact worth recording: a deleted vault is observed by its
    // ABSENCE (opening the path would CREATE it, which is why the drill must not
    // probe with `Vault::open` here), while a corrupt vault is observed by
    // failing to open and digest it.
    let fault_detected = if fault == "total_loss" {
        !vault_path.exists()
    } else {
        Vault::open(&vault_path)
            .and_then(|v| v.state_digest())
            .is_err()
    };
    assert!(
        fault_detected,
        "iteration {iteration}: the injected {fault} fault was not observable, so the drill would be vacuous"
    );

    // ---- recovery, timed ----
    let recovery_started = Instant::now();
    let recovered = restore_vault(&scope, backup_path.to_str().unwrap(), &vault_path);
    let rto_ms = millis(recovery_started.elapsed());
    assert!(
        recovered.ok,
        "iteration {iteration}: recovery from {fault} failed: {:?}",
        recovered.error
    );
    let view = recovered.value.unwrap();

    // ---- independent read-back and reconciliation ----
    let reopened = Vault::open(&vault_path).expect("reopen the recovered vault");
    let digest_after = reopened.state_digest().expect("digest the recovered vault");
    let events = reopened
        .list_conception_events(&scope.workspace_id)
        .expect("list recovered events");
    let canary_events_recovered = events
        .iter()
        .filter(|e| e.content.contains(&marker))
        .count();
    let lost_after_backup = events
        .iter()
        .filter(|e| e.content.contains("work after the backup"))
        .count();

    let sample = Sample {
        iteration,
        fault,
        backup_ms,
        rto_ms,
        mttr_ms: rto_ms,
        fault_detected,
        reconciled: view.reconciled && digest_after == digest_before,
        destination_recreated: view.destination_recreated,
        quarantined: view.destination_quarantined.is_some(),
        lost_after_backup,
        canary_events_recovered,
        digest_before,
        digest_after,
    };

    std::fs::remove_dir_all(&dir).ok();
    sample
}

#[test]
fn recovery_drill_measures_rto_rpo_and_mttr_against_reconciled_state() {
    let faults = ["total_loss", "corruption"];
    let mut samples = Vec::new();
    for iteration in 0..ITERATIONS {
        samples.push(run_iteration(iteration, faults[iteration % faults.len()]));
    }

    // Automated pass/fail, not an observation.
    for sample in &samples {
        assert!(
            sample.reconciled,
            "iteration {} ({}) did not reconcile: before={} after={}",
            sample.iteration, sample.fault, sample.digest_before, sample.digest_after
        );
        assert_eq!(
            sample.canary_events_recovered, SEEDED_EVENTS,
            "iteration {} ({}) recovered {} of {} canary events",
            sample.iteration, sample.fault, sample.canary_events_recovered, SEEDED_EVENTS
        );
        assert_eq!(
            sample.lost_after_backup, 0,
            "iteration {} ({}) recovered work committed AFTER the backup, so the \
             backup was not a point in time",
            sample.iteration, sample.fault
        );
        assert!(
            sample.rto_ms < millis(RTO_THRESHOLD),
            "iteration {} ({}) RTO {:.2}ms exceeds the {:.0}ms threshold",
            sample.iteration,
            sample.fault,
            sample.rto_ms,
            millis(RTO_THRESHOLD)
        );
    }

    let mut rto: Vec<f64> = samples.iter().map(|s| s.rto_ms).collect();
    rto.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median_rto = rto[rto.len() / 2];
    let backup_median = {
        let mut v: Vec<f64> = samples.iter().map(|s| s.backup_ms).collect();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        v[v.len() / 2]
    };

    let report = serde_json::json!({
        "drill": "recovery-drill",
        "covers": ["REQ-REL-005", "DOD-036"],
        "harness": "apps/desktop/src-tauri/tests/recovery_drill.rs",
        "candidate_commit": option_env!("LINCHPIN_CANDIDATE").unwrap_or("unrecorded"),
        "workload": {
            "events_before_backup": SEEDED_EVENTS,
            "events_after_backup": 1,
            "iterations": ITERATIONS,
            "fault_classes": faults,
            "state_written_by": "commands::record_conception (production command)",
            "recovery_by": "commands::restore_vault (production command)",
            "read_back_by": "storage::vault::Vault via an independent connection",
        },
        "objectives_measured": {
            "rpo_backed_up_state_events_lost": 0,
            "rpo_after_last_backup": "unbounded -- no off-device replication; work committed after the last backup IS lost (asserted)",
            "rto_ms_threshold": millis(RTO_THRESHOLD),
            "rto_ms_median": median_rto,
            "rto_ms_min": rto.first().copied().unwrap_or(0.0),
            "rto_ms_max": rto.last().copied().unwrap_or(0.0),
            "mttr_ms_median": median_rto,
            "backup_ms_median": backup_median,
        },
        "samples": samples.iter().map(|s| serde_json::json!({
            "iteration": s.iteration,
            "fault": s.fault,
            "fault_detected": s.fault_detected,
            "backup_ms": s.backup_ms,
            "rto_ms": s.rto_ms,
            "mttr_ms": s.mttr_ms,
            "reconciled": s.reconciled,
            "destination_recreated": s.destination_recreated,
            "destination_quarantined": s.quarantined,
            "canary_events_recovered": s.canary_events_recovered,
            "events_lost_after_backup": s.lost_after_backup,
            "digest_before_disaster": s.digest_before,
            "digest_after_recovery": s.digest_after,
        })).collect::<Vec<_>>(),
        "limits": [
            "single machine, single process, small vault on local disk",
            "no cross-machine, cross-version, or production-volume measurement",
            "no off-device replication: RPO after the last backup is unbounded",
        ],
    });

    // Three levels up from apps/desktop/src-tauri reaches the repository root.
    // Two was wrong on the first run: the report was written to
    // apps/.agent/evidence/..., a path nothing reads, and the run still passed.
    let evidence =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.agent/evidence/recovery-drill");
    assert!(
        evidence
            .parent()
            .is_some_and(|p| p.ends_with(".agent/evidence")),
        "the evidence path escaped the repository: {}",
        evidence.display()
    );
    std::fs::create_dir_all(&evidence).expect("create evidence dir");
    let report_path = evidence.join("report.json");
    std::fs::write(
        &report_path,
        serde_json::to_string_pretty(&report).expect("serialize report") + "\n",
    )
    .expect("write report");
    println!(
        "recovery-drill: {ITERATIONS} cycles, median RTO {median_rto:.2}ms, \
         median backup {backup_median:.2}ms, report {}",
        report_path.display()
    );
}
