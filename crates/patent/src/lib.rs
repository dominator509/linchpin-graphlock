use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PatentPackage {
    pub title: String,
    pub is_ready_for_review: bool,
    pub receipt_imported: bool,
}

impl PatentPackage {
    pub fn is_filed(&self) -> bool {
        self.receipt_imported
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_patent_filing_invariant() {
        let pkg = PatentPackage {
            title: "Invention Title".to_string(),
            is_ready_for_review: true,
            receipt_imported: false,
        };
        assert!(!pkg.is_filed()); // INV-004: ready_for_human_review is not filed
    }
}
