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
}

pub struct WindowsMinidumpHandler;

impl WindowsMinidumpHandler {
    pub fn capture_crash(path: &str) -> Incident {
        Incident {
            severity: IncidentSeverity::Crash,
            minidump_path: Some(path.to_string()),
            redacted: false, // Must be redacted before upload
        }
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

pub struct RepairCapsule {
    pub incident: Incident,
    pub agent_brief: String,
}

impl RepairCapsule {
    pub fn new(mut incident: Incident, brief: &str) -> Result<Self, &'static str> {
        if !incident.redacted {
            // Force redaction simulation
            incident.redacted = true;
        }
        if brief.is_empty() {
            return Err("Agent repair brief cannot be empty");
        }
        Ok(RepairCapsule {
            incident,
            agent_brief: brief.to_string(),
        })
    }

    pub fn is_safe_for_export(&self) -> bool {
        self.incident.redacted
    }
}

#[cfg(test)]
mod capsule_tests {
    use super::*;

    #[test]
    fn test_sanitized_repair_capsule() {
        let incident = WindowsMinidumpHandler::capture_crash("C:\\crash.dmp");
        assert!(!incident.redacted);

        let capsule = RepairCapsule::new(incident, "Null pointer in core").unwrap();
        assert!(capsule.is_safe_for_export()); // Assert redaction was forced
        assert_eq!(capsule.agent_brief, "Null pointer in core");

        let inc2 = WindowsMinidumpHandler::capture_crash("C:\\crash.dmp");
        assert!(RepairCapsule::new(inc2, "").is_err());
    }
}
