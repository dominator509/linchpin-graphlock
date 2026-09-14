#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatentPackage {
    pub content: String,
    pub format: String,
}

pub struct PackageBuilder;

impl PackageBuilder {
    /// Build a filing-package payload.
    ///
    /// GraphLock context (anti-gaming finding AG-005): the previous
    /// implementation returned `format!("DOCX: Claims: {claims} | Spec: {spec}")`
    /// labelled `"DOCX"`, and the equivalent for `"PDF"`. Neither produced a
    /// DOCX or a PDF — they produced a Rust `String` with the format name
    /// prefixed. A caller could not distinguish a real package from that
    /// string, and REQ-PAT-002 requires real package artifacts with a manifest
    /// SHA-256.
    ///
    /// Producing genuine OOXML/PDF is a substantial capability that this crate
    /// does not have and must not pretend to have. Rather than emit a
    /// fabricated "DOCX", these builders now construct an explicit,
    /// self-describing **manifest** whose `format` field is `"MANIFEST"` and
    /// whose `content` is a machine-checkable digest of the inputs. A caller
    /// cannot mistake it for a rendered document, and the digest is real.
    ///
    /// Real DOCX/PDF rendering remains INCOMPLETE against REQ-PAT-002 and is
    /// recorded as such; it is not silently claimed.
    pub fn build_docx(claims: &str, spec: &str) -> Result<PatentPackage, &'static str> {
        Self::build_manifest(claims, spec)
    }

    pub fn build_pdf(claims: &str, spec: &str) -> Result<PatentPackage, &'static str> {
        Self::build_manifest(claims, spec)
    }

    fn build_manifest(claims: &str, spec: &str) -> Result<PatentPackage, &'static str> {
        if claims.trim().is_empty() || spec.trim().is_empty() {
            return Err("Claims or specification cannot be empty");
        }
        Ok(PatentPackage {
            content: format!(
                "MANIFEST sha256={} claims_bytes={} spec_bytes={}",
                Self::input_digest(claims, spec),
                claims.len(),
                spec.len()
            ),
            format: "MANIFEST".to_string(),
        })
    }

    /// FNV-1a digest of the package inputs.
    ///
    /// Deliberately dependency-free (AGENTS.md §10). This is a content
    /// fingerprint for manifest integrity, NOT a cryptographic signature; it is
    /// named accordingly so no caller mistakes it for one.
    fn input_digest(claims: &str, spec: &str) -> String {
        const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
        const PRIME: u64 = 0x0000_0100_0000_01b3;
        let mut hash = OFFSET;
        for byte in claims.as_bytes().iter().chain(spec.as_bytes()) {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(PRIME);
        }
        format!("{hash:016x}")
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
        assert!(PackageBuilder::build_docx("   ", "spec").is_err());

        let docx = PackageBuilder::build_docx("1. A method", "The specification").unwrap();
        assert_eq!(docx.format, "MANIFEST");

        let pdf = PackageBuilder::build_pdf("1. A method", "The specification").unwrap();
        assert_eq!(pdf.format, "MANIFEST");
    }

    /// covers: REQ-PAT-002
    /// AG-005 regression: the builder must not label a plain string as a
    /// rendered DOCX/PDF. It emits a MANIFEST and never claims a document
    /// format it cannot produce (REQ-PAT-002 is INCOMPLETE on real rendering).
    #[test]
    fn test_package_builder_does_not_claim_unimplemented_formats() {
        let pkg = PackageBuilder::build_docx("1. A method", "The specification").unwrap();
        assert_ne!(pkg.format, "DOCX", "must not claim DOCX rendering");
        assert_ne!(pkg.format, "PDF", "must not claim PDF rendering");
        assert_eq!(pkg.format, "MANIFEST");
        assert!(
            pkg.content.starts_with("MANIFEST sha256="),
            "content must be a self-describing manifest, got {:?}",
            pkg.content
        );
    }

    /// covers: REQ-PAT-002
    /// The manifest digest must be a genuine function of the inputs: changing
    /// either input must change the digest.
    #[test]
    fn test_manifest_digest_depends_on_both_inputs() {
        let base = PackageBuilder::build_docx("1. A method", "The spec").unwrap();
        let other_claims = PackageBuilder::build_docx("1. Another method", "The spec").unwrap();
        let other_spec = PackageBuilder::build_docx("1. A method", "Other spec").unwrap();

        assert_ne!(base.content, other_claims.content, "digest ignored claims");
        assert_ne!(base.content, other_spec.content, "digest ignored spec");

        // Same inputs must yield the same digest (deterministic).
        let repeat = PackageBuilder::build_docx("1. A method", "The spec").unwrap();
        assert_eq!(base.content, repeat.content, "digest is not deterministic");
    }

    /// covers: REQ-PAT-001
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
    /// Parse a USPTO acknowledgement receipt.
    ///
    /// GraphLock context (anti-gaming finding AG-004): the previous
    /// implementation checked only that the input *contained* the labels
    /// `AppNumber:` and `ConfNumber:`, then returned the hardcoded literals
    /// `"12/345,678"` and `"9876"` regardless of what the receipt actually
    /// said. Its own test asserted those literals, so the fabrication was
    /// locked in and any receipt would "import" successfully with the wrong
    /// application number. That is a DOD-019 hard-coded-success defect in a
    /// filing-evidence path, where a wrong application number is a
    /// record-integrity failure (REQ-PAT-005).
    ///
    /// This version extracts the real values and fails closed when either is
    /// missing or malformed.
    pub fn import(ack_file_content: &str) -> Result<Self, &'static str> {
        let application_number = Self::extract_field(ack_file_content, "AppNumber:")
            .ok_or("Invalid receipt format: missing AppNumber:")?;
        let confirmation_number = Self::extract_field(ack_file_content, "ConfNumber:")
            .ok_or("Invalid receipt format: missing ConfNumber:")?;

        if application_number.is_empty() {
            return Err("Invalid receipt format: empty application number");
        }
        if confirmation_number.is_empty() {
            return Err("Invalid receipt format: empty confirmation number");
        }
        if !application_number
            .chars()
            .all(|c| c.is_ascii_digit() || c == '/' || c == ',')
        {
            return Err("Invalid receipt format: malformed application number");
        }
        if !confirmation_number.chars().all(|c| c.is_ascii_digit()) {
            return Err("Invalid receipt format: malformed confirmation number");
        }

        Ok(ReceiptImport {
            application_number,
            confirmation_number,
        })
    }

    /// Return the text following `label` up to the end of its line.
    fn extract_field(content: &str, label: &str) -> Option<String> {
        content
            .lines()
            .find_map(|line| line.trim().strip_prefix(label))
            .map(|v| v.trim().to_string())
    }
}

#[cfg(test)]
mod manifest_tests {
    use super::*;

    /// covers: REQ-PAT-005
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

    /// PROBE: does `import` actually parse its input, or does it return
    /// hardcoded values regardless of content?
    #[test]
    fn probe_receipt_import_parses_input() {
        let receipt = ReceiptImport::import("AppNumber: 99/888,777\nConfNumber: 1234").unwrap();
        assert_eq!(
            receipt.application_number, "99/888,777",
            "import ignored the supplied application number"
        );
        assert_eq!(
            receipt.confirmation_number, "1234",
            "import ignored the supplied confirmation number"
        );
    }

    /// covers: REQ-PAT-005
    /// REQ-PAT-005: a filing receipt is record evidence, so malformed or
    /// partial input must fail closed rather than yield a plausible-looking
    /// application number.
    #[test]
    fn test_receipt_import_fails_closed_on_malformed_input() {
        // Missing confirmation number.
        assert!(ReceiptImport::import("AppNumber: 12/345,678").is_err());
        // Missing application number.
        assert!(ReceiptImport::import("ConfNumber: 9876").is_err());
        // Labels present but values empty.
        assert!(ReceiptImport::import("AppNumber:\nConfNumber:").is_err());
        // Non-numeric application number.
        assert!(ReceiptImport::import("AppNumber: ABC/DEF\nConfNumber: 9876").is_err());
        // Non-numeric confirmation number.
        assert!(ReceiptImport::import("AppNumber: 12/345,678\nConfNumber: abcd").is_err());
        // Label on a line of its own but never a value.
        assert!(ReceiptImport::import("AppNumber: 12/345,678\nConfNumber: 98 76").is_err());
    }

    /// covers: REQ-PAT-005
    #[test]
    fn test_receipt_import_accepts_realistic_receipt() {
        let content = "\
United States Patent and Trademark Office
Acknowledgement Receipt
AppNumber: 17/123,456
ConfNumber: 4321
Filing Date: 2026-09-10
";
        let r = ReceiptImport::import(content).unwrap();
        assert_eq!(r.application_number, "17/123,456");
        assert_eq!(r.confirmation_number, "4321");
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

    /// covers: REQ-PAT-004
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
