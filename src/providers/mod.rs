pub mod error;
pub mod openai;
pub mod anthropic_compatible;
pub mod registry;
pub mod streaming;

use async_trait::async_trait;
use crate::models::{AnthropicRequest, CountTokensRequest, CountTokensResponse, ContentBlock};
use error::ProviderError;
use serde::{Deserialize, Serialize};
use bytes::Bytes;
use futures::stream::Stream;
use std::pin::Pin;

/// Provider response that maintains Anthropic API compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderResponse {
    pub id: String,
    pub r#type: String,
    pub role: String,
    pub content: Vec<ContentBlock>,
    pub model: String,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: Usage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// Main provider trait - all providers must implement this
/// Maintains Anthropic Messages API compatibility
#[async_trait]
pub trait AnthropicProvider: Send + Sync {
    /// Send a message request to the provider
    /// Must transform to/from Anthropic format as needed
    async fn send_message(&self, request: AnthropicRequest) -> Result<ProviderResponse, ProviderError>;

    /// Send a streaming message request to the provider
    /// Returns a stream of raw bytes (SSE format)
    async fn send_message_stream(
        &self,
        request: AnthropicRequest
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Bytes, ProviderError>> + Send>>, ProviderError>;

    /// Count tokens for a request
    /// Provider-specific implementation (tiktoken for OpenAI, etc.)
    async fn count_tokens(&self, request: CountTokensRequest) -> Result<CountTokensResponse, ProviderError>;

    /// Check if provider supports a specific model
    fn supports_model(&self, model: &str) -> bool;
}

/// Provider configuration from TOML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub provider_type: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub models: Vec<String>,
    pub enabled: Option<bool>,
}

impl ProviderConfig {
    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or(true)
    }
}

// Re-export provider implementations
pub use openai::OpenAIProvider;
pub use anthropic_compatible::AnthropicCompatibleProvider;
pub use registry::ProviderRegistry;
