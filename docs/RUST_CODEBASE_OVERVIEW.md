# Claude Code Mux - Codebase Overview for Python Developers

## Abstract

CCM is an async HTTP proxy in Rust that routes Claude Code requests to 18+ AI providers. It uses Axum (think FastAPI), Tokio (think asyncio), and trait-based polymorphism (think ABC/Protocol). ~7,500 lines of Rust.

---

## Executive Summary

**What it does**: Sits between Claude Code and AI providers, intelligently routing requests based on tools, thinking mode, and regex patterns. Provides failover across multiple providers per model.

**Core architecture** (Python equivalents in parentheses):
- `main.rs` - CLI entry point (like `if __name__ == "__main__"`)
- `server/mod.rs` - HTTP handlers with Axum (FastAPI routes)
- `router/mod.rs` - Request routing logic (middleware-like)
- `providers/` - Provider implementations (strategy pattern via traits ≈ Protocols/ABCs)
- `models/mod.rs` - Data structures (Pydantic models)
- `auth/` - OAuth 2.0 with PKCE (authlib equivalent)

**Key Rust concepts you'll encounter**:
| Rust | Python Equivalent |
|------|-------------------|
| `trait` | `Protocol` / `ABC` |
| `impl Trait for Struct` | class implementing ABC |
| `async fn` / `.await` | `async def` / `await` |
| `Arc<T>` | shared reference (like passing same object to threads) |
| `Option<T>` | `Optional[T]` / `T | None` |
| `Result<T, E>` | return value + exception combined |
| `Vec<T>` | `list[T]` |
| `&str` / `String` | `str` (borrowed vs owned) |

**Request flow**:
```
HTTP POST /v1/messages
  → server::handle_messages()      # Parse & validate
  → router.route()                 # Decide model/provider
  → provider_registry.get()        # Get provider instance
  → provider.send_message[_stream]()  # Make API call
  → Response (converted to Anthropic format)
```

---

## Detailed Sections

### 1. Project Structure

```
src/
├── main.rs              # CLI commands (start, stop, init, etc.)
├── cli/mod.rs           # Config loading from TOML
├── server/
│   ├── mod.rs           # HTTP server & handlers (1024 lines - the heart)
│   ├── oauth_handlers.rs # OAuth endpoints
│   └── openai_compat.rs # OpenAI format conversion
├── router/mod.rs        # Intelligent routing logic
├── providers/
│   ├── mod.rs           # Provider trait definition
│   ├── registry.rs      # Provider factory & lookup
│   ├── anthropic_compatible.rs  # Anthropic-format providers
│   ├── openai.rs        # OpenAI-format providers (with conversion)
│   ├── gemini.rs        # Google Gemini
│   ├── streaming.rs     # SSE stream handling
│   └── error.rs         # Error types
├── models/mod.rs        # Request/Response data structures
├── auth/
│   ├── oauth.rs         # OAuth 2.0 client
│   └── token_store.rs   # Token persistence
└── pid.rs               # Process ID tracking
```

### 2. Entry Point: `main.rs`

```rust
// Simplified view - this is like argparse + main()
#[tokio::main]  // Makes main() async (like asyncio.run())
async fn main() -> Result<()> {
    let cli = Cli::parse();  // clap = argparse

    match cli.command {
        Commands::Start { .. } => {
            let config = load_config()?;  // ? = raise if error
            server::start_server(config).await?;
        }
        Commands::Stop => { /* kill by PID */ }
        // ...
    }
}
```

**Key files to read**: `main.rs:1-100` for CLI setup

### 3. HTTP Server: `server/mod.rs`

This is the core - think of it as your FastAPI app.

```rust
// AppState = shared state across all handlers (like FastAPI's Depends)
pub struct AppState {
    pub config: AppConfig,
    pub router: Router,
    pub provider_registry: Arc<ProviderRegistry>,  // Arc = shared ownership
    pub token_store: TokenStore,
}

// Route setup (like @app.post("/v1/messages"))
pub async fn start_server(config: AppConfig) -> Result<()> {
    let app = Router::new()
        .route("/v1/messages", post(handle_messages))
        .route("/api/config", get(get_config).post(update_config))
        // ...
        .with_state(state);  // Inject shared state

    axum::serve(listener, app).await?;
}
```

**Main handler** (`handle_messages`):
```rust
async fn handle_messages(
    State(state): State<Arc<AppState>>,  // Dependency injection
    Json(request): Json<AnthropicRequest>,  // Auto-deserialize body
) -> Result<impl IntoResponse, AppError> {
    // 1. Route the request
    let decision = state.router.route(&mut request);

    // 2. Get model mappings (provider list with priorities)
    let mappings = find_model_mappings(&decision.model_name);

    // 3. Try each provider in priority order
    for mapping in mappings {
        let provider = state.provider_registry.get(&mapping.provider)?;

        match provider.send_message(request.clone()).await {
            Ok(response) => return Ok(Json(response)),
            Err(_) => continue,  // Try next provider
        }
    }

    Err(AppError::AllProvidersFailed)
}
```

**Key files to read**: `server/mod.rs:668-874` for the main handler

### 4. Router: `router/mod.rs`

Decides which model to use based on request content.

```rust
pub fn route(&self, request: &mut AnthropicRequest) -> RouteDecision {
    // Priority order (highest to lowest):
    // 1. WebSearch - has web_search tool
    // 2. Subagent - has <CCM-SUBAGENT-MODEL> tag in system prompt
    // 3. Think - has thinking.type = "enabled"
    // 4. Background - model matches background_regex
    // 5. Default

    if has_web_search_tool(&request.tools) {
        return RouteDecision { model: self.websearch.clone(), route_type: WebSearch };
    }

    if let Some(model) = extract_subagent_model(&request.system) {
        return RouteDecision { model, route_type: Subagent };
    }

    // ... etc
}
```

**Key files to read**: `router/mod.rs:1-150`

### 5. Provider System: `providers/`

Uses traits (like Python's Protocol/ABC) for polymorphism.

```rust
// The trait = interface that all providers must implement
#[async_trait]
pub trait AnthropicProvider: Send + Sync {
    async fn send_message(&self, req: AnthropicRequest)
        -> Result<ProviderResponse, ProviderError>;

    async fn send_message_stream(&self, req: AnthropicRequest)
        -> Result<Pin<Box<dyn Stream<Item=Result<Bytes>>>>>;

    fn supports_model(&self, model: &str) -> bool;
}

// Implementation for Anthropic-compatible providers
impl AnthropicProvider for AnthropicCompatibleProvider {
    async fn send_message(&self, req: AnthropicRequest) -> Result<...> {
        // Just forward - already in correct format
        self.client.post(&self.base_url)
            .json(&req)
            .send().await?
            .json().await
    }
}

// Implementation for OpenAI providers (with format conversion)
impl AnthropicProvider for OpenAIProvider {
    async fn send_message(&self, req: AnthropicRequest) -> Result<...> {
        let openai_req = convert_to_openai(req);  // Format conversion
        let openai_resp = self.client.post(...).json(&openai_req).send().await?;
        convert_to_anthropic(openai_resp)  // Convert back
    }
}
```

**Registry pattern** (`registry.rs`):
```rust
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn AnthropicProvider>>,
}

impl ProviderRegistry {
    pub fn get(&self, name: &str) -> Option<Arc<dyn AnthropicProvider>> {
        self.providers.get(name).cloned()
    }
}
```

**Key files to read**:
- `providers/mod.rs:1-50` for trait definition
- `providers/anthropic_compatible.rs:1-100` for simple implementation
- `providers/openai.rs:1-150` for conversion logic

### 6. Data Models: `models/mod.rs`

Like Pydantic models - defines request/response structures.

```rust
#[derive(Debug, Serialize, Deserialize)]  // Auto JSON serialization
pub struct AnthropicRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]  // Omit if None
    pub thinking: Option<ThinkingConfig>,
    pub stream: Option<bool>,
    // ...
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]  // Union type - try each variant
pub enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}
```

**Key files to read**: `models/mod.rs:1-200`

### 7. Async & Error Handling

**Rust's Result type** (no exceptions!):
```rust
// Python: might raise, caller uses try/except
def load_config() -> Config:
    return json.load(open("config.json"))

// Rust: returns Result, caller uses ? or match
fn load_config() -> Result<Config, Error> {
    let file = File::open("config.json")?;  // ? = return Err if failed
    let config = serde_json::from_reader(file)?;
    Ok(config)  // Explicitly return success
}
```

**Async/await** (very similar to Python):
```rust
// Python
async def fetch_data():
    response = await client.get(url)
    return await response.json()

// Rust
async fn fetch_data() -> Result<Data> {
    let response = client.get(url).send().await?;
    response.json().await
}
```

### 8. Configuration: TOML Format

```toml
[server]
port = 13456
host = "127.0.0.1"

[router]
default = "claude-sonnet"
think = "claude-opus"       # For planning/thinking mode
background = "claude-haiku" # For background tasks
websearch = "glm-4.6"       # When web_search tool present

[[providers]]
name = "anthropic"
provider_type = "anthropic"
api_key = "${ANTHROPIC_API_KEY}"  # Env var expansion

[[models]]
name = "claude-sonnet"

[[models.mappings]]
priority = 1
provider = "anthropic"
actual_model = "claude-sonnet-4-20250514"

[[models.mappings]]
priority = 2  # Fallback
provider = "openrouter"
actual_model = "anthropic/claude-sonnet"
```

---

## Common Patterns You'll See

### 1. Shared State with Arc
```rust
// Arc = Atomic Reference Counted (like shared_ptr in C++)
// Allows multiple owners of same data across async tasks
let state = Arc::new(AppState { ... });
let state_clone = state.clone();  // Cheap - just increments counter
```

### 2. Option Handling
```rust
// Instead of None checks
if let Some(value) = maybe_value {
    // use value
}

// Or with default
let value = maybe_value.unwrap_or_default();

// Or early return
let value = maybe_value?;  // Returns None from function if None
```

### 3. Error Propagation
```rust
// The ? operator - like "raise if error, else unwrap"
fn process() -> Result<Output, Error> {
    let step1 = do_thing_1()?;  // Returns Err early if failed
    let step2 = do_thing_2(step1)?;
    Ok(step2)
}
```

### 4. Pattern Matching
```rust
match decision.route_type {
    RouteType::WebSearch => use_websearch_model(),
    RouteType::Think => use_think_model(),
    RouteType::Background => use_background_model(),
    RouteType::Default => use_default_model(),
}
```

---

## Quick Reference: Where to Find Things

| If you want to... | Look at... |
|-------------------|------------|
| Add a CLI command | `main.rs` |
| Add an HTTP endpoint | `server/mod.rs` |
| Change routing logic | `router/mod.rs` |
| Add a new provider | `providers/*.rs` + `registry.rs` |
| Modify request/response format | `models/mod.rs` |
| Change OAuth flow | `auth/oauth.rs` |
| Add config options | `cli/mod.rs` |

---

## Building & Running

```bash
# Build (like pip install -e .)
cargo build --release

# Run
./target/release/ccm start

# Run tests
cargo test

# Check types without building
cargo check

# Format code
cargo fmt

# Lint
cargo clippy
```

---

## Tips for Reading the Code

1. **Start with `server/mod.rs:handle_messages`** - it's the main entry point for requests
2. **Follow the types** - Rust's type system tells you exactly what flows where
3. **`?` means "might fail"** - trace back to see what errors can occur
4. **`impl X for Y`** - find all implementations of a trait with grep
5. **Cargo.toml** - see all dependencies and their docs on docs.rs
