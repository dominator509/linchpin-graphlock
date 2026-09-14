#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchStatus {
    Pending,
    Active,
    Killed,
    Completed,
}

#[derive(Debug, Clone)]
pub struct ResearchTask {
    pub id: String,
    pub status: SearchStatus,
    pub citations: Vec<String>,
}

impl ResearchTask {
    pub fn new(id: &str) -> Self {
        ResearchTask {
            id: id.to_string(),
            status: SearchStatus::Pending,
            citations: Vec::new(),
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
        self.status = SearchStatus::Completed;
        Ok(())
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
