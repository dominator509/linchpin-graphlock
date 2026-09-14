#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sensitivity {
    Public,
    Confidential,
    Restricted,
}

#[derive(Debug, Clone)]
pub struct ExportPayload {
    pub content: String,
    pub sensitivity: Sensitivity,
}

pub struct DisclosureFirewall {
    pub block_restricted: bool,
}

impl Default for DisclosureFirewall {
    fn default() -> Self {
        Self::new()
    }
}

impl DisclosureFirewall {
    pub fn new() -> Self {
        DisclosureFirewall {
            block_restricted: true,
        }
    }

    pub fn filter_export(&self, payload: &ExportPayload) -> Result<(), &'static str> {
        if self.block_restricted && payload.sensitivity == Sensitivity::Restricted {
            return Err("Cannot export restricted payload under current firewall rules");
        }
        Ok(())
    }
}

#[cfg(test)]
mod firewall_tests {
    use super::*;

    #[test]
    fn test_disclosure_firewall() {
        let firewall = DisclosureFirewall::new();

        let public_payload = ExportPayload {
            content: "public info".to_string(),
            sensitivity: Sensitivity::Public,
        };
        assert!(firewall.filter_export(&public_payload).is_ok());

        let restricted_payload = ExportPayload {
            content: "secret invention".to_string(),
            sensitivity: Sensitivity::Restricted,
        };
        assert!(firewall.filter_export(&restricted_payload).is_err());
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionImpact {
    Low,
    Medium,
    High,
}

pub struct EgressPolicy {
    pub requires_approval_for_high_impact: bool,
}

impl Default for EgressPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl EgressPolicy {
    pub fn new() -> Self {
        EgressPolicy {
            requires_approval_for_high_impact: true,
        }
    }

    pub fn evaluate_action(
        &self,
        impact: ActionImpact,
        approved: bool,
    ) -> Result<(), &'static str> {
        if self.requires_approval_for_high_impact && impact == ActionImpact::High && !approved {
            return Err("High impact action requires explicit approval");
        }
        Ok(())
    }
}

#[cfg(test)]
mod policy_tests {
    use super::*;

    #[test]
    fn test_egress_policy() {
        let policy = EgressPolicy::new();
        assert!(policy.evaluate_action(ActionImpact::Low, false).is_ok());
        assert!(policy.evaluate_action(ActionImpact::High, false).is_err());
        assert!(policy.evaluate_action(ActionImpact::High, true).is_ok());
    }
}

pub struct InputHardener;

impl InputHardener {
    pub fn sanitize_path(path: &str) -> Result<String, &'static str> {
        if path.contains("..") || path.starts_with('/') {
            return Err("Path traversal or absolute path detected");
        }
        Ok(path.to_string())
    }

    pub fn sanitize_archive_entry(entry: &str) -> Result<String, &'static str> {
        // Naive archive path check
        Self::sanitize_path(entry)
    }
}

#[cfg(test)]
mod hardening_tests {
    use super::*;

    #[test]
    fn test_input_hardening() {
        assert!(InputHardener::sanitize_path("valid/path.txt").is_ok());
        assert!(InputHardener::sanitize_path("../invalid/path.txt").is_err());
        assert!(InputHardener::sanitize_path("/absolute/path.txt").is_err());
    }

    /// Deterministic xorshift64 PRNG. Avoids adding a dependency (AGENTS.md
    /// §10) while still generating a large, varied, reproducible corpus.
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.0 = x;
            x
        }

        fn pick<'a>(&mut self, alphabet: &[&'a str]) -> &'a str {
            alphabet[(self.next() % alphabet.len() as u64) as usize]
        }
    }

    /// Property test over generated inputs: `sanitize_path` must never panic,
    /// and must reject every path that resolves to an escape or an absolute
    /// root. This replaces the previous `test_fuzz_target_simulation`, which
    /// discarded results and asserted nothing about correctness (AG-003).
    #[test]
    fn test_sanitize_path_property_over_generated_corpus() {
        const ALPHABET: &[&str] = &[
            "", ".", "..", "a", "b", "c", "dir", "file.txt", "/", "\\", ":", "%2e", "~", "..\\",
        ];
        let mut rng = Rng(0x2545F4914F6CDD1D);
        let mut traversals_seen = 0usize;
        let mut accepted_seen = 0usize;

        for _ in 0..20_000 {
            let parts = (rng.next() % 5) as usize + 1;
            let mut candidate = String::new();
            for i in 0..parts {
                if i > 0 && rng.next() % 3 == 0 {
                    candidate.push('/');
                }
                candidate.push_str(rng.pick(ALPHABET));
            }

            // Must never panic.
            let verdict = InputHardener::sanitize_path(&candidate);

            let is_absolute = candidate.starts_with('/');
            let has_traversal = candidate.contains("..");
            if is_absolute || has_traversal {
                traversals_seen += 1;
                assert!(
                    verdict.is_err(),
                    "permissive acceptance of escaping path: {candidate:?}"
                );
            } else {
                accepted_seen += 1;
                assert!(
                    verdict.is_ok(),
                    "false rejection of benign path: {candidate:?}"
                );
            }
        }

        // Prove the corpus actually exercised both branches, so a degenerate
        // generator cannot make this test vacuous.
        assert!(
            traversals_seen > 1000,
            "corpus failed to generate enough escaping paths: {traversals_seen}"
        );
        assert!(
            accepted_seen > 1000,
            "corpus failed to generate enough benign paths: {accepted_seen}"
        );
    }

    /// Archive entries must be held to the same rule as plain paths; a zip-slip
    /// entry must not be accepted.
    #[test]
    fn test_archive_entry_rejects_zip_slip() {
        assert!(InputHardener::sanitize_archive_entry("ok/inside.txt").is_ok());
        assert!(InputHardener::sanitize_archive_entry("../../etc/passwd").is_err());
        assert!(InputHardener::sanitize_archive_entry("/etc/passwd").is_err());
    }
}

pub struct LogRedactor {
    secrets: Vec<String>,
}

impl Default for LogRedactor {
    fn default() -> Self {
        Self::new()
    }
}

impl LogRedactor {
    pub fn new() -> Self {
        LogRedactor {
            secrets: Vec::new(),
        }
    }

    pub fn register_secret(&mut self, secret: String) {
        self.secrets.push(secret);
    }

    pub fn redact(&self, log_message: &str) -> String {
        let mut redacted = log_message.to_string();
        for secret in &self.secrets {
            redacted = redacted.replace(secret, "[REDACTED]");
        }
        redacted
    }
}

#[cfg(test)]
mod redaction_tests {
    use super::*;

    #[test]
    fn test_secret_redaction_canary() {
        let mut redactor = LogRedactor::new();
        redactor.register_secret("sk-123456789".to_string());

        let raw_log = "Error connecting with api key sk-123456789 at endpoint X";
        let clean_log = redactor.redact(raw_log);

        assert!(!clean_log.contains("sk-123456789"));
        assert!(clean_log.contains("[REDACTED]"));
    }
}

pub struct UpdateThreatControl {
    pub allowed_signatures: Vec<String>,
}

impl Default for UpdateThreatControl {
    fn default() -> Self {
        Self::new()
    }
}

impl UpdateThreatControl {
    pub fn new() -> Self {
        UpdateThreatControl {
            allowed_signatures: Vec::new(),
        }
    }

    pub fn register_trusted_key(&mut self, key_hash: String) {
        self.allowed_signatures.push(key_hash);
    }

    pub fn verify_update_payload(&self, signature: &str) -> Result<(), &'static str> {
        if self.allowed_signatures.contains(&signature.to_string()) {
            Ok(())
        } else {
            Err("Untrusted update signature rejected")
        }
    }
}

#[cfg(test)]
mod threat_control_tests {
    use super::*;

    #[test]
    fn test_update_signature_verification() {
        let mut control = UpdateThreatControl::new();
        control.register_trusted_key("trusted_hash_abc123".to_string());

        assert!(control.verify_update_payload("trusted_hash_abc123").is_ok());
        assert!(
            control
                .verify_update_payload("malicious_hash_xyz999")
                .is_err()
        );
    }
}
