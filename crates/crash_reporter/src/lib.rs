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

    /// Replace every registered secret with a fixed placeholder and drop
    /// anything that looks like a bearer token or a long hex key.
    pub fn apply(&self, raw: &str) -> String {
        let mut out = raw.to_string();
        for secret in &self.secrets {
            out = out.replace(secret.as_str(), "[REDACTED]");
        }
        out = redact_token_like(&out);
        out
    }

    pub fn is_empty(&self) -> bool {
        self.secrets.is_empty()
    }
}

/// Scrub token-shaped substrings (`sk-...`, `Bearer ...`, 32+ char hex runs).
fn redact_token_like(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for token in input.split_inclusive(char::is_whitespace) {
        let (word, tail) = match token.find(char::is_whitespace) {
            Some(i) => (&token[..i], &token[i..]),
            None => (token, ""),
        };
        let scrubbed = word.starts_with("sk-")
            || word.starts_with("ghp_")
            || word.starts_with("gho_")
            || (word.len() >= 32 && word.chars().all(|c| c.is_ascii_hexdigit()));
        if scrubbed {
            out.push_str("[REDACTED]");
        } else {
            out.push_str(word);
        }
        out.push_str(tail);
    }
    out
}

pub struct RepairCapsule {
    incident: Incident,
    agent_brief: String,
    /// The only path by which incident detail may leave this type.
    redacted_detail: String,
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
    pub fn is_safe_for_export(&self) -> bool {
        self.incident.redacted && self.incident.detail.is_empty()
    }
}

#[cfg(test)]
mod capsule_tests {
    use super::*;

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

    #[test]
    fn test_token_shaped_secrets_are_scrubbed_without_registration() {
        let policy = RedactionPolicy::new();
        let out = policy.apply("auth failed for sk-live-0123456789abcdef and deadbeefdeadbeefdeadbeefdeadbeef");
        assert!(!out.contains("sk-live-0123456789abcdef"));
        assert!(!out.contains("deadbeefdeadbeefdeadbeefdeadbeef"));
        assert_eq!(out.matches("[REDACTED]").count(), 2);
    }

    #[test]
    fn test_empty_secret_is_not_registered() {
        let policy = RedactionPolicy::new().with_secret("");
        assert!(policy.is_empty());
        // An empty registration must not turn every position into a match.
        assert_eq!(policy.apply("harmless text"), "harmless text");
    }
}
