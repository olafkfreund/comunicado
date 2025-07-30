//! Plugin registry for tracking loaded plugins and their metadata
//!
//! The plugin registry maintains a centralized record of all loaded plugins,
//! their current status, and provides methods for plugin discovery and access.

use super::core::{Plugin, PluginError, PluginInfo, PluginResult, PluginStatus, PluginType};

use std::collections::HashMap;
use uuid::Uuid;

/// Registry for tracking loaded plugins
pub struct PluginRegistry {
    /// Map of plugin ID to plugin instance
    plugins: HashMap<Uuid, Box<dyn Plugin>>,
    /// Map of plugin ID to plugin information
    plugin_info: HashMap<Uuid, PluginInfo>,
    /// Map of plugin ID to current status
    plugin_status: HashMap<Uuid, PluginStatus>,
    /// Map of plugin name to plugin ID for name-based lookup
    name_to_id: HashMap<String, Uuid>,
    /// Map of plugin type to list of plugin IDs
    type_to_ids: HashMap<PluginType, Vec<Uuid>>,
}

impl PluginRegistry {
    /// Create a new plugin registry
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            plugin_info: HashMap::new(),
            plugin_status: HashMap::new(),
            name_to_id: HashMap::new(),
            type_to_ids: HashMap::new(),
        }
    }

    /// Register a new plugin
    pub fn register_plugin(
        &mut self,
        plugin: Box<dyn Plugin>,
        info: PluginInfo,
    ) -> PluginResult<()> {
        let plugin_id = info.id;

        // Check if plugin is already registered
        if self.plugins.contains_key(&plugin_id) {
            return Err(PluginError::AlreadyLoaded(info.name.clone()));
        }

        // Check for name conflicts
        if self.name_to_id.contains_key(&info.name) {
            return Err(PluginError::AlreadyLoaded(format!(
                "Plugin with name '{}' already exists",
                info.name
            )));
        }

        // Register plugin
        self.plugins.insert(plugin_id, plugin);
        self.plugin_status.insert(plugin_id, PluginStatus::Loaded);
        self.name_to_id.insert(info.name.clone(), plugin_id);

        // Update type mapping
        self.type_to_ids
            .entry(info.plugin_type)
            .or_insert_with(Vec::new)
            .push(plugin_id);

        // Store plugin info
        self.plugin_info.insert(plugin_id, info);

        Ok(())
    }

    /// Unregister a plugin
    pub fn unregister_plugin(&mut self, plugin_id: &Uuid) -> PluginResult<()> {
        // Remove plugin
        let plugin_removed = self.plugins.remove(plugin_id).is_some();
        
        if !plugin_removed {
            return Err(PluginError::NotFound(plugin_id.to_string()));
        }

        // Remove status
        self.plugin_status.remove(plugin_id);

        // Remove from name mapping
        if let Some(info) = self.plugin_info.get(plugin_id) {
            self.name_to_id.remove(&info.name);

            // Remove from type mapping
            if let Some(type_ids) = self.type_to_ids.get_mut(&info.plugin_type) {
                type_ids.retain(|&id| id != *plugin_id);
                if type_ids.is_empty() {
                    self.type_to_ids.remove(&info.plugin_type);
                }
            }
        }

        // Remove plugin info
        self.plugin_info.remove(plugin_id);

        Ok(())
    }

    /// Get a plugin by ID
    pub fn get_plugin(&self, plugin_id: &Uuid) -> Option<&dyn Plugin> {
        self.plugins.get(plugin_id).map(|p| p.as_ref())
    }

    /// Get a mutable plugin by ID (simplified version)
    pub fn get_plugin_mut(&mut self, plugin_id: &Uuid) -> bool {
        self.plugins.contains_key(plugin_id)
    }

    /// Get a plugin by name
    pub fn get_plugin_by_name(&self, name: &str) -> Option<&dyn Plugin> {
        self.name_to_id
            .get(name)
            .and_then(|id| self.get_plugin(id))
    }

    /// Check if plugin exists by name
    pub fn has_plugin_by_name(&mut self, name: &str) -> bool {
        self.name_to_id.contains_key(name)
    }

    /// Get plugin information by ID
    pub fn get_plugin_info(&self, plugin_id: &Uuid) -> Option<PluginInfo> {
        self.plugin_info.get(plugin_id).cloned()
    }

    /// Get plugin information by name
    pub fn get_plugin_info_by_name(&self, name: &str) -> Option<PluginInfo> {
        self.name_to_id
            .get(name)
            .and_then(|id| self.get_plugin_info(id))
    }

    /// Get plugin status
    pub fn get_plugin_status(&self, plugin_id: &Uuid) -> Option<PluginStatus> {
        self.plugin_status.get(plugin_id).cloned()
    }

    /// Set plugin status
    pub fn set_plugin_status(&mut self, plugin_id: Uuid, status: PluginStatus) {
        self.plugin_status.insert(plugin_id, status);
    }

    /// Check if a plugin is loaded
    pub fn is_loaded(&self, plugin_id: &Uuid) -> bool {
        self.plugins.contains_key(plugin_id)
    }

    /// Check if a plugin name is registered (immutable version)
    pub fn has_plugin_name(&self, name: &str) -> bool {
        self.name_to_id.contains_key(name)
    }

    /// Get all loaded plugins
    pub fn get_all_plugins(&self) -> Vec<PluginInfo> {
        self.plugin_info.values().cloned().collect()
    }

    /// Get all plugin IDs
    pub fn get_all_plugin_ids(&self) -> Vec<Uuid> {
        self.plugins.keys().copied().collect()
    }

    /// Get plugins by type
    pub fn get_plugins_by_type(&self, plugin_type: PluginType) -> Vec<PluginInfo> {
        self.type_to_ids
            .get(&plugin_type)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.plugin_info.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get plugin IDs by type
    pub fn get_plugin_ids_by_type(&self, plugin_type: PluginType) -> Vec<Uuid> {
        self.type_to_ids
            .get(&plugin_type)
            .cloned()
            .unwrap_or_default()
    }

    /// Get plugins by status
    pub fn get_plugins_by_status(&self, status: PluginStatus) -> Vec<PluginInfo> {
        self.plugin_status
            .iter()
            .filter(|(_, s)| **s == status)
            .filter_map(|(id, _)| self.plugin_info.get(id).cloned())
            .collect()
    }

    /// Get count of loaded plugins
    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }

    /// Get count of plugins by type
    pub fn plugin_count_by_type(&self, plugin_type: PluginType) -> usize {
        self.type_to_ids
            .get(&plugin_type)
            .map(|ids| ids.len())
            .unwrap_or(0)
    }

    /// Get count of plugins by status
    pub fn plugin_count_by_status(&self, status: PluginStatus) -> usize {
        self.plugin_status
            .values()
            .filter(|s| **s == status)
            .count()
    }

    /// Find plugins by capability
    pub fn find_plugins_by_capability(&self, capability: &str) -> Vec<PluginInfo> {
        self.plugin_info
            .values()
            .filter(|info| info.capabilities.contains(&capability.to_string()))
            .cloned()
            .collect()
    }

    /// Find plugins by tag
    pub fn find_plugins_by_tag(&self, tag: &str) -> Vec<PluginInfo> {
        self.plugin_info
            .values()
            .filter(|info| info.tags.contains(&tag.to_string()))
            .cloned()
            .collect()
    }

    /// Search plugins by name or description
    pub fn search_plugins(&self, query: &str) -> Vec<PluginInfo> {
        let query_lower = query.to_lowercase();
        
        self.plugin_info
            .values()
            .filter(|info| {
                info.name.to_lowercase().contains(&query_lower)
                    || info.description.to_lowercase().contains(&query_lower)
                    || info.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .cloned()
            .collect()
    }

    /// Get plugin statistics
    pub fn get_statistics(&self) -> PluginRegistryStatistics {
        let mut stats = PluginRegistryStatistics::default();

        stats.total_plugins = self.plugin_count();

        // Count by type
        for plugin_type in [
            PluginType::Email,
            PluginType::UI,
            PluginType::Calendar,
            PluginType::Notification,
            PluginType::Search,
            PluginType::ImportExport,
            PluginType::System,
            PluginType::AI,
            PluginType::Utility,
        ] {
            let count = self.plugin_count_by_type(plugin_type);
            match plugin_type {
                PluginType::Email => stats.email_plugins = count,
                PluginType::UI => stats.ui_plugins = count,
                PluginType::Calendar => stats.calendar_plugins = count,
                PluginType::Notification => stats.notification_plugins = count,
                PluginType::Search => stats.search_plugins = count,
                PluginType::ImportExport => stats.import_export_plugins = count,
                PluginType::System => stats.system_plugins = count,
                PluginType::AI => stats.ai_plugins = count,
                PluginType::Utility => stats.utility_plugins = count,
            }
        }

        // Count by status
        stats.running_plugins = self.plugin_count_by_status(PluginStatus::Running);
        stats.paused_plugins = self.plugin_count_by_status(PluginStatus::Paused);
        stats.error_plugins = self.plugin_status
            .values()
            .filter(|status| matches!(status, PluginStatus::Error(_)))
            .count();

        stats
    }

    /// Validate plugin dependencies
    pub fn validate_dependencies(&self, plugin_info: &PluginInfo) -> Vec<String> {
        let mut missing_dependencies = Vec::new();

        for dependency in &plugin_info.dependencies {
            if !dependency.optional {
                // Check if dependency is loaded
                let dependency_loaded = self.name_to_id.contains_key(&dependency.name);
                
                if !dependency_loaded {
                    missing_dependencies.push(dependency.name.clone());
                } else {
                    // Check version compatibility (simplified)
                    if let Some(dep_info) = self.get_plugin_info_by_name(&dependency.name) {
                        if dep_info.version != dependency.version {
                            missing_dependencies.push(format!(
                                "{} (version mismatch: expected {}, found {})",
                                dependency.name,
                                dependency.version,
                                dep_info.version
                            ));
                        }
                    }
                }
            }
        }

        missing_dependencies
    }

    /// Clear all plugins (useful for shutdown)
    pub fn clear(&mut self) {
        self.plugins.clear();
        self.plugin_info.clear();
        self.plugin_status.clear();
        self.name_to_id.clear();
        self.type_to_ids.clear();
    }
}

/// Plugin registry statistics
#[derive(Debug, Clone, Default)]
pub struct PluginRegistryStatistics {
    pub total_plugins: usize,
    pub email_plugins: usize,
    pub ui_plugins: usize,
    pub calendar_plugins: usize,
    pub notification_plugins: usize,
    pub search_plugins: usize,
    pub import_export_plugins: usize,
    pub system_plugins: usize,
    pub ai_plugins: usize,
    pub utility_plugins: usize,
    pub running_plugins: usize,
    pub paused_plugins: usize,
    pub error_plugins: usize,
}

impl PluginRegistryStatistics {
    /// Get active plugins count (running + initialized)
    pub fn active_plugins(&self) -> usize {
        self.running_plugins
    }

    /// Get inactive plugins count (everything except running)
    pub fn inactive_plugins(&self) -> usize {
        self.total_plugins - self.running_plugins
    }

    /// Check if registry is healthy (no error plugins)
    pub fn is_healthy(&self) -> bool {
        self.error_plugins == 0
    }

    /// Get plugin distribution summary
    pub fn get_type_distribution(&self) -> Vec<(PluginType, usize)> {
        vec![
            (PluginType::Email, self.email_plugins),
            (PluginType::UI, self.ui_plugins),
            (PluginType::Calendar, self.calendar_plugins),
            (PluginType::Notification, self.notification_plugins),
            (PluginType::Search, self.search_plugins),
            (PluginType::ImportExport, self.import_export_plugins),
            (PluginType::System, self.system_plugins),
            (PluginType::AI, self.ai_plugins),
            (PluginType::Utility, self.utility_plugins),
        ]
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}