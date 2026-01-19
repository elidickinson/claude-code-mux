mod openai_compat;
mod oauth_handlers;

use crate::cli::AppConfig;
use crate::models::{AnthropicRequest, RouteType};
use crate::router::Router;
use crate::providers::ProviderRegistry;
use crate::auth::TokenStore;
use crate::message_tracing::MessageTracer;
use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{
        Html, IntoResponse, Response,
    },
    routing::{get, post},
    Form, Json, Router as AxumRouter,
};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{debug, error, info};
use futures::stream::TryStreamExt;
use chrono::Local;

/// Reloadable components - rebuilt on config reload
pub struct ReloadableState {
    pub config: AppConfig,
    pub router: Router,
    pub provider_registry: Arc<ProviderRegistry>,
}

/// Application state shared across handlers
pub struct AppState {
    /// Reloadable state behind a single lock for atomic updates
    inner: std::sync::RwLock<Arc<ReloadableState>>,

    /// Persistent state - NOT reloaded
    pub token_store: TokenStore,
    pub config_path: std::path::PathBuf,
    pub message_tracer: Arc<MessageTracer>,
}

impl AppState {
    /// Get a snapshot of current reloadable state
    pub fn snapshot(&self) -> Arc<ReloadableState> {
        self.inner.read().unwrap().clone()
    }
}

const RECENT_REQUESTS_WINDOW: usize = 20;

/// Write routing information to file for statusline script
fn write_routing_info(model: &str, provider: &str, route_type: &RouteType) {
    if let Some(home) = dirs::home_dir() {
        let file_path = home.join(".claude-code-mux/last_routing.json");

        // Read existing recent requests history
        let mut recent: Vec<String> = Vec::new();
        if let Ok(existing_content) = std::fs::read_to_string(&file_path) {
            if let Ok(existing) = serde_json::from_str::<serde_json::Value>(&existing_content) {
                if let Some(items) = existing.get("recent").and_then(|t| t.as_array()) {
                    for item in items {
                        if let Some(entry) = item.as_str() {
                            recent.push(entry.to_string());
                        }
                    }
                }
            }
        }

        // Add current model/provider to recent
        let current_entry = format!("{}@{}", model, provider);
        recent.insert(0, current_entry);
        recent.truncate(RECENT_REQUESTS_WINDOW);

        // Create routing info
        let routing_info = serde_json::json!({
            "model": model,
            "provider": provider,
            "route_type": route_type.to_string(),
            "timestamp": Local::now().format("%H:%M:%S").to_string(),
            "recent": recent
        });

        if let Ok(json) = serde_json::to_string(&routing_info) {
            if let Err(e) = std::fs::write(file_path, json) {
                tracing::debug!("Failed to write routing info: {}", e);
            }
        } else {
            tracing::debug!("Failed to serialize routing info");
        }
    }
}

/// Start the HTTP server
pub async fn start_server(config: AppConfig, config_path: std::path::PathBuf) -> anyhow::Result<()> {
    let router = Router::new(config.clone());

    // Initialize OAuth token store FIRST (needed by provider registry)
    let token_store = TokenStore::default()
        .map_err(|e| anyhow::anyhow!("Failed to initialize token store: {}", e))?;

    let existing_tokens = token_store.list_providers();
    if !existing_tokens.is_empty() {
        info!("🔐 Loaded {} OAuth tokens from storage", existing_tokens.len());
    }

    // Initialize provider registry from config (with token store and model mappings)
    let provider_registry = Arc::new(
        ProviderRegistry::from_configs_with_models(&config.providers, Some(token_store.clone()), &config.models)
            .map_err(|e| anyhow::anyhow!("Failed to initialize provider registry: {}", e))?
    );

    info!("📦 Loaded {} providers with {} models",
        provider_registry.list_providers().len(),
        provider_registry.list_models().len()
    );

    // Initialize message tracer
    let message_tracer = Arc::new(MessageTracer::new(config.server.tracing.clone()));

    // Build reloadable state
    let reloadable = Arc::new(ReloadableState {
        config: config.clone(),
        router,
        provider_registry,
    });

    let state = Arc::new(AppState {
        inner: std::sync::RwLock::new(reloadable),
        token_store,
        config_path,
        message_tracer,
    });

    // Build router
    let app = AxumRouter::new()
        .route("/", get(serve_admin))
        .route("/v1/messages", post(handle_messages))
        .route("/v1/messages/count_tokens", post(handle_count_tokens))
        .route("/v1/chat/completions", post(handle_openai_chat_completions))
        .route("/health", get(health_check))
        .route("/api/models", get(get_models))
        .route("/api/providers", get(get_providers))
        .route("/api/models-config", get(get_models_config))
        .route("/api/config", get(get_config))
        .route("/api/config", post(update_config))
        .route("/api/config/json", get(get_config_json))
        .route("/api/config/json", post(update_config_json))
        .route("/api/reload", post(reload_config))
        // OAuth endpoints
        .route("/api/oauth/authorize", post(oauth_handlers::oauth_authorize))
        .route("/api/oauth/exchange", post(oauth_handlers::oauth_exchange))
        .route("/api/oauth/callback", get(oauth_handlers::oauth_callback))
        .route("/auth/callback", get(oauth_handlers::oauth_callback))  // OpenAI Codex uses this path
        .route("/api/oauth/tokens", get(oauth_handlers::oauth_list_tokens))
        .route("/api/oauth/tokens/delete", post(oauth_handlers::oauth_delete_token))
        .route("/api/oauth/tokens/refresh", post(oauth_handlers::oauth_refresh_token));

    // Clone state before moving it
    let oauth_state = state.clone();
    let app = app.with_state(state);

    // Bind to main address
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = TcpListener::bind(&addr).await?;

    info!("🚀 Server listening on {}", addr);

    // Start OAuth callback server on port 1455 (required for OpenAI Codex)
    // This is necessary because OpenAI's OAuth app only allows localhost:1455/auth/callback
    tokio::spawn(async move {
        let oauth_callback_app = AxumRouter::new()
            .route("/auth/callback", get(oauth_handlers::oauth_callback))
            .with_state(oauth_state);

        let oauth_addr = "127.0.0.1:1455";
        match TcpListener::bind(oauth_addr).await {
            Ok(oauth_listener) => {
                info!("🔐 OAuth callback server listening on {}", oauth_addr);
                if let Err(e) = axum::serve(oauth_listener, oauth_callback_app).await {
                    error!("OAuth callback server error: {}", e);
                }
            }
            Err(e) => {
                // Don't fail if port 1455 is already in use - just warn
                error!("⚠️  Failed to bind OAuth callback server on {}: {}", oauth_addr, e);
                error!("⚠️  OpenAI Codex OAuth will not work. Port 1455 must be available.");
            }
        }
    });

    // Start main server
    axum::serve(listener, app).await?;

    Ok(())
}

/// Serve Admin UI
async fn serve_admin() -> impl IntoResponse {
    Html(include_str!("admin.html"))
}

/// Health check endpoint
async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "claude-code-mux"
    }))
}

/// REMOVED: This endpoint was for LiteLLM integration which has been removed.
/// Models are now managed through the provider registry and config.
async fn get_models(State(_state): State<Arc<AppState>>) -> Result<Json<serde_json::Value>, AppError> {
    Err(AppError::ParseError("This endpoint has been removed. Use /api/models-config instead.".to_string()))
}

/// Get current routing configuration
async fn get_config(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let inner = state.snapshot();
    Json(serde_json::json!({
        "server": {
            "host": inner.config.server.host,
            "port": inner.config.server.port,
        },
        "router": {
            "default": inner.config.router.default,
            "background": inner.config.router.background,
            "think": inner.config.router.think,
            "websearch": inner.config.router.websearch,
        }
    }))
}

/// Update configuration
#[derive(serde::Deserialize)]
struct ConfigUpdate {
    // Router models
    default_model: String,
    background_model: Option<String>,
    think_model: Option<String>,
    websearch_model: Option<String>,
}

async fn update_config(
    State(state): State<Arc<AppState>>,
    Form(update): Form<ConfigUpdate>,
) -> Result<Html<String>, AppError> {
    // Read current config
    let config_path = &state.config_path;
    let config_str = std::fs::read_to_string(config_path)
        .map_err(|e| AppError::ParseError(format!("Failed to read config: {}", e)))?;

    let mut config: toml::Value = toml::from_str(&config_str)
        .map_err(|e| AppError::ParseError(format!("Failed to parse config: {}", e)))?;

    // Update router section
    if let Some(router) = config.get_mut("router").and_then(|v| v.as_table_mut()) {
        router.insert("default".to_string(), toml::Value::String(update.default_model));

        if let Some(bg) = update.background_model {
            router.insert("background".to_string(), toml::Value::String(bg));
        }

        if let Some(think) = update.think_model {
            router.insert("think".to_string(), toml::Value::String(think));
        }

        if let Some(ws) = update.websearch_model {
            router.insert("websearch".to_string(), toml::Value::String(ws));
        }
    }

    // Write back to file
    let new_config_str = toml::to_string_pretty(&config)
        .map_err(|e| AppError::ParseError(format!("Failed to serialize config: {}", e)))?;

    std::fs::write(config_path, new_config_str)
        .map_err(|e| AppError::ParseError(format!("Failed to write config: {}", e)))?;

    info!("✅ Configuration updated successfully");

    Ok(Html("<div class='px-4 py-3 rounded-xl bg-primary/20 border border-primary/50 text-foreground text-sm'>✅ Configuration saved successfully! Please restart the server to apply changes.</div>".to_string()))
}

/// Get providers configuration
async fn get_providers(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let inner = state.snapshot();
    Json(inner.config.providers.clone())
}

/// Get models configuration
async fn get_models_config(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let inner = state.snapshot();
    Json(inner.config.models.clone())
}

/// Get full configuration as JSON (for admin UI)
async fn get_config_json(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let inner = state.snapshot();
    Json(serde_json::json!({
        "server": {
            "host": inner.config.server.host,
            "port": inner.config.server.port,
        },
        "router": {
            "default": inner.config.router.default,
            "background": inner.config.router.background,
            "think": inner.config.router.think,
            "websearch": inner.config.router.websearch,
            "auto_map_regex": inner.config.router.auto_map_regex,
            "background_regex": inner.config.router.background_regex,
            "prompt_rules": inner.config.router.prompt_rules,
        },
        "providers": inner.config.providers,
        "models": inner.config.models,
    }))
}

/// Remove null values from JSON (TOML doesn't support null)
fn remove_null_values(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            map.retain(|_, v| !v.is_null());
            for (_, v) in map.iter_mut() {
                remove_null_values(v);
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr.iter_mut() {
                remove_null_values(item);
            }
        }
        _ => {}
    }
}

/// Update configuration via JSON (for admin UI)
async fn update_config_json(
    State(state): State<Arc<AppState>>,
    Json(mut new_config): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Remove null values (TOML doesn't support null)
    remove_null_values(&mut new_config);

    // Write back to config file
    let config_path = &state.config_path;

    // Read current config
    let config_str = std::fs::read_to_string(config_path)
        .map_err(|e| AppError::ParseError(format!("Failed to read config: {}", e)))?;

    let mut config: toml::Value = toml::from_str(&config_str)
        .map_err(|e| AppError::ParseError(format!("Failed to parse config: {}", e)))?;

    // Update providers section
    if let Some(providers) = new_config.get("providers") {
        // Convert from serde_json::Value to toml::Value
        let providers_toml: toml::Value = serde_json::from_str(&providers.to_string())
            .map_err(|e| AppError::ParseError(format!("Failed to convert providers: {}", e)))?;

        if let Some(table) = config.as_table_mut() {
            table.insert("providers".to_string(), providers_toml);
        }
    }

    // Update models section
    if let Some(models) = new_config.get("models") {
        // Convert from serde_json::Value to toml::Value
        let models_toml: toml::Value = serde_json::from_str(&models.to_string())
            .map_err(|e| AppError::ParseError(format!("Failed to convert models: {}", e)))?;

        if let Some(table) = config.as_table_mut() {
            table.insert("models".to_string(), models_toml);
        }
    }

    // Update router section if provided
    if let Some(router) = new_config.get("router") {
        if let Some(router_table) = config.get_mut("router").and_then(|v| v.as_table_mut()) {
            // Helper to update or remove a router field
            let update_field = |table: &mut toml::map::Map<String, toml::Value>, key: &str, value: Option<&serde_json::Value>| {
                if let Some(val) = value {
                    if let Some(s) = val.as_str() {
                        table.insert(key.to_string(), toml::Value::String(s.to_string()));
                    }
                } else {
                    // Remove field if not present in incoming config
                    table.remove(key);
                }
            };

            // Default is required, always update if present
            if let Some(default) = router.get("default") {
                if let Some(s) = default.as_str() {
                    router_table.insert("default".to_string(), toml::Value::String(s.to_string()));
                }
            }

            // Optional fields - remove if not present
            update_field(router_table, "think", router.get("think"));
            update_field(router_table, "websearch", router.get("websearch"));
            update_field(router_table, "background", router.get("background"));
            update_field(router_table, "auto_map_regex", router.get("auto_map_regex"));
            update_field(router_table, "background_regex", router.get("background_regex"));
        }
    }

    // Write back to file
    let new_config_str = toml::to_string_pretty(&config)
        .map_err(|e| AppError::ParseError(format!("Failed to serialize config: {}", e)))?;

    std::fs::write(config_path, new_config_str)
        .map_err(|e| AppError::ParseError(format!("Failed to write config: {}", e)))?;

    info!("✅ Configuration updated successfully via admin UI");

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Configuration saved successfully"
    })))
}

/// Reload configuration without restarting the server
async fn reload_config(State(state): State<Arc<AppState>>) -> Response {
    info!("🔄 Configuration reload requested via UI");

    // 1. Read and parse new config (all sync, no locks held)
    let config_str = match std::fs::read_to_string(&state.config_path) {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to read config: {}", e);
            return Html(format!("<div class='px-4 py-3 rounded-xl bg-red-500/20 border border-red-500/50 text-foreground text-sm'><strong>❌ Reload failed</strong><br/>Failed to read config: {}</div>", e)).into_response();
        }
    };

    let new_config: AppConfig = match toml::from_str(&config_str) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to parse config: {}", e);
            return Html(format!("<div class='px-4 py-3 rounded-xl bg-red-500/20 border border-red-500/50 text-foreground text-sm'><strong>❌ Reload failed</strong><br/>Failed to parse config: {}</div>", e)).into_response();
        }
    };

    // 2. Build new router (compiles regexes)
    let new_router = Router::new(new_config.clone());

    // 3. Build new provider registry (reuse existing token_store)
    let new_registry = match ProviderRegistry::from_configs_with_models(
        &new_config.providers,
        Some(state.token_store.clone()),
        &new_config.models,
    ) {
        Ok(r) => Arc::new(r),
        Err(e) => {
            error!("Failed to init providers: {}", e);
            return Html(format!("<div class='px-4 py-3 rounded-xl bg-red-500/20 border border-red-500/50 text-foreground text-sm'><strong>❌ Reload failed</strong><br/>Failed to init providers: {}</div>", e)).into_response();
        }
    };

    // 4. Create new reloadable state
    let new_inner = Arc::new(ReloadableState {
        config: new_config,
        router: new_router,
        provider_registry: new_registry,
    });

    // 5. Atomic swap (write lock held for microseconds)
    *state.inner.write().unwrap() = new_inner;

    info!("✅ Configuration reloaded successfully");
    Html("<div class='px-4 py-3 rounded-xl bg-green-500/20 border border-green-500/50 text-foreground text-sm'><strong>✅ Configuration reloaded</strong><br/>New settings are now active.</div>").into_response()
}

/// Handle /v1/chat/completions requests (OpenAI-compatible endpoint)
///
/// Note: This endpoint has limited functionality. The primary use case for this proxy
/// is Claude Code (Anthropic client) connecting via /v1/messages.
async fn handle_openai_chat_completions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(openai_request): Json<openai_compat::OpenAIRequest>,
) -> Result<Response, AppError> {
    let model = openai_request.model.clone();
    let start_time = std::time::Instant::now();

    // Get snapshot of reloadable state
    let inner = state.snapshot();

    // Streaming is not supported for /v1/chat/completions
    if openai_request.stream == Some(true) {
        return Err(AppError::ParseError(
            "Streaming is not supported for /v1/chat/completions. Use /v1/messages instead.".to_string()
        ));
    }

    // 1. Transform OpenAI request to Anthropic format
    let mut anthropic_request = openai_compat::transform_openai_to_anthropic(openai_request)
        .map_err(|e| AppError::ParseError(format!("Failed to transform OpenAI request: {}", e)))?;

    // 2. Route the request (may modify system prompt to remove CCM-SUBAGENT-MODEL tag)
    let decision = inner
        .router
        .route(&mut anthropic_request)
        .map_err(|e| AppError::RoutingError(e.to_string()))?;

    // 3. Try model mappings with fallback (1:N mapping)
    if let Some(model_config) = inner.config.models.iter().find(|m| m.name.eq_ignore_ascii_case(&decision.model_name)) {

        // Check for X-Provider header to override priority
        let forced_provider = headers
            .get("x-provider")
            .and_then(|v| v.to_str().ok())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        if let Some(ref provider_name) = forced_provider {
            info!("🎯 Using forced provider from X-Provider header: {}", provider_name);
        }

        // Sort mappings by priority (or filter by forced provider)
        let mut sorted_mappings = model_config.mappings.clone();

        if let Some(ref provider_name) = forced_provider {
            // Filter to only the specified provider
            sorted_mappings.retain(|m| m.provider == *provider_name);
            if sorted_mappings.is_empty() {
                return Err(AppError::RoutingError(format!(
                    "Provider '{}' not found in mappings for model '{}'",
                    provider_name, decision.model_name
                )));
            }
        } else {
            // Use priority ordering
            sorted_mappings.sort_by_key(|m| m.priority);
        }

        // Try each mapping in priority order (or just the forced one)
        for (idx, mapping) in sorted_mappings.iter().enumerate() {
            // Try to get provider from registry
            if let Some(provider) = inner.provider_registry.get_provider(&mapping.provider) {
                // Build retry indicator (only show if not first attempt)
                let retry_info = if idx > 0 {
                    format!(" [{}/{}]", idx + 1, sorted_mappings.len())
                } else {
                    String::new()
                };

                // Build route type display (include matched prompt snippet if available)
                let route_type_display = match &decision.matched_prompt {
                    Some(matched) => {
                        // Trim prompt to max 30 chars
                        let trimmed = if matched.len() > 30 {
                            format!("{}...", &matched[..27])
                        } else {
                            matched.clone()
                        };
                        format!("{}:{}", decision.route_type, trimmed)
                    }
                    None => decision.route_type.to_string(),
                };

                info!(
                    "[{:<15}:sync] {:<25} → {}/{}{}",
                    route_type_display,
                    model,
                    mapping.provider,
                    mapping.actual_model,
                    retry_info
                );

                // Update model to actual model name
                anthropic_request.model = mapping.actual_model.clone();

                // Inject continuation prompt if configured (skip for background tasks)
                if mapping.inject_continuation_prompt && decision.route_type != RouteType::Background {
                    if let Some(last_msg) = anthropic_request.messages.last_mut() {
                        if should_inject_continuation(last_msg) {
                            info!("💉 Injecting continuation prompt for model: {}", mapping.actual_model);
                            inject_continuation_text(last_msg);
                        }
                    }
                }

                match provider.send_message(anthropic_request.clone()).await {
                    Ok(anthropic_response) => {
                        // Calculate and log metrics
                        let latency_ms = start_time.elapsed().as_millis() as u64;
                        let tok_s = (anthropic_response.usage.output_tokens as f32 * 1000.0) / latency_ms as f32;
                        info!("📊 {}@{} {}ms {:.0}t/s {}tok", mapping.actual_model, mapping.provider, latency_ms, tok_s, anthropic_response.usage.output_tokens);

                        // Write routing info for statusline
                        write_routing_info(&mapping.actual_model, &mapping.provider, &decision.route_type);

                        // Transform Anthropic response to OpenAI format
                        let openai_response = openai_compat::transform_anthropic_to_openai(
                            anthropic_response,
                            model.clone(),
                        );

                        return Ok(Json(openai_response).into_response());
                    }
                    Err(e) => {
                        info!("⚠️ Provider {} failed: {}, trying next fallback", mapping.provider, e);
                        continue;
                    }
                }
            } else {
                info!("⚠️ Provider {} not found in registry, trying next fallback", mapping.provider);
                continue;
            }
        }

        error!("❌ All provider mappings failed for model: {}", decision.model_name);
        return Err(AppError::ProviderError(format!(
            "All {} provider mappings failed for model: {}",
            sorted_mappings.len(),
            decision.model_name
        )));
    } else {
        // No model mapping found, try direct provider registry lookup (backward compatibility)
        if let Ok(provider) = inner.provider_registry.get_provider_for_model(&decision.model_name) {
            info!("📦 Using provider from registry (direct lookup): {}", decision.model_name);

            // Update model to routed model
            anthropic_request.model = decision.model_name.clone();

            let anthropic_response = provider.send_message(anthropic_request)
                .await
                .map_err(|e| AppError::ProviderError(e.to_string()))?;

            // Transform to OpenAI format
            let openai_response = openai_compat::transform_anthropic_to_openai(
                anthropic_response,
                model,
            );

            return Ok(Json(openai_response).into_response());
        }

        error!("❌ No model mapping or provider found for model: {}", decision.model_name);
        return Err(AppError::ProviderError(format!(
            "No model mapping or provider found for model: {}",
            decision.model_name
        )));
    }
}

/// Check if message has tool results but no text content
/// (indicates model should continue after tool execution)
fn should_inject_continuation(msg: &crate::models::Message) -> bool {
    use crate::models::MessageContent;
    let has_tool_results = match &msg.content {
        MessageContent::Blocks(blocks) => blocks.iter().any(|b| b.is_tool_result()),
        _ => false,
    };

    let has_text = match &msg.content {
        MessageContent::Text(text) => !text.trim().is_empty(),
        MessageContent::Blocks(blocks) => {
            blocks.iter().any(|b| b.as_text().map(|t| !t.trim().is_empty()).unwrap_or(false))
        }
    };

    // Inject if message has tool results but no text
    has_tool_results && !has_text
}

/// Inject continuation text into the last user message
/// Prepends a text block to the existing message content (doesn't create a new message)
fn inject_continuation_text(msg: &mut crate::models::Message) {
    use crate::models::{MessageContent, ContentBlock};

    let continuation = "<system-reminder>If you have an active todo list, remember to mark items complete and continue to the next. Do not mention this reminder.</system-reminder>";

    match &mut msg.content {
        MessageContent::Text(text) => {
            // Convert to Blocks and prepend continuation
            let original_text = text.clone();
            msg.content = MessageContent::Blocks(vec![
                ContentBlock::text(continuation.to_string(), None),
                ContentBlock::text(original_text, None),
            ]);
        }
        MessageContent::Blocks(blocks) => {
            // Prepend continuation text to existing blocks
            blocks.insert(0, ContentBlock::text(continuation.to_string(), None));
        }
    }
}

/// Handle /v1/messages requests (both streaming and non-streaming)
async fn handle_messages(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request_json): Json<serde_json::Value>,
) -> Result<Response, AppError> {
    let model = request_json
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("unknown");
    let start_time = std::time::Instant::now();

    // Get snapshot of reloadable state
    let inner = state.snapshot();

    // Generate trace ID for correlating request/response
    let trace_id = state.message_tracer.new_trace_id();

    // DEBUG: Log request body for debugging
    if let Ok(json_str) = serde_json::to_string_pretty(&request_json) {
        tracing::debug!("📥 Incoming request body:\n{}", json_str);
    }

    // 1. Parse request for routing decision (mutable for tag extraction)
    let mut request_for_routing: AnthropicRequest = serde_json::from_value(request_json.clone())
        .map_err(|e| {
            // Log the full request on parse failure for debugging
            if let Ok(pretty) = serde_json::to_string_pretty(&request_json) {
                tracing::error!("❌ Failed to parse request: {}\n📋 Request body:\n{}", e, pretty);
            } else {
                tracing::error!("❌ Failed to parse request: {}", e);
            }
            AppError::ParseError(format!("Invalid request format: {}", e))
        })?;

    // 2. Route the request (may modify system prompt to remove CCM-SUBAGENT-MODEL tag)
    let decision = inner
        .router
        .route(&mut request_for_routing)
        .map_err(|e| AppError::RoutingError(e.to_string()))?;

    // 3. Try model mappings with fallback (1:N mapping)
    if let Some(model_config) = inner.config.models.iter().find(|m| m.name.eq_ignore_ascii_case(&decision.model_name)) {

        // Check for X-Provider header to override priority
        let forced_provider = headers
            .get("x-provider")
            .and_then(|v| v.to_str().ok())
            .filter(|s| !s.is_empty())  // Ignore empty strings
            .map(|s| s.to_string());

        if let Some(ref provider_name) = forced_provider {
            info!("🎯 Using forced provider from X-Provider header: {}", provider_name);
        }

        // Sort mappings by priority (or filter by forced provider)
        let mut sorted_mappings = model_config.mappings.clone();

        if let Some(ref provider_name) = forced_provider {
            // Filter to only the specified provider
            sorted_mappings.retain(|m| m.provider == *provider_name);
            if sorted_mappings.is_empty() {
                return Err(AppError::RoutingError(format!(
                    "Provider '{}' not found in mappings for model '{}'",
                    provider_name, decision.model_name
                )));
            }
        } else {
            // Use priority ordering
            sorted_mappings.sort_by_key(|m| m.priority);
        }

        // Try each mapping in priority order (or just the forced one)
        for (idx, mapping) in sorted_mappings.iter().enumerate() {
            // Try to get provider from registry
            if let Some(provider) = inner.provider_registry.get_provider(&mapping.provider) {
                // Trust the model mapping configuration - no need to validate

                // Parse request as Anthropic format
                let mut anthropic_request: AnthropicRequest = serde_json::from_value(request_json.clone())
                    .map_err(|e| AppError::ParseError(format!("Invalid request format: {}", e)))?;

                // Save original model name for response
                let original_model = anthropic_request.model.clone();

                // Update model to actual model name
                anthropic_request.model = mapping.actual_model.clone();

                // Apply routing modifications (system prompt, messages)
                anthropic_request.system = request_for_routing.system.clone();
                anthropic_request.messages = request_for_routing.messages.clone();

                // Inject continuation prompt if configured (skip for background tasks)
                if mapping.inject_continuation_prompt && decision.route_type != RouteType::Background {
                    if let Some(last_msg) = anthropic_request.messages.last_mut() {
                        if should_inject_continuation(last_msg) {
                            info!("💉 Injecting continuation prompt for model: {}", mapping.actual_model);
                            inject_continuation_text(last_msg);
                        }
                    }
                }

                // Check if streaming is requested
                let is_streaming = anthropic_request.stream == Some(true);

                // Build retry indicator (only show if not first attempt)
                let retry_info = if idx > 0 {
                    format!(" [{}/{}]", idx + 1, sorted_mappings.len())
                } else {
                    String::new()
                };

                let stream_mode = if is_streaming { "stream" } else { "sync" };

                // Build route type display (include matched prompt snippet if available)
                let route_type_display = match &decision.matched_prompt {
                    Some(matched) => {
                        // Trim prompt to max 30 chars
                        let trimmed = if matched.len() > 30 {
                            format!("{}...", &matched[..27])
                        } else {
                            matched.clone()
                        };
                        format!("{}:{}", decision.route_type, trimmed)
                    }
                    None => decision.route_type.to_string(),
                };

                info!(
                    "[{:<15}:{}] {:<25} → {}/{}{}",
                    route_type_display,
                    stream_mode,
                    model,
                    mapping.provider,
                    mapping.actual_model,
                    retry_info
                );

                // Trace the request
                state.message_tracer.trace_request(
                    &trace_id,
                    &anthropic_request,
                    &mapping.provider,
                    &decision.route_type,
                    is_streaming,
                );

                if is_streaming {
                    // Streaming request
                    match provider.send_message_stream(anthropic_request).await {
                        Ok(stream_response) => {
                            // Write routing info for statusline
                            write_routing_info(&mapping.actual_model, &mapping.provider, &decision.route_type);

                            // Convert provider stream to HTTP response
                            // The provider already returns properly formatted SSE bytes (event: + data: lines)
                            // We pass them through as-is without wrapping
                            let body_stream = stream_response.stream.map_err(|e| {
                                error!("Stream error: {}", e);
                                std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
                            });

                            let body = Body::from_stream(body_stream);
                            let mut response_builder = Response::builder()
                                .status(200)
                                .header("Content-Type", "text/event-stream")
                                .header("Cache-Control", "no-cache")
                                .header("Connection", "keep-alive");

                            // Forward Anthropic rate limit headers
                            for (name, value) in stream_response.headers {
                                response_builder = response_builder.header(name, value);
                            }

                            let response = response_builder.body(body).unwrap();

                            return Ok(response);
                        }
                        Err(e) => {
                            state.message_tracer.trace_error(&trace_id, &e.to_string());
                            info!("⚠️ Provider {} streaming failed: {}, trying next fallback", mapping.provider, e);
                            continue;
                        }
                    }
                } else {
                    // Non-streaming request (original behavior)
                    match provider.send_message(anthropic_request).await {
                        Ok(mut response) => {
                            // Restore original model name in response
                            response.model = original_model;
                            info!("✅ Request succeeded with provider: {}, response model: {}", mapping.provider, response.model);

                            // Calculate and log metrics
                            let latency_ms = start_time.elapsed().as_millis() as u64;
                            let tok_s = (response.usage.output_tokens as f32 * 1000.0) / latency_ms as f32;
                            info!("📊 {}@{} {}ms {:.0}t/s {}tok", mapping.actual_model, mapping.provider, latency_ms, tok_s, response.usage.output_tokens);

                            // Trace the response
                            state.message_tracer.trace_response(&trace_id, &response, latency_ms);

                            // Write routing info for statusline
                            write_routing_info(&mapping.actual_model, &mapping.provider, &decision.route_type);

                            return Ok(Json(response).into_response());
                        }
                        Err(e) => {
                            state.message_tracer.trace_error(&trace_id, &e.to_string());
                            info!("⚠️ Provider {} failed: {}, trying next fallback", mapping.provider, e);
                            continue;
                        }
                    }
                }
            } else {
                info!("⚠️ Provider {} not found in registry, trying next fallback", mapping.provider);
                continue;
            }
        }

        error!("❌ All provider mappings failed for model: {}", decision.model_name);
        return Err(AppError::ProviderError(format!(
            "All {} provider mappings failed for model: {}",
            sorted_mappings.len(),
            decision.model_name
        )));
    } else {
        // No model mapping found, try direct provider registry lookup (backward compatibility)
        if let Ok(provider) = inner.provider_registry.get_provider_for_model(&decision.model_name) {
            info!("📦 Using provider from registry (direct lookup): {}", decision.model_name);

            // Parse request as Anthropic format
            let mut anthropic_request: AnthropicRequest = serde_json::from_value(request_json.clone())
                .map_err(|e| AppError::ParseError(format!("Invalid request format: {}", e)))?;

            // Save original model name for response
            let original_model = anthropic_request.model.clone();

            // Update model to routed model
            anthropic_request.model = decision.model_name.clone();

            // Apply routing modifications (system prompt, messages)
            anthropic_request.system = request_for_routing.system.clone();
            anthropic_request.messages = request_for_routing.messages.clone();

            // Call provider
            let mut provider_response = provider.send_message(anthropic_request)
                .await
                .map_err(|e| AppError::ProviderError(e.to_string()))?;

            // Restore original model name in response
            provider_response.model = original_model;

            // Return provider response
            return Ok(Json(provider_response).into_response());
        }

        error!("❌ No model mapping or provider found for model: {}", decision.model_name);
        return Err(AppError::ProviderError(format!(
            "No model mapping or provider found for model: {}",
            decision.model_name
        )));
    }
}

/// Handle /v1/messages/count_tokens requests
async fn handle_count_tokens(
    State(state): State<Arc<AppState>>,
    Json(request_json): Json<serde_json::Value>,
) -> Result<Response, AppError> {
    let model = request_json.get("model").and_then(|m| m.as_str()).unwrap_or("unknown");
    debug!("Received count_tokens request for model: {}", model);

    // Get snapshot of reloadable state
    let inner = state.snapshot();

    // 1. Parse as CountTokensRequest first
    use crate::models::CountTokensRequest;
    let count_request: CountTokensRequest = serde_json::from_value(request_json.clone())
        .map_err(|e| AppError::ParseError(format!("Invalid count_tokens request format: {}", e)))?;

    // 2. Create a minimal AnthropicRequest for routing
    let mut routing_request = AnthropicRequest {
        model: count_request.model.clone(),
        messages: count_request.messages.clone(),
        max_tokens: 1024, // Dummy value for routing
        system: count_request.system.clone(),
        tools: count_request.tools.clone(),
        thinking: None,
        temperature: None,
        top_p: None,
        top_k: None,
        stop_sequences: None,
        stream: None,
        metadata: None,
    };
    let decision = inner
        .router
        .route(&mut routing_request)
        .map_err(|e| AppError::RoutingError(e.to_string()))?;

    debug!(
        "🧮 Routed count_tokens: {} → {} ({})",
        model, decision.model_name, decision.route_type
    );

    // 3. Try model mappings with fallback (1:N mapping)
    if let Some(model_config) = inner.config.models.iter().find(|m| m.name.eq_ignore_ascii_case(&decision.model_name)) {
        debug!("📋 Found {} provider mappings for token counting: {}", model_config.mappings.len(), decision.model_name);

        // Sort mappings by priority
        let mut sorted_mappings = model_config.mappings.clone();
        sorted_mappings.sort_by_key(|m| m.priority);

        // Try each mapping in priority order
        for (idx, mapping) in sorted_mappings.iter().enumerate() {
            debug!(
                "🔄 Trying token count mapping {}/{}: provider={}, actual_model={}",
                idx + 1,
                sorted_mappings.len(),
                mapping.provider,
                mapping.actual_model
            );

            // Try to get provider from registry
            if let Some(provider) = inner.provider_registry.get_provider(&mapping.provider) {
                // Trust the model mapping configuration - no need to validate

                // Update model to actual model name
                let mut count_request_for_provider = count_request.clone();
                count_request_for_provider.model = mapping.actual_model.clone();

                // Call provider's count_tokens
                match provider.count_tokens(count_request_for_provider).await {
                    Ok(response) => {
                        debug!("✅ Token count succeeded with provider: {}", mapping.provider);
                        return Ok(Json(response).into_response());
                    }
                    Err(e) => {
                        debug!("⚠️ Provider {} failed: {}, trying next fallback", mapping.provider, e);
                        continue;
                    }
                }
            } else {
                debug!("⚠️ Provider {} not found in registry, trying next fallback", mapping.provider);
                continue;
            }
        }

        error!("❌ All provider mappings failed for token counting: {}", decision.model_name);
        return Err(AppError::ProviderError(format!(
            "All {} provider mappings failed for token counting: {}",
            sorted_mappings.len(),
            decision.model_name
        )));
    } else {
        // No model mapping found, try direct provider registry lookup (backward compatibility)
        if let Ok(provider) = inner.provider_registry.get_provider_for_model(&decision.model_name) {
            debug!("📦 Using provider from registry (direct lookup) for token counting: {}", decision.model_name);

            // Update model to routed model
            let mut count_request_for_provider = count_request.clone();
            count_request_for_provider.model = decision.model_name.clone();

            // Call provider's count_tokens
            let response = provider.count_tokens(count_request_for_provider)
                .await
                .map_err(|e| AppError::ProviderError(e.to_string()))?;

            debug!("✅ Token count completed via provider");
            return Ok(Json(response).into_response());
        }

        error!("❌ No model mapping or provider found for token counting: {}", decision.model_name);
        return Err(AppError::ProviderError(format!(
            "No model mapping or provider found for token counting: {}",
            decision.model_name
        )));
    }
}

/// Application error types
#[derive(Debug)]
pub enum AppError {
    RoutingError(String),
    ParseError(String),
    ProviderError(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::RoutingError(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::ParseError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::ProviderError(msg) => (StatusCode::BAD_GATEWAY, msg),
        };

        let body = Json(serde_json::json!({
            "error": {
                "type": "error",
                "message": message
            }
        }));

        (status, body).into_response()
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::RoutingError(msg) => write!(f, "Routing error: {}", msg),
            AppError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            AppError::ProviderError(msg) => write!(f, "Provider error: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}
