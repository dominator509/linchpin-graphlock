use serde::{Serialize, Deserialize};
use evidence::EvidenceSnapshot;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchQuery {
    pub query_id: String,
    pub terms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchResult {
    pub claim_id: String,
    pub snapshot: EvidenceSnapshot,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_research_result() {
        let snap = EvidenceSnapshot::new("s1".into(), "https://uspto.gov".into(), b"test", "2026-09-09".into());
        let res = ResearchResult { claim_id: "c1".into(), snapshot: snap };
        assert_eq!(res.claim_id, "c1");
    }
}
