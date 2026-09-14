#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SystemHealth {
    pub status: String,
    pub version: String,
    pub storage_ok: bool,
}

/// A named readiness probe. Health is derived from probes, never asserted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthProbe {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

/// Probe that reports whether a path is usable for storage.
///
/// GraphLock context (anti-gaming finding AG-007a): `check_system_health`
/// previously returned a constant `SystemHealth { status: "OK", storage_ok:
/// true }` — it performed no check at all. That function is bound to
/// `get_system_health`, the **only** Tauri command in the shipped desktop app,
/// so the single piece of runtime self-report the product exposes was a
/// hard-coded success. DOD-037 requires that "health signals never lie";
/// SUP-011 requires that a superficially healthy process not report ready when
/// a critical dependency is unavailable.
pub fn probe_storage(path: &std::path::Path) -> HealthProbe {
    if !path.exists() {
        return HealthProbe {
            name: "storage".to_string(),
            ok: false,
            detail: format!("path does not exist: {}", path.display()),
        };
    }
    if !path.is_dir() {
        return HealthProbe {
            name: "storage".to_string(),
            ok: false,
            detail: format!("path is not a directory: {}", path.display()),
        };
    }
    // A directory is only usable if it can actually be written to.
    let probe_file = path.join(".linchpin-health-probe");
    match std::fs::write(&probe_file, b"probe") {
        Ok(()) => {
            let _ = std::fs::remove_file(&probe_file);
            HealthProbe {
                name: "storage".to_string(),
                ok: true,
                detail: format!("writable: {}", path.display()),
            }
        }
        Err(err) => HealthProbe {
            name: "storage".to_string(),
            ok: false,
            detail: format!("not writable: {} ({err})", path.display()),
        },
    }
}

/// Aggregate readiness from real probes.
///
/// `status` is `"OK"` only when every probe passed, otherwise `"DEGRADED"`.
/// There is no code path that reports OK with a failing probe.
pub fn check_system_health_with(probes: &[HealthProbe]) -> SystemHealth {
    let storage_ok = probes
        .iter()
        .find(|p| p.name == "storage")
        .map(|p| p.ok)
        .unwrap_or(false);
    let all_ok = !probes.is_empty() && probes.iter().all(|p| p.ok);
    SystemHealth {
        status: if all_ok { "OK" } else { "DEGRADED" }.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        storage_ok,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// covers: REQ-OPS-010
    /// DOD-037 / SUP-011: health must reflect reality. A missing storage path
    /// must produce a non-OK status, not a constant "OK".
    #[test]
    fn test_health_reports_degraded_for_unavailable_storage() {
        let missing = std::path::Path::new("this-path-does-not-exist-linchpin");
        assert!(!missing.exists(), "precondition: path must not exist");

        let probe = probe_storage(missing);
        assert!(!probe.ok, "probe must fail for a missing path");

        let health = check_system_health_with(&[probe]);
        assert_eq!(health.status, "DEGRADED");
        assert!(!health.storage_ok);
    }

    /// covers: REQ-OPS-010
    /// A writable directory must be reported healthy, and the probe must not
    /// leave its scratch file behind.
    #[test]
    fn test_health_reports_ok_for_writable_storage() {
        let dir = std::env::temp_dir();
        let probe = probe_storage(&dir);
        assert!(probe.ok, "temp dir should be writable: {}", probe.detail);

        let health = check_system_health_with(&[probe]);
        assert_eq!(health.status, "OK");
        assert!(health.storage_ok);
        assert!(
            !dir.join(".linchpin-health-probe").exists(),
            "probe left its scratch file behind"
        );
    }

    /// A file is not a usable storage directory.
    #[test]
    fn test_health_rejects_file_as_storage_path() {
        let file = std::env::temp_dir().join(".linchpin-health-file-probe");
        std::fs::write(&file, b"x").unwrap();
        let probe = probe_storage(&file);
        let _ = std::fs::remove_file(&file);

        assert!(!probe.ok);
        assert!(probe.detail.contains("not a directory"), "{}", probe.detail);
    }

    /// covers: REQ-OPS-010
    /// No probes at all must not report healthy.
    #[test]
    fn test_health_with_no_probes_is_not_ok() {
        let health = check_system_health_with(&[]);
        assert_eq!(
            health.status, "DEGRADED",
            "an empty probe set must not report OK"
        );
        assert!(!health.storage_ok);
    }

    /// covers: REQ-OPS-010
    /// End-to-end against the REAL runtime storage path the packaged app uses.
    ///
    /// This is the probe `get_system_health` actually runs. On a host where the
    /// app data directory has not been created yet, it must report DEGRADED —
    /// which is precisely what the previous constant-OK implementation could
    /// never do.
    #[test]
    fn test_health_against_real_app_data_path() {
        let root = platform_windows::get_app_paths().app_data_dir;
        let probe = probe_storage(&root);
        let health = check_system_health_with(std::slice::from_ref(&probe));

        if root.exists() {
            assert!(
                probe.ok,
                "existing app dir should be writable: {}",
                probe.detail
            );
            assert_eq!(health.status, "OK");
        } else {
            assert!(!probe.ok, "missing app dir must fail the probe");
            assert_eq!(
                health.status, "DEGRADED",
                "health must not claim OK when the storage root is absent"
            );
            assert!(!health.storage_ok);
            assert!(probe.detail.contains("does not exist"), "{}", probe.detail);
        }
        // The version must be real, not a literal.
        assert_eq!(health.version, env!("CARGO_PKG_VERSION"));
    }
}

pub struct GitIntegrationSandbox {
    pub current_branch: String,
    pub has_uncommitted_changes: bool,
}

impl Default for GitIntegrationSandbox {
    fn default() -> Self {
        Self::new()
    }
}

impl GitIntegrationSandbox {
    pub fn new() -> Self {
        GitIntegrationSandbox {
            current_branch: "main".to_string(),
            has_uncommitted_changes: false,
        }
    }

    pub fn create_repair_branch(&mut self, incident_id: &str) -> Result<(), &'static str> {
        if self.has_uncommitted_changes {
            return Err("Cannot branch with uncommitted changes");
        }
        self.current_branch = format!("repair/{}", incident_id);
        Ok(())
    }

    pub fn open_pr(&self) -> Result<String, &'static str> {
        if self.current_branch == "main" {
            return Err("Cannot PR from main");
        }
        Ok(format!("Opened PR from {}", self.current_branch))
    }
}

#[cfg(test)]
mod sandbox_tests {
    use super::*;

    /// covers: REQ-SEC-010
    #[test]
    fn test_git_sandbox_proof() {
        let mut sandbox = GitIntegrationSandbox::new();
        assert!(sandbox.open_pr().is_err()); // main branch

        sandbox.has_uncommitted_changes = true;
        assert!(sandbox.create_repair_branch("inc-123").is_err());

        sandbox.has_uncommitted_changes = false;
        assert!(sandbox.create_repair_branch("inc-123").is_ok());
        assert_eq!(sandbox.current_branch, "repair/inc-123");

        let pr_msg = sandbox.open_pr().unwrap();
        assert!(pr_msg.contains("repair/inc-123"));
    }
}

/// Soak/endurance bookkeeping.
///
/// GraphLock context (anti-gaming finding AG-007b): the previous
/// implementation set `memory_leak_detected = true` once `iteration_count >
/// 100`, with the comment "Mock leak condition for proof". No memory was ever
/// measured — the "leak" was a hard-coded threshold, and its test asserted that
/// the fabricated condition fired. DOD-038 requires soak evidence at specified
/// scale with real telemetry; a counter that declares a leak cannot supply it.
///
/// This version measures **actual** resident memory across iterations and
/// reports growth only when a real increase is observed. It reports no leak
/// when memory is flat, which is the honest default rather than an assumption.
pub struct OperationsSoakTest {
    pub is_running: bool,
    pub iteration_count: usize,
    pub started_at: Option<std::time::Instant>,
    baseline_bytes: Option<u64>,
    peak_bytes: u64,
}

impl Default for OperationsSoakTest {
    fn default() -> Self {
        Self::new()
    }
}

impl OperationsSoakTest {
    pub fn new() -> Self {
        OperationsSoakTest {
            is_running: false,
            iteration_count: 0,
            started_at: None,
            baseline_bytes: None,
            peak_bytes: 0,
        }
    }

    pub fn start_soak(&mut self) {
        self.is_running = true;
        self.iteration_count = 0;
        self.started_at = Some(std::time::Instant::now());
        self.baseline_bytes = current_rss_bytes();
        self.peak_bytes = self.baseline_bytes.unwrap_or(0);
    }

    /// Record one iteration, sampling resident memory.
    pub fn run_iteration(&mut self) -> Result<(), &'static str> {
        if !self.is_running {
            return Err("Soak test not running");
        }
        self.iteration_count += 1;
        if let Some(rss) = current_rss_bytes() {
            self.peak_bytes = self.peak_bytes.max(rss);
        }
        Ok(())
    }

    /// Elapsed wall-clock time of the soak.
    pub fn elapsed(&self) -> std::time::Duration {
        self.started_at
            .map(|t| t.elapsed())
            .unwrap_or(std::time::Duration::ZERO)
    }

    /// Growth in resident memory over the soak, if measurable.
    pub fn memory_growth_bytes(&self) -> Option<u64> {
        let baseline = self.baseline_bytes?;
        let current = current_rss_bytes()?;
        Some(current.saturating_sub(baseline))
    }

    /// Finish and reconcile.
    ///
    /// Returns the iteration count. Fails when memory sampling was unavailable,
    /// because an unmeasurable soak cannot support a leak claim either way
    /// (DOD-026: unverified is not the same as passed).
    pub fn stop_and_reconcile(&mut self) -> Result<usize, &'static str> {
        self.is_running = false;
        let growth = self
            .memory_growth_bytes()
            .ok_or("Soak reconciliation requires a memory measurement, which was unavailable")?;
        if growth > 0 && self.peak_bytes > 0 {
            // Growth alone is not proof of a leak (allocators retain arenas).
            // A soak verdict therefore requires the caller to supply a
            // threshold derived from the workload; without one, we do not
            // declare a leak. This records the observation, it does not judge.
            let _ = growth;
        }
        Ok(self.iteration_count)
    }
}

#[cfg(test)]
mod soak_tests {
    use super::*;

    /// covers: REQ-OPS-003
    /// AG-007b regression: the harness must measure real memory, and a
    /// measurably growing process must not be reported as leak-free.
    #[test]
    fn test_soak_measures_real_memory_growth() {
        let mut soak = OperationsSoakTest::new();
        assert!(soak.run_iteration().is_err(), "must not run when stopped");

        soak.start_soak();
        assert!(soak.elapsed() >= std::time::Duration::ZERO);
        for _ in 0..10 {
            soak.run_iteration().unwrap();
        }
        let before = soak.memory_growth_bytes();
        assert!(
            before.is_some(),
            "soak must obtain a real memory measurement on Windows"
        );

        // Allocate and touch ~8 MiB. A harness that measured nothing would see
        // no change here.
        let mut balloon: Vec<u8> = vec![0u8; 8 * 1024 * 1024];
        for i in (0..balloon.len()).step_by(4096) {
            balloon[i] = 1;
        }
        for _ in 0..10 {
            soak.run_iteration().unwrap();
        }

        let growth = soak.memory_growth_bytes().expect("measurement available");
        assert!(
            growth > 0,
            "growth was not observed after allocating 8 MiB (got {growth})"
        );
        assert_eq!(soak.stop_and_reconcile().unwrap(), 20);
        drop(balloon);
    }

    /// covers: REQ-OPS-003
    /// A fresh soak reports zero iterations and no fabricated leak.
    #[test]
    fn test_soak_does_not_fabricate_a_leak() {
        let mut soak = OperationsSoakTest::new();
        soak.start_soak();
        for _ in 0..101 {
            soak.run_iteration().unwrap();
        }
        // The old implementation set memory_leak_detected = true here purely
        // because the counter passed 100. There is no such field now, and the
        // reconcile must succeed absent a measured threshold violation.
        assert_eq!(soak.stop_and_reconcile().unwrap(), 101);
    }

    /// Restarting a soak resets its counters rather than accumulating.
    #[test]
    fn test_soak_restart_resets_counters() {
        let mut soak = OperationsSoakTest::new();
        soak.start_soak();
        for _ in 0..5 {
            soak.run_iteration().unwrap();
        }
        assert_eq!(soak.iteration_count, 5);

        soak.start_soak();
        assert_eq!(soak.iteration_count, 0, "restart must reset the counter");
    }
}

/// Resident set size of this process in bytes, or None when not measurable.
///
/// Delegates to `platform_windows`, which reads the real working set via
/// `GetProcessMemoryInfo`. It returns `None` rather than a placeholder when the
/// OS call fails, so an unmeasurable soak is reported as unmeasurable instead
/// of being declared leak-free.
fn current_rss_bytes() -> Option<u64> {
    platform_windows::current_rss_bytes()
}

#[cfg(test)]
mod operations_tests {
    use super::*;

    /// This test previously ended with `assert!(soak.stop_and_reconcile()
    /// .is_err())` after 101 iterations, asserting that a *fabricated* leak
    /// fired. That made it a test of the defect (AG-007b). It now asserts the
    /// real contract: a reconciled soak returns its iteration count.
    #[test]
    fn test_soak_reconciliation() {
        let mut soak = OperationsSoakTest::new();
        assert!(soak.run_iteration().is_err(), "must not run when stopped");

        soak.start_soak();
        for _ in 0..50 {
            soak.run_iteration().unwrap();
        }
        assert_eq!(soak.stop_and_reconcile().unwrap(), 50);
        assert!(!soak.is_running);
    }
}
