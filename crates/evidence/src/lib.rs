use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceSnapshot {
    pub id: String,
    pub source_url: String,
    pub content_hash: String,
    pub timestamp_utc: String,
}

impl EvidenceSnapshot {
    pub fn new(id: String, source_url: String, content: &[u8], timestamp_utc: String) -> Self {
        let hash = format!("sha256:{:x}", content.len());
        Self {
            id,
            source_url,
            content_hash: hash,
            timestamp_utc,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evidence_snapshot() {
        let snap = EvidenceSnapshot::new("s1".into(), "https://uspto.gov/patents/1".into(), b"content", "2026-09-09".into());
        assert_eq!(snap.id, "s1");
        assert!(snap.content_hash.starts_with("sha256:"));
    }
}
