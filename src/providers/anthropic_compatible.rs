use super::{AnthropicProvider, ProviderResponse, error::ProviderError};
use crate::models::{AnthropicRequest, CountTokensRequest, CountTokensResponse};
use async_trait::async_trait;
use reqwest::Client;
use std::pin::Pin;
use futures::stream::Stream;
use bytes::Bytes;

/// Generic Anthropic-compatible provider
/// Works with: Anthropic, OpenRouter, z.ai, Minimax, etc.
/// Any provider that accepts Anthropic Messages API format
pub struct AnthropicCompatibleProvider {
    name: String,
    api_key: String,
    base_url: String,
    client: Client,
    models: Vec<String>,
    /// Custom headers to add (e.g., "HTTP-Referer" for OpenRouter)
    custom_headers: Vec<(String, String)>,
}

impl AnthropicCompatibleProvider {
    pub fn new(
        name: String,
        api_key: String,
        base_url: String,
        models: Vec<String>,
        custom_headers: Option<Vec<(String, String)>>,
    ) -> Self {
        Self {
            name,
            api_key,
            base_url,
            client: Client::new(),
            models,
            custom_headers: custom_headers.unwrap_or_default(),
        }
    }

    /// Create Anthropic Native provider
    pub fn anthropic(api_key: String, models: Vec<String>) -> Self {
        Self::new(
            "anthropic".to_string(),
            api_key,
            "https://api.anthropic.com".to_string(),
            models,
            None,
        )
    }

    /// Create OpenRouter provider
    pub fn openrouter(api_key: String, models: Vec<String>) -> Self {
        Self::new(
            "openrouter".to_string(),
            api_key,
            "https://openrouter.ai/api".to_string(),
            models,
            Some(vec![
                ("HTTP-Referer".to_string(), "https://github.com/bahkchanhee/claude-code-mux".to_string()),
                ("X-Title".to_string(), "Claude Code Mux".to_string()),
            ]),
        )
    }

    /// Create z.ai provider (Anthropic-compatible)
    pub fn zai(api_key: String, models: Vec<String>) -> Self {
        Self::new(
            "z.ai".to_string(),
            api_key,
            "https://api.z.ai/api/anthropic".to_string(),
            models,
            None,
        )
    }

    /// Create Minimax provider (Anthropic-compatible)
    pub fn minimax(api_key: String, models: Vec<String>) -> Self {
        Self::new(
            "minimax".to_string(),
            api_key,
            "https://api.minimax.io/anthropic".to_string(),
            models,
            None,
        )
    }

    /// Create ZenMux provider (Anthropic-compatible proxy)
    pub fn zenmux(api_key: String, models: Vec<String>) -> Self {
        Self::new(
            "zenmux".to_string(),
            api_key,
            "https://zenmux.ai/api/anthropic".to_string(),
            models,
            None,
        )
    }

    /// Create Kimi For Coding provider (Anthropic-compatible)
    pub fn kimi_coding(api_key: String, models: Vec<String>) -> Self {
        Self::new(
            "kimi-coding".to_string(),
            api_key,
            "https://api.kimi.com/coding".to_string(),
            models,
            None,
        )
    }
}

#[async_trait]
impl AnthropicProvider for AnthropicCompatibleProvider {
    async fn send_message(&self, request: AnthropicRequest) -> Result<ProviderResponse, ProviderError> {
        let url = format!("{}/v1/messages", self.base_url);

        // Build request with authentication
        let mut req_builder = self.client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json");

        // Add custom headers (for OpenRouter, etc.)
        for (key, value) in &self.custom_headers {
            req_builder = req_builder.header(key, value);
        }

        // Send request (pass-through, no transformation needed!)
        let response = req_builder
            .json(&request)
            .send()
            .await?;

        // Check for errors
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ProviderError::ApiError {
                status,
                message: format!("{} API error: {}", self.name, error_text),
            });
        }

        // Get response body as text for debugging
        let response_text = response.text().await?;
        tracing::debug!("{} provider response body: {}", self.name, response_text);

        // Try to parse the response (already in Anthropic format!)
        let provider_response: ProviderResponse = serde_json::from_str(&response_text)
            .map_err(|e| {
                tracing::error!("Failed to parse {} response: {}", self.name, e);
                tracing::error!("Response body was: {}", response_text);
                e
            })?;

        Ok(provider_response)
    }

    async fn count_tokens(&self, request: CountTokensRequest) -> Result<CountTokensResponse, ProviderError> {
        // For Anthropic native, use their count_tokens endpoint
        if self.name == "anthropic" {
            let url = format!("{}/v1/messages/count_tokens", self.base_url);

            let response = self.client
                .post(&url)
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01")
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .await?;

            if !response.status().is_success() {
                let status = response.status().as_u16();
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                return Err(ProviderError::ApiError {
                    status,
                    message: error_text,
                });
            }

            let count_response: CountTokensResponse = response.json().await?;
            return Ok(count_response);
        }

        // For other providers, use character-based estimation
        let mut total_chars = 0;

        if let Some(ref system) = request.system {
            let system_text = match system {
                crate::models::SystemPrompt::Text(text) => text.clone(),
                crate::models::SystemPrompt::Blocks(blocks) => {
                    blocks.iter().map(|b| b.text.clone()).collect::<Vec<_>>().join("\n")
                }
            };
            total_chars += system_text.len();
        }

        for msg in &request.messages {
            use crate::models::MessageContent;
            let content = match &msg.content {
                MessageContent::Text(text) => text.clone(),
                MessageContent::Blocks(blocks) => {
                    blocks.iter()
                        .filter_map(|block| {
                            match block {
                                crate::models::ContentBlock::Text { text } => Some(text.clone()),
                                _ => None,
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                }
            };
            total_chars += content.len();
        }

        let estimated_tokens = (total_chars / 4) as u32;

        Ok(CountTokensResponse {
            input_tokens: estimated_tokens,
        })
    }

    async fn send_message_stream(
        &self,
        request: AnthropicRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Bytes, ProviderError>> + Send>>, ProviderError> {
        use futures::stream::TryStreamExt;

        let url = format!("{}/v1/messages", self.base_url);

        // Build request with authentication
        let mut req_builder = self.client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json");

        // Add custom headers
        for (key, value) in &self.custom_headers {
            req_builder = req_builder.header(key, value);
        }

        // Send request with stream=true
        let response = req_builder
            .json(&request)
            .send()
            .await?;

        // Check for errors
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ProviderError::ApiError {
                status,
                message: format!("{} API error: {}", self.name, error_text),
            });
        }

        // Return the byte stream directly
        let stream = response.bytes_stream().map_err(|e| ProviderError::HttpError(e));

        Ok(Box::pin(stream))
    }

    fn supports_model(&self, model: &str) -> bool {
        self.models.iter().any(|m| m == model)
    }
}
