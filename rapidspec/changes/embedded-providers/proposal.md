# Change: Embedded Multi-Provider Support with Model Swapping

## Why
**PRIMARY GOAL**: Enable Claude Code model swapping (default, think, websearch, image, background) like `/tmp/claude-code-router`, but with embedded multi-provider support instead of relying on LiteLLM middleware.

Currently CCM has intelligent routing (@src/router/mod.rs:1-290) that maps contexts to models, but still proxies through LiteLLM. To build a SaaS offering (like OpenRouter for Claude Code), we need:
1. **Model Swapping** - Direct control over which provider/model handles each request type
2. **Provider Independence** - Remove LiteLLM dependency for latency and control
3. **Anthropic API Compatibility** - Maintain `/v1/messages` and `/v1/messages/count_tokens`
4. **Future SaaS Features** - Credits system, admin UI, billing control

## Code Verification
- [x] Read @src/server/mod.rs:299-411 - Current LiteLLM delegation
- [x] Read @src/models/mod.rs:1-218 - Anthropic API models
- [x] Read @src/router/mod.rs:1-290 - Intelligent routing (websearch > subagent > think > background > default)
- [x] Read @Cargo.toml:42 - tiktoken-rs present but unused
- [x] Git history: No prior provider abstraction attempts
- [x] Existing patterns: Transparent proxy with format preservation
- [x] Rust version: Updated to 1.91.1 (from 1.86.0), compilation successful

## Current Routing System (@src/router/mod.rs)
CCM already implements intelligent model routing based on request context:
1. **WebSearch** (highest priority) - Detects `web_search` tool in request
2. **Subagent** - Extracts model from `<CCM-SUBAGENT-MODEL>` tag
3. **Think** - Detects Plan Mode via `thinking.type == "enabled"`
4. **Background** - Detects Haiku model variants
5. **Default** - Fallback model

**This routing logic must be preserved** and integrated with the new provider registry.

## What Changes

### Before (Verified Actual Code)
```rust
// @src/server/mod.rs:354-362
let litellm_url = format!("{}/v1/messages", state.config.litellm.endpoint);
let response = state.http_client
    .post(&litellm_url)
    .header("x-litellm-api-key", &state.config.litellm.api_key)
    .header("Content-Type", "application/json")
    .json(&request_json)
    .send()
    .await?;
```

```rust
// @src/server/mod.rs:374-389 (Token counting)
async fn handle_count_tokens(...) -> Result<Response, AppError> {
    request["model"] = serde_json::Value::String(
        state.config.router.default.clone()
    );

    let litellm_url = format!(
        "{}/v1/messages/count_tokens",
        state.config.litellm.endpoint
    );

    // Delegates to LiteLLM
    let response = state.http_client.post(&litellm_url)...
}
```

### After (Proposed)
```rust
// Provider trait abstraction
#[async_trait]
pub trait AnthropicProvider: Send + Sync {
    async fn send_message(&self, req: AnthropicRequest)
        -> Result<AnthropicResponse, ProviderError>;
    async fn count_tokens(&self, req: CountTokensRequest)
        -> Result<CountTokensResponse, ProviderError>;
}

// In handler - integrates with existing Router
let decision = state.router.route(&mut request)?; // Existing routing logic preserved
let provider = state.provider_registry
    .get_provider(&decision.model_name)?; // New: lookup provider by model name
let response = provider.send_message(request).await?; // Direct provider call

// Example provider registry mapping
// "gpt-4" → OpenAIProvider
// "claude-sonnet-4-5" → AnthropicProvider
// "gemini-pro" → GoogleProvider (future)
```

## Options Analysis

### Option 1: Full Embedded Multi-Provider ⭐ (Recommended)
**Architecture:**
```
Claude Code → CCM → OpenAI/Anthropic/Cohere (direct)
```

**Pros:**
- Complete LiteLLM independence - no middleware hop
- Native token counting (tiktoken-rs already in deps)
- Direct cost control - foundation for SaaS billing
- Lower latency - eliminates proxy hop (~50-100ms savings)
- Full control over error handling and retries
- Pattern validated by Bifrost & rig.rs

**Cons:**
- Significant implementation effort (2-3 weeks)
- Must maintain provider integrations ourselves
- Need to handle provider-specific quirks

**Cost:**
- Time: 2-3 weeks (phased rollout recommended)
- Risk: Medium (complex but well-researched)
- Complexity: High (~2500 lines new code)

**Pattern Match:** ✅ Follows Bifrost's trait-based abstraction + rig.rs patterns

---

### Option 2: Hybrid (Embedded + LiteLLM Fallback)
**Architecture:**
```
                ┌→ OpenAI (embedded)
Claude Code → CCM ┼→ Anthropic (embedded)
                └→ LiteLLM (fallback)
```

**Pros:**
- Gradual migration path - can implement incrementally
- Maintains LiteLLM's breadth for long-tail providers
- Lower immediate risk
- Control over critical providers (OpenAI, Anthropic)

**Cons:**
- Dual system complexity - two code paths to maintain
- Still depends on LiteLLM for some providers
- Configuration overhead

**Cost:**
- Time: 1 week for initial 2 providers
- Risk: Low
- Complexity: Medium (~1200 lines)

**Pattern Match:** ⚠️ Pragmatic but not aligned with SaaS goal

---

### Option 3: Keep LiteLLM, Add Local Token Counting
**Architecture:** Minimal change - just implement local token counting

**Pros:**
- Quickest implementation (2-3 days)
- Maintains LiteLLM provider breadth
- Very low risk

**Cons:**
- Doesn't advance SaaS goals
- Still has middleware latency
- No billing control

**Cost:**
- Time: 2-3 days
- Risk: Very Low
- Complexity: Low (~250 lines)

**Pattern Match:** ❌ Band-aid solution, not strategic

---

## Recommendation

**Option 1 with Phased Rollout**

### Phase 1 (Week 1) - Foundation + Admin UI
- Provider trait system + error handling
- Configuration structure (TOML)
- **Admin UI (`/admin`)** - Provider/model mapping configuration (REQUIRED FIRST)
- OpenAI provider implementation
- Local token counting (tiktoken-rs)

### Phase 2 (Week 2) - Core Providers
- Anthropic provider implementation
- Provider registry with routing
- Retry logic and circuit breakers
- Hybrid mode with LiteLLM fallback

### Phase 3 (Week 3) - SaaS Features
- Credits system (`/` endpoint)
- Usage tracking + billing data
- Remove LiteLLM dependency

**Why Option 1:**
1. **SaaS Alignment**: Full control required for commercial offering
2. **Performance**: Direct API calls eliminate middleware hop
3. **Research-Backed**: Bifrost proves this scales to production
4. **Cost Control**: Direct provider billing + markup capability
5. **Future-Proof**: Foundation for unlimited provider addition

**Evidence:**
- Bifrost successfully uses this pattern at scale
- LiteLLM analysis shows transformations are manageable
- Rust ecosystem provides all tooling (reqwest, async-trait, tiktoken-rs)
- User's goal matches OpenRouter model (confirmed)

## Impact

### Affected Files
- New: `src/providers/` (mod.rs, openai.rs, anthropic.rs, traits.rs)
- New: `src/credits/` (tracking, billing)
- Modified: `src/server/mod.rs` (routing to providers)
- Modified: `src/cli/mod.rs` (provider config)
- Modified: `Cargo.toml` (new dependencies: async-trait, thiserror)
- Removed: LiteLLM config sections

### Breaking Changes
- YES: Configuration format changes
- Migration: Auto-migrate existing config to new provider format
- Backward compat: Keep LiteLLM support in Phase 1-2 (hybrid mode)

### Affected Specs
- New spec: `rapidspec/specs/providers/spec.md`
- New spec: `rapidspec/specs/credits/spec.md`
- Modified: `rapidspec/specs/routing/spec.md` (if exists)

## Related
- Repository research: https://github.com/maximhq/bifrost (Go reference)
- Repository research: https://github.com/BerriAI/litellm (Python reference)
- Best practices: https://rig.rs/ (Rust LLM framework)
- Future: OpenRouter-like SaaS for Claude Code
