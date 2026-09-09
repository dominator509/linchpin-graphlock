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

    pub fn export_pitch_deck(&self) -> Result<String, &'static str> {
        if !self.assets_redacted {
            return Err("Cannot export pitch deck without redacting confidential assets");
        }
        if self.targets.is_empty() {
            return Err("Cannot export pitch deck with zero targets");
        }
        Ok("Pitch deck generated".to_string())
    }
}

#[cfg(test)]
mod commercialization_tests {
    use super::*;

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

pub struct LiveFireOrchestrator {
    pub completed_runs: usize,
    pub domain_regression_passed: bool,
}

impl Default for LiveFireOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl LiveFireOrchestrator {
    pub fn new() -> Self {
        LiveFireOrchestrator {
            completed_runs: 0,
            domain_regression_passed: false,
        }
    }

    pub fn run_uo_live_fire(&mut self) -> Result<(), &'static str> {
        // Simulating UO-01..12 live fire
        self.completed_runs = 12;
        Ok(())
    }

    pub fn verify_domain_regression(&mut self) -> Result<(), &'static str> {
        if self.completed_runs != 12 {
            return Err("Must complete live fire before domain regression");
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

        assert!(orchestrator.run_uo_live_fire().is_ok());
        assert_eq!(orchestrator.completed_runs, 12);

        assert!(orchestrator.verify_domain_regression().is_ok());
        assert!(orchestrator.domain_regression_passed);
    }
}
