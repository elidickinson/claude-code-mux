# Hot-Reload Implementation Plan

## Problem

When users click "Save & Restart" in the admin UI, the server restarts via a shell script that spawns a new process. This causes the new process to detach from the terminal, so users lose console log output.

## Solution

Implement hot-reload: reload configuration without restarting the process. The server stays running, keeps its terminal attachment, and atomically swaps in new config/router/registry state.

## Current Architecture

```rust
pub struct AppState {
    pub config: AppConfig,
    pub router: Router,
    pub provider_registry: Arc<ProviderRegistry>,
    pub token_store: TokenStore,
    pub config_path: std::path::PathBuf,
    pub message_tracer: Arc<MessageTracer>,
}
```

- State is created once at startup
- Passed to Axum via `app.with_state(Arc::new(state))`
- Immutable after server starts
- Restart required for config changes

## Proposed Architecture

### Versioned State Pattern

```rust
/// Reloadable components - rebuilt on config reload
struct ReloadableState {
    pub config: AppConfig,
    pub router: Router,
    pub provider_registry: Arc<ProviderRegistry>,
}

/// Main application state
pub struct AppState {
    /// Reloadable state behind a single lock for atomic updates
    pub inner: std::sync::RwLock<Arc<ReloadableState>>,

    /// Persistent state - NOT reloaded
    pub token_store: TokenStore,
    pub config_path: std::path::PathBuf,
    pub message_tracer: Arc<MessageTracer>,
}
```

### Why This Design

1. **Single lock** - No deadlock risk, no lock ordering concerns
2. **Atomic updates** - All reloadable components updated together
3. **Consistent snapshots** - Handlers see coherent state (config matches router matches registry)
4. **Minimal lock contention** - Lock held only for Arc clone (nanoseconds)
5. **No locks across await** - Clone Arc, release lock, then do async work

### Handler Pattern

```rust
async fn handle_messages(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<serde_json::Value>,
) -> Result<Response, AppError> {
    // Clone Arc while holding read lock briefly
    let inner = state.inner.read().unwrap().clone();
    // Lock released - inner is our snapshot for this request

    // Use inner.config, inner.router, inner.provider_registry
    let decision = inner.router.route(&mut request)?;

    if let Some(model_config) = inner.config.models.iter().find(...) {
        // ...
    }
}
```

### Reload Endpoint

```rust
async fn reload_config(State(state): State<Arc<AppState>>) -> Response {
    info!("Reloading configuration...");

    // 1. Read and parse new config (all sync, no locks held)
    let config_str = match std::fs::read_to_string(&state.config_path) {
        Ok(s) => s,
        Err(e) => return error_response(format!("Failed to read config: {}", e)),
    };

    let new_config: AppConfig = match toml::from_str(&config_str) {
        Ok(c) => c,
        Err(e) => return error_response(format!("Failed to parse config: {}", e)),
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
        Err(e) => return error_response(format!("Failed to init providers: {}", e)),
    };

    // 4. Create new reloadable state
    let new_inner = Arc::new(ReloadableState {
        config: new_config,
        router: new_router,
        provider_registry: new_registry,
    });

    // 5. Atomic swap (write lock held for microseconds)
    *state.inner.write().unwrap() = new_inner;

    info!("Configuration reloaded successfully");
    Html("Configuration reloaded").into_response()
}
```

## Implementation Steps

### 1. Create ReloadableState struct
- Add `ReloadableState` struct with config, router, provider_registry
- Modify `AppState` to use `RwLock<Arc<ReloadableState>>`
- Keep token_store, config_path, message_tracer on outer AppState

### 2. Update start_server()
- Build initial ReloadableState
- Wrap in Arc, wrap in RwLock
- Create AppState with the wrapped inner state

### 3. Add helper method to AppState
```rust
impl AppState {
    /// Get a snapshot of current reloadable state
    pub fn snapshot(&self) -> Arc<ReloadableState> {
        self.inner.read().unwrap().clone()
    }
}
```

### 4. Update all handlers
Replace direct `state.config` / `state.router` / `state.provider_registry` access with:
```rust
let inner = state.snapshot();
// Use inner.config, inner.router, inner.provider_registry
```

Handlers to update in `src/server/mod.rs`:
- `get_config`
- `update_config`
- `get_providers`
- `get_models_config`
- `get_config_json`
- `update_config_json`
- `handle_openai_chat_completions`
- `handle_messages`
- `handle_count_tokens`

Handlers to update in `src/server/oauth_handlers.rs`:
- Any that access state.config or state.provider_registry

### 5. Replace restart with reload
- Change `/api/restart` route to `/api/reload`
- Replace `restart_server` function with `reload_config`
- Remove `create_and_execute_restart_script` function

### 6. Update admin UI
In `src/server/admin.html`:
- Change "Save & Restart" button to call `/api/reload`
- Update button text to "Save & Reload"
- Update confirmation dialog text
- Update success/error messages

### 7. Optional: Keep restart as fallback
Could keep `/api/restart` for cases where reload isn't sufficient (e.g., port change requires restart). But probably unnecessary complexity.

## Edge Cases

### Concurrent reloads
Multiple simultaneous reload requests will serialize on the write lock. Last one wins. This is fine - no special handling needed.

### Reload during request processing
Requests in flight keep their snapshot. New requests after reload get new config. No inconsistency possible.

### Reload failure
If any step fails (parse error, provider init error), return error response immediately. Old state remains in place unchanged.

### What CAN'T be hot-reloaded
- Server port (requires rebinding socket)
- Host address (requires rebinding socket)
- OAuth callback port (separate server)

For these, users still need manual restart. Could add UI messaging for this.

## Testing

1. Start server, note console output works
2. Make config change via admin UI
3. Click "Save & Reload"
4. Verify console still shows logs
5. Verify new config is active (check routing, providers)
6. Test invalid config - verify graceful error, old config preserved

## Files to Modify

- `src/server/mod.rs` - Main implementation
- `src/server/oauth_handlers.rs` - Update state access pattern
- `src/server/admin.html` - UI changes
