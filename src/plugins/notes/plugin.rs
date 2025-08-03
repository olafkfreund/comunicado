//! Notes plugin implementation
//! 
//! This module implements the main plugin interface for the notes system,
//! integrating with Comunicado's plugin architecture.

use crate::plugins::core::{
    Plugin, PluginConfig, PluginError, PluginInfo, PluginResult, PluginType, PluginHealthStatus,
};
use super::types::{NotesConfig, Note, NoteSearchResult};
use super::manager::NoteManager;
use super::storage::NoteStorage;
use super::indexer::NoteIndexer;
use super::watcher::FileWatcher;

use std::any::Any;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Main notes plugin implementation
pub struct NotesPlugin {
    /// Plugin metadata
    info: PluginInfo,
    /// Plugin configuration
    config: NotesConfig,
    /// Note manager for core operations
    manager: Arc<RwLock<Option<NoteManager>>>,
    /// Current plugin status
    status: PluginHealthStatus,
}

impl NotesPlugin {
    /// Create a new notes plugin instance
    pub fn new() -> Self {
        let info = PluginInfo::new(
            "Comunicado Notes".to_string(),
            "1.0.0".to_string(),
            "Advanced note-taking with markdown support, wiki linking, and cross-app integration".to_string(),
            "Comunicado Team".to_string(),
            PluginType::Utility,
            "0.1.0".to_string(), // Min Comunicado version
        );

        Self {
            info,
            config: NotesConfig::default(),
            manager: Arc::new(RwLock::new(None)),
            status: PluginHealthStatus::Healthy,
        }
    }

    /// Create a new notes plugin with custom configuration
    pub fn with_config(config: NotesConfig) -> Self {
        let mut plugin = Self::new();
        plugin.config = config;
        plugin
    }

    /// Get a reference to the note manager
    pub async fn manager(&self) -> Arc<RwLock<Option<NoteManager>>> {
        self.manager.clone()
    }

    /// Create a new note
    pub async fn create_note(&self, title: String, content: String) -> PluginResult<Note> {
        let manager = self.manager.read().await;
        if let Some(ref mgr) = *manager {
            mgr.create_note(title, content).await
                .map_err(|e| PluginError::ExecutionFailed(e.to_string()))
        } else {
            Err(PluginError::ExecutionFailed("Plugin not initialized".to_string()))
        }
    }

    /// Search for notes
    pub async fn search_notes(&self, query: &str) -> PluginResult<Vec<NoteSearchResult>> {
        let manager = self.manager.read().await;
        if let Some(ref mgr) = *manager {
            mgr.search_notes(query).await
                .map_err(|e| PluginError::ExecutionFailed(e.to_string()))
        } else {
            Err(PluginError::ExecutionFailed("Plugin not initialized".to_string()))
        }
    }

    /// Get a note by ID
    pub async fn get_note(&self, note_id: &str) -> PluginResult<Option<Note>> {
        let manager = self.manager.read().await;
        if let Some(ref mgr) = *manager {
            mgr.get_note(note_id).await
                .map_err(|e| PluginError::ExecutionFailed(e.to_string()))
        } else {
            Err(PluginError::ExecutionFailed("Plugin not initialized".to_string()))
        }
    }

    /// Update plugin configuration
    pub async fn update_config(&mut self, new_config: NotesConfig) -> PluginResult<()> {
        self.config = new_config;
        
        // Reinitialize with new config if already running
        let mut manager = self.manager.write().await;
        if manager.is_some() {
            *manager = None;
            drop(manager);
            self.initialize_manager().await?;
        }
        
        Ok(())
    }

    /// Initialize the note manager
    async fn initialize_manager(&self) -> PluginResult<()> {
        let storage = NoteStorage::new(&self.config.default_directory)
            .await
            .map_err(|e| PluginError::InitializationFailed(format!("Failed to initialize storage: {}", e)))?;

        let indexer = NoteIndexer::new(Arc::new(storage.clone()))
            .await
            .map_err(|e| PluginError::InitializationFailed(format!("Failed to initialize indexer: {}", e)))?;

        let watcher = FileWatcher::new()
            .map_err(|e| PluginError::InitializationFailed(format!("Failed to initialize file watcher: {}", e)))?;

        let manager = NoteManager::new(storage, indexer, watcher, self.config.clone())
            .await
            .map_err(|e| PluginError::InitializationFailed(format!("Failed to initialize manager: {}", e)))?;

        let mut manager_lock = self.manager.write().await;
        *manager_lock = Some(manager);

        Ok(())
    }
}

impl Plugin for NotesPlugin {
    fn info(&self) -> PluginInfo {
        self.info.clone()
    }

    fn initialize(&mut self, config: &PluginConfig) -> PluginResult<()> {
        // Extract notes-specific configuration if provided
        if let Ok(notes_config) = config.get_config::<NotesConfig>("notes") {
            self.config = notes_config;
        }

        // Initialize the plugin asynchronously
        // Note: In a real implementation, we'd need to handle async initialization properly
        // This is a simplified version for the plugin interface
        
        Ok(())
    }

    fn start(&mut self) -> PluginResult<()> {
        // Start file watching and background tasks
        // This would be implemented with proper async runtime handling
        Ok(())
    }

    fn stop(&mut self) -> PluginResult<()> {
        // Stop all background tasks and clean up resources
        // Reset the manager
        let manager = self.manager.clone();
        tokio::spawn(async move {
            let mut mgr = manager.write().await;
            *mgr = None;
        });
        
        Ok(())
    }

    fn pause(&mut self) -> PluginResult<()> {
        // Pause file watching but keep data in memory
        self.status = PluginHealthStatus::Degraded("Paused".to_string());
        Ok(())
    }

    fn resume(&mut self) -> PluginResult<()> {
        // Resume file watching
        self.status = PluginHealthStatus::Healthy;
        Ok(())
    }

    fn config_schema(&self) -> Option<serde_json::Value> {
        // Return JSON schema for notes configuration
        Some(serde_json::json!({
            "type": "object",
            "properties": {
                "default_directory": {
                    "type": "string",
                    "description": "Default directory for notes"
                },
                "max_search_results": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 1000,
                    "description": "Maximum number of search results"
                },
                "auto_index": {
                    "type": "boolean",
                    "description": "Whether to automatically index notes"
                },
                "vim_mode": {
                    "type": "boolean",
                    "description": "Enable vim-style keybindings"
                }
            },
            "required": ["default_directory"]
        }))
    }

    fn validate_config(&self, config: &serde_json::Value) -> PluginResult<()> {
        // Validate the configuration against the schema
        if let Some(default_dir) = config.get("default_directory") {
            if !default_dir.is_string() {
                return Err(PluginError::ConfigurationError(
                    "default_directory must be a string".to_string()
                ));
            }
        } else {
            return Err(PluginError::ConfigurationError(
                "default_directory is required".to_string()
            ));
        }

        if let Some(max_results) = config.get("max_search_results") {
            if let Some(n) = max_results.as_u64() {
                if n == 0 || n > 1000 {
                    return Err(PluginError::ConfigurationError(
                        "max_search_results must be between 1 and 1000".to_string()
                    ));
                }
            } else {
                return Err(PluginError::ConfigurationError(
                    "max_search_results must be a number".to_string()
                ));
            }
        }

        Ok(())
    }

    fn update_config(&mut self, config: &PluginConfig) -> PluginResult<()> {
        // Update configuration and restart if necessary
        if let Ok(notes_config) = config.get_config::<NotesConfig>("notes") {
            // Use async runtime to update config
            let manager = self.manager.clone();
            let _new_config = notes_config.clone();
            
            tokio::spawn(async move {
                // This is a simplified approach - in reality we'd need proper error handling
                let mut mgr = manager.write().await;
                *mgr = None;
                // Reinitialize with new config would happen here
            });
            
            self.config = notes_config;
        }
        
        Ok(())
    }

    fn health_check(&self) -> PluginResult<PluginHealthStatus> {
        Ok(self.status.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Test utilities will be used when full implementation is complete

    #[test]
    fn test_plugin_creation() {
        let plugin = NotesPlugin::new();
        let info = plugin.info();
        
        assert_eq!(info.name, "Comunicado Notes");
        assert_eq!(info.version, "1.0.0");
        assert_eq!(info.plugin_type, PluginType::Utility);
        assert!(info.description.contains("note-taking"));
    }

    #[test]
    fn test_plugin_with_config() {
        let config = NotesConfig::test_default();
        let plugin = NotesPlugin::with_config(config.clone());
        
        assert_eq!(plugin.config, config);
    }

    #[test]
    fn test_config_schema() {
        let plugin = NotesPlugin::new();
        let schema = plugin.config_schema();
        
        assert!(schema.is_some());
        let schema_obj = schema.unwrap();
        assert!(schema_obj.get("properties").is_some());
        assert!(schema_obj["properties"].get("default_directory").is_some());
    }

    #[test]
    fn test_validate_config_valid() {
        let plugin = NotesPlugin::new();
        let config = serde_json::json!({
            "default_directory": "/home/user/notes",
            "max_search_results": 50,
            "auto_index": true
        });
        
        assert!(plugin.validate_config(&config).is_ok());
    }

    #[test]
    fn test_validate_config_missing_required() {
        let plugin = NotesPlugin::new();
        let config = serde_json::json!({
            "max_search_results": 50
        });
        
        let result = plugin.validate_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("default_directory is required"));
    }

    #[test]
    fn test_validate_config_invalid_type() {
        let plugin = NotesPlugin::new();
        let config = serde_json::json!({
            "default_directory": 123,
            "max_search_results": "not_a_number"
        });
        
        let result = plugin.validate_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be a string"));
    }

    #[test]
    fn test_validate_config_out_of_range() {
        let plugin = NotesPlugin::new();
        let config = serde_json::json!({
            "default_directory": "/tmp",
            "max_search_results": 2000
        });
        
        let result = plugin.validate_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("between 1 and 1000"));
    }

    #[tokio::test]
    async fn test_plugin_lifecycle() {
        let mut plugin = NotesPlugin::new();
        let config = PluginConfig::new(Uuid::new_v4());
        
        // Test initialization
        assert!(plugin.initialize(&config).is_ok());
        
        // Test start
        assert!(plugin.start().is_ok());
        
        // Test pause
        assert!(plugin.pause().is_ok());
        assert!(matches!(plugin.health_check().unwrap(), PluginHealthStatus::Degraded(_)));
        
        // Test resume
        assert!(plugin.resume().is_ok());
        assert!(matches!(plugin.health_check().unwrap(), PluginHealthStatus::Healthy));
        
        // Test stop
        assert!(plugin.stop().is_ok());
    }

    #[tokio::test]
    async fn test_uninitialized_operations() {
        let plugin = NotesPlugin::new();
        
        // Operations should fail when not initialized
        let result = plugin.create_note("Test".to_string(), "Content".to_string()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not initialized"));
        
        let result = plugin.search_notes("test").await;
        assert!(result.is_err());
        
        let result = plugin.get_note("test-id").await;
        assert!(result.is_err());
    }
}