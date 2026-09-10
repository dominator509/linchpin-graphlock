use std::time::SystemTime;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventId(pub Uuid);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityId(pub Uuid);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateMachineStatus {
    Draft,
    Active,
    Archived,
}

#[derive(Debug, Clone)]
pub struct DomainEntity {
    pub id: EntityId,
    pub status: StateMachineStatus,
    pub created_at: SystemTime,
}

impl Default for DomainEntity {
    fn default() -> Self {
        Self::new()
    }
}

impl DomainEntity {
    pub fn new() -> Self {
        DomainEntity {
            id: EntityId(Uuid::new_v4()),
            status: StateMachineStatus::Draft,
            created_at: SystemTime::now(),
        }
    }

    pub fn activate(&mut self) -> Result<(), &'static str> {
        if self.status != StateMachineStatus::Draft {
            return Err("Entity must be in Draft state to activate");
        }
        self.status = StateMachineStatus::Active;
        Ok(())
    }

    pub fn archive(&mut self) -> Result<(), &'static str> {
        if self.status == StateMachineStatus::Archived {
            return Err("Entity is already archived");
        }
        self.status = StateMachineStatus::Archived;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_entity_lifecycle() {
        let mut entity = DomainEntity::new();
        assert_eq!(entity.status, StateMachineStatus::Draft);

        assert!(entity.activate().is_ok());
        assert_eq!(entity.status, StateMachineStatus::Active);

        assert!(entity.activate().is_err());

        assert!(entity.archive().is_ok());
        assert_eq!(entity.status, StateMachineStatus::Archived);

        assert!(entity.archive().is_err());
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorOrigin {
    HumanConception(Uuid),
    AiSuggestion(Uuid),
}

#[derive(Debug, Clone)]
pub struct ContentBlock {
    pub id: EntityId,
    pub content: String,
    pub origin: AuthorOrigin,
}

impl ContentBlock {
    pub fn new_human(content: String, user_id: Uuid) -> Self {
        ContentBlock {
            id: EntityId(Uuid::new_v4()),
            content,
            origin: AuthorOrigin::HumanConception(user_id),
        }
    }

    pub fn new_ai(content: String, model_id: Uuid) -> Self {
        ContentBlock {
            id: EntityId(Uuid::new_v4()),
            content,
            origin: AuthorOrigin::AiSuggestion(model_id),
        }
    }
}

#[cfg(test)]
mod origin_tests {
    use super::*;

    #[test]
    fn test_human_vs_ai_origin() {
        let human_id = Uuid::new_v4();
        let ai_id = Uuid::new_v4();

        let human_block = ContentBlock::new_human("My idea".to_string(), human_id);
        let ai_block = ContentBlock::new_ai("Suggested idea".to_string(), ai_id);

        assert!(matches!(human_block.origin, AuthorOrigin::HumanConception(id) if id == human_id));
        assert!(matches!(ai_block.origin, AuthorOrigin::AiSuggestion(id) if id == ai_id));
    }
}

#[derive(Debug, Clone)]
pub struct ScoreVector {
    pub technical_feasibility: f32,
    pub market_potential: f32,
    pub legal_risk: f32,
}

#[derive(Debug, Clone)]
pub enum UncertaintyLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone)]
pub struct EvidenceNode {
    pub description: String,
    pub uri: String,
}

#[derive(Debug, Clone)]
pub struct OpportunityCandidate {
    pub id: EntityId,
    pub scores: ScoreVector,
    pub uncertainty: UncertaintyLevel,
    pub evidence: Vec<EvidenceNode>,
}

impl OpportunityCandidate {
    pub fn new(scores: ScoreVector, uncertainty: UncertaintyLevel) -> Self {
        OpportunityCandidate {
            id: EntityId(Uuid::new_v4()),
            scores,
            uncertainty,
            evidence: Vec::new(),
        }
    }

    pub fn add_evidence(&mut self, description: String, uri: String) {
        self.evidence.push(EvidenceNode { description, uri });
    }
}

#[cfg(test)]
mod opportunity_tests {
    use super::*;

    #[test]
    fn test_opportunity_candidate() {
        let scores = ScoreVector {
            technical_feasibility: 0.8,
            market_potential: 0.9,
            legal_risk: 0.2,
        };

        let mut candidate = OpportunityCandidate::new(scores, UncertaintyLevel::Medium);
        assert!(candidate.evidence.is_empty());

        candidate.add_evidence(
            "Prior art search".to_string(),
            "https://patents.google.com/123".to_string(),
        );
        assert_eq!(candidate.evidence.len(), 1);
        assert_eq!(candidate.evidence[0].description, "Prior art search");
    }
}

#[derive(Debug, Clone)]
pub struct Limitation {
    pub id: EntityId,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct PriorArtReference {
    pub id: EntityId,
    pub citation: String,
}

#[derive(Debug, Clone)]
pub struct ThreatMap {
    pub reference_id: EntityId,
    pub limitation_id: EntityId,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct DesignAround {
    pub id: EntityId,
    pub threat_map: ThreatMap,
    pub mitigation_strategy: String,
}

#[derive(Debug, Clone)]
pub struct ClaimGraph {
    pub limitations: Vec<Limitation>,
    pub references: Vec<PriorArtReference>,
    pub threats: Vec<ThreatMap>,
    pub design_arounds: Vec<DesignAround>,
}

impl Default for ClaimGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl ClaimGraph {
    pub fn new() -> Self {
        ClaimGraph {
            limitations: Vec::new(),
            references: Vec::new(),
            threats: Vec::new(),
            design_arounds: Vec::new(),
        }
    }

    pub fn add_limitation(&mut self, text: String) -> EntityId {
        let id = EntityId(Uuid::new_v4());
        self.limitations.push(Limitation {
            id: id.clone(),
            text,
        });
        id
    }

    pub fn add_reference(&mut self, citation: String) -> EntityId {
        let id = EntityId(Uuid::new_v4());
        self.references.push(PriorArtReference {
            id: id.clone(),
            citation,
        });
        id
    }

    pub fn map_threat(
        &mut self,
        reference_id: EntityId,
        limitation_id: EntityId,
        description: String,
    ) -> ThreatMap {
        let threat = ThreatMap {
            reference_id,
            limitation_id,
            description,
        };
        self.threats.push(threat.clone());
        threat
    }

    pub fn add_design_around(
        &mut self,
        threat_map: ThreatMap,
        mitigation_strategy: String,
    ) -> EntityId {
        let id = EntityId(Uuid::new_v4());
        self.design_arounds.push(DesignAround {
            id: id.clone(),
            threat_map,
            mitigation_strategy,
        });
        id
    }
}

#[cfg(test)]
mod claim_tests {
    use super::*;

    #[test]
    fn test_claim_graph_and_design_around() {
        let mut graph = ClaimGraph::new();

        let limit_id = graph.add_limitation("A distributed ledger".to_string());
        let ref_id = graph.add_reference("US1234567B2".to_string());

        let threat = graph.map_threat(ref_id, limit_id, "Reference teaches a ledger".to_string());
        assert_eq!(graph.threats.len(), 1);

        let _design_around_id =
            graph.add_design_around(threat, "Ours is non-deterministic".to_string());
        assert_eq!(graph.design_arounds.len(), 1);
        assert_eq!(
            graph.design_arounds[0].mitigation_strategy,
            "Ours is non-deterministic"
        );
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilingState {
    Preparation,
    Filed(String), // receipt or application number
    Commercialized,
}

#[derive(Debug, Clone)]
pub struct DocketRecord {
    pub id: EntityId,
    pub title: String,
    pub state: FilingState,
    pub public_disclosure: bool,
}

impl DocketRecord {
    pub fn new(title: String) -> Self {
        DocketRecord {
            id: EntityId(Uuid::new_v4()),
            title,
            state: FilingState::Preparation,
            public_disclosure: false,
        }
    }

    pub fn file_application(&mut self, receipt: String) -> Result<(), &'static str> {
        if self.state != FilingState::Preparation {
            return Err("Must be in preparation to file");
        }
        self.state = FilingState::Filed(receipt);
        Ok(())
    }

    pub fn mark_commercialized(&mut self) -> Result<(), &'static str> {
        match self.state {
            FilingState::Filed(_) => {
                self.state = FilingState::Commercialized;
                Ok(())
            }
            _ => Err("Must be filed before commercialization"),
        }
    }

    pub fn public_export(&mut self) -> Result<(), &'static str> {
        self.public_disclosure = true;
        Ok(())
    }
}

#[cfg(test)]
mod docket_tests {
    use super::*;

    #[test]
    fn test_docket_state_machine() {
        let mut docket = DocketRecord::new("Novel Algorithm".to_string());
        assert_eq!(docket.state, FilingState::Preparation);

        assert!(docket.mark_commercialized().is_err());

        assert!(docket.file_application("US123456".to_string()).is_ok());
        assert!(matches!(docket.state, FilingState::Filed(ref r) if r == "US123456"));

        assert!(docket.mark_commercialized().is_ok());
        assert_eq!(docket.state, FilingState::Commercialized);

        assert!(docket.public_export().is_ok());
        assert!(docket.public_disclosure);
    }
}
