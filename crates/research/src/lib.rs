#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchStatus {
    Pending,
    Active,
    Killed,
    Completed,
}

/// The single class a research claim carries (REQ-PAT-003).
///
/// REQ-PAT-003 requires that "every research claim carries exactly one class"
/// from this closed set. Modelling it as an enum makes "exactly one" a
/// property of the type rather than a convention: a claim cannot hold two
/// classes, and no class outside this list can be constructed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimClass {
    Observation,
    Hypothesis,
    Inference,
    LegalRuleSummary,
    MarketSignal,
    PatentThreat,
    CommercialTargetAssertion,
}

impl ClaimClass {
    /// Only `OBSERVATION` may be emitted directly from a source record.
    pub fn may_be_emitted_from_source_record(self) -> bool {
        matches!(self, ClaimClass::Observation)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ClaimClass::Observation => "OBSERVATION",
            ClaimClass::Hypothesis => "HYPOTHESIS",
            ClaimClass::Inference => "INFERENCE",
            ClaimClass::LegalRuleSummary => "LEGAL_RULE_SUMMARY",
            ClaimClass::MarketSignal => "MARKET_SIGNAL",
            ClaimClass::PatentThreat => "PATENT_THREAT",
            ClaimClass::CommercialTargetAssertion => "COMMERCIAL_TARGET_ASSERTION",
        }
    }
}

/// A research claim and the provenance REQ-PAT-003 demands for its class.
///
/// The rule has two halves and both are enforced here: "Only `OBSERVATION` may
/// be emitted directly from a source record; all others store inference method
/// and contrary evidence." A non-observation claim that omits either the
/// inference method or the contrary evidence is rejected rather than stored,
/// because an unlabelled inference presented alongside observations is exactly
/// the fabricated-certainty failure this requirement exists to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResearchClaim {
    pub text: String,
    pub class: ClaimClass,
    /// Present when the claim was read directly out of a source record.
    pub source_record: Option<String>,
    /// Required for every class other than `OBSERVATION`.
    pub inference_method: Option<String>,
    /// Required for every class other than `OBSERVATION`.
    pub contrary_evidence: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimError {
    /// A non-observation claim was emitted straight from a source record.
    SourceRecordNotAllowedForClass(ClaimClass),
    /// A non-observation claim omitted its inference method.
    MissingInferenceMethod(ClaimClass),
    /// A non-observation claim omitted its contrary evidence.
    MissingContraryEvidence(ClaimClass),
    /// A claim's text is empty, so it asserts nothing.
    EmptyText,
}

impl std::fmt::Display for ClaimError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClaimError::SourceRecordNotAllowedForClass(c) => write!(
                f,
                "class {} may not be emitted directly from a source record",
                c.as_str()
            ),
            ClaimError::MissingInferenceMethod(c) => {
                write!(f, "class {} requires an inference method", c.as_str())
            }
            ClaimError::MissingContraryEvidence(c) => {
                write!(f, "class {} requires contrary evidence", c.as_str())
            }
            ClaimError::EmptyText => write!(f, "claim text cannot be empty"),
        }
    }
}

impl ResearchClaim {
    /// An observation read directly from a source record.
    pub fn observation(text: &str, source_record: &str) -> Self {
        ResearchClaim {
            text: text.to_string(),
            class: ClaimClass::Observation,
            source_record: Some(source_record.to_string()),
            inference_method: None,
            contrary_evidence: None,
        }
    }

    /// Validate the claim against REQ-PAT-003 before it is stored or exported.
    pub fn validate(&self) -> Result<(), ClaimError> {
        if self.text.trim().is_empty() {
            return Err(ClaimError::EmptyText);
        }
        if self.class.may_be_emitted_from_source_record() {
            return Ok(());
        }
        if self.source_record.is_some() {
            return Err(ClaimError::SourceRecordNotAllowedForClass(self.class));
        }
        if self
            .inference_method
            .as_deref()
            .map(str::trim)
            .unwrap_or("")
            .is_empty()
        {
            return Err(ClaimError::MissingInferenceMethod(self.class));
        }
        if self
            .contrary_evidence
            .as_deref()
            .map(str::trim)
            .unwrap_or("")
            .is_empty()
        {
            return Err(ClaimError::MissingContraryEvidence(self.class));
        }
        Ok(())
    }

    /// Construct a non-observation claim, validating it before it exists.
    pub fn inferred(
        text: &str,
        class: ClaimClass,
        inference_method: &str,
        contrary_evidence: &str,
    ) -> Result<Self, ClaimError> {
        let claim = ResearchClaim {
            text: text.to_string(),
            class,
            source_record: None,
            inference_method: Some(inference_method.to_string()),
            contrary_evidence: Some(contrary_evidence.to_string()),
        };
        claim.validate()?;
        Ok(claim)
    }
}

#[derive(Debug, Clone)]
pub struct ResearchTask {
    pub id: String,
    pub status: SearchStatus,
    pub citations: Vec<String>,
    /// Scopes this task was asked to cover (REQ-OPS-002).
    pub requested_scopes: usize,
    /// Scopes actually covered.
    pub covered_scopes: usize,
    /// Present only when the task finished with incomplete coverage, so the work
    /// can be resumed rather than lost.
    pub checkpoint: Option<String>,
}

impl ResearchTask {
    pub fn new(id: &str) -> Self {
        ResearchTask {
            id: id.to_string(),
            status: SearchStatus::Pending,
            citations: Vec::new(),
            requested_scopes: 0,
            covered_scopes: 0,
            checkpoint: None,
        }
    }

    pub fn start(&mut self) -> Result<(), &'static str> {
        if self.status != SearchStatus::Pending {
            return Err("Must be pending to start");
        }
        self.status = SearchStatus::Active;
        Ok(())
    }

    pub fn kill(&mut self) -> Result<(), &'static str> {
        if self.status != SearchStatus::Active {
            return Err("Must be active to kill");
        }
        self.status = SearchStatus::Killed;
        Ok(())
    }

    pub fn add_citation(&mut self, citation: String) -> Result<(), &'static str> {
        if self.status == SearchStatus::Killed || self.status == SearchStatus::Completed {
            return Err("Cannot add citations to finished tasks");
        }
        self.citations.push(citation);
        Ok(())
    }

    pub fn complete(&mut self) -> Result<(), &'static str> {
        if self.status != SearchStatus::Active {
            return Err("Must be active to complete");
        }
        // REQ-OPS-002: "Partial research persists checkpoints and marks
        // incomplete coverage." A task that did not cover everything it was
        // asked to cannot be recorded as a plain completion, or the gap becomes
        // invisible and downstream reasoning treats unchecked scope as checked.
        if !self.coverage_complete() {
            return Err(
                "coverage is incomplete; record it with complete_partial so the gap is visible",
            );
        }
        self.status = SearchStatus::Completed;
        Ok(())
    }

    /// Declare how many scopes this task was asked to cover.
    pub fn set_requested_scopes(&mut self, requested: usize) {
        self.requested_scopes = requested;
    }

    /// Record progress: how many scopes have actually been covered so far.
    ///
    /// Needed because `complete()` requires full coverage, and without a way to
    /// report coverage a fully-covered run could never reach `complete()` at all.
    /// The first version of this API had exactly that gap, and the test caught it
    /// rather than the gap reaching production.
    pub fn record_coverage(&mut self, covered_scopes: usize) -> Result<(), &'static str> {
        if covered_scopes > self.requested_scopes {
            return Err("covered scopes cannot exceed requested scopes");
        }
        self.covered_scopes = covered_scopes;
        Ok(())
    }

    /// Finish with incomplete coverage, persisting a checkpoint (REQ-OPS-002).
    ///
    /// The checkpoint is mandatory rather than optional: "partial research
    /// persists checkpoints" is the clause's own requirement, and a partial
    /// result with no checkpoint cannot be resumed, so the work is lost.
    pub fn complete_partial(
        &mut self,
        covered_scopes: usize,
        checkpoint: &str,
    ) -> Result<(), &'static str> {
        if self.status != SearchStatus::Active {
            return Err("Must be active to complete");
        }
        if checkpoint.trim().is_empty() {
            return Err("partial research must persist a checkpoint");
        }
        if covered_scopes > self.requested_scopes {
            return Err("covered scopes cannot exceed requested scopes");
        }
        if covered_scopes == self.requested_scopes {
            return Err("coverage is complete; use complete rather than complete_partial");
        }
        self.covered_scopes = covered_scopes;
        self.checkpoint = Some(checkpoint.to_string());
        self.status = SearchStatus::Completed;
        Ok(())
    }

    /// True only when every requested scope was covered. A task that declared no
    /// scope has nothing uncovered, so it is trivially complete.
    pub fn coverage_complete(&self) -> bool {
        self.covered_scopes >= self.requested_scopes
    }

    /// Scopes that were requested but not covered.
    pub fn uncovered_scopes(&self) -> usize {
        self.requested_scopes.saturating_sub(self.covered_scopes)
    }
}

#[cfg(test)]
mod research_tests {
    use super::*;

    #[test]
    fn test_research_planner_lifecycle() {
        let mut task = ResearchTask::new("task-1");
        assert_eq!(task.status, SearchStatus::Pending);

        assert!(task.start().is_ok());
        assert_eq!(task.status, SearchStatus::Active);

        assert!(task.add_citation("US1234".to_string()).is_ok());

        assert!(task.kill().is_ok());
        assert_eq!(task.status, SearchStatus::Killed);

        assert!(task.add_citation("US5678".to_string()).is_err());
    }

    #[test]
    fn test_research_completion() {
        let mut task = ResearchTask::new("task-2");
        task.start().unwrap();
        task.add_citation("US9999".to_string()).unwrap();
        task.complete().unwrap();
        assert_eq!(task.status, SearchStatus::Completed);
    }

    /// covers: REQ-OPS-002
    /// "Partial research persists checkpoints and marks incomplete coverage."
    #[test]
    fn test_partial_research_persists_a_checkpoint_and_marks_coverage() {
        let mut task = ResearchTask::new("task-partial");
        task.start().unwrap();
        task.set_requested_scopes(5);
        assert!(!task.coverage_complete());
        assert_eq!(task.uncovered_scopes(), 5);

        // Finishing partially without a checkpoint must fail: an unresumable
        // partial result loses the work the clause exists to preserve.
        assert!(
            task.complete_partial(2, "").is_err(),
            "a partial completion without a checkpoint was accepted"
        );
        assert!(
            task.complete_partial(2, "   ").is_err(),
            "a blank checkpoint was accepted"
        );
        assert_eq!(
            task.status,
            SearchStatus::Active,
            "a rejected partial completion must not change the status"
        );
        assert!(task.checkpoint.is_none());

        // A plain complete() must not be usable to hide the gap.
        assert!(
            task.complete().is_err(),
            "incomplete coverage was recorded as a full completion"
        );
        assert_eq!(task.status, SearchStatus::Active);

        // With a checkpoint it succeeds, and the gap stays visible.
        task.complete_partial(2, "checkpoint://run-7/scope-2")
            .unwrap();
        assert_eq!(task.status, SearchStatus::Completed);
        assert_eq!(task.covered_scopes, 2);
        assert_eq!(task.requested_scopes, 5);
        assert_eq!(task.uncovered_scopes(), 3);
        assert!(
            !task.coverage_complete(),
            "partial coverage must not report as complete"
        );
        assert_eq!(
            task.checkpoint.as_deref(),
            Some("checkpoint://run-7/scope-2"),
            "the checkpoint must be persisted on the task"
        );
    }

    /// covers: REQ-OPS-002
    #[test]
    fn test_full_coverage_completes_without_a_checkpoint() {
        let mut task = ResearchTask::new("task-full");
        task.start().unwrap();
        task.set_requested_scopes(3);

        // complete_partial refuses when nothing is actually uncovered.
        assert!(task.complete_partial(3, "c").is_err());
        // Over-covering is nonsense and is rejected, both when recording
        // progress and when finishing.
        assert!(task.record_coverage(4).is_err());
        assert!(task.complete_partial(4, "c").is_err());

        // Full coverage must be recordable, or a complete run could never
        // finish: this assertion is what exposed that gap in the first place.
        task.record_coverage(3).unwrap();
        task.complete().unwrap();
        assert_eq!(task.status, SearchStatus::Completed);
        assert!(task.coverage_complete());
        assert_eq!(task.uncovered_scopes(), 0);
        assert!(
            task.checkpoint.is_none(),
            "a complete run needs no checkpoint"
        );
    }

    /// covers: REQ-OPS-002
    /// A task that declared no scope has nothing uncovered, so the existing
    /// completion path keeps working for scope-less tasks.
    #[test]
    fn test_task_with_no_declared_scope_is_trivially_complete() {
        let mut task = ResearchTask::new("task-noscope");
        assert!(task.coverage_complete());
        task.start().unwrap();
        task.complete().unwrap();
        assert_eq!(task.status, SearchStatus::Completed);
    }
}

/// Evaluates candidate design-around strategies.
///
/// GraphLock context (anti-gaming finding AG-009): `execute_independent_runs`
/// previously did this:
///
/// ```text
/// for _ in &self.strategies {
///     // Mock independent evaluation, all returning true (valid design around)
///     self.isolated_runs.push(true);
/// }
/// ```
///
/// It claimed to run independent evaluations and then unconditionally recorded
/// every strategy as valid. The result could never be false, so the tournament
/// carried no information — a prior-art design-around assessment that always
/// says "valid" is worse than none, because it invites reliance.
///
/// This version evaluates against an explicit, caller-supplied predicate. The
/// strategy text is the input; the verdict comes from the predicate, so a
/// strategy that does not satisfy it is recorded as a failure.
pub struct DesignAroundTournament {
    pub orchestrator_id: String,
    pub strategies: Vec<String>,
    pub isolated_runs: Vec<bool>,
}

impl DesignAroundTournament {
    pub fn new(id: &str) -> Self {
        DesignAroundTournament {
            orchestrator_id: id.to_string(),
            strategies: Vec::new(),
            isolated_runs: Vec::new(),
        }
    }

    pub fn submit_strategy(&mut self, strategy: String) {
        self.strategies.push(strategy);
    }

    /// Run each submitted strategy through `evaluate` and record the verdict.
    ///
    /// A strategy with no recorded verdict is not the same as a passing one, so
    /// the returned count is the number of evaluations actually performed.
    pub fn execute_independent_runs<F>(&mut self, evaluate: F) -> Result<usize, &'static str>
    where
        F: Fn(&str) -> bool,
    {
        if self.strategies.is_empty() {
            return Err("No strategies to evaluate");
        }

        self.isolated_runs.clear();
        for strategy in &self.strategies {
            self.isolated_runs.push(evaluate(strategy));
        }

        Ok(self.isolated_runs.len())
    }

    /// Number of strategies that actually passed evaluation.
    pub fn passing_count(&self) -> usize {
        self.isolated_runs.iter().filter(|p| **p).count()
    }

    /// Number of strategies that failed evaluation.
    pub fn failing_count(&self) -> usize {
        self.isolated_runs.iter().filter(|p| !**p).count()
    }
}

#[cfg(test)]
mod tournament_tests {
    use super::*;

    /// AG-009 regression: the verdict must come from the evaluation, not from a
    /// hard-coded `true`. A failing strategy must be recorded as failing.
    #[test]
    fn test_design_around_tournament() {
        let mut tournament = DesignAroundTournament::new("orchestrator-alpha");
        assert!(tournament.execute_independent_runs(|_| true).is_err());

        tournament.submit_strategy("Strategy A: Remove element".to_string());
        tournament.submit_strategy("Strategy B: Substitute process".to_string());

        let runs = tournament.execute_independent_runs(|_| true).unwrap();
        assert_eq!(runs, 2);
        assert_eq!(tournament.isolated_runs.len(), 2);
        assert!(tournament.isolated_runs[0]);
        assert_eq!(tournament.passing_count(), 2);
        assert_eq!(tournament.failing_count(), 0);
    }

    /// A strategy the predicate rejects must be recorded as a failure, and the
    /// old implementation could never produce this outcome.
    #[test]
    fn test_failing_strategy_is_recorded_as_failure() {
        let mut tournament = DesignAroundTournament::new("orch-1");
        tournament.submit_strategy("reads on the prior art".to_string());
        tournament.submit_strategy("genuinely distinct".to_string());

        // Predicate: only strategies that do not read on the reference pass.
        tournament
            .execute_independent_runs(|s| !s.contains("prior art"))
            .unwrap();

        assert_eq!(tournament.passing_count(), 1);
        assert_eq!(tournament.failing_count(), 1);
        assert_eq!(tournament.isolated_runs, vec![false, true]);
    }

    /// Re-running must replace prior verdicts rather than append to them.
    #[test]
    fn test_rerun_replaces_previous_verdicts() {
        let mut tournament = DesignAroundTournament::new("orch-2");
        tournament.submit_strategy("s1".to_string());
        tournament.execute_independent_runs(|_| false).unwrap();
        assert_eq!(tournament.failing_count(), 1);

        tournament.execute_independent_runs(|_| true).unwrap();
        assert_eq!(tournament.isolated_runs.len(), 1, "verdicts were appended");
        assert_eq!(tournament.passing_count(), 1);
    }
}

#[cfg(test)]
mod claim_tests {
    use super::*;

    /// covers: REQ-PAT-003
    /// Every claim carries exactly one class, and the class set is closed: the
    /// enum cannot represent a claim with two classes or an unlisted one.
    #[test]
    fn test_claim_classes_are_a_closed_set_of_seven() {
        let all = [
            ClaimClass::Observation,
            ClaimClass::Hypothesis,
            ClaimClass::Inference,
            ClaimClass::LegalRuleSummary,
            ClaimClass::MarketSignal,
            ClaimClass::PatentThreat,
            ClaimClass::CommercialTargetAssertion,
        ];
        let names: Vec<&str> = all.iter().map(|c| c.as_str()).collect();
        assert_eq!(names.len(), 7);
        assert_eq!(
            names,
            vec![
                "OBSERVATION",
                "HYPOTHESIS",
                "INFERENCE",
                "LEGAL_RULE_SUMMARY",
                "MARKET_SIGNAL",
                "PATENT_THREAT",
                "COMMERCIAL_TARGET_ASSERTION",
            ]
        );
        // Only OBSERVATION may come straight from a source record.
        let from_source: Vec<&str> = all
            .iter()
            .filter(|c| c.may_be_emitted_from_source_record())
            .map(|c| c.as_str())
            .collect();
        assert_eq!(from_source, vec!["OBSERVATION"]);
    }

    /// covers: REQ-PAT-003
    #[test]
    fn test_observation_may_be_emitted_from_a_source_record() {
        let claim = ResearchClaim::observation("US1234567B2 claims a valve.", "US1234567B2");
        assert_eq!(claim.class, ClaimClass::Observation);
        assert!(claim.validate().is_ok());
    }

    /// covers: REQ-PAT-003
    /// The other six classes must store inference method AND contrary evidence.
    #[test]
    fn test_non_observation_classes_require_method_and_contrary_evidence() {
        let classes = [
            ClaimClass::Hypothesis,
            ClaimClass::Inference,
            ClaimClass::LegalRuleSummary,
            ClaimClass::MarketSignal,
            ClaimClass::PatentThreat,
            ClaimClass::CommercialTargetAssertion,
        ];
        for class in classes {
            // Both present: accepted.
            let ok = ResearchClaim::inferred(
                "the market is moving this way",
                class,
                "extrapolated from three filings",
                "one filing contradicts this",
            );
            assert!(ok.is_ok(), "{class:?} with full provenance was rejected");

            // Missing contrary evidence: rejected, naming the class.
            let mut missing_contrary = ok.unwrap();
            missing_contrary.contrary_evidence = None;
            assert_eq!(
                missing_contrary.validate(),
                Err(ClaimError::MissingContraryEvidence(class))
            );
            // Blank strings are not provenance either.
            missing_contrary.contrary_evidence = Some("   ".to_string());
            assert_eq!(
                missing_contrary.validate(),
                Err(ClaimError::MissingContraryEvidence(class))
            );

            // Missing inference method: rejected, naming the class.
            let mut missing_method = ResearchClaim::inferred(
                "the market is moving this way",
                class,
                "extrapolated from three filings",
                "one filing contradicts this",
            )
            .unwrap();
            missing_method.inference_method = None;
            assert_eq!(
                missing_method.validate(),
                Err(ClaimError::MissingInferenceMethod(class))
            );
        }
    }

    /// covers: REQ-PAT-003
    /// A non-observation class must not be emitted directly from a source
    /// record: that is the shape that would present an inference as a fact.
    #[test]
    fn test_non_observation_may_not_claim_a_source_record() {
        for class in [ClaimClass::Hypothesis, ClaimClass::Inference] {
            let mut claim =
                ResearchClaim::inferred("t", class, "method", "contrary").expect("valid claim");
            claim.source_record = Some("US1234567B2".to_string());
            assert_eq!(
                claim.validate(),
                Err(ClaimError::SourceRecordNotAllowedForClass(class))
            );
        }
    }

    /// covers: REQ-PAT-003
    #[test]
    fn test_empty_claim_text_is_rejected() {
        let mut claim = ResearchClaim::observation("", "src");
        assert_eq!(claim.validate(), Err(ClaimError::EmptyText));
        claim.text = "   ".to_string();
        assert_eq!(claim.validate(), Err(ClaimError::EmptyText));
    }
}
