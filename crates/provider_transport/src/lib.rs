use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderCapabilities {
    pub provider_name: String,
    pub supports_streaming: bool,
    pub is_local: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobEnvelope {
    pub job_id: String,
    pub prompt: String,
}

pub trait ProviderTransport {
    fn probe(&self) -> ProviderCapabilities;
    fn execute(&self, job: JobEnvelope) -> Result<String, String>;
}

pub struct LocalModelTransport;

impl ProviderTransport for LocalModelTransport {
    fn probe(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            provider_name: "LocalLlama".to_string(),
            supports_streaming: true,
            is_local: true,
        }
    }

    fn execute(&self, job: JobEnvelope) -> Result<String, String> {
        Ok(format!("Processed local job {}: {}", job.job_id, job.prompt))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_provider_transport() {
        let transport = LocalModelTransport;
        let caps = transport.probe();
        assert_eq!(caps.provider_name, "LocalLlama");
        assert!(caps.is_local);
        let res = transport.execute(JobEnvelope { job_id: "j1".into(), prompt: "Hello".into() }).unwrap();
        assert!(res.contains("Processed local job j1"));
    }
}
