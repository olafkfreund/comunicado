//! Core plugin traits and types
//!
//! This module defines the fundamental interfaces that all plugins must implement,
//! as well as common data structures used throughout the plugin system.

use serde::{Deserialize, Serialize};
use std::any::Any;
use std::fmt;
use thiserror::Error;
use uuid::Uuid;

/// Result type for plugin operations
pub type PluginResult<T> = Result<T, PluginError>;

/// Plugin-specific error types
#[derive(Debug, Error)]
pub enum PluginError {
    #[error("Plugin initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Plugin execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Plugin configuration error: {0}")]
    ConfigurationError(String),

    #[error("Plugin dependency not found: {0}")]
    DependencyNotFound(String),

    #[error("Plugin version incompatible: required {required}, found {found}")]
    VersionIncompatible { required: String, found: String },

    #[error("Plugin already loaded: {0}")]
    AlreadyLoaded(String),

    #[error("Plugin not found: {0}")]
    NotFound(String),

    #[error("Plugin disabled: {0}")]
    Disabled(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Unknown plugin error: {0}")]
    Unknown(String),
}

/// Plugin type categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PluginType {
    /// Plugins that process email messages
    Email,
    /// Plugins that extend UI functionality
    UI,
    /// Plugins that enhance calendar features
    Calendar,
    /// Plugins that handle notifications
    Notification,
    /// Plugins that extend search capabilities
    Search,
    /// Plugins that add import/export formats
    ImportExport,
    /// Plugins that provide system integration
    System,
    /// Plugins that add AI/ML capabilities
    AI,
    /// Generic utility plugins
    Utility,
}

impl PluginType {
    /// Get a human-readable description of the plugin type
    pub fn description(&self) -> &'static str {
        match self {
            PluginType::Email => "Email processing and filtering",
            PluginType::UI => "User interface extensions",
            PluginType::Calendar => "Calendar and scheduling enhancements",
            PluginType::Notification => "Notification handling and routing",
            PluginType::Search => "Search and indexing capabilities",
            PluginType::ImportExport => "Data import and export formats",
            PluginType::System => "System integration and automation",
            PluginType::AI => "AI and machine learning features",
            PluginType::Utility => "General utility and helper functions",
        }
    }

    /// Get the icon associated with this plugin type
    pub fn icon(&self) -> &'static str {
        match self {
            PluginType::Email => "📧",
            PluginType::UI => "🎨",
            PluginType::Calendar => "📅",
            PluginType::Notification => "🔔",
            PluginType::Search => "🔍",
            PluginType::ImportExport => "📁",
            PluginType::System => "⚙️",
            PluginType::AI => "🤖",
            PluginType::Utility => "🔧",
        }
    }
}

/// Plugin status tracking
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginStatus {
    /// Plugin is not loaded
    Unloaded,
    /// Plugin is loaded but not initialized
    Loaded,
    /// Plugin is initialized and ready
    Initialized,
    /// Plugin is currently running
    Running,
    /// Plugin is paused/suspended
    Paused,
    /// Plugin encountered an error
    Error(String),
    /// Plugin is disabled by user
    Disabled,
}

impl fmt::Display for PluginStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginStatus::Unloaded => write!(f, "Unloaded"),
            PluginStatus::Loaded => write!(f, "Loaded"),
            PluginStatus::Initialized => write!(f, "Initialized"),
            PluginStatus::Running => write!(f, "Running"),
            PluginStatus::Paused => write!(f, "Paused"),
            PluginStatus::Error(err) => write!(f, "Error: {}", err),
            PluginStatus::Disabled => write!(f, "Disabled"),
        }
    }
}

/// Plugin metadata and information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    /// Unique plugin identifier
    pub id: Uuid,
    /// Human-readable plugin name
    pub name: String,
    /// Plugin version (semantic versioning)
    pub version: String,
    /// Plugin description
    pub description: String,
    /// Plugin author information
    pub author: String,
    /// Plugin author email
    pub author_email: Option<String>,
    /// Plugin homepage URL
    pub homepage: Option<String>,
    /// Plugin repository URL
    pub repository: Option<String>,
    /// Plugin license
    pub license: Option<String>,
    /// Plugin type category
    pub plugin_type: PluginType,
    /// Minimum Comunicado version required
    pub min_comunicado_version: String,
    /// Maximum Comunicado version supported
    pub max_comunicado_version: Option<String>,
    /// Plugin dependencies
    pub dependencies: Vec<PluginDependency>,
    /// Plugin configuration schema
    pub config_schema: Option<serde_json::Value>,
    /// Plugin capabilities/features
    pub capabilities: Vec<String>,
    /// Plugin tags for categorization
    pub tags: Vec<String>,
}

impl PluginInfo {
    /// Create a new PluginInfo with minimal required fields
    pub fn new(
        name: String,
        version: String,
        description: String,
        author: String,
        plugin_type: PluginType,
        min_comunicado_version: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            version,
            description,
            author,
            author_email: None,
            homepage: None,
            repository: None,
            license: None,
            plugin_type,
            min_comunicado_version,
            max_comunicado_version: None,
            dependencies: Vec::new(),
            config_schema: None,
            capabilities: Vec::new(),
            tags: Vec::new(),
        }
    }

    /// Check if this plugin is compatible with the given Comunicado version
    pub fn is_compatible(&self, comunicado_version: &str) -> bool {
        // In a real implementation, this would use proper semantic version comparison
        // For now, we'll do a simple string comparison
        if comunicado_version < self.min_comunicado_version.as_str() {
            return false;
        }

        if let Some(ref max_version) = self.max_comunicado_version {
            if comunicado_version > max_version.as_str() {
                return false;
            }
        }

        true
    }

    /// Get a display string for the plugin
    pub fn display_name(&self) -> String {
        format!("{} v{}", self.name, self.version)
    }
}

/// Plugin dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    /// Name of the required plugin
    pub name: String,
    /// Required version (semantic versioning)
    pub version: String,
    /// Whether this dependency is optional
    pub optional: bool,
}

/// Plugin configuration management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Plugin identifier
    pub plugin_id: Uuid,
    /// Whether the plugin is enabled
    pub enabled: bool,
    /// Plugin-specific configuration data
    pub config: serde_json::Value,
    /// User-defined plugin priority
    pub priority: i32,
    /// Plugin execution settings
    pub settings: PluginSettings,
}

impl PluginConfig {
    /// Create a new plugin configuration with default values
    pub fn new(plugin_id: Uuid) -> Self {
        Self {
            plugin_id,
            enabled: true,
            config: serde_json::Value::Object(serde_json::Map::new()),
            priority: 0,
            settings: PluginSettings::default(),
        }
    }

    /// Get a configuration value by key
    pub fn get_config<T>(&self, key: &str) -> PluginResult<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let value = self.config
            .get(key)
            .ok_or_else(|| PluginError::ConfigurationError(format!("Key '{}' not found", key)))?;
        
        serde_json::from_value(value.clone())
            .map_err(|e| PluginError::ConfigurationError(format!("Failed to deserialize key '{}': {}", key, e)))
    }

    /// Set a configuration value
    pub fn set_config<T>(&mut self, key: &str, value: T) -> PluginResult<()>
    where
        T: Serialize,
    {
        let json_value = serde_json::to_value(value)
            .map_err(|e| PluginError::ConfigurationError(format!("Failed to serialize value: {}", e)))?;
        
        if let serde_json::Value::Object(ref mut map) = self.config {
            map.insert(key.to_string(), json_value);
        } else {
            return Err(PluginError::ConfigurationError("Config is not an object".to_string()));
        }

        Ok(())
    }
}

/// Plugin execution settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSettings {
    /// Maximum execution time in milliseconds
    pub max_execution_time: u64,
    /// Maximum memory usage in bytes
    pub max_memory_usage: u64,
    /// Whether to run plugin in sandbox
    pub sandboxed: bool,
    /// Plugin log level
    pub log_level: String,
    /// Whether to auto-restart on failure
    pub auto_restart: bool,
    /// Maximum restart attempts
    pub max_restart_attempts: u32,
}

impl Default for PluginSettings {
    fn default() -> Self {
        Self {
            max_execution_time: 5000, // 5 seconds
            max_memory_usage: 100 * 1024 * 1024, // 100MB
            sandboxed: true,
            log_level: "info".to_string(),
            auto_restart: false,
            max_restart_attempts: 3,
        }
    }
}

/// Core plugin trait that all plugins must implement
pub trait Plugin: Send + Sync {
    /// Get plugin metadata and information
    fn info(&self) -> PluginInfo;

    /// Initialize the plugin
    /// This is called once when the plugin is first loaded
    fn initialize(&mut self, _config: &PluginConfig) -> PluginResult<()> {
        // Default implementation does nothing
        Ok(())
    }

    /// Start the plugin
    /// This is called when the plugin should begin active operation
    fn start(&mut self) -> PluginResult<()> {
        // Default implementation does nothing
        Ok(())
    }

    /// Stop the plugin
    /// This is called when the plugin should cease operation
    fn stop(&mut self) -> PluginResult<()> {
        // Default implementation does nothing
        Ok(())
    }

    /// Pause the plugin
    /// This is called when the plugin should temporarily suspend operation
    fn pause(&mut self) -> PluginResult<()> {
        // Default implementation does nothing
        Ok(())
    }

    /// Resume the plugin
    /// This is called when the plugin should resume from a paused state
    fn resume(&mut self) -> PluginResult<()> {
        // Default implementation does nothing
        Ok(())
    }

    /// Get plugin configuration schema
    /// Returns a JSON schema describing the plugin's configuration options
    fn config_schema(&self) -> Option<serde_json::Value> {
        None
    }

    /// Validate plugin configuration
    /// This is called before applying new configuration
    fn validate_config(&self, _config: &serde_json::Value) -> PluginResult<()> {
        // Default implementation accepts any configuration
        Ok(())
    }

    /// Update plugin configuration
    /// This is called when the plugin configuration changes
    fn update_config(&mut self, _config: &PluginConfig) -> PluginResult<()> {
        // Default implementation does nothing
        Ok(())
    }

    /// Get plugin health status
    /// Returns information about the plugin's current health and status
    fn health_check(&self) -> PluginResult<PluginHealthStatus> {
        Ok(PluginHealthStatus::Healthy)
    }

    /// Get plugin as Any trait object for downcasting
    fn as_any(&self) -> &dyn Any;

    /// Get mutable plugin as Any trait object for downcasting
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Plugin health status information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginHealthStatus {
    /// Plugin is healthy and functioning normally
    Healthy,
    /// Plugin is degraded but still functional
    Degraded(String),
    /// Plugin is unhealthy and may not function correctly
    Unhealthy(String),
    /// Plugin health status is unknown
    Unknown,
}

impl fmt::Display for PluginHealthStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginHealthStatus::Healthy => write!(f, "Healthy"),
            PluginHealthStatus::Degraded(reason) => write!(f, "Degraded: {}", reason),
            PluginHealthStatus::Unhealthy(reason) => write!(f, "Unhealthy: {}", reason),
            PluginHealthStatus::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Plugin execution context
/// Provides plugins with access to Comunicado's core functionality
#[derive(Debug)]
pub struct PluginContext {
    /// Plugin identifier
    pub plugin_id: Uuid,
    /// Current plugin configuration
    pub config: PluginConfig,
    /// Application version
    pub app_version: String,
    /// Plugin execution environment
    pub environment: PluginEnvironment,
}

/// Plugin execution environment information
#[derive(Debug, Clone)]
pub struct PluginEnvironment {
    /// Plugin data directory
    pub data_dir: std::path::PathBuf,
    /// Plugin log directory
    pub log_dir: std::path::PathBuf,
    /// Plugin cache directory
    pub cache_dir: std::path::PathBuf,
    /// Plugin temporary directory
    pub temp_dir: std::path::PathBuf,
    /// Environment variables available to the plugin
    pub env_vars: std::collections::HashMap<String, String>,
}

impl PluginEnvironment {
    /// Create a new plugin environment
    pub fn new(base_dir: std::path::PathBuf, plugin_id: Uuid) -> std::io::Result<Self> {
        let plugin_dir = base_dir.join("plugins").join(plugin_id.to_string());
        
        let data_dir = plugin_dir.join("data");
        let log_dir = plugin_dir.join("logs");
        let cache_dir = plugin_dir.join("cache");
        let temp_dir = plugin_dir.join("temp");

        // Create directories if they don't exist
        std::fs::create_dir_all(&data_dir)?;
        std::fs::create_dir_all(&log_dir)?;
        std::fs::create_dir_all(&cache_dir)?;
        std::fs::create_dir_all(&temp_dir)?;

        Ok(Self {
            data_dir,
            log_dir,
            cache_dir,
            temp_dir,
            env_vars: std::env::vars().collect(),
        })
    }
}