//! Plugin architecture for Comunicado
//!
//! This module provides a comprehensive plugin system that allows third-party developers
//! to extend Comunicado's functionality through well-defined interfaces.
//!
//! # Plugin Types
//!
//! - **Email Plugins**: Process incoming/outgoing emails, add filters, modify content
//! - **UI Plugins**: Add custom UI components, modify layouts, add new views
//! - **Calendar Plugins**: Extend calendar functionality, add new calendar sources
//! - **Notification Plugins**: Custom notification handlers and routing
//! - **Search Plugins**: Enhanced search capabilities and indexing
//! - **Import/Export Plugins**: Support for additional data formats
//!
//! # Architecture
//!
//! The plugin system uses a trait-based approach with dynamic loading capabilities:
//!
//! ```rust
//! use comunicado::plugins::{Plugin, PluginManager, PluginType};
//!
//! // Plugin implementation
//! struct MyPlugin;
//!
//! impl Plugin for MyPlugin {
//!     fn info(&self) -> PluginInfo {
//!         PluginInfo {
//!             name: "My Custom Plugin".to_string(),
//!             version: "1.0.0".to_string(),
//!             description: "Adds custom functionality".to_string(),
//!             plugin_type: PluginType::Email,
//!             author: "Developer Name".to_string(),
//!         }
//!     }
//!
//!     fn initialize(&mut self) -> PluginResult<()> {
//!         // Plugin initialization logic
//!         Ok(())
//!     }
//! }
//! ```

pub mod core;
pub mod loader;
pub mod manager;
pub mod notes;
pub mod registry;
pub mod types;

// Re-export main types for convenience
pub use core::{Plugin, PluginError, PluginInfo, PluginResult, PluginStatus, PluginType};
pub use loader::PluginLoader;
pub use manager::PluginManager;
pub use registry::PluginRegistry;

// Notes plugin
pub use notes::NotesPlugin;

// Plugin trait specializations
pub use types::{
    CalendarEventResult, CalendarPlugin, CalendarPluginContext, EmailPlugin, EmailPluginContext,
    EmailProcessResult, ImportExportPlugin, ImportExportPluginContext, ImportExportResult,
    NotificationPlugin, NotificationPluginContext, NotificationResult, SearchPlugin,
    SearchPluginContext, SearchResult, UIComponentResult, UIPlugin, UIPluginContext,
};
