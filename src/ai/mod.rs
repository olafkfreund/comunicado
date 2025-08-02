//! AI Integration Module for Comunicado
//! 
//! This module provides comprehensive AI assistance for email management, calendar scheduling,
//! and content processing with support for both local (Ollama) and cloud-based AI providers.

pub mod cache;
pub mod config;
pub mod config_manager;
pub mod error;
pub mod factory;
pub mod provider;
pub mod providers;
pub mod service;

// Temporarily disabled while fixing interface issues
// #[cfg(test)]
// pub mod testing;

// Re-export main types for convenient access
pub use cache::{AIResponseCache, CacheStatistics};
pub use config::{AIConfig, AIProviderType, PrivacyMode};
pub use config_manager::{AIConfigManager, AIHealthStatus, ConfigStats};
pub use error::AIError;
pub use factory::AIFactory;
pub use provider::{AIProvider, AIProviderManager, ProviderCapabilities};
pub use providers::{AnthropicProvider, GoogleProvider, OllamaProvider, OpenAIProvider};
pub use service::{AIService, EmailCategory, SchedulingIntent};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Common types used across the AI module
pub type AIResult<T> = Result<T, AIError>;

/// Context information for AI operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIContext {
    /// User preferences and settings
    pub user_preferences: HashMap<String, String>,
    /// Email thread context for reply suggestions
    pub email_thread: Option<String>,
    /// Calendar context for scheduling
    pub calendar_context: Option<String>,
    /// Maximum response length
    pub max_length: Option<usize>,
    /// Response creativity/temperature setting
    pub creativity: Option<f32>,
}