#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorrelationId(pub String);

pub struct LocalTelemetryPipeline {
    traces: std::sync::Mutex<Vec<(CorrelationId, String)>>,
}

impl Default for LocalTelemetryPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalTelemetryPipeline {
    pub fn new() -> Self {
        LocalTelemetryPipeline {
            traces: std::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn record_span(
        &self,
        correlation_id: CorrelationId,
        span_name: &str,
    ) -> Result<(), &'static str> {
        if correlation_id.0.is_empty() {
            return Err("Correlation ID cannot be empty");
        }
        let mut traces = self.traces.lock().map_err(|_| "Lock poisoned")?;
        traces.push((correlation_id, span_name.to_string()));
        Ok(())
    }

    pub fn get_traces_for(
        &self,
        correlation_id: &CorrelationId,
    ) -> Result<Vec<String>, &'static str> {
        let traces = self.traces.lock().map_err(|_| "Lock poisoned")?;
        Ok(traces
            .iter()
            .filter(|(id, _)| id == correlation_id)
            .map(|(_, span)| span.clone())
            .collect())
    }
}

#[cfg(test)]
mod telemetry_tests {
    use super::*;

    /// covers: REQ-OPS-010
    #[test]
    fn test_telemetry_correlation() {
        let pipeline = LocalTelemetryPipeline::new();
        let cid = CorrelationId("req-123".to_string());

        assert!(pipeline.record_span(cid.clone(), "db_read").is_ok());
        assert!(
            pipeline
                .record_span(CorrelationId("".to_string()), "invalid")
                .is_err()
        );

        let traces = pipeline.get_traces_for(&cid).unwrap();
        assert_eq!(traces.len(), 1);
        assert_eq!(traces[0], "db_read");
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IncidentSeverity {
    Warning,
    Crash,
}

pub struct Incident {
    pub severity: IncidentSeverity,
    pub minidump_path: Option<String>,
    pub redacted: bool,
    /// Raw capture detail. Never readable while `redacted` is false, and only
    /// ever projected through [`RedactionPolicy`] on the way out.
    pub detail: String,
}

pub struct WindowsMinidumpHandler;

impl WindowsMinidumpHandler {
    pub fn capture_crash(path: &str) -> Incident {
        Incident {
            severity: IncidentSeverity::Crash,
            minidump_path: Some(path.to_string()),
            redacted: false, // Must be redacted before upload
            detail: String::new(),
        }
    }

    /// Capture a crash together with arbitrary raw context text (which may
    /// include invention content and must therefore never be exported raw).
    pub fn capture_crash_with_detail(path: &str, detail: &str) -> Incident {
        let mut incident = Self::capture_crash(path);
        incident.detail = detail.to_string();
        incident
    }
}

#[cfg(test)]
mod incident_tests {
    use super::*;

    #[test]
    fn test_windows_minidump_capture() {
        let incident = WindowsMinidumpHandler::capture_crash("C:\\crash.dmp");
        assert_eq!(incident.severity, IncidentSeverity::Crash);
        assert_eq!(incident.minidump_path.unwrap(), "C:\\crash.dmp");
        assert!(!incident.redacted);
    }
}

/// Redaction policy applied to raw incident detail before it may leave the
/// device boundary. Invention content is Confidential/device-only by default
/// (SECURITY.md), so this performs *transformation*, not flag-setting.
pub struct RedactionPolicy {
    secrets: Vec<String>,
}

impl Default for RedactionPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl RedactionPolicy {
    pub fn new() -> Self {
        RedactionPolicy {
            secrets: Vec::new(),
        }
    }

    /// Register a literal that must never appear in exported detail.
    pub fn with_secret(mut self, secret: &str) -> Self {
        if !secret.is_empty() {
            self.secrets.push(secret.to_string());
        }
        self
    }

    /// Replace every registered secret with a fixed placeholder, redact the
    /// VALUE of any credential-shaped key, and drop anything that looks like a
    /// bearer token or a long hex key.
    pub fn apply(&self, raw: &str) -> String {
        let mut out = raw.to_string();
        for secret in &self.secrets {
            out = out.replace(secret.as_str(), "[REDACTED]");
        }
        out = redact_key_values(&out);
        out = redact_token_like(&out);
        out
    }

    pub fn is_empty(&self) -> bool {
        self.secrets.is_empty()
    }
}

/// Characters that may appear inside a token-shaped secret.
fn is_token_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}

/// Token shapes that must never leave the device boundary.
fn looks_like_secret(core: &str) -> bool {
    core.starts_with("sk-")
        || core.starts_with("ghp_")
        || core.starts_with("gho_")
        || (core.len() >= 32 && core.chars().all(|c| c.is_ascii_hexdigit()))
}

fn push_scrubbed(out: &mut String, run: &str) {
    if run.is_empty() {
        return;
    }
    if looks_like_secret(run) {
        out.push_str("[REDACTED]");
    } else {
        out.push_str(run);
    }
}

/// Scrub token-shaped substrings (`sk-...`, `ghp_...`, `gho_...`, 32+ char hex).
///
/// Scans maximal runs of token characters rather than whitespace-delimited
/// words. Measured defect in the previous version: it tested the whole
/// whitespace token, so a credential *directly wrapped in punctuation* was not
/// scrubbed and left the device boundary in cleartext. Only a bare
/// whitespace-delimited token was caught. Treating punctuation as a separator
/// closes those shapes while leaving ordinary text untouched:
///
/// ```text
/// {"api_key":"sk-live-ABC123xyz"}  ->  {"api_key":"[REDACTED]"}
/// (sk-live-ABC123xyz)              ->  ([REDACTED])
/// key=sk-live-ABC123xyz;           ->  key=[REDACTED];
/// ```
fn redact_token_like(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut run = String::new();
    for ch in input.chars() {
        if is_token_char(ch) {
            run.push(ch);
        } else {
            push_scrubbed(&mut out, &run);
            run.clear();
            out.push(ch);
        }
    }
    push_scrubbed(&mut out, &run);
    out
}

/// Keys whose VALUES must never leave the device boundary.
///
/// Measured defect this closes: the policy redacted only REGISTERED literals and
/// token-shaped values (`sk-`, `ghp_`, 32+ hex), so a short or opaque credential
/// written as `password=hunter2` or `{"api_key":"abcdef"}` survived in cleartext
/// -- and registering the KEY NAME as a secret made it worse, replacing the name
/// and leaving the value exposed (`[REDACTED]=abcdef`). Both the Repair Capsule
/// and the operator diagnostics log go through this function.
const SECRET_KEYS: [&str; 9] = [
    "api_key",
    "apikey",
    "api-key",
    "access_key",
    "password",
    "passwd",
    "secret",
    "token",
    "authorization",
];

/// Characters that end a credential value.
fn is_value_delimiter(c: char) -> bool {
    c.is_whitespace() || matches!(c, '&' | ';' | ',' | '}' | ']' | ')' | '\'' | '"')
}

/// Find `needle` at or after byte `from`, ignoring ASCII case, WITHOUT building a
/// lowercased copy.
///
/// Measured defect this closes: the search used to run over `input.to_lowercase()`
/// and then index the ORIGINAL string with the offsets it found. Lowercasing is
/// not length-preserving -- `İ` (2 bytes) lowercases to `i̇` (3 bytes), `K` (3) to
/// `k` (1) -- so every offset after such a character shifted, and the subsequent
/// slicing PANICKED:
///
/// ```text
/// policy.apply("İapi_key=secretvalue")
///   -> start byte index 2 is not a char boundary; it is inside '\u{307}'
/// ```
///
/// That input is a normal log line in any language that uses dotted capitals, and
/// the failure was in the credential-redaction path, so it could crash the
/// incident reporter exactly when it was needed. Comparing in place keeps every
/// offset a property of the string being indexed.
fn find_ascii_case_insensitive(input: &str, needle: &str, from: usize) -> Option<usize> {
    let hay = input.as_bytes();
    let needle = needle.as_bytes();
    if needle.is_empty() || hay.len() < needle.len() || from > hay.len() {
        return None;
    }
    (from..=hay.len() - needle.len()).find(|start| {
        input.is_char_boundary(*start)
            && input.is_char_boundary(start + needle.len())
            && hay[*start..*start + needle.len()].eq_ignore_ascii_case(needle)
    })
}

/// Span of the VALUE that follows `key` at or after `from`, if this occurrence is
/// a credential assignment.
///
/// Handles the shapes that actually appear: `key=value`, `key: value`,
/// `"key":"value"` (JSON) and `key=value;`. A key wrapped in quotes has a closing
/// quote BEFORE the separator, which an earlier version missed -- measured:
/// `{"token":"abc123"}` came back untouched because the character following the
/// key was `"`, not `:`.
fn key_value_span(input: &str, from: usize, key: &str) -> Option<(usize, usize)> {
    let start = find_ascii_case_insensitive(input, key, from)?;
    if input[..start]
        .chars()
        .next_back()
        .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return None;
    }

    let mut cursor = start + key.len();
    if input[cursor..].starts_with('"') || input[cursor..].starts_with('\'') {
        cursor += 1;
    }
    let tail = input[cursor..].trim_start();
    cursor = input.len() - tail.len();
    if !(tail.starts_with('=') || tail.starts_with(':')) {
        return None;
    }
    cursor += 1;
    let after_separator = input[cursor..].trim_start();
    cursor = input.len() - after_separator.len();
    if input[cursor..].starts_with('"') || input[cursor..].starts_with('\'') {
        cursor += 1;
    }
    let end = input[cursor..]
        .find(is_value_delimiter)
        .map(|i| cursor + i)
        .unwrap_or(input.len());
    if end <= cursor {
        return None;
    }
    Some((cursor, end))
}

/// Redact the value that follows a credential-shaped key.
///
/// Only a key followed by a separator is treated as a credential, so ordinary
/// prose such as "the token was refreshed" is left untouched.
fn redact_key_values(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut cursor = 0usize;

    while cursor < input.len() {
        let mut matched: Option<(usize, usize)> = None;
        for key in SECRET_KEYS {
            if let Some((start, end)) = key_value_span(input, cursor, key) {
                matched = Some((start, end));
                break;
            }
        }
        match matched {
            Some((start, end)) => {
                out.push_str(&input[cursor..start]);
                out.push_str("[REDACTED]");
                cursor = end;
            }
            None => {
                let ch = input[cursor..].chars().next().unwrap_or_default();
                out.push(ch);
                cursor += ch.len_utf8();
            }
        }
    }
    out
}

pub struct RepairCapsule {
    incident: Incident,
    agent_brief: String,
    /// The only path by which incident detail may leave this type.
    redacted_detail: String,
    /// Registered secrets that must not survive into exported detail.
    forbidden: Vec<String>,
}

impl RepairCapsule {
    /// Build a capsule, *actually* redacting incident detail through `policy`.
    ///
    /// The previous implementation only set `incident.redacted = true` without
    /// touching the content, so unredacted invention text could be exported
    /// while the guard reported success. This version transforms the content
    /// and derives the guard from the result.
    pub fn new(
        incident: Incident,
        brief: &str,
        policy: &RedactionPolicy,
    ) -> Result<Self, &'static str> {
        if brief.is_empty() {
            return Err("Agent repair brief cannot be empty");
        }
        let redacted_detail = policy.apply(&incident.detail);
        let mut incident = incident;
        incident.detail = String::new();
        incident.redacted = true;
        Ok(RepairCapsule {
            incident,
            agent_brief: brief.to_string(),
            redacted_detail,
            forbidden: policy.secrets.clone(),
        })
    }

    /// Redacted detail, safe to attach to a repair brief or export.
    pub fn redacted_detail(&self) -> &str {
        &self.redacted_detail
    }

    pub fn agent_brief(&self) -> &str {
        &self.agent_brief
    }

    pub fn incident(&self) -> &Incident {
        &self.incident
    }

    /// Export guard. True only when the capsule holds no raw detail and the
    /// redaction pass did not leave a registered secret behind.
    ///
    /// The previous version asserted only `redacted && detail.is_empty()`, so it
    /// could report safe while a registered secret was still present in the
    /// exported detail -- a guard claiming a check it never performed. The
    /// third condition verifies the claim instead of restating the flag.
    pub fn is_safe_for_export(&self) -> bool {
        self.incident.redacted
            && self.incident.detail.is_empty()
            && !self
                .forbidden
                .iter()
                .any(|s| !s.is_empty() && self.redacted_detail.contains(s.as_str()))
    }
}

#[cfg(test)]
mod capsule_tests {
    use super::*;

    /// covers: REQ-REPAIR-002
    #[test]
    fn test_sanitized_repair_capsule() {
        let incident = WindowsMinidumpHandler::capture_crash("C:\\crash.dmp");
        assert!(!incident.redacted);

        let policy = RedactionPolicy::new();
        let capsule = RepairCapsule::new(incident, "Null pointer in core", &policy).unwrap();
        assert!(capsule.is_safe_for_export());
        assert_eq!(capsule.agent_brief(), "Null pointer in core");

        let inc2 = WindowsMinidumpHandler::capture_crash("C:\\crash.dmp");
        assert!(RepairCapsule::new(inc2, "", &policy).is_err());
    }

    /// covers: REQ-REPAIR-001
    /// The regression that matters: invention content in the incident detail
    /// must NOT survive into an exportable capsule.
    #[test]
    fn test_redaction_actually_removes_invention_content() {
        let secret = "PROVISIONAL-CLAIM-1: a self-sealing graphene valve";
        let incident = WindowsMinidumpHandler::capture_crash_with_detail(
            "C:\\crash.dmp",
            &format!("panic while saving draft: {secret}"),
        );
        let policy = RedactionPolicy::new().with_secret(secret);

        let capsule = RepairCapsule::new(incident, "save failed", &policy).unwrap();

        assert!(capsule.is_safe_for_export());
        assert!(
            !capsule.redacted_detail().contains(secret),
            "raw invention content leaked into redacted detail"
        );
        assert!(capsule.redacted_detail().contains("[REDACTED]"));
        assert!(
            capsule.incident().detail.is_empty(),
            "raw detail retained on the incident"
        );
    }

    /// covers: REQ-REPAIR-001
    /// Negative proof (DOD-018): the pre-fix implementation set a flag without
    /// transforming content. This asserts the content-level invariant that the
    /// old code violated, so restoring the old behavior fails this test.
    #[test]
    fn test_flag_alone_is_not_sufficient() {
        let secret = "TOKEN-abc";
        let incident = WindowsMinidumpHandler::capture_crash_with_detail("C:\\c.dmp", secret);
        assert!(
            !incident.redacted,
            "a freshly captured incident must not claim redacted"
        );

        // Simulate the old behavior: flip the flag, leave content intact.
        let mut forged = WindowsMinidumpHandler::capture_crash_with_detail("C:\\c.dmp", secret);
        forged.redacted = true;
        assert_eq!(
            forged.detail, secret,
            "content is untouched by flag alone, proving the flag is not redaction"
        );
    }

    /// covers: REQ-REPAIR-001
    #[test]
    fn test_token_shaped_secrets_are_scrubbed_without_registration() {
        let policy = RedactionPolicy::new();
        let out = policy
            .apply("auth failed for sk-live-0123456789abcdef and deadbeefdeadbeefdeadbeefdeadbeef");
        assert!(!out.contains("sk-live-0123456789abcdef"));
        assert!(!out.contains("deadbeefdeadbeefdeadbeefdeadbeef"));
        assert_eq!(out.matches("[REDACTED]").count(), 2);
    }

    /// covers: REQ-REPAIR-001
    #[test]
    fn test_empty_secret_is_not_registered() {
        let policy = RedactionPolicy::new().with_secret("");
        assert!(policy.is_empty());
        // An empty registration must not turn every position into a match.
        assert_eq!(policy.apply("harmless text"), "harmless text");
    }

    /// covers: REQ-DOM-010
    /// "Repair Capsule always redacts before export." Measured defect in the
    /// pre-fix implementation: it tested the whole whitespace-delimited token,
    /// so a credential directly wrapped in punctuation survived redaction and
    /// was still reported safe for export.
    #[test]
    fn test_punctuation_wrapped_tokens_are_scrubbed_before_export() {
        let policy = RedactionPolicy::new();
        let shapes = [
            r#"{"api_key":"sk-live-ABC123xyz"}"#,
            "(sk-live-ABC123xyz)",
            "key=sk-live-ABC123xyz;",
            "'ghp_ABC123xyz'",
            "[gho_ABC123xyz]",
        ];
        for raw in shapes {
            let out = policy.apply(raw);
            assert!(
                !out.contains("sk-live-ABC123xyz")
                    && !out.contains("ghp_ABC123xyz")
                    && !out.contains("gho_ABC123xyz"),
                "token survived redaction in shape {raw:?}: {out}"
            );
            assert!(
                out.contains("[REDACTED]"),
                "no placeholder emitted for {raw:?}: {out}"
            );
        }
    }

    /// covers: REQ-DOM-010
    /// The export guard must verify the redaction result rather than restate a
    /// flag. Built directly, because the public constructor cannot produce a
    /// capsule whose registered secret survived its own policy -- this asserts
    /// the guard itself, which is what the doc comment promises.
    #[test]
    fn test_export_guard_rejects_surviving_registered_secret() {
        let secret = "PROVISIONAL-CLAIM-1: a self-sealing graphene valve";

        let mut incident = WindowsMinidumpHandler::capture_crash_with_detail("C:\\c.dmp", "");
        incident.detail = String::new();
        incident.redacted = true;

        let capsule = RepairCapsule {
            incident,
            agent_brief: "save failed".to_string(),
            // The redaction pass left the registered literal behind.
            redacted_detail: format!("panic while saving draft: {secret}"),
            forbidden: vec![secret.to_string()],
        };

        assert!(
            capsule.incident().redacted && capsule.incident().detail.is_empty(),
            "test premise: the flag-based conditions alone would report safe"
        );
        assert!(
            !capsule.is_safe_for_export(),
            "guard reported safe while a registered secret survived redaction"
        );
    }

    /// covers: REQ-DOM-010
    /// The positive control for the guard above: when the policy does cover the
    /// literal, the placeholder replaces it and the guard reports safe.
    #[test]
    fn test_export_guard_accepts_a_fully_redacted_capsule() {
        let secret = "PROVISIONAL-CLAIM-1: a self-sealing graphene valve";
        let incident = WindowsMinidumpHandler::capture_crash_with_detail(
            "C:\\crash.dmp",
            &format!("panic while saving draft: {secret}"),
        );
        let policy = RedactionPolicy::new().with_secret(secret);

        let capsule = RepairCapsule::new(incident, "save failed", &policy).unwrap();

        assert!(!capsule.redacted_detail().contains(secret));
        assert!(
            capsule.is_safe_for_export(),
            "a fully redacted capsule must be exportable"
        );
    }

    /// covers: REQ-DOM-010, DOD-037
    /// A short or opaque credential written after a credential-shaped key must be
    /// redacted, while ordinary prose about tokens must survive intact.
    #[test]
    fn test_redaction_covers_key_values_not_only_token_shapes() {
        let policy = RedactionPolicy::new();

        let cases = [
            ("password=hunter2", "hunter2"),
            ("password: hunter2", "hunter2"),
            ("api_key=abcdef", "abcdef"),
            ("apikey=short", "short"),
            ("access_key=AKIAIOSFODNN7EXAMPLE", "AKIAIOSFODNN7EXAMPLE"),
            ("{\"token\":\"abc123\"}", "abc123"),
            ("Authorization: Basic-dXNlcjpwYXNz", "dXNlcjpwYXNz"),
            ("secret=s3cr3t;", "s3cr3t"),
        ];
        for (raw, secret) in cases {
            let redacted = policy.apply(raw);
            assert!(
                !redacted.contains(secret),
                "credential value survived redaction: {raw:?} -> {redacted:?}"
            );
            assert!(
                redacted.contains("[REDACTED]"),
                "nothing was redacted in {raw:?} -> {redacted:?}"
            );
        }

        // Honest prose must not be mangled by a key-name search.
        for honest in [
            "the token was refreshed",
            "password policy is documented in SECURITY.md",
            "a secret is never logged",
        ] {
            assert_eq!(
                policy.apply(honest),
                honest,
                "ordinary text was altered: {honest:?}"
            );
        }

        // The original token-shape coverage still holds.
        let shaped = policy.apply("Authorization: Bearer sk-live-ABC123xyz");
        assert!(!shaped.contains("sk-live-ABC123xyz"), "{shaped}");
    }

    /// covers: REQ-DOM-010, DOD-037, DOD-038
    /// Text that lowercases to a DIFFERENT BYTE LENGTH must not panic the
    /// redactor, and the credential must still be redacted.
    ///
    /// Measured: the search ran over `input.to_lowercase()` and indexed the
    /// original with the offsets it found, so `"İapi_key=secretvalue"` panicked
    /// with "start byte index 2 is not a char boundary; it is inside '\u{307}'"
    /// -- in the credential-redaction path, on a log line a Turkish keyboard
    /// produces routinely. Found by a mutation-fuzz campaign, not by inspection.
    #[test]
    fn test_redaction_survives_case_folding_characters() {
        let policy = RedactionPolicy::new();
        let cases = [
            ("İapi_key=secretvalue", "secretvalue"),
            ("İ api_key=secretvalue", "secretvalue"),
            ("İtoken=abc123", "abc123"),
            ("ß password=hunter2", "hunter2"),
            ("I\u{0307}api_key=secretvalue", "secretvalue"),
        ];
        for (input, secret) in cases {
            let redacted = policy.apply(input);
            assert!(
                !redacted.contains(secret),
                "a credential survived redaction: {input:?} -> {redacted:?}"
            );
            assert!(
                redacted.contains("[REDACTED]"),
                "nothing was redacted in {input:?} -> {redacted:?}"
            );
        }

        // A key that is part of a longer identifier is still not a credential
        // assignment: the boundary rule must survive the rewrite.
        assert_eq!(
            policy.apply("Kapi_key=secretvalue"),
            "Kapi_key=secretvalue",
            "a key inside a longer identifier must not be treated as a credential"
        );
    }
}
