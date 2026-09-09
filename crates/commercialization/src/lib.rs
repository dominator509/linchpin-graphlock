use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DisclosureLevel {
    Confidential,
    NonConfidential,
    PublicSafe,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutreachAsset {
    pub asset_id: String,
    pub disclosure_level: DisclosureLevel,
}

impl OutreachAsset {
    pub fn is_public_export_safe(&self) -> bool {
        matches!(self.disclosure_level, DisclosureLevel::PublicSafe)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disclosure_firewall() {
        let confidential = OutreachAsset { asset_id: "a1".into(), disclosure_level: DisclosureLevel::Confidential };
        let public_safe = OutreachAsset { asset_id: "a2".into(), disclosure_level: DisclosureLevel::PublicSafe };
        assert!(!confidential.is_public_export_safe()); // INV-005
        assert!(public_safe.is_public_export_safe());
    }
}
