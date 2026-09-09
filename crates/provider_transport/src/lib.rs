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

#[async_trait]
pub trait ProviderTransport: Send + Sync + Debug {
    async fn generate(&self, request: ModelRequest) -> Result<ModelResponse, String>;
    fn identity(&self) -> &str;
}

#[derive(Debug, Clone)]
pub struct LocalModelAdapter {
    id: String,
}

impl LocalModelAdapter {
    pub fn new() -> Self {
        LocalModelAdapter {
            id: "local-llama3-proof".to_string(),
        }
    }
}

impl Default for LocalModelAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProviderTransport for LocalModelAdapter {
    async fn generate(&self, request: ModelRequest) -> Result<ModelResponse, String> {
        if request.prompt.is_empty() {
            return Err("Prompt cannot be empty".to_string());
        }
        Ok(ModelResponse {
            text: format!("Local generation for: {}", request.prompt),
            metadata: "simulated-local-inference".to_string(),
        })
    }

    fn identity(&self) -> &str {
        &self.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_local_model_adapter() {
        let adapter = LocalModelAdapter::new();
        assert_eq!(adapter.identity(), "local-llama3-proof");

        let request = ModelRequest {
            prompt: "Hello local model".to_string(),
            model_id: "llama3".to_string(),
        };

        let response = adapter.generate(request).await.unwrap();
        assert!(response.text.contains("Hello local model"));
        assert_eq!(response.metadata, "simulated-local-inference");

        let err_req = ModelRequest {
            prompt: "".to_string(),
            model_id: "llama3".to_string(),
        };
        assert!(adapter.generate(err_req).await.is_err());
    }
}

#[derive(Debug, Clone)]
pub struct OpenAiAdapter {
    id: String,
    api_key_ref: String, // reference to keyring, not the raw key
}

impl OpenAiAdapter {
    pub fn new(api_key_ref: &str) -> Self {
        OpenAiAdapter {
            id: "openai-official".to_string(),
            api_key_ref: api_key_ref.to_string(),
        }
    }
}

#[async_trait]
impl ProviderTransport for OpenAiAdapter {
    async fn generate(&self, request: ModelRequest) -> Result<ModelResponse, String> {
        // Without making actual network calls, simulate terms-gated response
        if self.api_key_ref.is_empty() {
            return Err("Missing credentials reference".to_string());
        }
        Ok(ModelResponse {
            text: format!("OpenAI mock generation for: {}", request.prompt),
            metadata: "openai-gated-api".to_string(),
        })
    }

    fn identity(&self) -> &str {
        &self.id
    }
}

#[cfg(test)]
mod openai_tests {
    use super::*;

    #[tokio::test]
    async fn test_openai_adapter() {
        let adapter = OpenAiAdapter::new("keyring:openai_prod_key");
        assert_eq!(adapter.identity(), "openai-official");

        let request = ModelRequest {
            prompt: "Analyze patent".to_string(),
            model_id: "gpt-4".to_string(),
        };

        let response = adapter.generate(request).await.unwrap();
        assert!(response.text.contains("Analyze patent"));
        assert_eq!(response.metadata, "openai-gated-api");

        let err_adapter = OpenAiAdapter::new("");
        let err_req = ModelRequest {
            prompt: "Test".to_string(),
            model_id: "gpt-4".to_string(),
        };
        assert!(err_adapter.generate(err_req).await.is_err());
    }
}
