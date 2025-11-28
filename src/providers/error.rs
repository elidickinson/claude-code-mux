use thiserror::Error;

/// Provider-specific errors
#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("JSON serialization failed: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Model not supported by provider: {0}")]
    ModelNotSupported(String),

    #[error("Provider API error: {status} - {message}")]
    ApiError { status: u16, message: String },

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Authentication error: {0}")]
    AuthError(String),
}

impl ProviderError {
    /// Check if this is a client error (4xx) that should not be retried with fallback providers
    /// Client errors indicate invalid request parameters that won't succeed on other providers
    pub fn is_client_error(&self) -> bool {
        match self {
            ProviderError::ApiError { status, .. } => *status >= 400 && *status < 500,
            _ => false,
        }
    }
}
