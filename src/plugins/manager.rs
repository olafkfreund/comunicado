//! Plugin manager for loading, managing, and coordinating plugins
//!
//! The PluginManager is responsible for the entire plugin lifecycle:
//! - Discovery and loading of plugins
//! - Configuration management
//! - Execution coordination
//! - Health monitoring
//! - Dependency resolution

use super::core::{PluginConfig, PluginError, PluginInfo, PluginResult, PluginStatus, PluginType, PluginContext, PluginEnvironment};
use super::registry::PluginRegistry;
use super::loader::PluginLoader;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

/// Plugin manager for coordinating all plugin operations
pub struct PluginManager {
    /// Plugin registry for tracking loaded plugins
    registry: Arc<RwLock<PluginRegistry>>,
    /// Plugin loader for dynamic loading
    loader: PluginLoader,
    /// Plugin configurations
    configs: Arc<RwLock<HashMap<Uuid, PluginConfig>>>,
    /// Plugin execution contexts
    contexts: Arc<RwLock<HashMap<Uuid, PluginContext>>>,
    /// Plugin health monitoring
    health_monitor: Arc<Mutex<PluginHealthMonitor>>,
    /// Plugin directories to scan
    plugin_directories: Vec<PathBuf>,
    /// Application version for compatibility checking
    app_version: String,
    /// Base directory for plugin data
    base_data_dir: PathBuf,
    /// Plugin execution settings
    execution_settings: PluginExecutionSettings,
}

/// Plugin execution settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginExecutionSettings {
    /// Maximum number of plugins to run concurrently
    pub max_concurrent_plugins: usize,
    /// Default plugin execution timeout
    pub default_timeout: Duration,
    /// Whether to enable plugin sandboxing
    pub enable_sandboxing: bool,
    /// Whether to auto-load plugins on startup
    pub auto_load_on_startup: bool,
    /// Plugin discovery scan interval
    pub scan_interval: Duration,
}

impl Default for PluginExecutionSettings {
    fn default() -> Self {
        Self {
            max_concurrent_plugins: 10,
            default_timeout: Duration::from_secs(30),
            enable_sandboxing: true,
            auto_load_on_startup: true,
            scan_interval: Duration::from_secs(300), // 5 minutes
        }
    }
}

/// Plugin health monitoring system
#[allow(dead_code)]
struct PluginHealthMonitor {
    /// Plugin health status tracking
    health_status: HashMap<Uuid, PluginHealthStatus>,
    /// Plugin performance metrics
    metrics: HashMap<Uuid, PluginMetrics>,
    /// Last health check times
    last_health_check: HashMap<Uuid, Instant>,
    /// Health check interval
    health_check_interval: Duration,
}

/// Plugin health status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginHealthStatus {
    Healthy,
    Degraded(String),
    Unhealthy(String),
    Unknown,
}

/// Plugin performance metrics
#[derive(Debug, Clone)]
pub struct PluginMetrics {
    /// Total execution time
    #[allow(dead_code)]
    total_execution_time: Duration,
    /// Number of successful executions
    #[allow(dead_code)]
    successful_executions: u64,
    /// Number of failed executions
    #[allow(dead_code)]
    failed_executions: u64,
    /// Average execution time
    #[allow(dead_code)]
    average_execution_time: Duration,
    /// Memory usage statistics
    #[allow(dead_code)]
    memory_usage: MemoryUsage,
    /// Last execution time
    #[allow(dead_code)]
    last_execution: Option<Instant>,
}

/// Memory usage statistics
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct MemoryUsage {
    /// Current memory usage in bytes
    current: u64,
    /// Peak memory usage in bytes
    peak: u64,
    /// Average memory usage in bytes
    average: u64,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new(
        plugin_directories: Vec<PathBuf>,
        app_version: String,
        base_data_dir: PathBuf,
    ) -> PluginResult<Self> {
        let registry = Arc::new(RwLock::new(PluginRegistry::new()));
        let loader = PluginLoader::new();
        let configs = Arc::new(RwLock::new(HashMap::new()));
        let contexts = Arc::new(RwLock::new(HashMap::new()));
        let health_monitor = Arc::new(Mutex::new(PluginHealthMonitor::new()));

        Ok(Self {
            registry,
            loader,
            configs,
            contexts,
            health_monitor,
            plugin_directories,
            app_version,
            base_data_dir,
            execution_settings: PluginExecutionSettings::default(),
        })
    }

    /// Initialize the plugin manager
    pub async fn initialize(&mut self) -> PluginResult<()> {
        // Create base directories
        tokio::fs::create_dir_all(&self.base_data_dir).await
            .map_err(|e| PluginError::Io(e))?;

        // Load plugin configurations
        self.load_configurations().await?;

        // Scan for available plugins
        self.scan_plugins().await?;

        // Auto-load plugins if enabled
        if self.execution_settings.auto_load_on_startup {
            self.auto_load_plugins().await?;
        }

        // Start health monitoring
        self.start_health_monitoring().await?;

        Ok(())
    }

    /// Scan for available plugins in configured directories
    pub async fn scan_plugins(&mut self) -> PluginResult<Vec<PluginInfo>> {
        let mut discovered_plugins = Vec::new();

        for plugin_dir in &self.plugin_directories {
            if !plugin_dir.exists() {
                continue;
            }

            let mut entries = tokio::fs::read_dir(plugin_dir).await
                .map_err(|e| PluginError::Io(e))?;

            while let Some(entry) = entries.next_entry().await
                .map_err(|e| PluginError::Io(e))? {
                
                if let Ok(plugin_info) = self.scan_plugin_directory(&entry.path()).await {
                    discovered_plugins.push(plugin_info);
                }
            }
        }

        Ok(discovered_plugins)
    }

    /// Scan a specific directory for plugin metadata
    async fn scan_plugin_directory(&self, path: &std::path::Path) -> PluginResult<PluginInfo> {
        let manifest_path = path.join("plugin.json");
        
        if !manifest_path.exists() {
            return Err(PluginError::NotFound("plugin.json not found".to_string()));
        }

        let manifest_content = tokio::fs::read_to_string(&manifest_path).await
            .map_err(|e| PluginError::Io(e))?;

        let plugin_info: PluginInfo = serde_json::from_str(&manifest_content)
            .map_err(|e| PluginError::Serialization(e))?;

        // Validate plugin compatibility
        if !plugin_info.is_compatible(&self.app_version) {
            return Err(PluginError::VersionIncompatible {
                required: plugin_info.min_comunicado_version.clone(),
                found: self.app_version.clone(),
            });
        }

        Ok(plugin_info)
    }

    /// Load a specific plugin by ID
    pub async fn load_plugin(&mut self, plugin_id: Uuid) -> PluginResult<()> {
        // Check if plugin is already loaded
        {
            let registry = self.registry.read().unwrap();
            if registry.is_loaded(&plugin_id) {
                return Err(PluginError::AlreadyLoaded(plugin_id.to_string()));
            }
        }

        // Find plugin in discovered plugins
        let plugin_info = self.find_plugin_info(plugin_id)?;

        // Load plugin using the loader
        let plugin = self.loader.load_plugin(&plugin_info).await?;

        // Create plugin environment
        let environment = PluginEnvironment::new(self.base_data_dir.clone(), plugin_id)
            .map_err(|e| PluginError::Io(e))?;

        // Get or create plugin configuration
        let config = self.get_or_create_config(plugin_id);

        // Create plugin context
        let context = PluginContext {
            plugin_id,
            config: config.clone(),
            app_version: self.app_version.clone(),
            environment,
        };

        // Register plugin
        {
            let mut registry = self.registry.write().unwrap();
            registry.register_plugin(plugin, plugin_info.clone())?;
        }

        // Store configuration and context
        {
            let mut configs = self.configs.write().unwrap();
            configs.insert(plugin_id, config);
        }

        {
            let mut contexts = self.contexts.write().unwrap();
            contexts.insert(plugin_id, context);
        }

        // Initialize plugin
        self.initialize_plugin(plugin_id).await?;

        Ok(())
    }

    /// Initialize a loaded plugin
    async fn initialize_plugin(&mut self, plugin_id: Uuid) -> PluginResult<()> {
        let _config = {
            let configs = self.configs.read().unwrap();
            configs.get(&plugin_id).cloned()
                .ok_or_else(|| PluginError::NotFound("Plugin config not found".to_string()))?
        };

        {
            let mut registry = self.registry.write().unwrap();
            if registry.get_plugin_mut(&plugin_id) {
                // Plugin initialization would happen here
                // For now, just set status
                registry.set_plugin_status(plugin_id, PluginStatus::Initialized);
            }
        }

        Ok(())
    }

    /// Start a plugin
    pub async fn start_plugin(&mut self, plugin_id: Uuid) -> PluginResult<()> {
        {
            let mut registry = self.registry.write().unwrap();
            if registry.get_plugin_mut(&plugin_id) {
                // Plugin start would happen here
                registry.set_plugin_status(plugin_id, PluginStatus::Running);
            } else {
                return Err(PluginError::NotFound(plugin_id.to_string()));
            }
        }

        Ok(())
    }

    /// Stop a plugin
    pub async fn stop_plugin(&mut self, plugin_id: Uuid) -> PluginResult<()> {
        {
            let mut registry = self.registry.write().unwrap();
            if registry.get_plugin_mut(&plugin_id) {
                // Plugin stop would happen here
                registry.set_plugin_status(plugin_id, PluginStatus::Loaded);
            } else {
                return Err(PluginError::NotFound(plugin_id.to_string()));
            }
        }

        Ok(())
    }

    /// Pause a plugin
    pub async fn pause_plugin(&mut self, plugin_id: Uuid) -> PluginResult<()> {
        {
            let mut registry = self.registry.write().unwrap();
            if registry.get_plugin_mut(&plugin_id) {
                // Plugin pause would happen here
                registry.set_plugin_status(plugin_id, PluginStatus::Paused);
            } else {
                return Err(PluginError::NotFound(plugin_id.to_string()));
            }
        }

        Ok(())
    }

    /// Resume a plugin
    pub async fn resume_plugin(&mut self, plugin_id: Uuid) -> PluginResult<()> {
        {
            let mut registry = self.registry.write().unwrap();
            if registry.get_plugin_mut(&plugin_id) {
                // Plugin resume would happen here
                registry.set_plugin_status(plugin_id, PluginStatus::Running);
            } else {
                return Err(PluginError::NotFound(plugin_id.to_string()));
            }
        }

        Ok(())
    }

    /// Unload a plugin
    pub async fn unload_plugin(&mut self, plugin_id: Uuid) -> PluginResult<()> {
        // Stop plugin if running
        if self.is_plugin_running(plugin_id) {
            self.stop_plugin(plugin_id).await?;
        }

        // Remove from registry
        {
            let mut registry = self.registry.write().unwrap();
            registry.unregister_plugin(&plugin_id)?;
        }

        // Remove configuration and context
        {
            let mut configs = self.configs.write().unwrap();
            configs.remove(&plugin_id);
        }

        {
            let mut contexts = self.contexts.write().unwrap();
            contexts.remove(&plugin_id);
        }

        // Remove from health monitoring
        {
            let mut health_monitor = self.health_monitor.lock().await;
            health_monitor.remove_plugin(plugin_id);
        }

        Ok(())
    }

    /// Get list of all loaded plugins
    pub fn get_loaded_plugins(&self) -> Vec<PluginInfo> {
        let registry = self.registry.read().unwrap();
        registry.get_all_plugins()
    }

    /// Get plugins by type
    pub fn get_plugins_by_type(&self, plugin_type: PluginType) -> Vec<PluginInfo> {
        let registry = self.registry.read().unwrap();
        registry.get_plugins_by_type(plugin_type)
    }

    /// Check if a plugin is loaded
    pub fn is_plugin_loaded(&self, plugin_id: Uuid) -> bool {
        let registry = self.registry.read().unwrap();
        registry.is_loaded(&plugin_id)
    }

    /// Check if a plugin is running
    pub fn is_plugin_running(&self, plugin_id: Uuid) -> bool {
        let registry = self.registry.read().unwrap();
        match registry.get_plugin_status(&plugin_id) {
            Some(PluginStatus::Running) => true,
            _ => false,
        }
    }

    /// Update plugin configuration
    pub async fn update_plugin_config(&mut self, plugin_id: Uuid, config: PluginConfig) -> PluginResult<()> {
        // Validate configuration with plugin
        {
            let registry = self.registry.read().unwrap();
            if let Some(_plugin) = registry.get_plugin(&plugin_id) {
                // Plugin config validation would happen here
            }
        }

        // Update stored configuration
        {
            let mut configs = self.configs.write().unwrap();
            configs.insert(plugin_id, config.clone());
        }

        // Update plugin with new configuration
        {
            let mut registry = self.registry.write().unwrap();
            if registry.get_plugin_mut(&plugin_id) {
                // Plugin config update would happen here
            }
        }

        // Save configuration to disk
        self.save_configuration(plugin_id, &config).await?;

        Ok(())
    }

    /// Get plugin configuration
    pub fn get_plugin_config(&self, plugin_id: Uuid) -> Option<PluginConfig> {
        let configs = self.configs.read().unwrap();
        configs.get(&plugin_id).cloned()
    }

    /// Get plugin health status
    pub async fn get_plugin_health(&self, plugin_id: Uuid) -> Option<PluginHealthStatus> {
        let health_monitor = self.health_monitor.lock().await;
        health_monitor.get_health_status(plugin_id)
    }

    /// Get plugin metrics
    pub async fn get_plugin_metrics(&self, plugin_id: Uuid) -> Option<PluginMetrics> {
        let health_monitor = self.health_monitor.lock().await;
        health_monitor.get_metrics(plugin_id)
    }

    /// Auto-load plugins based on configuration
    async fn auto_load_plugins(&mut self) -> PluginResult<()> {
        let available_plugins = self.scan_plugins().await?;
        
        for plugin_info in available_plugins {
            let config = self.get_or_create_config(plugin_info.id);
            
            if config.enabled {
                if let Err(e) = self.load_plugin(plugin_info.id).await {
                    eprintln!("Failed to auto-load plugin {}: {}", plugin_info.name, e);
                }
            }
        }

        Ok(())
    }

    /// Start health monitoring for all plugins
    async fn start_health_monitoring(&self) -> PluginResult<()> {
        // In a real implementation, this would start a background task
        // that periodically checks plugin health
        Ok(())
    }

    /// Load plugin configurations from disk
    async fn load_configurations(&mut self) -> PluginResult<()> {
        let config_dir = self.base_data_dir.join("configs");
        
        if !config_dir.exists() {
            tokio::fs::create_dir_all(&config_dir).await
                .map_err(|e| PluginError::Io(e))?;
            return Ok(());
        }

        let mut entries = tokio::fs::read_dir(&config_dir).await
            .map_err(|e| PluginError::Io(e))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| PluginError::Io(e))? {
            
            if let Some(extension) = entry.path().extension() {
                if extension == "json" {
                    if let Ok(config) = self.load_configuration(&entry.path()).await {
                        let mut configs = self.configs.write().unwrap();
                        configs.insert(config.plugin_id, config);
                    }
                }
            }
        }

        Ok(())
    }

    /// Load a single plugin configuration
    async fn load_configuration(&self, path: &std::path::Path) -> PluginResult<PluginConfig> {
        let content = tokio::fs::read_to_string(path).await
            .map_err(|e| PluginError::Io(e))?;

        let config: PluginConfig = serde_json::from_str(&content)
            .map_err(|e| PluginError::Serialization(e))?;

        Ok(config)
    }

    /// Save plugin configuration to disk
    async fn save_configuration(&self, plugin_id: Uuid, config: &PluginConfig) -> PluginResult<()> {
        let config_dir = self.base_data_dir.join("configs");
        let config_file = config_dir.join(format!("{}.json", plugin_id));

        let content = serde_json::to_string_pretty(config)
            .map_err(|e| PluginError::Serialization(e))?;

        tokio::fs::write(&config_file, content).await
            .map_err(|e| PluginError::Io(e))?;

        Ok(())
    }

    /// Get or create a plugin configuration
    fn get_or_create_config(&self, plugin_id: Uuid) -> PluginConfig {
        let configs = self.configs.read().unwrap();
        configs.get(&plugin_id).cloned()
            .unwrap_or_else(|| PluginConfig::new(plugin_id))
    }

    /// Find plugin info by ID
    fn find_plugin_info(&self, plugin_id: Uuid) -> PluginResult<PluginInfo> {
        let registry = self.registry.read().unwrap();
        registry.get_plugin_info(&plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.to_string()))
    }
}

impl PluginHealthMonitor {
    fn new() -> Self {
        Self {
            health_status: HashMap::new(),
            metrics: HashMap::new(),
            last_health_check: HashMap::new(),
            health_check_interval: Duration::from_secs(60),
        }
    }

    fn get_health_status(&self, plugin_id: Uuid) -> Option<PluginHealthStatus> {
        self.health_status.get(&plugin_id).cloned()
    }

    fn get_metrics(&self, plugin_id: Uuid) -> Option<PluginMetrics> {
        self.metrics.get(&plugin_id).cloned()
    }

    fn remove_plugin(&mut self, plugin_id: Uuid) {
        self.health_status.remove(&plugin_id);
        self.metrics.remove(&plugin_id);
        self.last_health_check.remove(&plugin_id);
    }
}