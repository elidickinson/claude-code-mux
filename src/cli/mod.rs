use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use anyhow::{Context, Result};
use crate::providers::ProviderConfig;

/// Application configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    #[serde(default)]
    pub server: ServerConfig,
    pub router: RouterConfig,
    #[serde(default)]
    pub providers: Vec<ProviderConfig>,
    #[serde(default)]
    pub models: Vec<ModelConfig>,
}

/// Server configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_host")]
    pub host: String,
    pub api_key: Option<String>,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default)]
    pub timeouts: TimeoutConfig,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            host: default_host(),
            api_key: None,
            log_level: default_log_level(),
            timeouts: TimeoutConfig::default(),
        }
    }
}

fn default_port() -> u16 {
    3456
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

/// Timeout configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TimeoutConfig {
    #[serde(default = "default_api_timeout")]
    pub api_timeout_ms: u64,
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout_ms: u64,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            api_timeout_ms: default_api_timeout(),
            connect_timeout_ms: default_connect_timeout(),
        }
    }
}

fn default_api_timeout() -> u64 {
    600_000 // 10 minutes
}

fn default_connect_timeout() -> u64 {
    10_000 // 10 seconds
}

/// Router configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RouterConfig {
    pub default: String,
    pub background: Option<String>,
    pub think: Option<String>,
    pub websearch: Option<String>,
    /// Regex pattern for auto-mapping models (e.g., "^claude-").
    /// If empty/null, defaults to Claude models only.
    pub auto_map_regex: Option<String>,
    /// Regex pattern for detecting background tasks (e.g., "(?i)claude.*haiku").
    /// If empty/null, defaults to claude-haiku pattern.
    pub background_regex: Option<String>,
}

/// Model configuration with 1:N provider mappings
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelConfig {
    /// External model name (used in API requests)
    pub name: String,
    /// List of provider mappings with priorities (fallback support)
    pub mappings: Vec<ModelMapping>,
}

/// Model mapping to a specific provider
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelMapping {
    /// Priority for this mapping (1 = highest priority)
    pub priority: u32,
    /// Provider name
    pub provider: String,
    /// Actual model name to use with the provider
    pub actual_model: String,
}

impl ModelConfig {}

impl AppConfig {
    /// Get default config file path
    /// Returns ~/.claude-code-mux/config.toml (cross-platform)
    pub fn default_path() -> Result<PathBuf> {
        let home = dirs::home_dir()
            .context("Failed to get home directory")?;
        let config_dir = home.join(".claude-code-mux");
        std::fs::create_dir_all(&config_dir)
            .with_context(|| format!("Failed to create config directory: {}", config_dir.display()))?;
        Ok(config_dir.join("config.toml"))
    }

    /// Load configuration from a TOML file
    pub fn from_file(path: &PathBuf) -> Result<Self> {
        // Check if file exists, if not create a default one
        if !path.exists() {
            Self::create_default_config(path)?;
        }

        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let mut config: AppConfig = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        // Resolve environment variables
        config.resolve_env_vars()?;

        Ok(config)
    }

    /// Create a default configuration file
    fn create_default_config(path: &PathBuf) -> Result<()> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
        }

        // Generate default config content
        let default_config = Self::default_config_content();

        // Write to file
        std::fs::write(path, default_config)
            .with_context(|| format!("Failed to write default config file: {}", path.display()))?;

        eprintln!("Created default config file at: {}", path.display());
        eprintln!("Please edit the config file to add your providers and models.");
        eprintln!("You can also configure via the web UI at http://127.0.0.1:13456");

        Ok(())
    }

    /// Generate default configuration content as TOML string
    fn default_config_content() -> String {
        r#"# Claude Code Mux Configuration
#
# This is a minimal default configuration.
# Configure your providers and models via the web UI at http://127.0.0.1:13456
# or edit this file directly.

[server]
host = "127.0.0.1"
port = 13456
log_level = "info"

[server.timeouts]
api_timeout_ms = 600000      # 10 minutes
connect_timeout_ms = 10000   # 10 seconds

[router]
# Default model to use when no routing conditions are met
# You MUST configure at least one provider and model before using CCM
default = "placeholder-model"

# Optional: Model for background tasks (e.g., "glm-4.5-air")
# background = ""

# Optional: Model for thinking/reasoning tasks (e.g., "claude-opus-4-1")
# think = ""

# Optional: Model for web search tasks (e.g., "glm-4.6")
# websearch = ""

# Optional: Regex pattern for auto-mapping models (e.g., "^claude-")
# auto_map_regex = ""

# Optional: Regex pattern for detecting background tasks (e.g., "(?i)claude.*haiku")
# background_regex = ""

# Providers configuration
# Add providers via the web UI or edit this section
# Example:
# [[providers]]
# name = "my-provider"
# provider_type = "anthropic"  # or "openai", "openrouter", etc.
# auth_type = "api_key"        # or "oauth"
# api_key = "your-api-key-here"
# enabled = true
# models = []

# Models configuration
# Add models via the web UI or edit this section
# Example:
# [[models]]
# name = "my-model"
#
# [[models.mappings]]
# provider = "my-provider"
# actual_model = "claude-sonnet-4-5"
# priority = 1
"#.to_string()
    }

    /// Resolve environment variables in configuration
    fn resolve_env_vars(&mut self) -> Result<()> {
        // Resolve server API key
        if let Some(ref key) = self.server.api_key {
            if key.starts_with('$') {
                let env_var = &key[1..];
                self.server.api_key = std::env::var(env_var).ok();
            }
        }

        // Resolve provider API keys (only for enabled providers)
        for provider in &mut self.providers {
            // Skip disabled providers
            if !provider.is_enabled() {
                continue;
            }

            // Only resolve env vars for API key auth
            if let Some(ref api_key) = provider.api_key {
                if api_key.starts_with('$') {
                    let env_var = &api_key[1..];
                    if let Ok(value) = std::env::var(env_var) {
                        provider.api_key = Some(value);
                    } else {
                        anyhow::bail!("Environment variable {} not found for provider {}", env_var, provider.name);
                    }
                }
            }
        }

        Ok(())
    }
}

// TODO: Re-enable these tests by adding tempfile to dev-dependencies
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use std::io::Write;
//     use tempfile::NamedTempFile;
//
//     #[test]
//     fn test_parse_toml_config() {
//         let config_content = r#"
// [server]
// port = 3456
// host = "127.0.0.1"
// log_level = "info"
//
// [server.timeouts]
// api_timeout_ms = 600000
// connect_timeout_ms = 10000
//
// [litellm]
// endpoint = "http://localhost:4000"
// api_key = "anything"
//
// [router]
// default = "default"
// think = "think"
//         "#;
//
//         let mut temp_file = NamedTempFile::new().unwrap();
//         temp_file.write_all(config_content.as_bytes()).unwrap();
//
//         let config = AppConfig::from_file(&temp_file.path().to_path_buf()).unwrap();
//
//         assert_eq!(config.server.port, 3456);
//         assert_eq!(config.litellm.endpoint, "http://localhost:4000");
//         assert_eq!(config.litellm.api_key, "anything");
//         assert_eq!(config.router.default, "default");
//     }
// }
