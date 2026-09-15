#[derive(Debug, Clone)]
pub struct TargetCompany {
    pub name: String,
    pub valuation_match: f32,
}

/// One explicit assumption behind a valuation range (REQ-COM-003).
#[derive(Debug, Clone, PartialEq)]
pub struct Assumption {
    pub name: String,
    pub low: f64,
    pub high: f64,
}

impl Assumption {
    pub fn new(name: &str, low: f64, high: f64) -> Result<Self, ValuationError> {
        if name.trim().is_empty() {
            return Err(ValuationError::BlankAssumptionName);
        }
        if low > high {
            return Err(ValuationError::InvertedAssumption(name.to_string()));
        }
        Ok(Assumption {
            name: name.to_string(),
            low,
            high,
        })
    }

    /// How much this assumption moves the outcome. This is what makes the
    /// sensitivity table meaningful rather than decorative.
    pub fn spread(&self) -> f64 {
        (self.high - self.low).abs()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValuationError {
    /// "ranges tied to explicit assumptions" -- a range with no assumption
    /// behind it is an unsourced number.
    NoAssumptions,
    /// The range itself is inverted.
    InvertedRange,
    BlankAssumptionName,
    InvertedAssumption(String),
    /// "not certified appraisals" -- language that asserts appraisal authority.
    NotAPlanningScenario(String),
}

impl std::fmt::Display for ValuationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValuationError::NoAssumptions => write!(
                f,
                "a valuation range requires at least one explicit assumption"
            ),
            ValuationError::InvertedRange => write!(f, "a valuation range cannot be inverted"),
            ValuationError::BlankAssumptionName => write!(f, "an assumption must be named"),
            ValuationError::InvertedAssumption(n) => {
                write!(f, "assumption {n:?} has a low bound above its high bound")
            }
            ValuationError::NotAPlanningScenario(label) => write!(
                f,
                "valuation label {label:?} asserts appraisal authority; \
                 outputs must be labelled planning scenarios"
            ),
        }
    }
}

/// Phrases that assert appraisal or certification authority.
const APPRAISAL_LANGUAGE: [&str; 6] = [
    "certified",
    "certificate of value",
    "appraisal",
    "appraised",
    "audited valuation",
    "guaranteed",
];

/// A valuation output: a range, its explicit assumptions, and a scenario label
/// (REQ-COM-003).
///
/// REQ-COM-003: "Valuation outputs are ranges tied to explicit assumptions and
/// labelled planning scenarios, not certified appraisals. Sensitivity tables
/// show which assumptions dominate." All three clauses are enforced here:
///
///  * the value is a RANGE, and it cannot be inverted;
///  * it must carry at least one explicit assumption, so a bare number cannot be
///    produced;
///  * its label must not assert appraisal or certification authority -- the same
///    fabricated-certainty failure REQ-UI-003 guards in product copy, applied to
///    the number itself;
///  * `sensitivity()` ranks assumptions by how much they move the outcome, so
///    "which assumptions dominate" is answered rather than asserted.
#[derive(Debug, Clone, PartialEq)]
pub struct ValuationRange {
    pub low: f64,
    pub high: f64,
    pub assumptions: Vec<Assumption>,
    pub scenario_label: String,
}

impl ValuationRange {
    pub fn planning_scenario(
        scenario_label: &str,
        low: f64,
        high: f64,
        assumptions: Vec<Assumption>,
    ) -> Result<Self, ValuationError> {
        let lowered = scenario_label.to_lowercase();
        if let Some(bad) = APPRAISAL_LANGUAGE
            .iter()
            .find(|phrase| lowered.contains(*phrase))
        {
            return Err(ValuationError::NotAPlanningScenario(bad.to_string()));
        }
        if assumptions.is_empty() {
            return Err(ValuationError::NoAssumptions);
        }
        if low > high {
            return Err(ValuationError::InvertedRange);
        }
        Ok(ValuationRange {
            low,
            high,
            assumptions,
            scenario_label: scenario_label.to_string(),
        })
    }

    /// True when the output is a range rather than a point estimate.
    pub fn is_a_range(&self) -> bool {
        self.high > self.low
    }

    /// Sensitivity table: assumptions ranked by how much they move the outcome,
    /// most influential first. Ties keep insertion order so the table is stable.
    pub fn sensitivity(&self) -> Vec<(String, f64)> {
        let mut ranked: Vec<(usize, String, f64)> = self
            .assumptions
            .iter()
            .enumerate()
            .map(|(i, a)| (i, a.name.clone(), a.spread()))
            .collect();
        ranked.sort_by(|a, b| {
            b.2.partial_cmp(&a.2)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.0.cmp(&b.0))
        });
        ranked.into_iter().map(|(_, n, s)| (n, s)).collect()
    }

    /// The single assumption that dominates the outcome, if any.
    pub fn dominant_assumption(&self) -> Option<&Assumption> {
        self.assumptions.iter().max_by(|a, b| {
            a.spread()
                .partial_cmp(&b.spread())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }
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

#[cfg(test)]
mod valuation_tests {
    use super::*;

    fn assumptions() -> Vec<Assumption> {
        vec![
            Assumption::new("market size", 100.0, 400.0).unwrap(),
            Assumption::new("royalty rate", 0.02, 0.05).unwrap(),
            Assumption::new("remaining life", 8.0, 12.0).unwrap(),
        ]
    }

    /// covers: REQ-COM-003
    /// "Valuation outputs are ranges tied to explicit assumptions."
    #[test]
    fn test_valuation_is_a_range_tied_to_explicit_assumptions() {
        let v = ValuationRange::planning_scenario(
            "Conservative planning scenario",
            1_000_000.0,
            2_500_000.0,
            assumptions(),
        )
        .expect("a well-formed planning scenario");
        assert!(v.is_a_range(), "the output must be a range, not a point");
        assert_eq!(v.assumptions.len(), 3);
        assert_eq!(v.low, 1_000_000.0);
        assert_eq!(v.high, 2_500_000.0);
    }

    /// covers: REQ-COM-003
    /// A range with no assumption behind it is an unsourced number, and an
    /// inverted range is not a range at all.
    #[test]
    fn test_valuation_requires_assumptions_and_a_sane_range() {
        assert_eq!(
            ValuationRange::planning_scenario("Planning scenario", 1.0, 2.0, Vec::new()),
            Err(ValuationError::NoAssumptions),
            "a valuation with no explicit assumption must be refused"
        );
        assert_eq!(
            ValuationRange::planning_scenario("Planning scenario", 5.0, 1.0, assumptions()),
            Err(ValuationError::InvertedRange)
        );
        // A single point is permitted but is NOT a range, so callers can tell.
        let point = ValuationRange::planning_scenario("Planning scenario", 3.0, 3.0, assumptions())
            .unwrap();
        assert!(!point.is_a_range());
    }

    /// covers: REQ-COM-003
    /// "not certified appraisals" -- a label asserting appraisal authority is
    /// refused rather than stored.
    #[test]
    fn test_valuation_label_must_not_assert_appraisal_authority() {
        for bad in [
            "Certified valuation",
            "Independent appraisal",
            "Audited valuation result",
            "Guaranteed return scenario",
            "APPRAISED value",
        ] {
            assert_eq!(
                ValuationRange::planning_scenario(bad, 1.0, 2.0, assumptions()),
                Err(ValuationError::NotAPlanningScenario(
                    APPRAISAL_LANGUAGE
                        .iter()
                        .find(|p| bad.to_lowercase().contains(*p))
                        .unwrap()
                        .to_string()
                )),
                "label {bad:?} should have been refused"
            );
        }
        // An honest planning label is accepted.
        assert!(
            ValuationRange::planning_scenario(
                "Illustrative planning scenario only",
                1.0,
                2.0,
                assumptions()
            )
            .is_ok()
        );
    }

    /// covers: REQ-COM-003
    /// "Sensitivity tables show which assumptions dominate."
    #[test]
    fn test_sensitivity_ranks_the_dominant_assumption_first() {
        let v = ValuationRange::planning_scenario("Planning scenario", 1.0, 2.0, assumptions())
            .unwrap();

        let table = v.sensitivity();
        assert_eq!(table.len(), 3);
        // market size spans 300, remaining life 4, royalty rate 0.03.
        assert_eq!(table[0].0, "market size");
        assert_eq!(table[0].1, 300.0);
        assert_eq!(table[1].0, "remaining life");
        assert_eq!(table[2].0, "royalty rate");

        let dominant = v.dominant_assumption().expect("a dominant assumption");
        assert_eq!(dominant.name, "market size");

        // Ties are stable: insertion order wins.
        let tied = ValuationRange::planning_scenario(
            "Planning scenario",
            0.0,
            1.0,
            vec![
                Assumption::new("first", 0.0, 1.0).unwrap(),
                Assumption::new("second", 0.0, 1.0).unwrap(),
            ],
        )
        .unwrap();
        assert_eq!(tied.sensitivity()[0].0, "first");
    }

    /// covers: REQ-COM-003
    #[test]
    fn test_assumption_bounds_are_validated() {
        assert_eq!(
            Assumption::new("  ", 0.0, 1.0),
            Err(ValuationError::BlankAssumptionName)
        );
        assert_eq!(
            Assumption::new("inverted", 5.0, 1.0),
            Err(ValuationError::InvertedAssumption("inverted".to_string()))
        );
        assert_eq!(Assumption::new("ok", 1.0, 5.0).unwrap().spread(), 4.0);
    }
}
