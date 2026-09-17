//! Abbreviated endurance ("soak") trial, labeled as such (DOD-038).
//!
//! DOD-038 RULE: "Required soak, endurance, fuzz, performance, stress, and
//! recovery durations/workloads are completed at their specified scale;
//! abbreviated trials are labeled separately."
//! OR ELSE: "Status is DEFERRED_LONG_RUNNING, EXTERNAL_REQUIRED, PARTIAL, or FAIL
//! -- never PASS for the full requirement."
//!
//! This test exists because the clause's own wording contemplates an abbreviated
//! trial, PROVIDED it is labeled separately. CORRECTED: this comment used to say the
//! repository specifies no soak duration or workload anywhere. That was WRONG -- the
//! pack's suite library does specify it. `.agent/verification/E2E_SUITE_LIBRARY.md`
//! carries the source of E2E-SoakResourceLeakTesting.md (the registry's own
//! `source_file` for E2E-018), which requires 24, 48, 72+ hours of sustained nominal
//! load with continuous telemetry and the flatline invariant, and forbids
//! time-bounded runners. That value is NOT completed here: what this run can honestly
//! do is execute a bounded endurance trial against the real write/read/recover paths,
//! label it ABBREVIATED in the evidence, record that the specified scale is
//! uncompleted, and let the clause stand as DEFERRED_LONG_RUNNING. It deliberately
//! does NOT claim the clause.
//!
//! WHAT IT EXERCISES CONTINUOUSLY
//!   * the production write path (`record_conception`) under one client key per
//!     cycle, so a duplicate would be caught rather than accumulated;
//!   * periodic full ledger read-back through an independent connection;
//!   * periodic backup + fault + restore cycles, so recovery runs under load
//!     instead of only in a quiet drill;
//!   * resource sampling: vault bytes and the process working set, sampled by an
//!     EXTERNAL observer (PowerShell) rather than self-reported.
//!
//! Duration is bounded by `LINCHPIN_SOAK_SECONDS` (default 600) and the heartbeat
//! interval by `LINCHPIN_SOAK_HEARTBEAT` (default 30). A short run and a long run
//! produce the same shape of evidence; the label is what differs, and the label is
//! derived from the duration actually achieved.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use linchpin_desktop_lib::commands::{
    backup_vault, list_conception_events, record_conception_keyed, restore_vault, WorkspaceScope,
};

/// Below this, the trial is reported as a SMOKE subset rather than an abbreviated
/// endurance run, so a short accidental run cannot be dressed up as a long one.
const ABBREVIATED_FLOOR_SECONDS: u64 = 300;
/// Growth bound: a vault that exceeds this per stored event is growing unboundedly.
const MAX_BYTES_PER_EVENT: u64 = 64 * 1024;
/// Default duration for an ordinary `cargo test` pass. Deliberately far below the
/// abbreviated floor: the unit lane must not spend ten minutes soaking, and a
/// short run is labelled and does NOT overwrite the evidence from a real trial.
const DEFAULT_SECONDS: u64 = 30;

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn unique_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("linchpin-soak-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

/// Working set of this process, sampled by an EXTERNAL observer.
///
/// Self-reported memory would be worth little, and this crate has no Windows API
/// dependency; asking the operating system through PowerShell keeps the number
/// independent of the code under test. A sample that cannot be taken is reported
/// as `null` rather than as zero.
fn working_set_bytes() -> Option<u64> {
    let output = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!("(Get-Process -Id {}).WorkingSet64", std::process::id()),
        ])
        .output()
        .ok()?;
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u64>()
        .ok()
}

fn vault_bytes(path: &Path) -> u64 {
    std::fs::metadata(path).map(|meta| meta.len()).unwrap_or(0)
}

/// The epoch the trial was run against, so the evidence states its provenance.
///
/// Without it a reader cannot tell which candidate the numbers describe; with it,
/// a source change makes the mismatch visible instead of leaving stale numbers
/// looking current.
fn epoch_digest() -> Option<String> {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.agent/verification/state/EPOCH.json");
    let text = std::fs::read_to_string(path).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&text).ok()?;
    parsed
        .get("epoch_digest")
        .and_then(|value| value.as_str())
        .map(str::to_string)
}

#[test]
fn abbreviated_endurance_trial_runs_the_real_paths_and_labels_itself() {
    let seconds = env_u64("LINCHPIN_SOAK_SECONDS", DEFAULT_SECONDS);
    let heartbeat = env_u64("LINCHPIN_SOAK_HEARTBEAT", 30);
    let label = if seconds >= ABBREVIATED_FLOOR_SECONDS {
        "ABBREVIATED"
    } else {
        "SMOKE_SUBSET"
    };

    let dir = unique_dir();
    let vault_path = dir.join("linchpin-vault.db");
    let backup_path = dir.join("soak-backup.db");
    let scope = WorkspaceScope {
        workspace_id: "ws-soak".to_string(),
    };
    let run_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let started_wall = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let start = Instant::now();
    let deadline = start + Duration::from_secs(seconds);
    let mut next_heartbeat = start + Duration::from_secs(heartbeat);

    let mut cycles: u64 = 0;
    let mut errors: u64 = 0;
    let mut replays: u64 = 0;
    let mut read_backs: u64 = 0;
    let mut recoveries: u64 = 0;
    let mut heartbeats: Vec<serde_json::Value> = Vec::new();

    while Instant::now() < deadline {
        cycles += 1;
        let key = format!("soak-{run_id}-{cycles}");
        let content = format!("soak cycle {cycles} for run {run_id}");

        let write = record_conception_keyed(&scope, &key, &content, true, Some(&vault_path));
        match write.value {
            Some(view) => {
                if view.replayed {
                    replays += 1;
                }
                if !view.persisted {
                    errors += 1;
                    eprintln!("cycle {cycles}: reported success without persisting");
                }
            }
            None => {
                errors += 1;
                eprintln!("cycle {cycles}: write failed: {:?}", write.error);
            }
        }

        // Periodic full read-back through the product's own read path.
        if cycles.is_multiple_of(20) {
            let ledger = list_conception_events(&scope, &vault_path);
            match ledger.value {
                Some(view) => {
                    read_backs += 1;
                    if view.count as u64 != cycles - replays {
                        errors += 1;
                        eprintln!(
                            "cycle {cycles}: ledger holds {} events but {} were written",
                            view.count,
                            cycles - replays
                        );
                    }
                }
                None => {
                    errors += 1;
                    eprintln!("cycle {cycles}: ledger read failed: {:?}", ledger.error);
                }
            }
        }

        // Periodic recovery under load: back up, destroy the file, restore, and
        // confirm the ledger reconciles afterwards.
        if cycles.is_multiple_of(50) {
            let backup = backup_vault(&scope, backup_path.to_str().unwrap(), &vault_path);
            match backup.value {
                None => {
                    errors += 1;
                    eprintln!("cycle {cycles}: backup failed: {:?}", backup.error);
                }
                Some(view) => {
                    let captured = view.state_digest;
                    std::fs::remove_file(&vault_path).ok();
                    for suffix in ["-wal", "-shm"] {
                        std::fs::remove_file(dir.join(format!("linchpin-vault.db{suffix}"))).ok();
                    }
                    let restored =
                        restore_vault(&scope, backup_path.to_str().unwrap(), &vault_path);
                    match restored.value {
                        Some(restored_view)
                            if restored_view.reconciled
                                && restored_view.digest_after == captured =>
                        {
                            recoveries += 1;
                        }
                        other => {
                            errors += 1;
                            eprintln!("cycle {cycles}: recovery did not reconcile: {other:?}");
                        }
                    }
                }
            }
        }

        if Instant::now() >= next_heartbeat {
            let elapsed = start.elapsed().as_secs();
            let heartbeat_sample = serde_json::json!({
                "elapsed_s": elapsed,
                "cycles": cycles,
                "errors": errors,
                "vault_bytes": vault_bytes(&vault_path),
                "working_set_bytes": working_set_bytes(),
            });
            println!(
                "soak heartbeat: t={elapsed}s cycles={cycles} errors={errors} vault={}B rss={:?}",
                heartbeat_sample["vault_bytes"], heartbeat_sample["working_set_bytes"]
            );
            heartbeats.push(heartbeat_sample);
            next_heartbeat = Instant::now() + Duration::from_secs(heartbeat);
        }
    }

    let actual_seconds = start.elapsed().as_secs();
    let events = list_conception_events(&scope, &vault_path)
        .value
        .map(|ledger| ledger.count as u64)
        .unwrap_or(0);
    let final_vault_bytes = vault_bytes(&vault_path);
    let bytes_per_event = final_vault_bytes
        .checked_div(events)
        .unwrap_or(final_vault_bytes);
    let first_rss = heartbeats
        .iter()
        .find_map(|sample| sample["working_set_bytes"].as_u64());
    let last_rss = heartbeats
        .iter()
        .rev()
        .find_map(|sample| sample["working_set_bytes"].as_u64());
    let rss_growth = match (first_rss, last_rss) {
        (Some(first), Some(last)) => Some(last.saturating_sub(first)),
        _ => None,
    };

    let report = serde_json::json!({
        "trial": label,
        "covers": ["DOD-038"],
        "harness": "apps/desktop/src-tauri/tests/soak_abbreviated.rs",
        "epoch_digest": epoch_digest(),
        "label_reason": if label == "ABBREVIATED" {
            format!("ran the requested {seconds}s, which is above the {ABBREVIATED_FLOOR_SECONDS}s floor for an abbreviated trial; the clause's specified full scale is 24/48/72+ hours (E2E-SoakResourceLeakTesting.md, referenced by the E2E-018 registry row) and is NOT completed by this run, so it stays labeled ABBREVIATED and DOD-038 stays DEFERRED_LONG_RUNNING")
        } else {
            format!("ran {actual_seconds}s, below the {ABBREVIATED_FLOOR_SECONDS}s floor; reported as a smoke subset rather than an endurance run")
        },
        "requested_seconds": seconds,
        "actual_seconds": actual_seconds,
        "heartbeat_interval_s": heartbeat,
        "started_unix": started_wall,
        "finished_unix": started_wall + actual_seconds,
        "workload": {
            "operation": "record_conception_keyed (production command) on one vault",
            "cycles": cycles,
            "periodic_ledger_readback_every": 20,
            "periodic_backup_fault_restore_every": 50,
            "read_backs": read_backs,
            "recoveries_under_load": recoveries,
            "unexpected_replays": replays,
        },
        "outcomes": {
            "errors": errors,
            "events_stored": events,
            "final_vault_bytes": final_vault_bytes,
            "bytes_per_event": bytes_per_event,
            "working_set_first_bytes": first_rss,
            "working_set_last_bytes": last_rss,
            "working_set_growth_bytes": rss_growth,
        },
        "heartbeats": heartbeats,
        "thresholds": {
            "max_bytes_per_event": MAX_BYTES_PER_EVENT,
            "errors_allowed": 0,
        },
        // CORRECTED CLAIM. This field used to read "no soak, endurance, fuzz or
        // stress duration/workload is specified anywhere in the repository, so
        // 'their specified scale' has no defined value to complete". That was
        // WRONG, and the error was in the measurement rather than in the wording:
        // the pack's own suite library DOES specify the scale for this clause's
        // soak term. `.agent/verification/E2E_SUITE_LIBRARY.md` carries the source
        // of E2E-SoakResourceLeakTesting.md (the source_file of registry row
        // E2E-018), which requires "an extended period (24, 48, 72+ hours)" of
        // sustained nominal load, continuous telemetry, and the flatline invariant,
        // and explicitly FORBIDS running it on time-bounded CI runners -- it
        // requires dedicated, persistent infrastructure. So the full-scale value
        // exists and is uncompleted, which is DEFERRED_LONG_RUNNING, not
        // "unspecified". What remains unspecified is the scale of the OTHER terms
        // (fuzz, stress, performance, recovery): no numeric duration or workload for
        // them appears in the repository or in the suite library.
        "specification_gap": "the soak term's scale IS specified by the pack: .agent/verification/E2E_SUITE_LIBRARY.md (source of E2E-018, E2E-SoakResourceLeakTesting.md) requires 24/48/72+ hours of sustained nominal load with continuous telemetry and requires dedicated persistent infrastructure, explicitly forbidding time-bounded runners. That value is NOT completed by this trial. No numeric scale is specified for the fuzz, stress, performance or recovery terms.",
        "not_claimed": "this run does NOT satisfy DOD-038 for the full requirement: it is an ABBREVIATED trial, labeled separately, and the clause's 24/48/72+ hour soak scale remains uncompleted (DEFERRED_LONG_RUNNING)",
    });

    let evidence = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.agent/evidence/soak");
    assert!(
        evidence
            .parent()
            .is_some_and(|parent| parent.ends_with(".agent/evidence")),
        "the evidence path escaped the repository: {}",
        evidence.display()
    );

    // A short run must NOT overwrite the evidence from a real trial. Measured
    // risk this closes: the test runs in the ordinary unit lane with a 30s
    // default, so writing unconditionally would replace an abbreviated trial's
    // heartbeats with a smoke subset's and silently degrade the evidence.
    if label != "ABBREVIATED" {
        println!(
            "soak: {label} run ({actual_seconds}s) -- evidence NOT written; the \
             abbreviated trial's report is preserved. Run with \
             LINCHPIN_SOAK_SECONDS={ABBREVIATED_FLOOR_SECONDS} or more to produce evidence."
        );
        std::fs::remove_dir_all(&dir).ok();
        assert_eq!(errors, 0, "the {label} run observed {errors} error(s)");
        return;
    }

    std::fs::create_dir_all(&evidence).expect("create evidence dir");
    std::fs::write(
        evidence.join("report.json"),
        serde_json::to_string_pretty(&report).expect("serialize") + "\n",
    )
    .expect("write report");
    std::fs::write(
        evidence.join("SUMMARY.md"),
        format!(
            "# DOD-038 abbreviated endurance trial ({label})\n\n\
             Generated by `apps/desktop/src-tauri/tests/soak_abbreviated.rs`. Do not hand-edit.\n\n\
             - Duration: **{actual_seconds}s** of a requested {seconds}s\n\
             - Cycles: **{cycles}** writes through the production command\n\
             - Ledger read-backs: {read_backs}; recovery cycles under load: {recoveries}\n\
             - Errors: **{errors}**; unexpected replays: {replays}\n\
             - Vault: {final_vault_bytes} bytes for {events} events ({bytes_per_event} bytes/event)\n\
             - Working set: {first_rss:?} -> {last_rss:?} ({rss_growth:?} growth)\n\n\
             **This is an ABBREVIATED trial and is labeled as such.** The clause's\n\
             soak scale IS specified by the pack -- `.agent/verification/E2E_SUITE_LIBRARY.md`\n\
             (source of E2E-018) requires 24/48/72+ hours of sustained nominal load with\n\
             continuous telemetry on dedicated persistent infrastructure, and forbids\n\
             time-bounded runners -- and that value is NOT completed here. Fuzz, stress,\n\
             performance and recovery have no numeric scale in the repository or the pack.\n\
             DOD-038 is therefore DEFERRED_LONG_RUNNING and this run does not claim it.\n"
        ),
    )
    .expect("write summary");

    println!(
        "soak: {label} trial finished after {actual_seconds}s, {cycles} cycles, {errors} error(s), \
         {events} events, {bytes_per_event} bytes/event"
    );

    assert_eq!(errors, 0, "the endurance trial observed {errors} error(s)");
    assert!(
        bytes_per_event < MAX_BYTES_PER_EVENT,
        "the vault grew to {bytes_per_event} bytes per event, over the {MAX_BYTES_PER_EVENT} bound"
    );
    assert!(
        events > 0 && events == cycles - replays,
        "the ledger holds {events} events for {cycles} cycles ({replays} replays)"
    );

    std::fs::remove_dir_all(&dir).ok();
}
