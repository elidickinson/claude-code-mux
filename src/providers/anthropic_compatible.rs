use super::{AnthropicProvider, ProviderResponse, error::ProviderError};
use crate::models::{AnthropicRequest, CountTokensRequest, CountTokensResponse, MessageContent, ContentBlock};
use crate::auth::{TokenStore, OAuthClient, OAuthConfig};
use async_trait::async_trait;
use reqwest::Client;
use std::pin::Pin;
use futures::stream::Stream;
use bytes::Bytes;
use secrecy::ExposeSecret;

/// Check if a signature looks like it came from Anthropic.
/// Anthropic signatures are long base64 strings (200+ chars).
/// Other providers (MiniMax, OpenAI, etc.) may use various signature formats.
fn is_anthropic_signature(sig: &str) -> bool {
    // Anthropic signatures are very long base64 strings
    // Be conservative: only treat very long signatures (>150 chars) as Anthropic
    sig.len() > 150
}

/// Strip thinking blocks with incompatible signatures for the target provider.
/// Different providers sign thinking blocks with their own keys.
/// IMPORTANT: When switching between providers, we must be conservative and strip
/// ALL signed thinking blocks to avoid signature validation errors.
///
/// Strategy:
/// - For Anthropic native API: Keep only Anthropic signatures OR unsigned blocks
/// - For other providers: Strip ALL signed blocks (Anthropic and non-Anthropic)
///   to avoid any signature incompatibility issues
///
/// Also removes messages that become empty after stripping.
fn strip_incompatible_thinking_blocks(request: &mut AnthropicRequest, is_anthropic_target: bool) {
    let mut stripped_count = 0;

    for message in &mut request.messages {
        if let MessageContent::Blocks(blocks) = &mut message.content {
            let before_len = blocks.len();
            blocks.retain(|block| {
                match block {
                    ContentBlock::Thinking { raw } => {
                        let sig = raw.get("signature").and_then(|v| v.as_str()).unwrap_or("");

                        // Empty signature = unsigned block, always keep
                        if sig.is_empty() {
                            return true;
                        }

                        let sig_len = sig.len();
                        let is_anthropic_sig = is_anthropic_signature(sig);

                        // For Anthropic native API: only keep Anthropic signatures
                        let should_keep = if is_anthropic_target {
                            is_anthropic_sig
                        } else {
                            // For non-Anthropic providers: strip ALL signed thinking blocks
                            // This is conservative but safe - avoids signature validation errors
                            false
                        };

                        if !should_keep {
                            tracing::debug!(
                                "🧹 Stripping thinking block: target={}, sig_len={}, is_anthropic_sig={}",
                                if is_anthropic_target { "anthropic" } else { "non-anthropic" },
                                sig_len,
                                is_anthropic_sig
                            );
                        }

                        should_keep
                    }
                    _ => true,
                }
            });
            stripped_count += before_len - blocks.len();
        }
    }

    // Remove messages that became empty (except final assistant message which can be empty)
    let len = request.messages.len();
    request.messages.retain(|msg| {
        match &msg.content {
            MessageContent::Text(t) => !t.is_empty(),
            MessageContent::Blocks(b) => !b.is_empty(),
        }
    });
    let removed_msgs = len - request.messages.len();

    if stripped_count > 0 || removed_msgs > 0 {
        tracing::debug!("🧹 Stripped {} thinking blocks, removed {} empty messages", stripped_count, removed_msgs);
    }
}

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
    /// OAuth provider ID (if using OAuth instead of API key)
    oauth_provider: Option<String>,
    /// Token store for OAuth authentication
    token_store: Option<TokenStore>,
}

impl AnthropicCompatibleProvider {
    pub fn new(
        name: String,
        api_key: String,
        base_url: String,
        models: Vec<String>,
        oauth_provider: Option<String>,
        token_store: Option<TokenStore>,
    ) -> Self {
        Self {
            name,
            api_key,
            base_url,
            client: Client::new(),
            models,
            custom_headers: Vec::new(),
            oauth_provider,
            token_store,
        }
    }

    /// Create with custom headers
    #[allow(dead_code)]
    pub fn with_headers(
        name: String,
        api_key: String,
        base_url: String,
        models: Vec<String>,
        custom_headers: Vec<(String, String)>,
        oauth_provider: Option<String>,
        token_store: Option<TokenStore>,
    ) -> Self {
        Self {
            name,
            api_key,
            base_url,
            client: Client::new(),
            models,
            custom_headers,
            oauth_provider,
            token_store,
        }
    }

    /// Get authentication header value (API key or OAuth Bearer token)
    async fn get_auth_header(&self) -> Result<String, ProviderError> {
        // If OAuth provider is configured, use Bearer token
        if let Some(ref oauth_provider_id) = self.oauth_provider {
            if let Some(ref token_store) = self.token_store {
                // Try to get token from store
                if let Some(token) = token_store.get(oauth_provider_id) {
                    // Check if token needs refresh
                    if token.needs_refresh() {
                        tracing::info!("🔄 Token for '{}' needs refresh, refreshing...", oauth_provider_id);

                        // Refresh token
                        let config = OAuthConfig::anthropic();
                        let oauth_client = OAuthClient::new(config, token_store.clone());

                        match oauth_client.refresh_token(oauth_provider_id).await {
                            Ok(new_token) => {
                                tracing::info!("✅ Token refreshed successfully");
                                return Ok(new_token.access_token.expose_secret().to_string());
                            }
                            Err(e) => {
                                tracing::error!("❌ Failed to refresh token: {}", e);
                                return Err(ProviderError::AuthError(format!(
                                    "Failed to refresh OAuth token: {}", e
                                )));
                            }
                        }
                    } else {
                        // Token is still valid
                        return Ok(token.access_token.expose_secret().to_string());
                    }
                } else {
                    return Err(ProviderError::AuthError(format!(
                        "OAuth provider '{}' configured but no token found in store",
                        oauth_provider_id
                    )));
                }
            } else {
                return Err(ProviderError::AuthError(
                    "OAuth provider configured but TokenStore not available".to_string()
                ));
            }
        }

        // Fall back to API key
        Ok(self.api_key.clone())
    }

    /// Check if using OAuth authentication
    fn is_oauth(&self) -> bool {
        self.oauth_provider.is_some() && self.token_store.is_some()
    }

    /// Create Anthropic Native provider
    #[allow(dead_code)]
    pub fn anthropic(api_key: String, models: Vec<String>) -> Self {
        Self::new(
            "anthropic".to_string(),
            api_key,
            "https://api.anthropic.com".to_string(),
            models,
            None,
            None,
        )
    }

    /// Create OpenRouter provider
    #[allow(dead_code)]
    pub fn openrouter(api_key: String, models: Vec<String>) -> Self {
        Self::with_headers(
            "openrouter".to_string(),
            api_key,
            "https://openrouter.ai/api".to_string(),
            models,
            vec![
                ("HTTP-Referer".to_string(), "https://github.com/bahkchanhee/claude-code-mux".to_string()),
                ("X-Title".to_string(), "Claude Code Mux".to_string()),
            ],
            None,
            None,
        )
    }

    /// Create z.ai provider (Anthropic-compatible)
    pub fn zai(api_key: String, models: Vec<String>, token_store: Option<TokenStore>) -> Self {
        Self::new(
            "z.ai".to_string(),
            api_key,
            "https://api.z.ai/api/anthropic".to_string(),
            models,
            None,
            token_store,
        )
    }

    /// Create Minimax provider (Anthropic-compatible)
    pub fn minimax(api_key: String, models: Vec<String>, token_store: Option<TokenStore>) -> Self {
        Self::new(
            "minimax".to_string(),
            api_key,
            "https://api.minimax.io/anthropic".to_string(),
            models,
            None,
            token_store,
        )
    }

    /// Create ZenMux provider (Anthropic-compatible proxy)
    pub fn zenmux(api_key: String, models: Vec<String>, token_store: Option<TokenStore>) -> Self {
        Self::new(
            "zenmux".to_string(),
            api_key,
            "https://zenmux.ai/api/anthropic".to_string(),
            models,
            None,
            token_store,
        )
    }

    /// Create Kimi For Coding provider (Anthropic-compatible)
    pub fn kimi_coding(api_key: String, models: Vec<String>, token_store: Option<TokenStore>) -> Self {
        Self::new(
            "kimi-coding".to_string(),
            api_key,
            "https://api.kimi.com/coding".to_string(),
            models,
            None,
            token_store,
        )
    }
}

#[async_trait]
impl AnthropicProvider for AnthropicCompatibleProvider {
    async fn send_message(&self, request: AnthropicRequest) -> Result<ProviderResponse, ProviderError> {
        let url = format!("{}/v1/messages", self.base_url);

        // Strip thinking blocks with incompatible signatures
        let mut request = request;
        let is_anthropic = self.base_url.contains("anthropic.com");
        strip_incompatible_thinking_blocks(&mut request, is_anthropic);

        // Get authentication header value (API key or OAuth token)
        let auth_value = self.get_auth_header().await?;

        // Build request with authentication
        let mut req_builder = self.client
            .post(&url)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json");

        // Set auth header based on OAuth vs API key
        if self.is_oauth() {
            // OAuth: Use Authorization Bearer token
            req_builder = req_builder
                .header("Authorization", format!("Bearer {}", auth_value))
                .header("anthropic-beta", "oauth-2025-04-20,claude-code-20250219,interleaved-thinking-2025-05-14,fine-grained-tool-streaming-2025-05-14");
            tracing::debug!("🔐 Using OAuth Bearer token for {}", self.name);
        } else {
            // API Key: Use x-api-key
            req_builder = req_builder.header("x-api-key", auth_value);
        }

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

            // If 401 and using OAuth, token might be invalid/expired
            if status == 401 && self.is_oauth() {
                tracing::warn!("🔄 Received 401, OAuth token may be invalid or expired");
            }

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

            // Get authentication
            let auth_value = self.get_auth_header().await?;

            let mut req_builder = self.client
                .post(&url)
                .header("anthropic-version", "2023-06-01")
                .header("Content-Type", "application/json");

            // Set auth header
            if self.is_oauth() {
                req_builder = req_builder
                    .header("Authorization", format!("Bearer {}", auth_value))
                    .header("anthropic-beta", "oauth-2025-04-20,claude-code-20250219,interleaved-thinking-2025-05-14,fine-grained-tool-streaming-2025-05-14");
            } else {
                req_builder = req_builder.header("x-api-key", auth_value);
            }

            let response = req_builder
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
                                crate::models::ContentBlock::Text { text, .. } => Some(text.clone()),
                                crate::models::ContentBlock::ToolResult { content, .. } => {
                                    Some(content.to_string())
                                }
                                crate::models::ContentBlock::Thinking { raw } => {
                                    raw.get("thinking").and_then(|v| v.as_str()).map(|s| s.to_string())
                                }
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

        // Strip thinking blocks with incompatible signatures
        let mut request = request;
        let is_anthropic = self.base_url.contains("anthropic.com");
        strip_incompatible_thinking_blocks(&mut request, is_anthropic);

        // Get authentication header value
        let auth_value = self.get_auth_header().await?;

        // Build request with authentication
        let mut req_builder = self.client
            .post(&url)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json");

        // Set auth header based on OAuth vs API key
        if self.is_oauth() {
            req_builder = req_builder
                .header("Authorization", format!("Bearer {}", auth_value))
                .header("anthropic-beta", "oauth-2025-04-20,claude-code-20250219,interleaved-thinking-2025-05-14,fine-grained-tool-streaming-2025-05-14");
            tracing::debug!("🔐 Using OAuth Bearer token for streaming on {}", self.name);
        } else {
            req_builder = req_builder.header("x-api-key", auth_value);
        }

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

            if status == 401 && self.is_oauth() {
                tracing::warn!("🔄 Received 401 on streaming, OAuth token may be invalid or expired");
            }

            return Err(ProviderError::ApiError {
                status,
                message: format!("{} API error: {}", self.name, error_text),
            });
        }

        // Wrap stream with logging to capture cache statistics
        use crate::providers::streaming::LoggingSseStream;
        let byte_stream = response.bytes_stream().map_err(|e| ProviderError::HttpError(e));
        let logging_stream = LoggingSseStream::new(byte_stream, self.name.clone());

        Ok(Box::pin(logging_stream))
    }

    fn supports_model(&self, model: &str) -> bool {
        self.models.iter().any(|m| m.eq_ignore_ascii_case(model))
    }
}
