//! Provider transport adapters.
//!
//! GraphLock context: this file previously contained two adapters that
//! fabricated model output (anti-gaming finding AG-001). `LocalModelAdapter`
//! returned `format!("Local generation for: {prompt}")` and `OpenAiAdapter`
//! returned `format!("OpenAI mock generation for: {prompt}")` with no
//! inference and no network call, while their tests asserted the mock strings.
//!
//! Per DOD-019/DOD-020 a production path may not return fabricated success, and
//! per DOD-010 a mock may not be the sole proof of a claimed integration. The
//! fake success paths are therefore removed. What remains is:
//!
//!   * [`ProviderTransport`] -- the boundary contract.
//!   * [`LocalModelAdapter`] -- a real HTTP client for a loopback
//!     llama.cpp/Ollama server, matching `PROVIDER_TRANSPORT_MATRIX.md`
//!     ("Local | llama.cpp / Ollama | local process/HTTP on loopback").
//!   * [`UnimplementedTransport`] -- an explicit, non-succeeding placeholder
//!     used for lanes that are not yet wired (OpenAI/Codex CLI, Anthropic,
//!     xAI). It always returns [`TransportError::Unimplemented`], so it can
//!     never be mistaken for working inference. This is the honest alternative
//!     to returning fabricated text.
//!
//! Unreachability note (measured): `grep provider_transport
//! apps/desktop/src-tauri/src/*.rs` returns nothing, so none of this is yet
//! reachable from the desktop boundary. The capability claim stays INCOMPLETE
//! until a real dependency execution is captured at the acceptance boundary.

use async_trait::async_trait;
use std::fmt::Debug;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRequest {
    pub prompt: String,
    pub model_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelResponse {
    pub text: String,
    pub metadata: String,
}

/// Failure modes for a provider call. Distinguishing these is required by
/// DOD-014: a wrong credential or unavailable dependency must fail closed and
/// accurately, never degrade into simulated success.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    /// The prompt was empty or otherwise invalid.
    InvalidRequest(String),
    /// The configured endpoint is not reachable (connection refused, DNS, etc).
    Unreachable(String),
    /// The endpoint answered but rejected or failed the request.
    ProviderFailure { status: u16, body: String },
    /// The endpoint answered successfully but the payload was not usable.
    InvalidResponse(String),
    /// This transport has no real implementation yet. Always an error, never a
    /// fabricated success.
    Unimplemented(String),
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportError::InvalidRequest(m) => write!(f, "invalid request: {m}"),
            TransportError::Unreachable(m) => write!(f, "endpoint unreachable: {m}"),
            TransportError::ProviderFailure { status, body } => {
                write!(f, "provider returned {status}: {body}")
            }
            TransportError::InvalidResponse(m) => write!(f, "invalid response: {m}"),
            TransportError::Unimplemented(m) => write!(f, "not implemented: {m}"),
        }
    }
}

impl std::error::Error for TransportError {}

#[async_trait]
pub trait ProviderTransport: Send + Sync + Debug {
    async fn generate(&self, request: ModelRequest) -> Result<ModelResponse, TransportError>;
    fn identity(&self) -> &str;
    /// True only when this transport can reach a real inference boundary.
    /// Callers must check this rather than assuming `generate` will succeed.
    fn is_live(&self) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalFlavor {
    /// llama.cpp server: POST {"prompt":..} -> {"content":..}
    LlamaCpp,
    /// Ollama: POST {"model":..,"prompt":..,"stream":false} -> {"response":..}
    Ollama,
}

/// HTTP client for a loopback llama.cpp or Ollama server.
#[derive(Debug, Clone)]
pub struct LocalModelAdapter {
    id: String,
    endpoint: String,
    model_id: String,
    client: reqwest::Client,
    flavor: LocalFlavor,
}

impl LocalModelAdapter {
    /// Build an adapter for a loopback endpoint.
    ///
    /// The endpoint is validated to be loopback-only: `PROVIDER_TRANSPORT_MATRIX.md`
    /// specifies "local process/HTTP on loopback", and invention content is
    /// Confidential/device-only by default, so a non-loopback host is refused
    /// rather than silently exfiltrating prompts.
    pub fn new(
        endpoint: &str,
        model_id: &str,
        flavor: LocalFlavor,
    ) -> Result<Self, TransportError> {
        let parsed = reqwest::Url::parse(endpoint)
            .map_err(|e| TransportError::InvalidRequest(format!("bad endpoint {endpoint}: {e}")))?;
        let host = parsed
            .host_str()
            .ok_or_else(|| TransportError::InvalidRequest("endpoint has no host".to_string()))?;
        let is_loopback = matches!(host, "localhost" | "127.0.0.1" | "::1");
        if !is_loopback {
            return Err(TransportError::InvalidRequest(format!(
                "local transport requires a loopback endpoint, got host {host:?}"
            )));
        }
        Ok(LocalModelAdapter {
            id: match flavor {
                LocalFlavor::LlamaCpp => "local-llamacpp".to_string(),
                LocalFlavor::Ollama => "local-ollama".to_string(),
            },
            endpoint: endpoint.trim_end_matches('/').to_string(),
            model_id: model_id.to_string(),
            client: reqwest::Client::new(),
            flavor,
        })
    }

    fn request_url(&self) -> String {
        match self.flavor {
            LocalFlavor::LlamaCpp => format!("{}/completion", self.endpoint),
            LocalFlavor::Ollama => format!("{}/api/generate", self.endpoint),
        }
    }

    fn request_body(&self, prompt: &str) -> serde_json::Value {
        match self.flavor {
            LocalFlavor::LlamaCpp => serde_json::json!({
                "prompt": prompt,
                "stream": false,
            }),
            LocalFlavor::Ollama => serde_json::json!({
                "model": self.model_id,
                "prompt": prompt,
                "stream": false,
            }),
        }
    }

    /// Extract the generated text from a provider payload, or fail closed.
    fn extract_text(&self, body: &serde_json::Value) -> Result<String, TransportError> {
        let field = match self.flavor {
            LocalFlavor::LlamaCpp => "content",
            LocalFlavor::Ollama => "response",
        };
        body.get(field)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                TransportError::InvalidResponse(format!("missing string field {field:?}"))
            })
    }
}

#[async_trait]
impl ProviderTransport for LocalModelAdapter {
    async fn generate(&self, request: ModelRequest) -> Result<ModelResponse, TransportError> {
        if request.prompt.trim().is_empty() {
            return Err(TransportError::InvalidRequest(
                "prompt cannot be empty".to_string(),
            ));
        }
        let url = self.request_url();
        let response = self
            .client
            .post(&url)
            .json(&self.request_body(&request.prompt))
            .send()
            .await
            .map_err(|e| TransportError::Unreachable(format!("{url}: {e}")))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| TransportError::InvalidResponse(format!("unreadable body: {e}")))?;
        if !status.is_success() {
            return Err(TransportError::ProviderFailure {
                status: status.as_u16(),
                body: text,
            });
        }
        let parsed: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| TransportError::InvalidResponse(format!("not JSON: {e}")))?;
        let output = self.extract_text(&parsed)?;
        Ok(ModelResponse {
            text: output,
            metadata: format!("{}:{}", self.id, self.model_id),
        })
    }

    fn identity(&self) -> &str {
        &self.id
    }

    fn is_live(&self) -> bool {
        true
    }
}

/// Explicit placeholder for provider lanes that are not yet wired to a real
/// boundary (OpenAI/Codex CLI, Anthropic, xAI ACP, generic API fallback).
///
/// This type exists so that unimplemented lanes fail honestly instead of
/// returning fabricated text. `generate` never succeeds.
#[derive(Debug, Clone)]
pub struct UnimplementedTransport {
    id: String,
    reason: String,
}

impl UnimplementedTransport {
    pub fn new(id: &str, reason: &str) -> Self {
        UnimplementedTransport {
            id: id.to_string(),
            reason: reason.to_string(),
        }
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

#[async_trait]
impl ProviderTransport for UnimplementedTransport {
    async fn generate(&self, _request: ModelRequest) -> Result<ModelResponse, TransportError> {
        Err(TransportError::Unimplemented(self.reason.clone()))
    }

    fn identity(&self) -> &str {
        &self.id
    }

    fn is_live(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_adapter_rejects_non_loopback_endpoint() {
        let err =
            LocalModelAdapter::new("https://api.example.com", "llama3", LocalFlavor::Ollama)
                .unwrap_err();
        assert!(
            matches!(err, TransportError::InvalidRequest(ref m) if m.contains("loopback")),
            "non-loopback endpoint must be refused, got {err:?}"
        );
    }

    #[test]
    fn test_local_adapter_accepts_loopback_and_builds_urls() {
        let a =
            LocalModelAdapter::new("http://127.0.0.1:11434", "llama3", LocalFlavor::Ollama).unwrap();
        assert_eq!(a.identity(), "local-ollama");
        assert!(a.is_live());
        assert_eq!(a.request_url(), "http://127.0.0.1:11434/api/generate");

        let b =
            LocalModelAdapter::new("http://localhost:8080/", "m", LocalFlavor::LlamaCpp).unwrap();
        assert_eq!(b.identity(), "local-llamacpp");
        assert_eq!(b.request_url(), "http://localhost:8080/completion");
    }

    #[test]
    fn test_extract_text_fails_closed_on_missing_field() {
        let a =
            LocalModelAdapter::new("http://127.0.0.1:11434", "m", LocalFlavor::Ollama).unwrap();
        let err = a
            .extract_text(&serde_json::json!({"wrong": "x"}))
            .unwrap_err();
        assert!(matches!(err, TransportError::InvalidResponse(_)));

        let ok = a
            .extract_text(&serde_json::json!({"response": "real text"}))
            .unwrap();
        assert_eq!(ok, "real text");
    }

    /// DOD-014 negative case: an unreachable endpoint must produce a real
    /// `Unreachable` error, not fabricated output. Port 1 is reserved and
    /// nothing listens there, so the connection fails fast.
    #[tokio::test]
    async fn test_unreachable_endpoint_fails_closed_not_fabricated() {
        let adapter =
            LocalModelAdapter::new("http://127.0.0.1:1", "llama3", LocalFlavor::Ollama).unwrap();
        let result = adapter
            .generate(ModelRequest {
                prompt: "hello".to_string(),
                model_id: "llama3".to_string(),
            })
            .await;
        assert!(
            matches!(result, Err(TransportError::Unreachable(_))),
            "expected Unreachable, got {result:?}"
        );
    }

    #[tokio::test]
    async fn test_empty_prompt_is_rejected_before_any_io() {
        let adapter =
            LocalModelAdapter::new("http://127.0.0.1:1", "llama3", LocalFlavor::Ollama).unwrap();
        let err = adapter
            .generate(ModelRequest {
                prompt: "   ".to_string(),
                model_id: "llama3".to_string(),
            })
            .await
            .unwrap_err();
        assert!(matches!(err, TransportError::InvalidRequest(_)));
    }

    /// The unimplemented lane must never return success.
    #[tokio::test]
    async fn test_unimplemented_transport_never_succeeds() {
        let t = UnimplementedTransport::new("openai-codex-cli", "Codex CLI boundary not wired");
        assert!(!t.is_live());
        let result = t
            .generate(ModelRequest {
                prompt: "Analyze patent".to_string(),
                model_id: "gpt-4".to_string(),
            })
            .await;
        assert!(
            matches!(result, Err(TransportError::Unimplemented(_))),
            "unimplemented lane must fail closed, got {result:?}"
        );
    }
}
