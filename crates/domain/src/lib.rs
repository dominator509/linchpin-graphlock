use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProvenanceType {
    HumanConception,
    AiSuggestion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptionEvent {
    pub id: String,
    pub title: String,
    pub description: String,
    pub provenance: ProvenanceType,
    pub timestamp_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimLimitation {
    pub id: String,
    pub limitation_text: String,
    pub support_anchor_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimSet {
    pub id: String,
    pub limitations: Vec<ClaimLimitation>,
}

impl ClaimSet {
    pub fn is_fully_supported(&self) -> bool {
        self.limitations.iter().all(|l| l.support_anchor_id.is_some())
    }
}

pub fn create_human_conception(id: String, title: String, description: String, timestamp: String) -> ConceptionEvent {
    ConceptionEvent {
        id,
        title,
        description,
        provenance: ProvenanceType::HumanConception,
        timestamp_utc: timestamp,
    }
}

pub fn create_ai_suggestion(id: String, title: String, description: String, timestamp: String) -> ConceptionEvent {
    ConceptionEvent {
        id,
        title,
        description,
        provenance: ProvenanceType::AiSuggestion,
        timestamp_utc: timestamp,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provenance_invariants() {
        let human = create_human_conception("h1".into(), "Title".into(), "Desc".into(), "2026-09-09".into());
        let ai = create_ai_suggestion("a1".into(), "Title".into(), "Desc".into(), "2026-09-09".into());
        assert_eq!(human.provenance, ProvenanceType::HumanConception);
        assert_eq!(ai.provenance, ProvenanceType::AiSuggestion);
    }

    #[test]
    fn test_claim_support() {
        let lim1 = ClaimLimitation { id: "l1".into(), limitation_text: "Lim 1".into(), support_anchor_id: Some("anch1".into()) };
        let lim2 = ClaimLimitation { id: "l2".into(), limitation_text: "Lim 2".into(), support_anchor_id: None };

        let supported_set = ClaimSet { id: "c1".into(), limitations: vec![lim1.clone()] };
        let unsupported_set = ClaimSet { id: "c2".into(), limitations: vec![lim1, lim2] };

        assert!(supported_set.is_fully_supported());
        assert!(!unsupported_set.is_fully_supported());
    }
}
