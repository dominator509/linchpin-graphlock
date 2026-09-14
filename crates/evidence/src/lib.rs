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
                if i > 0 && rng.next().is_multiple_of(3) {
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

/// Scrubs secrets from log lines before they leave the device.
///
/// GraphLock context (anti-gaming finding AG-011): `redact` previously only
/// replaced explicitly-registered literals **and** happily registered an empty
/// string, in which case `str::replace("")` matches at every position and
/// mangles the whole message. A redactor that only removes strings you already
/// knew about provides little protection: a token that reaches the log by a
/// path the caller did not anticipate passes through untouched, which is
/// exactly the case SECURITY.md cares about.
///
/// This version (a) ignores empty registrations so the redactor cannot destroy
/// a message, and (b) additionally scrubs token-shaped substrings that were
/// never registered. It is still not a guarantee — defence in depth, not a
/// substitute for not logging secrets.
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

    /// Register a literal to scrub. Empty values are ignored: registering `""`
    /// would otherwise match at every byte offset and replace the entire
    /// message with placeholders.
    pub fn register_secret(&mut self, secret: String) {
        if !secret.is_empty() {
            self.secrets.push(secret);
        }
    }

    /// Number of registered secrets.
    pub fn secret_count(&self) -> usize {
        self.secrets.len()
    }

    pub fn redact(&self, log_message: &str) -> String {
        let mut redacted = log_message.to_string();
        for secret in &self.secrets {
            redacted = redacted.replace(secret.as_str(), "[REDACTED]");
        }
        redact_token_like(&redacted)
    }
}

/// Replace token-shaped words with a placeholder.
///
/// Recognizes common credential prefixes and long hex runs, matching the
/// behaviour of `crash_reporter`'s policy so the two redactors agree.
fn redact_token_like(input: &str) -> String {
    const PREFIXES: [&str; 5] = ["sk-", "ghp_", "gho_", "ghs_", "xoxb-"];
    let mut out = String::with_capacity(input.len());
    for token in input.split_inclusive(char::is_whitespace) {
        let (word, tail) = match token.find(char::is_whitespace) {
            Some(i) => (&token[..i], &token[i..]),
            None => (token, ""),
        };
        let looks_secret = PREFIXES.iter().any(|p| word.starts_with(p))
            || (word.len() >= 32 && word.chars().all(|c| c.is_ascii_hexdigit()));
        if looks_secret {
            out.push_str("[REDACTED]");
        } else {
            out.push_str(word);
        }
        out.push_str(tail);
    }
    out
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

    /// PROBE: an unregistered but obviously secret-shaped value must not pass
    /// through a "redactor" untouched.
    #[test]
    fn probe_redactor_scrubs_unregistered_secrets() {
        let redactor = LogRedactor::new();
        let raw = "auth failed for sk-live-0123456789abcdef and ghp_ABCDEFGHIJKLMNOP";
        let clean = redactor.redact(raw);

        assert!(
            !clean.contains("sk-live-0123456789abcdef"),
            "leaked: {clean}"
        );
        assert!(!clean.contains("ghp_ABCDEFGHIJKLMNOP"), "leaked: {clean}");
    }

    /// An empty registration must not turn every position into a match.
    #[test]
    fn test_redactor_ignores_empty_secret() {
        let mut redactor = LogRedactor::new();
        redactor.register_secret(String::new());
        assert_eq!(redactor.redact("harmless text"), "harmless text");
    }

    /// Redaction must be idempotent and must not mangle ordinary text.
    #[test]
    fn test_redactor_preserves_ordinary_text() {
        let mut redactor = LogRedactor::new();
        redactor.register_secret("hunter2".to_string());

        let clean = redactor.redact("user hunter2 logged in from 10.0.0.1");
        assert_eq!(clean, "user [REDACTED] logged in from 10.0.0.1");
        assert_eq!(redactor.redact(&clean), clean, "not idempotent");
    }
}

/// Allow-list check for update payload signer identifiers.
///
/// GraphLock context (anti-gaming finding AG-012): `verify_update_payload`
/// compared a caller-supplied string against a list using
/// `allowed_signatures.contains(&signature.to_string())`. That is a **lookup in
/// an in-memory allow-list**, not signature verification: it performs no
/// cryptographic check, holds no public key, and would accept any string equal
/// to an allow-listed value. The method name asserted a security property the
/// code did not provide. In an update path (REQ-REL-005, DOD-035) that is
/// dangerous, because a caller could reasonably believe a payload's signature
/// had been validated.
///
/// The type is renamed to describe what it actually does. Cryptographic
/// verification is INCOMPLETE and is recorded as such rather than implied by a
/// misleading name.
#[derive(Default)]
pub struct UpdateSignerAllowlist {
    allowed_signers: Vec<String>,
}

impl UpdateSignerAllowlist {
    pub fn new() -> Self {
        UpdateSignerAllowlist {
            allowed_signers: Vec::new(),
        }
    }

    /// Allow a signer identifier. Empty values are ignored so an empty entry
    /// cannot match arbitrary input.
    pub fn register_trusted_key(&mut self, key_hash: String) {
        if !key_hash.is_empty() && !self.allowed_signers.contains(&key_hash) {
            self.allowed_signers.push(key_hash);
        }
    }

    /// True when `signer_id` is on the allow-list.
    ///
    /// This is an **identity allow-list, not signature verification**. Callers
    /// must not treat a `true` result as proof that a payload was
    /// cryptographically signed by that signer.
    pub fn is_allowlisted(&self, signer_id: &str) -> bool {
        !signer_id.is_empty() && self.allowed_signers.contains(&signer_id.to_string())
    }

    pub fn allowed_count(&self) -> usize {
        self.allowed_signers.len()
    }
}

#[cfg(test)]
mod threat_control_tests {
    use super::*;

    #[test]
    fn test_update_signature_verification() {
        let mut control = UpdateSignerAllowlist::new();
        control.register_trusted_key("trusted_hash_abc123".to_string());

        assert!(control.is_allowlisted("trusted_hash_abc123"));
        assert!(!control.is_allowlisted("malicious_hash_xyz999"));
    }

    /// AG-012 regression: an empty signer id must never match, and an empty
    /// registration must never be stored (it would match arbitrary input).
    #[test]
    fn test_allowlist_rejects_empty_values() {
        let mut control = UpdateSignerAllowlist::new();
        control.register_trusted_key(String::new());
        assert_eq!(control.allowed_count(), 0, "empty key was registered");
        assert!(!control.is_allowlisted(""), "empty signer matched");
        assert!(!control.is_allowlisted("anything"));

        control.register_trusted_key("k1".to_string());
        assert!(
            !control.is_allowlisted(""),
            "empty signer matched a real list"
        );
    }

    /// Registering the same signer twice must not duplicate it.
    #[test]
    fn test_allowlist_registration_is_idempotent() {
        let mut control = UpdateSignerAllowlist::new();
        control.register_trusted_key("k1".to_string());
        control.register_trusted_key("k1".to_string());
        assert_eq!(control.allowed_count(), 1);
    }

    /// Documents the limitation explicitly: this is not crypto. A caller can
    /// observe that no cryptographic material is held.
    #[test]
    fn test_allowlist_holds_no_cryptographic_material() {
        let mut control = UpdateSignerAllowlist::new();
        control.register_trusted_key("k1".to_string());
        // The only state is a list of identifier strings; there is no key,
        // no signature, and no digest. Cryptographic verification is not
        // implemented (recorded as INCOMPLETE, not claimed).
        assert_eq!(control.allowed_count(), 1);
    }
}
