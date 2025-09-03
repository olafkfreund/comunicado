//! Window and pane management for multiplexers

use super::{MultiplexerError, MultiplexerResult, PaneArrangement};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Window manager for handling layouts and arrangements
pub struct WindowManager {
    layouts: HashMap<String, WindowLayout>,
    current_layout: Option<String>,
}

/// Window layout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowLayout {
    pub name: String,
    pub arrangement: PaneArrangement,
    pub panes: Vec<PaneConfig>,
    pub default_sizes: HashMap<String, (u16, u16)>,
}

/// Individual pane configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaneConfig {
    pub id: String,
    pub name: String,
    pub command: Option<String>,
    pub working_directory: Option<String>,
    pub environment: HashMap<String, String>,
}

impl WindowManager {
    pub fn new() -> MultiplexerResult<Self> {
        let mut manager = Self {
            layouts: HashMap::new(),
            current_layout: None,
        };
        
        // Initialize with default layouts
        manager.create_default_layouts()?;
        
        Ok(manager)
    }

    /// Create default window layouts
    fn create_default_layouts(&mut self) -> MultiplexerResult<()> {
        // Email-focused layout
        let email_layout = WindowLayout {
            name: "email".to_string(),
            arrangement: PaneArrangement::ThreePane,
            panes: vec![
                PaneConfig {
                    id: "sidebar".to_string(),
                    name: "Folders".to_string(),
                    command: None,
                    working_directory: None,
                    environment: HashMap::new(),
                },
                PaneConfig {
                    id: "list".to_string(),
                    name: "Email List".to_string(),
                    command: None,
                    working_directory: None,
                    environment: HashMap::new(),
                },
                PaneConfig {
                    id: "content".to_string(),
                    name: "Email Content".to_string(),
                    command: None,
                    working_directory: None,
                    environment: HashMap::new(),
                },
            ],
            default_sizes: [
                ("sidebar".to_string(), (25, 100)),
                ("list".to_string(), (35, 100)),
                ("content".to_string(), (40, 100)),
            ].iter().cloned().collect(),
        };

        // Calendar-focused layout
        let calendar_layout = WindowLayout {
            name: "calendar".to_string(),
            arrangement: PaneArrangement::Horizontal,
            panes: vec![
                PaneConfig {
                    id: "calendar".to_string(),
                    name: "Calendar View".to_string(),
                    command: None,
                    working_directory: None,
                    environment: HashMap::new(),
                },
                PaneConfig {
                    id: "details".to_string(),
                    name: "Event Details".to_string(),
                    command: None,
                    working_directory: None,
                    environment: HashMap::new(),
                },
            ],
            default_sizes: [
                ("calendar".to_string(), (70, 100)),
                ("details".to_string(), (30, 100)),
            ].iter().cloned().collect(),
        };

        self.layouts.insert("email".to_string(), email_layout);
        self.layouts.insert("calendar".to_string(), calendar_layout);
        
        Ok(())
    }

    /// Apply a window layout
    pub fn apply_layout(&mut self, layout_name: &str) -> MultiplexerResult<()> {
        if !self.layouts.contains_key(layout_name) {
            return Err(MultiplexerError::SessionError(
                format!("Layout '{}' not found", layout_name)
            ));
        }
        
        self.current_layout = Some(layout_name.to_string());
        Ok(())
    }

    /// Get available layouts
    pub fn available_layouts(&self) -> Vec<&str> {
        self.layouts.keys().map(|s| s.as_str()).collect()
    }

    /// Get current layout
    pub fn current_layout(&self) -> Option<&WindowLayout> {
        self.current_layout.as_ref()
            .and_then(|name| self.layouts.get(name))
    }
}