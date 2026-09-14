#[derive(Debug, Clone)]
pub struct TargetCompany {
    pub name: String,
    pub valuation_match: f32,
}

pub struct DataRoom {
    pub targets: Vec<TargetCompany>,
    pub assets_redacted: bool,
}

impl Default for DataRoom {
    fn default() -> Self {
        Self::new()
    }
}

impl DataRoom {
    pub fn new() -> Self {
        DataRoom {
            targets: Vec::new(),
            assets_redacted: false,
        }
    }

    pub fn add_target(&mut self, name: &str, match_score: f32) {
        self.targets.push(TargetCompany {
            name: name.to_string(),
            valuation_match: match_score,
        });
    }

    pub fn redact_for_non_confidential_export(&mut self) {
        self.assets_redacted = true;
    }

    /// Produce a disclosure-safe go-to-market payload.
    ///
    /// GraphLock context (anti-gaming finding AG-006b): this previously returned
    /// the constant `"Pitch deck generated"`, discarding the room's targets
    /// entirely, so the export carried no information and every call produced
    /// byte-identical output regardless of content. It now renders the
    /// non-confidential target list it is given.
    ///
    /// REQ-COM-004 / REQ-UI-003: the output is deliberately labelled as draft
    /// planning material and asserts no valuation or outcome.
    pub fn export_pitch_deck(&self) -> Result<String, &'static str> {
        if !self.assets_redacted {
            return Err("Cannot export pitch deck without redacting confidential assets");
        }
        if self.targets.is_empty() {
            return Err("Cannot export pitch deck with zero targets");
        }
        let mut out = String::from(
            "DRAFT commercialization one-pager (planning material; not a valuation)\n\
             Targets (non-confidential):\n",
        );
        for target in &self.targets {
            out.push_str(&format!(
                "  - {} (adjacency score {:.2})\n",
                target.name, target.valuation_match
            ));
        }
        out.push_str("No confidential enabling detail is included.\n");
        Ok(out)
    }
}

#[cfg(test)]
mod commercialization_tests {
    use super::*;

    /// covers: REQ-COM-001, REQ-COM-004
    #[test]
    fn test_data_room_workflow() {
        let mut room = DataRoom::new();
        assert!(room.export_pitch_deck().is_err());

        room.add_target("MegaCorp", 0.95);
        assert!(room.export_pitch_deck().is_err()); // Not redacted yet

        room.redact_for_non_confidential_export();
        assert!(room.export_pitch_deck().is_ok());
    }
}

/// Outcome of a single UO live-fire attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UoRun {
    pub outcome_id: String,
    pub passed: bool,
    pub detail: String,
}

/// Orchestrates UO-01..12 live-fire runs.
///
/// GraphLock context (anti-gaming finding AG-006a): the previous
/// implementation set the success counter to 12 without running anything:
///
/// ```text
/// pub fn run_uo_live_fire(&mut self) -> Result<(), &'static str> {
///     // Simulating UO-01..12 live fire
///     self.completed_runs = 12;
///     Ok(())
/// }
/// ```
///
/// Its test asserted `completed_runs == 12`. That is a fabricated verification
/// result in the exact structure REQ-SHIP-001 depends on ("Release requires
/// UO-01..12 live-fire"), so a release gate reading this counter would have been
/// told live-fire passed when no run occurred.
///
/// This version records only runs that are actually reported to it. There is no
/// way to reach 12 without submitting 12 passing [`UoRun`] records, and a
/// failing run is retained rather than silently dropped.
pub struct LiveFireOrchestrator {
    pub runs: Vec<UoRun>,
    pub domain_regression_passed: bool,
}

impl Default for LiveFireOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl LiveFireOrchestrator {
    /// The canonical UO outcome IDs that release requires (SPEC-000).
    pub const REQUIRED_OUTCOMES: [&'static str; 12] = [
        "UO-01", "UO-02", "UO-03", "UO-04", "UO-05", "UO-06", "UO-07", "UO-08", "UO-09", "UO-10",
        "UO-11", "UO-12",
    ];

    pub fn new() -> Self {
        LiveFireOrchestrator {
            runs: Vec::new(),
            domain_regression_passed: false,
        }
    }

    /// Record the result of one live-fire attempt.
    ///
    /// This performs no execution itself: it is the ledger a real runner writes
    /// into. Calling it is the only way `completed_runs` changes.
    pub fn record_run(&mut self, outcome_id: &str, passed: bool, detail: &str) {
        self.runs.push(UoRun {
            outcome_id: outcome_id.to_string(),
            passed,
            detail: detail.to_string(),
        });
    }

    /// Count of distinct UO outcomes that actually passed.
    pub fn completed_runs(&self) -> usize {
        let mut passed: Vec<&str> = self
            .runs
            .iter()
            .filter(|r| r.passed)
            .map(|r| r.outcome_id.as_str())
            .collect();
        passed.sort_unstable();
        passed.dedup();
        passed.len()
    }

    /// Every required UO outcome with at least one passing run.
    pub fn missing_outcomes(&self) -> Vec<&'static str> {
        Self::REQUIRED_OUTCOMES
            .iter()
            .copied()
            .filter(|required| {
                !self
                    .runs
                    .iter()
                    .any(|r| r.passed && r.outcome_id == *required)
            })
            .collect()
    }

    pub fn verify_domain_regression(&mut self) -> Result<(), &'static str> {
        let missing = self.missing_outcomes();
        if !missing.is_empty() {
            return Err("Must complete UO-01..12 live fire before domain regression");
        }
        self.domain_regression_passed = true;
        Ok(())
    }
}

#[cfg(test)]
mod live_fire_tests {
    use super::*;

    #[test]
    fn test_uo_live_fire_and_regression() {
        let mut orchestrator = LiveFireOrchestrator::new();
        assert!(orchestrator.verify_domain_regression().is_err());

        for id in LiveFireOrchestrator::REQUIRED_OUTCOMES {
            orchestrator.record_run(id, true, "observed in live run");
        }
        assert_eq!(orchestrator.completed_runs(), 12);
        assert!(orchestrator.missing_outcomes().is_empty());

        assert!(orchestrator.verify_domain_regression().is_ok());
        assert!(orchestrator.domain_regression_passed);
    }

    /// covers: REQ-SHIP-001
    /// AG-006a regression: the counter must not reach 12 without 12 real runs.
    #[test]
    fn test_completed_runs_cannot_be_fabricated() {
        let mut orchestrator = LiveFireOrchestrator::new();
        assert_eq!(
            orchestrator.completed_runs(),
            0,
            "a fresh orchestrator must report zero completed runs"
        );
        assert_eq!(orchestrator.missing_outcomes().len(), 12);

        // A single recorded run moves the count by exactly one.
        orchestrator.record_run("UO-01", true, "partial");
        assert_eq!(orchestrator.completed_runs(), 1);
        assert_eq!(orchestrator.missing_outcomes().len(), 11);
    }

    /// covers: REQ-SHIP-001
    /// Failing runs must not count toward completion.
    #[test]
    fn test_failing_runs_do_not_count_as_completed() {
        let mut orchestrator = LiveFireOrchestrator::new();
        for id in LiveFireOrchestrator::REQUIRED_OUTCOMES {
            orchestrator.record_run(id, false, "failed in live run");
        }
        assert_eq!(orchestrator.completed_runs(), 0);
        assert!(orchestrator.verify_domain_regression().is_err());
        assert!(!orchestrator.domain_regression_passed);
    }

    /// covers: REQ-SHIP-001
    /// Duplicate passes for the same outcome count once.
    #[test]
    fn test_duplicate_runs_count_once() {
        let mut orchestrator = LiveFireOrchestrator::new();
        for _ in 0..5 {
            orchestrator.record_run("UO-01", true, "repeat");
        }
        assert_eq!(orchestrator.completed_runs(), 1);
        assert_eq!(orchestrator.missing_outcomes().len(), 11);
    }
}
