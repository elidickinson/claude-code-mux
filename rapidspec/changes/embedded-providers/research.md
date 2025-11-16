# Research: embedded-providers

## Best Practices

### Rust LLM Provider Abstraction
Based on analysis of rig.rs, async-llm, and community patterns:

1. **Trait-Based Provider System**
   - Use `#[async_trait]` for async methods in traits
   - Define unified request/response types
   - Provider-specific implementations behind trait
   - Example: https://rig.rs/ - production-ready LLM framework

2. **Configuration Management**
   - Use `config-rs` or `confique` for layered configs
   - Support TOML, environment variables, programmatic overrides
   - Validate on load with custom deserializers

3. **API Key Security**
   - Use `secrecy` crate for in-memory protection
   - Never log API keys
   - Support multiple secret sources (env, file, AWS Secrets Manager)

4. **Error Handling**
   - Libraries: Use `thiserror` for structured errors
   - Applications: Use `anyhow` for error propagation
   - Map provider errors to unified `ProviderError` enum

### Industry Standards

- **OpenAI API**: De facto standard for chat completions
- **Anthropic Messages API**: Newer standard with advanced features (thinking, tool use)
- **Token Counting**: tiktoken (BPE) for OpenAI, provider-specific for others
- **Streaming**: Server-Sent Events (SSE) with `text/event-stream`

## Framework Documentation

### Tokio/Axum (Async Runtime & Web Framework)
- Documentation: https://tokio.rs, https://docs.rs/axum
- Pattern: Service trait for async request/response
- Current usage: Already integrated in CCM

### reqwest (HTTP Client)
- Documentation: https://docs.rs/reqwest
- Pattern: Shared `Client` with connection pooling
- Features needed: JSON, streaming, timeouts, retries
- Middleware: `reqwest-middleware` for retry logic

### tiktoken-rs (Token Counting)
- Documentation: https://docs.rs/tiktoken-rs
- Usage: `tiktoken_rs::get_bpe_from_model("gpt-4")`
- Limitation: Only supports OpenAI models
- Alternative: `rs-bpe` (15x faster on small strings)

### async-trait (Async in Traits)
- Documentation: https://docs.rs/async-trait
- Usage: `#[async_trait] trait Provider { async fn call(...) }`
- Note: Native async traits coming in Rust, but not stable yet

## Reference Implementations

### 1. **Bifrost** (https://github.com/maximhq/bifrost)
**Approach:**
- Go-based multi-provider proxy
- Unified `Provider` interface with ChatCompletion, Responses, Embedding methods
- RouteConfig pattern for declarative endpoint setup
- Multi-key support with weighted load balancing

**Pros:**
- Production-proven at scale
- Clean separation: parsing → conversion → execution → response
- Handles 10+ providers successfully

**Cons:**
- Go, not Rust
- More complex than needed for initial MVP

**Applicability:** HIGH - Architecture patterns directly transferable

**Key Files:**
- `/tmp/bifrost/core/schemas/provider.go` - Provider interface
- `/tmp/bifrost/transports/bifrost-http/integrations/anthropic.go` - Anthropic API
- `/tmp/bifrost/core/providers/anthropic/anthropic.go` - Anthropic implementation

---

### 2. **LiteLLM** (https://github.com/BerriAI/litellm)
**Approach:**
- Python-based universal LLM proxy
- 100+ provider integrations
- Provider detection from model name or API base
- Transformation layers: `get_llm_provider()` → `transform_request()` → `transform_response()`

**Pros:**
- Comprehensive provider support
- Well-documented transformation logic
- Handles edge cases (streaming, tool calling, vision)

**Cons:**
- Python, not Rust
- Complex codebase (thousands of lines)

**Applicability:** MEDIUM - Reference for transformations, not architecture

**Key Files:**
- `/tmp/litellm/litellm/litellm_core_utils/get_llm_provider_logic.py` - Routing
- `/tmp/litellm/litellm/llms/anthropic/experimental_pass_through/messages/handler.py` - Anthropic
- `/tmp/litellm/litellm/llms/openai/chat/transformation.py` - OpenAI transformations

---

### 3. **rig** (https://rig.rs/)
**Approach:**
- Rust LLM framework
- Unified API across providers (OpenAI, Anthropic, Cohere, Ollama)
- High-level abstractions for agents, RAG, embeddings

**Pros:**
- Pure Rust, idiomatic
- Production-ready
- Good documentation

**Cons:**
- Opinionated framework (we need low-level proxy)

**Applicability:** MEDIUM - Good reference for Rust patterns

---

### 4. **graniet/llm** (https://github.com/graniet/llm)
**Approach:**
- Rust CLI tool
- Supports OpenAI, Claude, Gemini, Ollama, ElevenLabs
- Unified `ChatProvider` and `CompletionProvider` traits

**Pros:**
- Simple, focused
- Good trait design

**Cons:**
- CLI tool, not server
- Limited features

**Applicability:** HIGH - Trait design patterns

## Security & Performance Considerations

### Security
1. **API Key Management**
   - Store in environment variables, never commit to git
   - Use `secrecy` crate to prevent accidental logging
   - Consider AWS Secrets Manager for production

2. **Request Validation**
   - Validate all incoming requests against schema
   - Prevent injection attacks (SQL, command)
   - Rate limit per user/IP

3. **Provider Isolation**
   - Each provider gets own HTTP client (connection pooling)
   - Timeout per provider prevents cascade failures
   - Circuit breaker pattern for failing providers

### Performance
1. **Token Counting**
   - Cache token counts (HashMap with TTL)
   - Use fast estimators for non-critical paths
   - Consider `rs-bpe` (15x faster than tiktoken-rs)

2. **HTTP Connections**
   - Connection pooling (reqwest default: 10 per host)
   - Keep-alive enabled
   - Timeout: 30s default, configurable per provider

3. **Streaming**
   - Zero-copy where possible
   - Use `response.bytes_stream()` without buffering
   - Backpressure handling (tokio channels)

## Trade-offs

| Aspect | Option 1: Embedded | Option 2: Keep LiteLLM |
|--------|-------------------|----------------------|
| Latency | ✅ 50-100ms faster | ❌ Middleware hop |
| Maintenance | ⚠️ More code to maintain | ✅ Offloaded to LiteLLM |
| Provider Breadth | ⚠️ Start with 2-3 | ✅ 100+ providers |
| Control | ✅ Full control | ❌ Limited |
| SaaS Features | ✅ Direct billing | ❌ Hard to implement |
| Implementation Time | ⚠️ 2-3 weeks | ✅ Already done |
| Cost Optimization | ✅ Direct pricing | ⚠️ No markup control |

**Recommendation:** Option 1 (Embedded) for long-term SaaS goals, accept short-term implementation cost.
