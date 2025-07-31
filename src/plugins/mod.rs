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
pub mod manager;
pub mod registry;
pub mod loader;
pub mod types;


// Re-export main types for convenience
pub use core::{Plugin, PluginInfo, PluginResult, PluginError, PluginType, PluginStatus};
pub use manager::PluginManager;
pub use registry::PluginRegistry;
pub use loader::PluginLoader;

// Plugin trait specializations
pub use types::{
    EmailPlugin, EmailPluginContext, EmailProcessResult,
    UIPlugin, UIPluginContext, UIComponentResult,
    CalendarPlugin, CalendarPluginContext, CalendarEventResult,
    NotificationPlugin, NotificationPluginContext, NotificationResult,
    SearchPlugin, SearchPluginContext, SearchResult,
    ImportExportPlugin, ImportExportPluginContext, ImportExportResult,
};