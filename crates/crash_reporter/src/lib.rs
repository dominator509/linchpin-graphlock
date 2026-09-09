use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepairCapsule {
    pub incident_id: String,
    pub sanitized_logs: String,
    pub reproduction_steps: String,
}

impl RepairCapsule {
    pub fn create_sanitized(id: String, logs: &str, steps: String) -> Self {
        let redacted = logs.replace("secret", "[REDACTED]").replace("api_key", "[REDACTED]");
        Self {
            incident_id: id,
            sanitized_logs: redacted,
            reproduction_steps: steps,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repair_capsule_creation() {
        let capsule = RepairCapsule::create_sanitized("inc1".into(), "log with secret_key", "1. Do x".into());
        assert_eq!(capsule.incident_id, "inc1");
        assert!(!capsule.sanitized_logs.contains("secret_key"));
    }
}
