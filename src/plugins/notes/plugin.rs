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
use super::advanced_search::AdvancedSearchEngine;
use super::tui::NoteTUI;
use super::email_integration::EmailIntegrationService;
use super::mobile_integration::MobileNotesIntegration;
use super::calendar_integration::CalendarNotesIntegration;

use std::any::Any;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main notes plugin implementation
pub struct NotesPlugin {
    /// Plugin metadata
    info: PluginInfo,
    /// Plugin configuration
    config: NotesConfig,
    /// Note manager for core operations
    manager: Arc<RwLock<Option<NoteManager>>>,
    /// Advanced search engine
    search_engine: Arc<RwLock<Option<AdvancedSearchEngine>>>,
    /// TUI interface
    tui: Arc<RwLock<Option<NoteTUI>>>,
    /// Email integration service
    email_integration: Arc<RwLock<Option<EmailIntegrationService>>>,
    /// Mobile integration service
    mobile_integration: Arc<RwLock<Option<MobileNotesIntegration>>>,
    /// Calendar integration service
    calendar_integration: Arc<RwLock<Option<CalendarNotesIntegration>>>,
    /// Current plugin status
    status: PluginHealthStatus,
}

impl NotesPlugin {
    /// Create a new notes plugin instance
    pub fn new() -> Self {
        let mut info = PluginInfo::new(
            "Comunicado Notes".to_string(),
            "1.0.0".to_string(),
            "Advanced note-taking with markdown support, wiki linking, TUI interface, and cross-app integration".to_string(),
            "Comunicado Team".to_string(),
            PluginType::Utility,
            "0.1.0".to_string(), // Min Comunicado version
        );

        // Add capabilities
        info.capabilities = vec![
            "markdown_support".to_string(),
            "wiki_linking".to_string(),
            "full_text_search".to_string(),
            "tui_interface".to_string(),
            "email_integration".to_string(),
            "mobile_integration".to_string(),
            "calendar_integration".to_string(),
            "file_watching".to_string(),
            "advanced_search".to_string(),
        ];

        // Add tags
        info.tags = vec![
            "notes".to_string(),
            "markdown".to_string(),
            "tui".to_string(),
            "productivity".to_string(),
            "search".to_string(),
        ];

        Self {
            info,
            config: NotesConfig::default(),
            manager: Arc::new(RwLock::new(None)),
            search_engine: Arc::new(RwLock::new(None)),
            tui: Arc::new(RwLock::new(None)),
            email_integration: Arc::new(RwLock::new(None)),
            mobile_integration: Arc::new(RwLock::new(None)),
            calendar_integration: Arc::new(RwLock::new(None)),
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

    /// Get a reference to the search engine
    pub async fn search_engine(&self) -> Arc<RwLock<Option<AdvancedSearchEngine>>> {
        self.search_engine.clone()
    }

    /// Get a reference to the TUI interface
    pub async fn tui(&self) -> Arc<RwLock<Option<NoteTUI>>> {
        self.tui.clone()
    }

    /// Get a reference to the email integration service
    pub async fn email_integration(&self) -> Arc<RwLock<Option<EmailIntegrationService>>> {
        self.email_integration.clone()
    }

    /// Get a reference to the mobile integration service
    pub async fn mobile_integration(&self) -> Arc<RwLock<Option<MobileNotesIntegration>>> {
        self.mobile_integration.clone()
    }

    /// Get a reference to the calendar integration service
    pub async fn calendar_integration(&self) -> Arc<RwLock<Option<CalendarNotesIntegration>>> {
        self.calendar_integration.clone()
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

    /// Launch the TUI interface for notes management
    pub async fn launch_tui(&self) -> PluginResult<()> {
        let tui_guard = self.tui.read().await;
        if tui_guard.is_some() {
            // TUI would be launched in a separate thread/task
            // For now, just return success as this would be handled by the main application
            Ok(())
        } else {
            Err(PluginError::ExecutionFailed("TUI not initialized".to_string()))
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
            self.initialize_components().await?;
        }
        
        Ok(())
    }

    /// Initialize all components
    async fn initialize_components(&self) -> PluginResult<()> {
        // Initialize storage
        let storage = NoteStorage::new(&self.config.default_directory)
            .await
            .map_err(|e| PluginError::InitializationFailed(format!("Failed to initialize storage: {}", e)))?;
        let storage_arc = Arc::new(storage.clone());

        // Initialize indexer
        let indexer = NoteIndexer::new(storage_arc.clone())
            .await
            .map_err(|e| PluginError::InitializationFailed(format!("Failed to initialize indexer: {}", e)))?;

        // Initialize file watcher
        let watcher = FileWatcher::new()
            .map_err(|e| PluginError::InitializationFailed(format!("Failed to initialize file watcher: {}", e)))?;

        // Initialize manager
        let manager = NoteManager::new(storage, indexer, watcher, self.config.clone())
            .await
            .map_err(|e| PluginError::InitializationFailed(format!("Failed to initialize manager: {}", e)))?;

        // Initialize advanced search engine
        let search_engine = AdvancedSearchEngine::new(storage_arc.clone());

        // Initialize TUI
        let tui = NoteTUI::new(storage_arc.clone(), Arc::new(search_engine))
            .await
            .map_err(|e| PluginError::InitializationFailed(format!("Failed to initialize TUI: {}", e)))?;

        // Initialize integrations (simplified for plugin system)
        // Note: In a real deployment, these would be initialized with proper dependencies
        let email_integration = EmailIntegrationService::new(storage_arc.clone());
        
        // For now, skip mobile and calendar integrations as they require external dependencies
        // that aren't available in the plugin context
        let mobile_integration = None;
        let calendar_integration = None;

        // Set all components
        {
            let mut manager_lock = self.manager.write().await;
            *manager_lock = Some(manager);
        }

        {
            let mut search_engine_lock = self.search_engine.write().await;
            *search_engine_lock = Some(AdvancedSearchEngine::new(storage_arc.clone()));
        }

        {
            let mut tui_lock = self.tui.write().await;
            *tui_lock = Some(tui);
        }

        {
            let mut email_integration_lock = self.email_integration.write().await;
            *email_integration_lock = Some(email_integration);
        }

        {
            let mut mobile_integration_lock = self.mobile_integration.write().await;
            *mobile_integration_lock = mobile_integration;
        }

        {
            let mut calendar_integration_lock = self.calendar_integration.write().await;
            *calendar_integration_lock = calendar_integration;
        }

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
        let manager = self.manager.clone();
        let search_engine = self.search_engine.clone();
        let tui = self.tui.clone();
        let email_integration = self.email_integration.clone();
        let mobile_integration = self.mobile_integration.clone();
        let calendar_integration = self.calendar_integration.clone();
        
        tokio::spawn(async move {
            // Clean up all components
            {
                let mut mgr = manager.write().await;
                *mgr = None;
            }
            {
                let mut se = search_engine.write().await;
                *se = None;
            }
            {
                let mut tui_guard = tui.write().await;
                *tui_guard = None;
            }
            {
                let mut ei = email_integration.write().await;
                *ei = None;
            }
            {
                let mut mi = mobile_integration.write().await;
                *mi = None;
            }
            {
                let mut ci = calendar_integration.write().await;
                *ci = None;
            }
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
                    "description": "Enable vim-style keybindings in TUI"
                },
                "enable_tui": {
                    "type": "boolean",
                    "description": "Enable TUI interface"
                },
                "enable_email_integration": {
                    "type": "boolean",
                    "description": "Enable email to notes integration"
                },
                "enable_mobile_integration": {
                    "type": "boolean",
                    "description": "Enable mobile SMS to notes integration"
                },
                "enable_calendar_integration": {
                    "type": "boolean",
                    "description": "Enable calendar event to notes integration"
                },
                "tui_theme": {
                    "type": "string",
                    "enum": ["default", "dark", "light"],
                    "description": "TUI color theme"
                },
                "auto_save_interval": {
                    "type": "integer",
                    "minimum": 10,
                    "maximum": 300,
                    "description": "Auto-save interval in seconds"
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
        let config = PluginConfig::new(uuid::Uuid::new_v4());
        
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

    #[test]
    fn test_plugin_system_integration() {
        // Test that the plugin can be used through the plugin system trait
        let plugin = NotesPlugin::new();
        let info = plugin.info();
        
        // Verify plugin is properly integrated
        assert_eq!(info.name, "Comunicado Notes");
        assert_eq!(info.plugin_type, PluginType::Utility);
        assert!(info.capabilities.contains(&"markdown_support".to_string()));
        assert!(info.capabilities.contains(&"tui_interface".to_string()));
        assert!(info.capabilities.contains(&"email_integration".to_string()));
        
        // Test that it can be cast as Plugin trait object
        let plugin_trait: &dyn Plugin = &plugin;
        let plugin_info = plugin_trait.info();
        assert_eq!(plugin_info.name, "Comunicado Notes");
        
        // Test health check
        let health = plugin_trait.health_check().unwrap();
        assert_eq!(health, PluginHealthStatus::Healthy);
    }
}