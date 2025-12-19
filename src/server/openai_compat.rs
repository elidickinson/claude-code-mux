use serde::{Deserialize, Serialize};
use crate::models::{AnthropicRequest, MessageContent, ContentBlock, SystemPrompt};
use crate::providers::ProviderResponse;

/// OpenAI Chat Completions request format
#[derive(Debug, Deserialize)]
pub struct OpenAIRequest {
    pub model: String,
    pub messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct OpenAIMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<OpenAIContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Content can be string or array of content parts
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum OpenAIContent {
    String(String),
    Parts(Vec<OpenAIContentPart>),
}

/// Content part (text or image_url)
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum OpenAIContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: OpenAIImageUrl },
}

/// Image URL object
#[derive(Debug, Clone, Deserialize)]
pub struct OpenAIImageUrl {
    pub url: String,
}

/// OpenAI Chat Completions response format
#[derive(Debug, Serialize)]
pub struct OpenAIResponse {
    pub id: String,
    #[serde(rename = "object")]
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<OpenAIChoice>,
    pub usage: OpenAIUsage,
}

#[derive(Debug, Serialize)]
pub struct OpenAIChoice {
    pub index: u32,
    pub message: OpenAIResponseMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OpenAIResponseMessage {
    pub role: String,
    pub content: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OpenAIUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Transform OpenAI request to Anthropic format
pub fn transform_openai_to_anthropic(openai_req: OpenAIRequest) -> Result<AnthropicRequest, String> {
    let mut messages = Vec::new();
    let mut system_prompt: Option<SystemPrompt> = None;

    // Process messages
    for msg in openai_req.messages {
        match msg.role.as_str() {
            "system" => {
                // Extract system message
                if let Some(content) = msg.content {
                    let text = match content {
                        OpenAIContent::String(s) => s,
                        OpenAIContent::Parts(parts) => {
                            parts.iter()
                                .filter_map(|p| {
                                    if let OpenAIContentPart::Text { text } = p {
                                        Some(text.clone())
                                    } else {
                                        None
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join("\n")
                        }
                    };
                    system_prompt = Some(SystemPrompt::Text(text));
                }
            }
            "user" | "assistant" => {
                // Convert user/assistant messages
                let content = if let Some(openai_content) = msg.content {
                    match openai_content {
                        OpenAIContent::String(text) => MessageContent::Text(text),
                        OpenAIContent::Parts(parts) => {
                            let blocks: Vec<ContentBlock> = parts.iter()
                                .filter_map(|part| {
                                    match part {
                                        OpenAIContentPart::Text { text } => {
                                            Some(ContentBlock::Text { text: text.clone(), cache_control: None })
                                        }
                                        OpenAIContentPart::ImageUrl { image_url } => {
                                            // Parse data URL or external URL
                                            if image_url.url.starts_with("data:") {
                                                // data:image/png;base64,iVBORw0KG...
                                                if let Some(comma_idx) = image_url.url.find(',') {
                                                    let header = &image_url.url[..comma_idx];
                                                    let data = &image_url.url[comma_idx + 1..];

                                                    let media_type = if header.contains("image/jpeg") {
                                                        "image/jpeg"
                                                    } else if header.contains("image/png") {
                                                        "image/png"
                                                    } else if header.contains("image/gif") {
                                                        "image/gif"
                                                    } else if header.contains("image/webp") {
                                                        "image/webp"
                                                    } else {
                                                        "image/png" // default
                                                    };

                                                    Some(ContentBlock::Image {
                                                        source: crate::models::ImageSource {
                                                            r#type: "base64".to_string(),
                                                            media_type: Some(media_type.to_string()),
                                                            data: Some(data.to_string()),
                                                            url: None,
                                                        }
                                                    })
                                                } else {
                                                    None
                                                }
                                            } else {
                                                // External URL
                                                Some(ContentBlock::Image {
                                                    source: crate::models::ImageSource {
                                                        r#type: "url".to_string(),
                                                        media_type: None,
                                                        data: None,
                                                        url: Some(image_url.url.clone()),
                                                    }
                                                })
                                            }
                                        }
                                    }
                                })
                                .collect();

                            if blocks.is_empty() {
                                MessageContent::Text(String::new())
                            } else {
                                MessageContent::Blocks(blocks)
                            }
                        }
                    }
                } else {
                    MessageContent::Text(String::new())
                };

                messages.push(crate::models::Message {
                    role: msg.role,
                    content,
                });
            }
            _ => {
                // Skip other roles (tool, function, etc.)
                tracing::warn!("Skipping unsupported message role: {}", msg.role);
            }
        }
    }

    Ok(AnthropicRequest {
        model: openai_req.model,
        messages,
        max_tokens: openai_req.max_tokens.unwrap_or(4096),
        thinking: None,
        temperature: openai_req.temperature,
        top_p: openai_req.top_p,
        top_k: None,
        stop_sequences: openai_req.stop,
        stream: openai_req.stream,
        metadata: None,
        system: system_prompt,
        tools: None, // TODO: Transform tools if needed
    })
}

/// Transform Anthropic response to OpenAI format
pub fn transform_anthropic_to_openai(
    anthropic_resp: ProviderResponse,
    model: String,
) -> OpenAIResponse {
    // Extract text content from content blocks
    let content = anthropic_resp.content.iter()
        .filter_map(|block| {
            match block {
                ContentBlock::Text { text, .. } => Some(text.clone()),
                _ => None,
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let content = if content.is_empty() {
        None
    } else {
        Some(content)
    };

    // Map finish_reason
    let finish_reason = anthropic_resp.stop_reason.as_ref().map(|reason| {
        match reason.as_str() {
            "end_turn" => "stop",
            "max_tokens" => "length",
            "stop_sequence" => "stop",
            _ => "stop",
        }
        .to_string()
    });

    OpenAIResponse {
        id: anthropic_resp.id,
        object: "chat.completion".to_string(),
        created: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        model,
        choices: vec![OpenAIChoice {
            index: 0,
            message: OpenAIResponseMessage {
                role: anthropic_resp.role,
                content,
            },
            finish_reason,
        }],
        usage: OpenAIUsage {
            prompt_tokens: anthropic_resp.usage.input_tokens,
            completion_tokens: anthropic_resp.usage.output_tokens,
            total_tokens: anthropic_resp.usage.input_tokens + anthropic_resp.usage.output_tokens,
        },
    }
}

