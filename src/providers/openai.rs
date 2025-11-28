use super::{AnthropicProvider, ProviderResponse, ContentBlock, Usage, error::ProviderError};
use crate::models::{AnthropicRequest, CountTokensRequest, CountTokensResponse, MessageContent};
use crate::auth::{OAuthClient, OAuthConfig, TokenStore};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use reqwest::Client;
use std::pin::Pin;
use futures::stream::Stream;
use bytes::Bytes;
use base64::{Engine as _, engine::general_purpose};

/// Official Codex instructions from OpenAI
/// Source: https://github.com/openai/codex (rust-v0.58.0)
const CODEX_INSTRUCTIONS: &str = include_str!("codex_instructions.md");

/// OpenAI Chat Completions request format
#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OpenAITool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<serde_json::Value>,
}

/// OpenAI Responses API request format (for Codex models)
#[derive(Debug, Serialize)]
struct OpenAIResponsesRequest {
    model: String,
    input: OpenAIResponsesInput,
    /// System instructions for the model (required for ChatGPT Codex)
    instructions: String,
    /// Whether to store the conversation (must be false for ChatGPT backend)
    store: bool,
    /// Enable streaming responses
    stream: bool,
    // Note: ChatGPT Codex does NOT support max_output_tokens, max_tokens, temperature, top_p, stop
}

/// Input for Responses API can be string or array of messages
#[derive(Debug, Serialize)]
#[serde(untagged)]
enum OpenAIResponsesInput {
    Text(String),
    Messages(Vec<OpenAIResponsesMessage>),
}

/// Message format for Responses API
#[derive(Debug, Serialize)]
struct OpenAIResponsesMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
}

/// Content can be string or array of content parts
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum OpenAIContent {
    String(String),
    Parts(Vec<OpenAIContentPart>),
}

/// Content part (text or image_url)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum OpenAIContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: OpenAIImageUrl },
}

/// Image URL object
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIImageUrl {
    url: String,
}

/// Tool call in assistant message
#[derive(Debug, Serialize, Deserialize)]
struct OpenAIToolCall {
    id: String,
    r#type: String, // "function"
    function: OpenAIFunctionCall,
}

/// Function call details
#[derive(Debug, Serialize, Deserialize)]
struct OpenAIFunctionCall {
    name: String,
    arguments: String, // JSON string
}

/// Tool definition
#[derive(Debug, Serialize, Deserialize)]
struct OpenAITool {
    r#type: String, // "function"
    function: OpenAIFunctionDef,
}

/// Function definition
#[derive(Debug, Serialize, Deserialize)]
struct OpenAIFunctionDef {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parameters: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<OpenAIContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAIToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

/// OpenAI Chat Completions response format
#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    id: String,
    #[serde(default, rename = "object")]
    _object: String,
    model: String,
    choices: Vec<OpenAIChoice>,
    usage: OpenAIUsage,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    #[serde(default)]
    total_tokens: u32,
}

/// OpenAI Responses API response format (for Codex models)
#[derive(Debug, Deserialize)]
struct OpenAIResponsesResponse {
    id: String,
    model: String,
    output: Vec<ResponsesOutput>,
    usage: ResponsesUsage,
}

#[derive(Debug, Deserialize)]
struct ResponsesOutput {
    #[serde(rename = "type")]
    output_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<Vec<ResponsesContentBlock>>,
}

#[derive(Debug, Deserialize)]
struct ResponsesContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResponsesUsage {
    input_tokens: u32,
    output_tokens: u32,
}

/// OpenAI Streaming Chunk (for SSE transformation)
#[derive(Debug, Deserialize)]
struct OpenAIStreamChunk {
    id: String,
    #[serde(default)]
    model: String,
    choices: Vec<OpenAIStreamChoice>,
    #[serde(default)]
    created: u64,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamChoice {
    delta: OpenAIStreamDelta,
    #[serde(default)]
    index: usize,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamDelta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    reasoning: Option<String>, // For GLM/Cerebras models
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<serde_json::Value>>,
}

/// OpenAI provider implementation
pub struct OpenAIProvider {
    name: String,
    api_key: String,
    base_url: String,
    client: Client,
    models: Vec<String>,
    custom_headers: Vec<(String, String)>,
    /// OAuth provider ID (if using OAuth instead of API key)
    oauth_provider: Option<String>,
    /// Token store for OAuth authentication
    token_store: Option<TokenStore>,
}

impl OpenAIProvider {
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

    /// Check if the model is a Codex model that requires /v1/responses endpoint
    fn is_codex_model(model: &str) -> bool {
        model.to_lowercase().contains("codex")
    }

    /// Parse SSE (Server-Sent Events) response from ChatGPT Codex
    fn parse_sse_response(sse_text: &str) -> Result<Vec<ContentBlock>, ProviderError> {
        // Find the response.completed event and extract both reasoning and message
        let lines: Vec<&str> = sse_text.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            if line.starts_with("event: response.completed") {
                // Next line should be data: {...}
                if i + 1 < lines.len() {
                    let data_line = lines[i + 1];
                    if data_line.starts_with("data: ") {
                        let json_str = &data_line[6..];  // Skip "data: "
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_str) {
                            // Extract both reasoning and message from response.output array
                            // Note: Codex models have reasoning at output[0], message at output[1]
                            if let Some(response) = json.get("response") {
                                if let Some(output) = response.get("output").and_then(|v| v.as_array()) {
                                    let mut content_blocks = Vec::new();

                                    // Extract reasoning and message in order
                                    for output_item in output {
                                        if let Some(output_type) = output_item.get("type").and_then(|v| v.as_str()) {
                                            if let Some(content) = output_item.get("content").and_then(|v| v.as_array()) {
                                                if let Some(first_content) = content.first() {
                                                    if let Some(text) = first_content.get("text").and_then(|v| v.as_str()) {
                                                        match output_type {
                                                            "reasoning" => {
                                                                // Convert OpenAI reasoning to Claude thinking block
                                                                content_blocks.push(ContentBlock::Thinking {
                                                                    thinking: text.to_string(),
                                                                    signature: String::new(), // OpenAI doesn't have signature
                                                                });
                                                            }
                                                            "message" => {
                                                                content_blocks.push(ContentBlock::Text {
                                                                    text: text.to_string(),
                                                                });
                                                            }
                                                            _ => {}
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    if !content_blocks.is_empty() {
                                        return Ok(content_blocks);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Err(ProviderError::ApiError {
            status: 500,
            message: "Failed to parse SSE response: no content found".to_string(),
        })
    }

    /// Transform Anthropic request to OpenAI Responses API format
    fn transform_to_responses_request(&self, request: &AnthropicRequest) -> Result<OpenAIResponsesRequest, ProviderError> {
        // Use official Codex instructions (system message is handled separately in user messages if needed)
        let instructions = CODEX_INSTRUCTIONS.to_string();

        // Convert messages to Responses API input format
        let mut messages = Vec::new();

        // Add system message as a user message if present (Codex doesn't have separate system role)
        if let Some(ref system) = request.system {
            let system_text = match system {
                crate::models::SystemPrompt::Text(text) => text.clone(),
                crate::models::SystemPrompt::Blocks(blocks) => {
                    blocks.iter()
                        .map(|b| b.text.clone())
                        .collect::<Vec<_>>()
                        .join("\n")
                }
            };
            // Prepend system message as user message
            messages.push(OpenAIResponsesMessage {
                role: "user".to_string(),
                content: Some(system_text),
            });
        }

        // Transform messages
        for msg in &request.messages {
            let content = match &msg.content {
                MessageContent::Text(text) => text.clone(),
                MessageContent::Blocks(blocks) => {
                    let text = blocks.iter()
                        .filter_map(|block| {
                            match block {
                                crate::models::ContentBlock::Text { text } => Some(text.clone()),
                                _ => None,
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    // Responses API requires content, use empty string if none
                    if text.is_empty() {
                        String::new()
                    } else {
                        text
                    }
                }
            };

            messages.push(OpenAIResponsesMessage {
                role: msg.role.clone(),
                content: Some(content),  // Always provide content
            });
        }

        Ok(OpenAIResponsesRequest {
            model: request.model.clone(),
            input: OpenAIResponsesInput::Messages(messages),
            instructions,
            store: false,  // Required: ChatGPT backend requires store=false
            stream: true,  // Required: ChatGPT Codex requires stream=true
        })
    }

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

    /// OpenRouter - OpenAI-compatible with optional referer headers
    pub fn openrouter(name: String, api_key: String, models: Vec<String>) -> Self {
        Self::with_headers(
            name,
            api_key,
            "https://openrouter.ai/api/v1".to_string(),
            models,
            vec![
                ("HTTP-Referer".to_string(), "https://github.com/bahkchanhee/claude-code-mux".to_string()),
                ("X-Title".to_string(), "Claude Code Mux".to_string()),
            ],
            None,
            None,
        )
    }

    /// Deepinfra - Fully OpenAI-compatible
    pub fn deepinfra(name: String, api_key: String, models: Vec<String>) -> Self {
        Self::new(
            name,
            api_key,
            "https://api.deepinfra.com/v1/openai".to_string(),
            models,
            None,
            None,
        )
    }

    /// NovitaAI - OpenAI-compatible with source header
    pub fn novita(name: String, api_key: String, models: Vec<String>) -> Self {
        Self::with_headers(
            name,
            api_key,
            "https://api.novita.ai/v3/openai".to_string(),
            models,
            vec![("X-Novita-Source".to_string(), "claude-code-mux".to_string())],
            None,
            None,
        )
    }

    /// Baseten - OpenAI-compatible
    pub fn baseten(name: String, api_key: String, models: Vec<String>) -> Self {
        Self::new(
            name,
            api_key,
            "https://inference.baseten.co/v1".to_string(),
            models,
            None,
            None,
        )
    }

    /// Together AI - OpenAI-compatible
    pub fn together(name: String, api_key: String, models: Vec<String>) -> Self {
        Self::new(
            name,
            api_key,
            "https://api.together.xyz/v1".to_string(),
            models,
            None,
            None,
        )
    }

    /// Fireworks AI - OpenAI-compatible
    pub fn fireworks(name: String, api_key: String, models: Vec<String>) -> Self {
        Self::new(
            name,
            api_key,
            "https://api.fireworks.ai/inference/v1".to_string(),
            models,
            None,
            None,
        )
    }

    /// Groq - Fast OpenAI-compatible inference
    pub fn groq(name: String, api_key: String, models: Vec<String>) -> Self {
        Self::new(
            name,
            api_key,
            "https://api.groq.com/openai/v1".to_string(),
            models,
            None,
            None,
        )
    }

    /// Nebius - OpenAI-compatible
    pub fn nebius(name: String, api_key: String, models: Vec<String>) -> Self {
        Self::new(
            name,
            api_key,
            "https://api.studio.nebius.ai/v1".to_string(),
            models,
            None,
            None,
        )
    }

    /// Cerebras - Fast OpenAI-compatible inference
    pub fn cerebras(name: String, api_key: String, models: Vec<String>) -> Self {
        Self::new(
            name,
            api_key,
            "https://api.cerebras.ai/v1".to_string(),
            models,
            None,
            None,
        )
    }

    pub fn moonshot(name: String, api_key: String, models: Vec<String>) -> Self {
        Self::new(
            name,
            api_key,
            "https://api.moonshot.cn/v1".to_string(),
            models,
            None,
            None,
        )
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
                        let config = OAuthConfig::openai_codex();
                        let oauth_client = OAuthClient::new(config, token_store.clone());

                        match oauth_client.refresh_token(oauth_provider_id).await {
                            Ok(new_token) => {
                                tracing::info!("✅ Token refreshed successfully");
                                return Ok(new_token.access_token);
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
                        return Ok(token.access_token);
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

    /// Extract ChatGPT account ID from JWT access token
    fn extract_account_id(access_token: &str) -> Option<String> {
        // JWT format: header.payload.signature
        let parts: Vec<&str> = access_token.split('.').collect();
        if parts.len() != 3 {
            return None;
        }

        // Decode the payload (base64url)
        let payload = parts[1];
        let decoded = general_purpose::URL_SAFE_NO_PAD.decode(payload).ok()?;
        let json_str = String::from_utf8(decoded).ok()?;

        // Parse JSON and extract chatgpt_account_id from the correct claim path
        let json: serde_json::Value = serde_json::from_str(&json_str).ok()?;
        json.get("https://api.openai.com/auth")?
            .get("chatgpt_account_id")?
            .as_str()
            .map(|s| s.to_string())
    }

    /// Transform Anthropic request to OpenAI format
    fn transform_request(&self, request: &AnthropicRequest) -> Result<OpenAIRequest, ProviderError> {
        let mut openai_messages = Vec::new();

        // Add system message if present
        if let Some(ref system) = request.system {
            let system_text = match system {
                crate::models::SystemPrompt::Text(text) => text.clone(),
                crate::models::SystemPrompt::Blocks(blocks) => {
                    blocks.iter()
                        .map(|b| b.text.clone())
                        .collect::<Vec<_>>()
                        .join("\n")
                }
            };
            openai_messages.push(OpenAIMessage {
                role: "system".to_string(),
                content: Some(OpenAIContent::String(system_text)),
                reasoning: None,
                tool_calls: None,
                tool_call_id: None,
            });
        }

        // Transform messages
        for msg in &request.messages {
            match &msg.content {
                MessageContent::Text(text) => {
                    // Simple text message
                    openai_messages.push(OpenAIMessage {
                        role: msg.role.clone(),
                        content: Some(OpenAIContent::String(text.clone())),
                        reasoning: None,
                        tool_calls: None,
                        tool_call_id: None,
                    });
                }
                MessageContent::Blocks(blocks) => {
                    // Check if we have any tool results - they need separate messages
                    let tool_results: Vec<_> = blocks.iter()
                        .filter_map(|block| {
                            if let crate::models::ContentBlock::ToolResult { tool_use_id, content } = block {
                                Some((tool_use_id.clone(), content.to_string()))
                            } else {
                                None
                            }
                        })
                        .collect();

                    // Extract tool_calls from ToolUse blocks
                    let tool_calls: Vec<_> = blocks.iter()
                        .filter_map(|block| {
                            if let crate::models::ContentBlock::ToolUse { id, name, input } = block {
                                Some(OpenAIToolCall {
                                    id: id.clone(),
                                    r#type: "function".to_string(),
                                    function: OpenAIFunctionCall {
                                        name: name.clone(),
                                        arguments: serde_json::to_string(input).unwrap_or_default(),
                                    },
                                })
                            } else {
                                None
                            }
                        })
                        .collect();

                    // Build content parts (text and images, excluding tool use/result)
                    let mut content_parts = Vec::new();
                    for block in blocks {
                        match block {
                            crate::models::ContentBlock::Text { text } => {
                                content_parts.push(OpenAIContentPart::Text {
                                    text: text.clone(),
                                });
                            }
                            crate::models::ContentBlock::Image { source } => {
                                // Convert Anthropic image format to OpenAI format
                                let url = if source.r#type == "base64" {
                                    // data:image/{media_type};base64,{data}
                                    let media_type = source.media_type.as_ref()
                                        .map(|s| s.as_str())
                                        .unwrap_or("image/png");
                                    let data = source.data.as_ref()
                                        .map(|s| s.as_str())
                                        .unwrap_or("");
                                    format!("data:{};base64,{}", media_type, data)
                                } else if let Some(url) = &source.url {
                                    url.clone()
                                } else {
                                    continue; // Skip invalid image sources
                                };

                                content_parts.push(OpenAIContentPart::ImageUrl {
                                    image_url: OpenAIImageUrl { url },
                                });
                            }
                            crate::models::ContentBlock::ToolUse { .. } => {
                                // Already handled in tool_calls
                            }
                            crate::models::ContentBlock::ToolResult { .. } => {
                                // Will be handled as separate messages below
                            }
                            crate::models::ContentBlock::Thinking { .. } => {
                                // OpenAI doesn't have thinking blocks, skip
                            }
                        }
                    }

                    // IMPORTANT: OpenAI requires tool messages to come BEFORE user messages
                    // If this message has both tool_results and content, we must:
                    // 1. First emit tool messages for the results
                    // 2. Then emit a user message with the content

                    // Add separate tool result messages FIRST
                    for (tool_use_id, result_content) in tool_results {
                        openai_messages.push(OpenAIMessage {
                            role: "tool".to_string(),
                            content: Some(OpenAIContent::String(result_content)),
                            reasoning: None,
                            tool_calls: None,
                            tool_call_id: Some(tool_use_id),
                        });
                    }

                    // Then add main message with content and/or tool_calls
                    if !content_parts.is_empty() || !tool_calls.is_empty() {
                        let content = if content_parts.is_empty() {
                            None
                        } else if content_parts.len() == 1 {
                            // Single text part - use string format for compatibility
                            if let OpenAIContentPart::Text { text } = &content_parts[0] {
                                Some(OpenAIContent::String(text.clone()))
                            } else {
                                Some(OpenAIContent::Parts(content_parts.clone()))
                            }
                        } else {
                            Some(OpenAIContent::Parts(content_parts))
                        };

                        openai_messages.push(OpenAIMessage {
                            role: msg.role.clone(),
                            content,
                            reasoning: None,
                            tool_calls: if tool_calls.is_empty() { None } else { Some(tool_calls) },
                            tool_call_id: None,
                        });
                    }
                }
            }
        }

        // Transform tools if present
        let tools = request.tools.as_ref().map(|anthropic_tools| {
            anthropic_tools.iter()
                .filter_map(|tool| {
                    // Anthropic tools have name, description, input_schema
                    Some(OpenAITool {
                        r#type: "function".to_string(),
                        function: OpenAIFunctionDef {
                            name: tool.name.as_ref()?.clone(),
                            description: tool.description.clone(),
                            parameters: tool.input_schema.clone(),
                        },
                    })
                })
                .collect()
        });

        Ok(OpenAIRequest {
            model: request.model.clone(),
            messages: openai_messages,
            max_tokens: Some(request.max_tokens),
            temperature: request.temperature,
            top_p: request.top_p,
            stop: request.stop_sequences.clone(),
            stream: request.stream,
            tools,
            tool_choice: None, // TODO: Add tool_choice support if needed
        })
    }

    /// Transform OpenAI response to Anthropic format
    fn transform_response(&self, response: OpenAIResponse) -> ProviderResponse {
        let choice = response.choices.into_iter().next()
            .expect("OpenAI response must have at least one choice");

        let mut content_blocks = Vec::new();

        // Extract text from content or reasoning (for GLM models via Cerebras)
        let text = if let Some(content) = choice.message.content {
            match content {
                OpenAIContent::String(s) => s,
                OpenAIContent::Parts(parts) => {
                    // Extract text from all text parts
                    parts.iter()
                        .filter_map(|part| {
                            if let OpenAIContentPart::Text { text } = part {
                                Some(text.clone())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                }
            }
        } else if let Some(reasoning) = choice.message.reasoning {
            reasoning
        } else {
            String::new()
        };

        // Add text content if present
        if !text.is_empty() {
            content_blocks.push(ContentBlock::Text { text });
        }

        // Transform tool_calls to Anthropic tool_use format
        if let Some(tool_calls) = choice.message.tool_calls {
            for tool_call in tool_calls {
                // Parse arguments from JSON string
                let input = serde_json::from_str(&tool_call.function.arguments)
                    .unwrap_or(serde_json::json!({}));

                content_blocks.push(ContentBlock::ToolUse {
                    id: tool_call.id,
                    name: tool_call.function.name,
                    input,
                });
            }
        }

        ProviderResponse {
            id: response.id,
            r#type: "message".to_string(),
            role: "assistant".to_string(),
            content: content_blocks,
            model: response.model,
            stop_reason: choice.finish_reason,
            stop_sequence: None,
            usage: Usage {
                input_tokens: response.usage.prompt_tokens,
                output_tokens: response.usage.completion_tokens,
            },
        }
    }

    /// Transform Responses API response to Anthropic format
    fn transform_responses_response(&self, response: OpenAIResponsesResponse) -> ProviderResponse {
        // Extract text from output messages
        let text = response.output.iter()
            .filter(|output| output.output_type == "message")
            .filter_map(|output| output.content.as_ref())
            .flat_map(|content_blocks| {
                content_blocks.iter()
                    .filter(|block| block.block_type == "output_text")
                    .filter_map(|block| block.text.clone())
            })
            .collect::<Vec<_>>()
            .join("\n");

        ProviderResponse {
            id: response.id,
            r#type: "message".to_string(),
            role: "assistant".to_string(),
            content: vec![ContentBlock::Text {
                text,
            }],
            model: response.model,
            stop_reason: Some("end_turn".to_string()),
            stop_sequence: None,
            usage: Usage {
                input_tokens: response.usage.input_tokens,
                output_tokens: response.usage.output_tokens,
            },
        }
    }

    /// Transform OpenAI streaming chunk to Anthropic format
    /// Returns raw SSE-formatted bytes (event: type / data: json)
    fn transform_openai_chunk_to_anthropic_sse(chunk: &OpenAIStreamChunk, message_id: &str, is_first: &mut bool, has_content_block: &mut bool, stream_ended: &mut bool) -> String {
        let mut output = String::new();

        // First chunk: emit message_start
        if *is_first {
            *is_first = false;
            let message_start = serde_json::json!({
                "type": "message_start",
                "message": {
                    "id": message_id,
                    "type": "message",
                    "role": "assistant",
                    "content": [],
                    "model": chunk.model,
                    "stop_reason": null,
                    "stop_sequence": null,
                    "usage": {
                        "input_tokens": 0,
                        "output_tokens": 0
                    }
                }
            });
            output.push_str(&format!("event: message_start\ndata: {}\n\n", message_start));
        }

        // Process delta content
        for choice in &chunk.choices {
            // Handle text content (content or reasoning fields)
            let text_content = choice.delta.content.as_ref()
                .or(choice.delta.reasoning.as_ref()); // Support reasoning field for GLM/Cerebras

            if let Some(text) = text_content {
                // Skip empty text
                if text.is_empty() {
                    continue;
                }
                
                // Emit content_block_start if this is the first content
                if !*has_content_block {
                    *has_content_block = true;
                    let block_start = serde_json::json!({
                        "type": "content_block_start",
                        "index": 0,
                        "content_block": {
                            "type": "text",
                            "text": ""
                        }
                    });
                    output.push_str(&format!("event: content_block_start\ndata: {}\n\n", block_start));
                }

                // Emit content_block_delta
                let delta = serde_json::json!({
                    "type": "content_block_delta",
                    "index": 0,
                    "delta": {
                        "type": "text_delta",
                        "text": text
                    }
                });
                output.push_str(&format!("event: content_block_delta\ndata: {}\n\n", delta));
            }
            
            // Handle tool calls (OpenAI function calling → Anthropic tool_use)
            if let Some(ref tool_calls) = choice.delta.tool_calls {
                tracing::debug!("🔧 Transforming {} tool_calls to Anthropic tool_use format", tool_calls.len());
                
                // First, close any open text content block
                if *has_content_block {
                    let block_stop = serde_json::json!({
                        "type": "content_block_stop",
                        "index": 0
                    });
                    output.push_str(&format!("event: content_block_stop\ndata: {}\n\n", block_stop));
                    *has_content_block = false;
                }
                
                // Transform each tool call to Anthropic format
                for (idx, tool_call) in tool_calls.iter().enumerate() {
                    // Extract tool info
                    if let Some(ref function) = tool_call.get("function") {
                        let default_tool_id = format!("tool_{}", idx);
                        let tool_id = tool_call.get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or(&default_tool_id);
                        let tool_name = function.get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        let tool_args = function.get("arguments")
                            .and_then(|v| v.as_str())
                            .unwrap_or("{}");
                        
                        tracing::debug!("🔨 Tool: {} (id: {})", tool_name, tool_id);
                        
                        // Parse arguments to validate JSON
                        let tool_input: serde_json::Value = serde_json::from_str(tool_args)
                            .unwrap_or(serde_json::json!({}));
                        
                        // Send tool_use content_block_start
                        let block_start = serde_json::json!({
                            "type": "content_block_start",
                            "index": idx + 1,  // Index after text content
                            "content_block": {
                                "type": "tool_use",
                                "id": tool_id,
                                "name": tool_name
                            }
                        });
                        output.push_str(&format!("event: content_block_start\ndata: {}\n\n", block_start));
                        
                        // Send tool input as delta
                        let input_delta = serde_json::json!({
                            "type": "content_block_delta",
                            "index": idx + 1,
                            "delta": {
                                "type": "input_json_delta",
                                "partial_json": serde_json::to_string(&tool_input).unwrap_or_default()
                            }
                        });
                        output.push_str(&format!("event: content_block_delta\ndata: {}\n\n", input_delta));
                        
                        // Close tool_use block
                        let block_stop = serde_json::json!({
                            "type": "content_block_stop",
                            "index": idx + 1
                        });
                        output.push_str(&format!("event: content_block_stop\ndata: {}\n\n", block_stop));
                    }
                }
                
                continue;
            }

            // Handle finish_reason (stream end)
            if let Some(reason) = &choice.finish_reason {
                *stream_ended = true; // Mark that stream ended properly
                
                if *has_content_block {
                    // Emit content_block_stop
                    let block_stop = serde_json::json!({
                        "type": "content_block_stop",
                        "index": 0
                    });
                    output.push_str(&format!("event: content_block_stop\ndata: {}\n\n", block_stop));
                }

                // Emit message_delta with stop reason
                let stop_reason = match reason.as_str() {
                    "stop" => "end_turn",
                    "length" => "max_tokens",
                    "tool_calls" => "end_turn", // Tool calls also end the turn
                    _ => "end_turn"
                };
                let message_delta = serde_json::json!({
                    "type": "message_delta",
                    "delta": {
                        "stop_reason": stop_reason,
                        "stop_sequence": null
                    },
                    "usage": {
                        "output_tokens": 0
                    }
                });
                output.push_str(&format!("event: message_delta\ndata: {}\n\n", message_delta));

                // Emit message_stop
                let message_stop = serde_json::json!({
                    "type": "message_stop"
                });
                output.push_str(&format!("event: message_stop\ndata: {}\n\n", message_stop));
            }
        }

        output
    }
}

#[async_trait]
impl AnthropicProvider for OpenAIProvider {
    async fn send_message(&self, request: AnthropicRequest) -> Result<ProviderResponse, ProviderError> {
        // Get authentication token (API key or OAuth)
        let auth_value = self.get_auth_header().await?;

        // Determine base URL: OAuth uses ChatGPT backend, API key uses configured base_url
        let base_url = if self.is_oauth() {
            "https://chatgpt.com/backend-api"
        } else {
            &self.base_url
        };

        // Check if we should use Responses API endpoint:
        // - OAuth: Always use /codex/responses for all models
        // - API Key: Only use /responses for models containing "codex"
        let use_responses_api = if self.is_oauth() {
            true  // OAuth always uses Codex endpoint
        } else {
            Self::is_codex_model(&request.model)  // API Key only for codex models
        };

        if use_responses_api {
            // Use /v1/responses endpoint for Codex models
            let responses_request = self.transform_to_responses_request(&request)?;

            // OAuth (ChatGPT Codex) uses /codex/responses, API Key uses /responses
            let endpoint = if self.is_oauth() {
                "/codex/responses"
            } else {
                "/responses"
            };
            let url = format!("{}{}", base_url, endpoint);

            tracing::debug!("Using {} endpoint for Codex model: {}", endpoint, request.model);

            let mut req_builder = self.client
                .post(&url)
                .header("Authorization", format!("Bearer {}", auth_value))
                .header("Content-Type", "application/json")
                .header("accept", "text/event-stream");

            // For OAuth (ChatGPT Codex), add Codex-specific headers
            if self.is_oauth() {
                if let Some(account_id) = Self::extract_account_id(&auth_value) {
                    req_builder = req_builder
                        .header("chatgpt-account-id", account_id)
                        .header("OpenAI-Beta", "responses=experimental")
                        .header("originator", "codex_cli_rs")
                        // Browser-like headers to avoid Cloudflare bot detection
                        .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
                        .header("Origin", "https://chatgpt.com")
                        .header("Referer", "https://chatgpt.com/")
                        .header("sec-ch-ua", "\"Google Chrome\";v=\"131\", \"Chromium\";v=\"131\", \"Not_A Brand\";v=\"24\"")
                        .header("sec-ch-ua-mobile", "?0")
                        .header("sec-ch-ua-platform", "\"macOS\"")
                        .header("sec-fetch-dest", "empty")
                        .header("sec-fetch-mode", "cors")
                        .header("sec-fetch-site", "same-origin");
                    tracing::debug!("🔐 Using OAuth Bearer token for ChatGPT Codex on {}", self.name);
                }
            }

            // Add custom headers
            for (key, value) in &self.custom_headers {
                req_builder = req_builder.header(key, value);
            }

            let response = req_builder
                .json(&responses_request)
                .send()
                .await?;

            if !response.status().is_success() {
                let status = response.status().as_u16();
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                tracing::error!("Responses API error ({}): {}", status, error_text);
                return Err(ProviderError::ApiError {
                    status,
                    message: error_text,
                });
            }

            let response_text = response.text().await?;
            tracing::debug!("Responses API response body: {}", response_text);

            // Parse SSE (Server-Sent Events) format
            // Format: event: xxx\ndata: {...}\n\n
            // This extracts both reasoning (converted to thinking) and message blocks
            let content_blocks = Self::parse_sse_response(&response_text)?;

            // Return direct response (SSE doesn't need transform)
            Ok(ProviderResponse {
                id: "sse-response".to_string(),
                r#type: "message".to_string(),
                role: "assistant".to_string(),
                content: content_blocks,
                model: request.model.clone(),
                stop_reason: Some("end_turn".to_string()),
                stop_sequence: None,
                usage: Usage {
                    input_tokens: 0,  // SSE doesn't provide token counts
                    output_tokens: 0,
                },
            })
        } else {
            // Use standard /v1/chat/completions endpoint for non-Codex models
            let openai_request = self.transform_request(&request)?;
            let url = format!("{}/chat/completions", base_url);

            let mut req_builder = self.client
                .post(&url)
                .header("Authorization", format!("Bearer {}", auth_value))
                .header("Content-Type", "application/json");

            // For OAuth (ChatGPT), add account-specific headers
            if self.is_oauth() {
                if let Some(account_id) = Self::extract_account_id(&auth_value) {
                    req_builder = req_builder
                        .header("chatgpt-account-id", account_id)
                        // Browser-like headers to avoid Cloudflare bot detection
                        .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
                        .header("Origin", "https://chatgpt.com")
                        .header("Referer", "https://chatgpt.com/")
                        .header("sec-ch-ua", "\"Google Chrome\";v=\"131\", \"Chromium\";v=\"131\", \"Not_A Brand\";v=\"24\"")
                        .header("sec-ch-ua-mobile", "?0")
                        .header("sec-ch-ua-platform", "\"macOS\"")
                        .header("sec-fetch-dest", "empty")
                        .header("sec-fetch-mode", "cors")
                        .header("sec-fetch-site", "same-origin");
                    tracing::debug!("🔐 Using OAuth Bearer token for ChatGPT on {}", self.name);
                }
            }

            // Add custom headers (for OpenRouter, NovitaAI, etc.)
            for (key, value) in &self.custom_headers {
                req_builder = req_builder.header(key, value);
            }

            let response = req_builder
                .json(&openai_request)
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

            // Get response body as text for debugging
            let response_text = response.text().await?;
            tracing::debug!("OpenAI provider response body: {}", response_text);

            // Try to parse the response
            let openai_response: OpenAIResponse = serde_json::from_str(&response_text)
                .map_err(|e| {
                    tracing::error!("Failed to parse OpenAI response: {}", e);
                    tracing::error!("Response body was: {}", response_text);
                    e
                })?;

            Ok(self.transform_response(openai_response))
        }
    }

    async fn count_tokens(&self, request: CountTokensRequest) -> Result<CountTokensResponse, ProviderError> {
        // For OpenAI, we'll use tiktoken-rs for local token counting
        // This is a placeholder - actual implementation would use tiktoken

        // Rough estimate: ~4 chars per token
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
            let content = match &msg.content {
                MessageContent::Text(text) => text.clone(),
                MessageContent::Blocks(blocks) => {
                    blocks.iter()
                        .filter_map(|block| {
                            match block {
                                crate::models::ContentBlock::Text { text } => Some(text.clone()),
                                crate::models::ContentBlock::ToolResult { content, .. } => {
                                    Some(content.to_string())
                                }
                                crate::models::ContentBlock::Thinking { thinking, .. } => {
                                    Some(thinking.clone())
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

        // Get authentication token (API key or OAuth)
        let auth_value = self.get_auth_header().await?;

        // Determine base URL: OAuth uses ChatGPT backend, API key uses configured base_url
        let base_url = if self.is_oauth() {
            "https://chatgpt.com/backend-api"
        } else {
            &self.base_url
        };

        // Check if this is a Codex model
        let is_codex = Self::is_codex_model(&request.model);

        let (url, request_body) = if is_codex {
            // Use /v1/responses endpoint for Codex models
            tracing::debug!("Using /v1/responses endpoint for Codex model (streaming): {}", request.model);
            let responses_request = self.transform_to_responses_request(&request)?;
            let body = serde_json::to_value(&responses_request)
                .map_err(|e| ProviderError::SerializationError(e))?;
            (format!("{}/responses", base_url), body)
        } else {
            // Use standard /v1/chat/completions endpoint
            let openai_request = self.transform_request(&request)?;
            let body = serde_json::to_value(&openai_request)
                .map_err(|e| ProviderError::SerializationError(e))?;
            (format!("{}/chat/completions", base_url), body)
        };

        // Send streaming request
        let mut req_builder = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", auth_value))
            .header("Content-Type", "application/json")
            .header("accept", "text/event-stream");

        // For OAuth (ChatGPT Codex), add Codex-specific headers
        if self.is_oauth() && is_codex {
            if let Some(account_id) = Self::extract_account_id(&auth_value) {
                req_builder = req_builder
                    .header("chatgpt-account-id", account_id)
                    .header("OpenAI-Beta", "responses=experimental")
                    .header("originator", "codex_cli_rs");
                tracing::debug!("🔐 Using OAuth Bearer token for ChatGPT Codex streaming on {}", self.name);
            }
        } else if self.is_oauth() {
            // For non-Codex OAuth (if needed in the future)
            if let Some(account_id) = Self::extract_account_id(&auth_value) {
                req_builder = req_builder.header("chatgpt-account-id", account_id);
                tracing::debug!("🔐 Using OAuth Bearer token for streaming on {}", self.name);
            }
        }

        let response = req_builder
            .json(&request_body)
            .send()
            .await?;

        // Check for errors
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ProviderError::ApiError {
                status,
                message: error_text,
            });
        }

        // Transform OpenAI SSE format to Anthropic SSE format
        use futures::stream::StreamExt;
        use crate::providers::streaming::SseStream;
        use std::sync::{Arc, Mutex};
        
        let message_id = format!("msg_{}", uuid::Uuid::new_v4());
        let is_first = Arc::new(Mutex::new(true));
        let has_content_block = Arc::new(Mutex::new(false));
        let stream_ended_properly = Arc::new(Mutex::new(false)); // Track if finish_reason was received
        let has_content_for_cleanup = has_content_block.clone(); // Clone before moving into closure
        let stream_ended_for_cleanup = stream_ended_properly.clone();
        
        // Convert response bytes stream to SSE events
        let sse_stream = SseStream::new(response.bytes_stream());
        
        // Transform OpenAI SSE events to Anthropic format
        let transformed_stream = sse_stream.then(move |result| {
            let message_id = message_id.clone();
            let is_first = is_first.clone();
            let has_content_block = has_content_block.clone();
            let stream_ended_properly = stream_ended_properly.clone();
            
            async move {
                match result {
                    Ok(sse_event) => {
                        tracing::debug!("📦 Received SSE chunk: {}", sse_event.data.chars().take(100).collect::<String>());
                        
                        // Skip empty data
                        if sse_event.data.trim().is_empty() {
                            tracing::debug!("⏭️ Skipping empty SSE event");
                            return Ok(Bytes::new());
                        }
                        
                        if sse_event.data.trim() == "[DONE]" {
                            tracing::debug!("✅ Stream finished with [DONE]");
                            return Ok(Bytes::new());
                        }
                        
                        // Parse OpenAI chunk
                        match serde_json::from_str::<OpenAIStreamChunk>(&sse_event.data) {
                            Ok(chunk) => {
                                tracing::debug!("✨ Transforming chunk with {} choices", chunk.choices.len());
                                
                                // Transform to Anthropic format (raw SSE bytes)
                                let sse_output = Self::transform_openai_chunk_to_anthropic_sse(
                                    &chunk,
                                    &message_id,
                                    &mut *is_first.lock().unwrap(),
                                    &mut *has_content_block.lock().unwrap(),
                                    &mut *stream_ended_properly.lock().unwrap()
                                );
                                
                                tracing::debug!("📤 Sending {} bytes", sse_output.len());
                                
                                // Return as raw bytes (already SSE-formatted)
                                Ok(Bytes::from(sse_output))
                            }
                            Err(e) => {
                                tracing::warn!("❌ Failed to parse OpenAI chunk: {} - Data: {}", e, sse_event.data);
                                Ok(Bytes::new())
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("💥 Stream error: {}", e);
                        Err(ProviderError::HttpError(e))
                    }
                }
            }
        })
        .try_filter(|bytes| futures::future::ready(!bytes.is_empty()));

        // Add stream finalization to handle Cerebras bug where stream closes without finish_reason
        let finalized_stream = transformed_stream.chain(futures::stream::once(async move {
            // Only send end events if stream didn't end properly AND we have an open content block
            let has_content = *has_content_for_cleanup.lock().unwrap();
            let ended_properly = *stream_ended_for_cleanup.lock().unwrap();
            
            if has_content && !ended_properly {
                tracing::warn!("⚠️ Stream ended without finish_reason - sending end events (Cerebras bug workaround)");
                
                let mut output = String::new();
                
                // Close content block
                let block_stop = serde_json::json!({
                    "type": "content_block_stop",
                    "index": 0
                });
                output.push_str(&format!("event: content_block_stop\ndata: {}\n\n", block_stop));
                
                // Send message_delta with end_turn
                let message_delta = serde_json::json!({
                    "type": "message_delta",
                    "delta": {
                        "stop_reason": "end_turn",
                        "stop_sequence": null
                    },
                    "usage": {
                        "output_tokens": 0
                    }
                });
                output.push_str(&format!("event: message_delta\ndata: {}\n\n", message_delta));
                
                // Send message_stop
                let message_stop = serde_json::json!({
                    "type": "message_stop"
                });
                output.push_str(&format!("event: message_stop\ndata: {}\n\n", message_stop));
                
                Ok(Bytes::from(output))
            } else {
                Ok(Bytes::new())
            }
        }))
        .try_filter(|bytes| futures::future::ready(!bytes.is_empty()));

        Ok(Box::pin(finalized_stream))
    }

    fn supports_model(&self, model: &str) -> bool {
        self.models.iter().any(|m| m == model)
    }
}
