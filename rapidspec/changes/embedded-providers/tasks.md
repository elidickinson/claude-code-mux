# Implementation Tasks

## Phase 1: Foundation + Admin UI (Week 1)

### 1.1 Provider Trait System (2 hours) - Checkpoint ⏸
- [ ] Create `src/providers/mod.rs` with provider traits
- [ ] Define `AnthropicProvider` trait with `send_message()` and `count_tokens()`
- [ ] Add error types with `thiserror` in `src/providers/error.rs`
- [ ] Update `Cargo.toml`: add `async-trait`, `thiserror`

**Checkpoint:** Compile succeeds, traits defined

### 1.2 Configuration Structure (2 hours) - Checkpoint ⏸
- [ ] Extend `src/cli/mod.rs` with `ProvidersConfig`
- [ ] Add provider config struct: `ProviderConfig { name, type, api_key, models, base_url }`
- [ ] Support environment variable resolution for API keys
- [ ] Update `config/default.toml` with provider examples

**Checkpoint:** Config loads correctly, `cargo run -- model` shows provider list

### 1.3 Admin UI for Provider Configuration (4 hours) - Checkpoint ⏸ **[REQUIRED FIRST]**
- [ ] Create HTML template for `/admin` endpoint
- [ ] Provider management UI (add/edit/delete providers)
- [ ] Model → Provider mapping configuration (like LiteLLM/Bifrost)
- [ ] Test provider connection button
- [ ] Persist config changes to TOML

**Checkpoint:** Admin can configure providers and model mappings via UI

### 1.4 OpenAI Provider Implementation (4 hours) - Checkpoint ⏸
- [ ] Create `src/providers/openai.rs`
- [ ] Implement `OpenAIProvider` struct with HTTP client
- [ ] Transform Anthropic request → OpenAI Chat Completions format
- [ ] Transform OpenAI response → Anthropic Messages format
- [ ] Handle streaming responses (SSE)

**Checkpoint:** Can call OpenAI GPT-4 through `/v1/messages`

### 1.5 Token Counting (3 hours) - Checkpoint ⏸
- [ ] Create `src/providers/tokens.rs`
- [ ] Implement `TokenCounter` trait
- [ ] Add `TiktokenCounter` for OpenAI models (use tiktoken-rs)
- [ ] Add caching layer (in-memory HashMap)
- [ ] Update `handle_count_tokens()` to use local counting

**Checkpoint:** `/v1/messages/count_tokens` works without LiteLLM

## Phase 2: Core Providers (Week 2)

### 2.1 Anthropic Provider (3 hours) - Checkpoint ⏸
- [ ] Create `src/providers/anthropic.rs`
- [ ] Implement `AnthropicProvider` for native Anthropic API
- [ ] Pass-through implementation (no transformation needed)
- [ ] Handle streaming responses
- [ ] Token counting via Anthropic's tokenizer JSON

**Checkpoint:** Can call Claude directly through `/v1/messages`

### 2.2 Provider Registry (4 hours) - Checkpoint ⏸
- [ ] Create `src/providers/registry.rs`
- [ ] Implement `ProviderRegistry` with `HashMap<String, Box<dyn AnthropicProvider>>`
- [ ] Load providers from config on startup
- [ ] Route model name → provider lookup
- [ ] Update `AppState` to include registry

**Checkpoint:** Routing works for both OpenAI and Anthropic

### 2.3 Error Handling & Retries (3 hours) - Checkpoint ⏸
- [ ] Implement retry logic with exponential backoff
- [ ] Add timeout handling (per-provider config)
- [ ] Map provider errors to `ProviderError`
- [ ] Add circuit breaker pattern (optional, recommended)

**Checkpoint:** Handles failures gracefully, retries work

### 2.4 Hybrid Mode Support (2 hours) - Checkpoint ⏸
- [ ] Keep LiteLLM fallback in config (optional)
- [ ] Check provider registry first, fallback to LiteLLM
- [ ] Add feature flag for hybrid mode

**Checkpoint:** Can gradually migrate models

## Phase 3: SaaS Features (Week 3)

### 3.1 Credits System (6 hours) - Checkpoint ⏸
- [ ] Create `src/credits/mod.rs`
- [ ] Define credits data model (User, Balance, Transaction)
- [ ] Implement in-memory storage (migrate to DB later)
- [ ] Track usage per request (tokens → cost → credits)
- [ ] Add middleware to deduct credits

**Checkpoint:** Basic credits tracking works

### 3.2 Dashboard UI (`/`) (4 hours) - Checkpoint ⏸
- [ ] Create HTML template for credits dashboard
- [ ] Show current balance, usage stats
- [ ] List available models
- [ ] Add credit purchase UI placeholder
- [ ] Update `serve_ui()` to render dashboard

**Checkpoint:** User can see balance and models

### 3.3 Usage Tracking & Billing (3 hours) - Checkpoint ⏸
- [ ] Add usage logging to database/file
- [ ] Track per-user costs (provider cost + markup)
- [ ] Generate billing reports
- [ ] Export usage data (CSV/JSON)

**Checkpoint:** Complete usage data available

### 3.4 Remove LiteLLM (1 hour) - Checkpoint ⏸
- [ ] Remove LiteLLM config from TOML
- [ ] Remove hybrid mode code
- [ ] Update documentation
- [ ] Remove `litellm` references

**Checkpoint:** Fully independent from LiteLLM

## Testing

### Unit Tests
- [ ] Provider transformation tests (Anthropic ↔ OpenAI)
- [ ] Token counting accuracy tests
- [ ] Provider registry tests
- [ ] Credits deduction tests

### Integration Tests
- [ ] E2E test: `/v1/messages` with OpenAI
- [ ] E2E test: `/v1/messages` with Anthropic
- [ ] E2E test: `/v1/messages/count_tokens`
- [ ] E2E test: Credits deduction flow
- [ ] E2E test: Admin UI provider management

### Manual Testing
- [ ] Test with Claude Code client
- [ ] Verify streaming works correctly
- [ ] Check error messages are user-friendly
- [ ] Performance test (latency comparison vs LiteLLM)

## Documentation
- [ ] Update README.md with new architecture
- [ ] Add provider configuration guide
- [ ] Document credits system
- [ ] Add migration guide from LiteLLM
- [ ] API documentation for `/` and `/admin`

## Validation
- [ ] All tests passing
- [ ] No cargo warnings
- [ ] Code review complete
- [ ] Performance benchmarks acceptable
- [ ] Ready to deploy
