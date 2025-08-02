//! AI-specific error types and handling

use thiserror::Error;

/// Result type for AI operations
pub type AIResult<T> = Result<T, AIError>;

/// Comprehensive error types for AI operations
#[derive(Error, Debug, Clone)]
pub enum AIError {
    #[error("AI provider is unavailable: {message}")]
    ProviderUnavailable { message: String },

    #[error("Authentication failed: {provider}")]
    AuthenticationFailure { provider: String },

    #[error("Rate limit exceeded for provider: {provider}, retry after: {retry_after:?}")]
    RateLimitExceeded { 
        provider: String, 
        retry_after: Option<std::time::Duration> 
    },

    #[error("Content was filtered by AI provider: {reason}")]
    ContentFiltered { reason: String },

    #[error("Invalid response from AI provider: {details}")]
    InvalidResponse { details: String },

    #[error("AI configuration error: {message}")]
    ConfigurationError { message: String },

    #[error("Network error: {message}")]
    NetworkError { message: String },

    #[error("AI provider timeout after {timeout:?}")]
    Timeout { timeout: std::time::Duration },

    #[error("Insufficient API quota or credits for provider: {provider}")]
    InsufficientQuota { provider: String },

    #[error("Model not found: {model} for provider: {provider}")]
    ModelNotFound { model: String, provider: String },

    #[error("Request too large: {size} bytes exceeds limit")]
    RequestTooLarge { size: usize },

    #[error("AI service internal error: {message}")]
    InternalError { message: String },

    #[error("Feature not supported by provider: {provider}, feature: {feature}")]
    FeatureNotSupported { provider: String, feature: String },

    #[error("Cache error: {message}")]
    CacheError { message: String },
}

impl AIError {
    /// Create a provider unavailable error
    pub fn provider_unavailable(message: impl Into<String>) -> Self {
        AIError::ProviderUnavailable {
            message: message.into(),
        }
    }

    /// Create an authentication failure error
    pub fn auth_failure(provider: impl Into<String>) -> Self {
        AIError::AuthenticationFailure {
            provider: provider.into(),
        }
    }

    /// Create a rate limit exceeded error
    pub fn rate_limit(provider: impl Into<String>, retry_after: Option<std::time::Duration>) -> Self {
        AIError::RateLimitExceeded {
            provider: provider.into(),
            retry_after,
        }
    }

    /// Create a content filtered error
    pub fn content_filtered(reason: impl Into<String>) -> Self {
        AIError::ContentFiltered {
            reason: reason.into(),
        }
    }

    /// Create an invalid response error
    pub fn invalid_response(details: impl Into<String>) -> Self {
        AIError::InvalidResponse {
            details: details.into(),
        }
    }

    /// Create a configuration error
    pub fn config_error(message: impl Into<String>) -> Self {
        AIError::ConfigurationError {
            message: message.into(),
        }
    }

    /// Create a network error
    pub fn network_error(message: impl Into<String>) -> Self {
        AIError::NetworkError {
            message: message.into(),
        }
    }

    /// Create a timeout error
    pub fn timeout(timeout: std::time::Duration) -> Self {
        AIError::Timeout { timeout }
    }

    /// Create an insufficient quota error
    pub fn insufficient_quota(provider: impl Into<String>) -> Self {
        AIError::InsufficientQuota {
            provider: provider.into(),
        }
    }

    /// Create a model not found error
    pub fn model_not_found(model: impl Into<String>, provider: impl Into<String>) -> Self {
        AIError::ModelNotFound {
            model: model.into(),
            provider: provider.into(),
        }
    }

    /// Create a request too large error
    pub fn request_too_large(size: usize) -> Self {
        AIError::RequestTooLarge { size }
    }

    /// Create an internal error
    pub fn internal_error(message: impl Into<String>) -> Self {
        AIError::InternalError {
            message: message.into(),
        }
    }

    /// Create a feature not supported error
    pub fn feature_not_supported(provider: impl Into<String>, feature: impl Into<String>) -> Self {
        AIError::FeatureNotSupported {
            provider: provider.into(),
            feature: feature.into(),
        }
    }

    /// Create a cache error
    pub fn cache_error(message: impl Into<String>) -> Self {
        AIError::CacheError {
            message: message.into(),
        }
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            AIError::NetworkError { .. }
                | AIError::Timeout { .. }
                | AIError::ProviderUnavailable { .. }
                | AIError::RateLimitExceeded { .. }
                | AIError::InternalError { .. }
        )
    }

    /// Get suggested retry delay based on error type
    pub fn retry_delay(&self) -> Option<std::time::Duration> {
        match self {
            AIError::RateLimitExceeded { retry_after, .. } => *retry_after,
            AIError::NetworkError { .. } => Some(std::time::Duration::from_secs(1)),
            AIError::Timeout { .. } => Some(std::time::Duration::from_secs(2)),
            AIError::ProviderUnavailable { .. } => Some(std::time::Duration::from_secs(5)),
            AIError::InternalError { .. } => Some(std::time::Duration::from_secs(3)),
            _ => None,
        }
    }

    /// Check if this error indicates provider needs switching
    pub fn should_switch_provider(&self) -> bool {
        matches!(
            self,
            AIError::AuthenticationFailure { .. }
                | AIError::InsufficientQuota { .. }
                | AIError::ModelNotFound { .. }
                | AIError::FeatureNotSupported { .. }
        )
    }
}

/// Convert common error types to AIError
impl From<std::io::Error> for AIError {
    fn from(err: std::io::Error) -> Self {
        AIError::network_error(err.to_string())
    }
}

impl From<reqwest::Error> for AIError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            AIError::timeout(std::time::Duration::from_secs(30))
        } else if err.is_connect() {
            AIError::provider_unavailable(err.to_string())
        } else {
            AIError::network_error(err.to_string())
        }
    }
}

impl From<serde_json::Error> for AIError {
    fn from(err: serde_json::Error) -> Self {
        AIError::invalid_response(format!("JSON parsing error: {}", err))
    }
}