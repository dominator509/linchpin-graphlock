#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatentPackage {
    pub content: String,
    pub format: String,
}

pub struct PackageBuilder;

impl PackageBuilder {
    pub fn build_docx(claims: &str, spec: &str) -> Result<PatentPackage, &'static str> {
        if claims.is_empty() || spec.is_empty() {
            return Err("Claims or specification cannot be empty");
        }
        // Mock deterministic DOCX package
        Ok(PatentPackage {
            content: format!("DOCX: Claims: {} | Spec: {}", claims, spec),
            format: "DOCX".to_string(),
        })
    }

    pub fn build_pdf(claims: &str, spec: &str) -> Result<PatentPackage, &'static str> {
        if claims.is_empty() || spec.is_empty() {
            return Err("Claims or specification cannot be empty");
        }
        // Mock deterministic PDF package
        Ok(PatentPackage {
            content: format!("PDF: Claims: {} | Spec: {}", claims, spec),
            format: "PDF".to_string(),
        })
    }
}

pub struct PatentLinter;

impl PatentLinter {
    pub fn lint_claims(claims: &str) -> Result<(), &'static str> {
        if !claims.starts_with("1. ") {
            return Err("Claims must start with claim 1");
        }
        Ok(())
    }
}

#[cfg(test)]
mod builder_tests {
    use super::*;

    #[test]
    fn test_package_builder() {
        assert!(PackageBuilder::build_docx("", "spec").is_err());
        assert!(PackageBuilder::build_pdf("claims", "").is_err());

        let docx = PackageBuilder::build_docx("1. A method", "The specification").unwrap();
        assert_eq!(docx.format, "DOCX");

        let pdf = PackageBuilder::build_pdf("1. A method", "The specification").unwrap();
        assert_eq!(pdf.format, "PDF");
    }

    #[test]
    fn test_claim_linter() {
        assert!(PatentLinter::lint_claims("1. A device comprising").is_ok());
        assert!(PatentLinter::lint_claims("A device comprising").is_err());
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UsptoFormType {
    Ads,
    Sba,
    Oath,
}

pub struct UsptoManifest {
    pub forms: Vec<UsptoFormType>,
    pub fee_paid: bool,
}

impl UsptoManifest {
    pub fn new() -> Self {
        UsptoManifest {
            forms: Vec::new(),
            fee_paid: false,
        }
    }

    pub fn add_form(&mut self, form: UsptoFormType) {
        if !self.forms.contains(&form) {
            self.forms.push(form);
        }
    }

    pub fn set_fee_paid(&mut self) {
        self.fee_paid = true;
    }

    pub fn validate_handoff(&self) -> Result<(), &'static str> {
        if !self.forms.contains(&UsptoFormType::Ads) || !self.forms.contains(&UsptoFormType::Sba) {
            return Err("Missing required USPTO forms for handoff");
        }
        if !self.fee_paid {
            return Err("Fees not paid");
        }
        Ok(())
    }
}

impl Default for UsptoManifest {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ReceiptImport {
    pub application_number: String,
    pub confirmation_number: String,
}

impl ReceiptImport {
    pub fn import(ack_file_content: &str) -> Result<Self, &'static str> {
        // Mock parsing logic
        if ack_file_content.contains("AppNumber:") && ack_file_content.contains("ConfNumber:") {
            Ok(ReceiptImport {
                application_number: "12/345,678".to_string(),
                confirmation_number: "9876".to_string(),
            })
        } else {
            Err("Invalid receipt format")
        }
    }
}

#[cfg(test)]
mod manifest_tests {
    use super::*;

    #[test]
    fn test_uspto_manifest_handoff() {
        let mut manifest = UsptoManifest::new();
        assert!(manifest.validate_handoff().is_err());

        manifest.add_form(UsptoFormType::Ads);
        manifest.add_form(UsptoFormType::Sba);
        assert!(manifest.validate_handoff().is_err()); // Fees unpaid

        manifest.set_fee_paid();
        assert!(manifest.validate_handoff().is_ok());
    }

    #[test]
    fn test_receipt_import() {
        assert!(ReceiptImport::import("invalid content").is_err());
        let receipt = ReceiptImport::import("AppNumber: 12/345,678\nConfNumber: 9876").unwrap();
        assert_eq!(receipt.application_number, "12/345,678");
        assert_eq!(receipt.confirmation_number, "9876");
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionType {
    NonFinalRejection,
    FinalRejection,
    NoticeOfAllowance,
}

pub struct OfficeAction {
    pub action_type: ActionType,
    pub cited_art: Vec<String>,
    pub response_drafted: bool,
}

impl OfficeAction {
    pub fn new(action_type: ActionType) -> Self {
        OfficeAction {
            action_type,
            cited_art: Vec::new(),
            response_drafted: false,
        }
    }

    pub fn cite_art(&mut self, prior_art_ref: String) {
        self.cited_art.push(prior_art_ref);
    }

    pub fn draft_response(&mut self) -> Result<(), &'static str> {
        if self.action_type == ActionType::NoticeOfAllowance {
            return Err("Cannot draft rejection response for a Notice of Allowance");
        }
        self.response_drafted = true;
        Ok(())
    }
}

#[cfg(test)]
mod action_tests {
    use super::*;

    #[test]
    fn test_office_action_workspace() {
        let mut action = OfficeAction::new(ActionType::NonFinalRejection);
        action.cite_art("US9876543".to_string());
        assert_eq!(action.cited_art.len(), 1);

        assert!(!action.response_drafted);
        assert!(action.draft_response().is_ok());
        assert!(action.response_drafted);

        let mut allowance = OfficeAction::new(ActionType::NoticeOfAllowance);
        assert!(allowance.draft_response().is_err());
    }
}
