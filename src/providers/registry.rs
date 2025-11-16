use super::{AnthropicProvider, ProviderConfig, OpenAIProvider, AnthropicCompatibleProvider, error::ProviderError};
use std::collections::HashMap;
use std::sync::Arc;

/// Provider registry that manages all configured providers
pub struct ProviderRegistry {
    /// Map of provider name -> provider instance
    providers: HashMap<String, Arc<Box<dyn AnthropicProvider>>>,
    /// Map of model name -> provider name for fast lookup
    model_to_provider: HashMap<String, String>,
}

impl ProviderRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            model_to_provider: HashMap::new(),
        }
    }

    /// Load providers from configuration
    pub fn from_configs(configs: &[ProviderConfig]) -> Result<Self, ProviderError> {
        let mut registry = Self::new();

        for config in configs {
            // Skip disabled providers
            if !config.is_enabled() {
                continue;
            }

            // Create provider instance based on type
            let provider: Box<dyn AnthropicProvider> = match config.provider_type.as_str() {
                // OpenAI
                "openai" => Box::new(OpenAIProvider::new(
                    config.api_key.clone(),
                    config.base_url.clone(),
                    config.models.clone(),
                )),

                // Anthropic-compatible providers
                "anthropic" => Box::new(AnthropicCompatibleProvider::new(
                    config.name.clone(),
                    config.api_key.clone(),
                    config.base_url.clone().unwrap_or_else(|| "https://api.anthropic.com".to_string()),
                    config.models.clone(),
                    None,
                )),
                "z.ai" => Box::new(AnthropicCompatibleProvider::zai(
                    config.api_key.clone(),
                    config.models.clone(),
                )),
                "minimax" => Box::new(AnthropicCompatibleProvider::minimax(
                    config.api_key.clone(),
                    config.models.clone(),
                )),
                "zenmux" => Box::new(AnthropicCompatibleProvider::zenmux(
                    config.api_key.clone(),
                    config.models.clone(),
                )),
                "kimi-coding" => Box::new(AnthropicCompatibleProvider::kimi_coding(
                    config.api_key.clone(),
                    config.models.clone(),
                )),

                // OpenAI-compatible providers
                "openrouter" => Box::new(OpenAIProvider::openrouter(
                    config.api_key.clone(),
                    config.models.clone(),
                )),
                "deepinfra" => Box::new(OpenAIProvider::deepinfra(
                    config.api_key.clone(),
                    config.models.clone(),
                )),
                "novita" => Box::new(OpenAIProvider::novita(
                    config.api_key.clone(),
                    config.models.clone(),
                )),
                "baseten" => Box::new(OpenAIProvider::baseten(
                    config.api_key.clone(),
                    config.models.clone(),
                )),
                "together" => Box::new(OpenAIProvider::together(
                    config.api_key.clone(),
                    config.models.clone(),
                )),
                "fireworks" => Box::new(OpenAIProvider::fireworks(
                    config.api_key.clone(),
                    config.models.clone(),
                )),
                "groq" => Box::new(OpenAIProvider::groq(
                    config.api_key.clone(),
                    config.models.clone(),
                )),
                "nebius" => Box::new(OpenAIProvider::nebius(
                    config.api_key.clone(),
                    config.models.clone(),
                )),
                "cerebras" => Box::new(OpenAIProvider::cerebras(
                    config.api_key.clone(),
                    config.models.clone(),
                )),
                "moonshot" => Box::new(OpenAIProvider::moonshot(
                    config.api_key.clone(),
                    config.models.clone(),
                )),

                other => {
                    return Err(ProviderError::ConfigError(
                        format!("Unknown provider type: {}", other)
                    ));
                }
            };

            // NOTE: models field in provider config is deprecated
            // Model mappings are now defined in [[models]] section
            // We only register the provider by name

            // Add provider to registry
            registry.providers.insert(config.name.clone(), Arc::new(provider));
        }

        Ok(registry)
    }

    /// Get a provider by name
    pub fn get_provider(&self, name: &str) -> Option<Arc<Box<dyn AnthropicProvider>>> {
        self.providers.get(name).cloned()
    }

    /// Get a provider for a specific model
    pub fn get_provider_for_model(&self, model: &str) -> Result<Arc<Box<dyn AnthropicProvider>>, ProviderError> {
        // First, check if we have a direct model → provider mapping
        if let Some(provider_name) = self.model_to_provider.get(model) {
            if let Some(provider) = self.providers.get(provider_name) {
                return Ok(provider.clone());
            }
        }

        // If no direct mapping, search through all providers
        for provider in self.providers.values() {
            if provider.supports_model(model) {
                return Ok(provider.clone());
            }
        }

        Err(ProviderError::ModelNotSupported(model.to_string()))
    }

    /// List all available models
    pub fn list_models(&self) -> Vec<String> {
        self.model_to_provider.keys().cloned().collect()
    }

    /// List all providers
    pub fn list_providers(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_registry() {
        let registry = ProviderRegistry::new();
        assert!(registry.list_models().is_empty());
        assert!(registry.list_providers().is_empty());
    }

    #[test]
    fn test_get_provider_for_model_not_found() {
        let registry = ProviderRegistry::new();
        let result = registry.get_provider_for_model("gpt-4");
        assert!(result.is_err());
    }
}
