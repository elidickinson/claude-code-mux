# Investigation: embedded-providers

## Current State Analysis

### Architecture Overview
CCM is a transparent Anthropic API proxy that delegates to LiteLLM for provider diversity. Current flow:
```
Claude Code → CCM (routing) → LiteLLM → 100+ Providers
```

### Files Analyzed
- @src/server/mod.rs:299-411 - HTTP handlers for `/v1/messages` and `/v1/messages/count_tokens`
- @src/models/mod.rs:1-218 - Complete Anthropic API data structures
- @src/router/mod.rs:1-290 - Intelligent routing (websearch > subagent > think > background > default)
- @src/cli/mod.rs:78-88 - LiteLLM configuration structure
- @Cargo.toml:42 - tiktoken-rs present but unused

### Current Behavior

**Request Flow (@src/server/mod.rs:299-363):**
1. Parse Anthropic request
2. Route via `router.route()` to select model
3. Replace model name with routed model
4. Strip Anthropic-specific params for non-Claude models (e.g., `thinking`)
5. Forward JSON as-is to LiteLLM
6. Return LiteLLM's response (streaming or non-streaming)

**Token Counting (@src/server/mod.rs:365-411):**
- Currently delegates entirely to LiteLLM
- Replaces model with default model
- Makes HTTP POST to `{litellm}/v1/messages/count_tokens`
- Returns LiteLLM's response

**Key Insight:** CCM maintains Anthropic API format end-to-end. LiteLLM handles translation to provider-native formats.

## Git History

### Repository State
```bash
git log --oneline --all | head -5
```
Recent commits focused on:
- Clean up unused code warnings
- Fix thinking parameter handling for non-Anthropic models
- Dynamic port in restart script
- UI configuration for LiteLLM endpoint/API key

### Past Decisions
- **No provider abstraction attempts** - Clean slate for implementation
- **Anthropic API format chosen** - Maintains compatibility with Claude Code
- **Transparent proxy pattern** - Minimal transformation, delegate complexity to LiteLLM
- **Intelligent routing** - Context-aware (not load balancing), unique to CCM

**Key Quote (@src/server/mod.rs:437):** `// Send the JSON as-is (complete proxy!)`

## Existing Patterns

### Conventions to Follow

1. **Trait-Based Abstraction**
   - Already uses traits: `Send + Sync` bounds in async handlers
   - Pattern: Define `trait AnthropicProvider` similar to Bifrost's `Provider` interface

2. **Configuration with Environment Variables**
   - @src/cli/mod.rs:114-131 - Resolves `$VAR_NAME` from environment
   - Pattern: `api_key = "$OPENAI_API_KEY"` in TOML

3. **Error Handling with anyhow**
   - Current: `anyhow::Result<T>` throughout
   - Recommend: Switch to `thiserror` for library errors, keep `anyhow` for application

4. **Async HTTP with reqwest**
   - @src/server/mod.rs:32-38 - Shared `reqwest::Client` in AppState
   - Pattern: Reuse for provider HTTP calls

5. **Streaming with Axum**
   - @src/server/mod.rs:507-539 - Zero-copy SSE streaming
   - Pattern: Apply same pattern for provider streams

6. **Intelligent Routing System** - **MUST BE PRESERVED**
   - @src/router/mod.rs:1-290 - Context-aware model swapping (PRIMARY FEATURE)
   - Priority: websearch > subagent > think > background > default
   - Pattern: Router returns model name → Provider registry maps to actual provider
   - Example flow:
     ```
     Request → Router.route() → "gpt-4" → ProviderRegistry.get("gpt-4") → OpenAIProvider
     ```
   - This is the core functionality - like /tmp/claude-code-router

### Similar Implementations in Codebase
None - this is the first provider abstraction.

**Reference Patterns:**
- Bifrost (@/tmp/bifrost): Go-based multi-provider with RouteConfig pattern
- LiteLLM (@/tmp/litellm): Python transformation layers, 100+ providers
- rig.rs (external): Rust LLM framework with unified provider traits

## Challenges & Considerations

### Technical Challenges
1. **Request/Response Transformation**
   - Anthropic Messages API ↔ OpenAI Chat Completions format
   - Must handle: messages, system prompts, tool calls, streaming
   - Solution: Dedicated transformation functions per provider

2. **Token Counting Accuracy**
   - tiktoken-rs only supports OpenAI tokenizers
   - Anthropic uses different tokenizer (JSON config in LiteLLM repo)
   - Solution: Provider-specific token counters with trait

3. **Streaming SSE Compatibility**
   - Different providers use different SSE formats
   - Anthropic: `event: type\ndata: {...}`
   - OpenAI: `data: {...}`
   - Solution: Format detection + transformation layer

4. **Error Mapping**
   - Each provider has different error formats
   - Must map to consistent `ProviderError`
   - Solution: Error transformation per provider

### Architectural Considerations
1. **Configuration Migration**
   - Breaking change from LiteLLM config to provider config
   - Need auto-migration script
   - Hybrid mode for gradual rollout

2. **Performance**
   - Current: 1 HTTP hop (CCM → LiteLLM)
   - Proposed: 1 HTTP hop (CCM → Provider)
   - Expected improvement: 50-100ms latency reduction

3. **Maintainability**
   - Trade: Eliminate LiteLLM dependency vs maintain providers ourselves
   - Mitigation: Start with 2-3 providers, add as needed

4. **SaaS Requirements**
   - Credits/billing system needed (not yet implemented)
   - User authentication (future)
   - Rate limiting per user (future)
   - Usage analytics (future)

### Security Considerations
1. API keys stored in config file (current) - OK for now
2. Need secrets management for production (AWS Secrets Manager, Vault)
3. Provider API keys exposed to CCM process - acceptable risk
4. User credit limits prevent abuse

### Backward Compatibility
- Keep LiteLLM support in Phase 1-2 (hybrid mode)
- Auto-migrate config on first run
- Deprecation notice for LiteLLM-only setups
