//! Keyboard customization system for user-defined shortcuts
//!
//! This module provides comprehensive keyboard shortcut customization including:
//! - Key binding definition and management
//! - Context-aware shortcut mapping
//! - Conflict detection and resolution
//! - Import/export of key binding configurations
//! - Dynamic shortcut registration

use chrono::{DateTime, Utc};
use crossterm::event::{KeyCode, KeyModifiers};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

/// Keyboard customization errors
#[derive(Error, Debug)]
pub enum KeyboardCustomizationError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Key binding conflict: {key:?} already bound to {existing_action} in context {context}")]
    KeyBindingConflict {
        key: KeyCombination,
        existing_action: String,
        context: String,
    },
    
    #[error("Invalid key combination: {0}")]
    InvalidKeyCombination(String),
    
    #[error("Action not found: {0}")]
    ActionNotFound(String),
    
    #[error("Context not found: {0}")]
    ContextNotFound(String),
    
    #[error("Configuration validation failed: {0}")]
    ValidationFailed(String),
    
    #[error("Import failed: {0}")]
    ImportFailed(String),
}

pub type KeyboardResult<T> = Result<T, KeyboardCustomizationError>;

/// Represents a key combination (key + modifiers)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyCombination {
    pub key: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyCombination {
    /// Create a new key combination
    pub fn new(key: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { key, modifiers }
    }
    
    /// Create from a key without modifiers
    pub fn from_key(key: KeyCode) -> Self {
        Self::new(key, KeyModifiers::NONE)
    }
    
    /// Create with Ctrl modifier
    pub fn ctrl(key: KeyCode) -> Self {
        Self::new(key, KeyModifiers::CONTROL)
    }
    
    /// Create with Alt modifier
    pub fn alt(key: KeyCode) -> Self {
        Self::new(key, KeyModifiers::ALT)
    }
    
    /// Create with Shift modifier
    pub fn shift(key: KeyCode) -> Self {
        Self::new(key, KeyModifiers::SHIFT)
    }
    
    /// Parse from string representation (e.g., "Ctrl+C", "Alt+F4", "Escape")
    pub fn parse(s: &str) -> KeyboardResult<Self> {
        let parts: Vec<&str> = s.split('+').collect();
        if parts.is_empty() {
            return Err(KeyboardCustomizationError::InvalidKeyCombination(s.to_string()));
        }
        
        let mut modifiers = KeyModifiers::NONE;
        let key_str = parts.last().unwrap();
        
        // Parse modifiers
        for part in &parts[..parts.len() - 1] {
            match part.to_lowercase().as_str() {
                "ctrl" | "control" => modifiers |= KeyModifiers::CONTROL,
                "alt" => modifiers |= KeyModifiers::ALT,
                "shift" => modifiers |= KeyModifiers::SHIFT,
                _ => return Err(KeyboardCustomizationError::InvalidKeyCombination(
                    format!("Unknown modifier: {}", part)
                )),
            }
        }
        
        // Parse key
        let key = match key_str.to_lowercase().as_str() {
            "escape" | "esc" => KeyCode::Esc,
            "enter" | "return" => KeyCode::Enter,
            "tab" => KeyCode::Tab,
            "space" => KeyCode::Char(' '),
            "backspace" => KeyCode::Backspace,
            "delete" | "del" => KeyCode::Delete,
            "home" => KeyCode::Home,
            "end" => KeyCode::End,
            "pageup" | "pgup" => KeyCode::PageUp,
            "pagedown" | "pgdn" => KeyCode::PageDown,
            "up" => KeyCode::Up,
            "down" => KeyCode::Down,
            "left" => KeyCode::Left,
            "right" => KeyCode::Right,
            "insert" | "ins" => KeyCode::Insert,
            s if s.starts_with('f') && s.len() > 1 => {
                let f_num = s[1..].parse::<u8>()
                    .map_err(|_| KeyboardCustomizationError::InvalidKeyCombination(s.to_string()))?;
                if f_num >= 1 && f_num <= 12 {
                    KeyCode::F(f_num)
                } else {
                    return Err(KeyboardCustomizationError::InvalidKeyCombination(s.to_string()));
                }
            }
            s if s.len() == 1 => {
                let ch = s.chars().next().unwrap();
                KeyCode::Char(ch)
            }
            _ => return Err(KeyboardCustomizationError::InvalidKeyCombination(key_str.to_string())),
        };
        
        Ok(Self::new(key, modifiers))
    }
    
    /// Check if this key combination matches an input event
    pub fn matches(&self, key: KeyCode, modifiers: KeyModifiers) -> bool {
        self.key == key && self.modifiers == modifiers
    }
}

impl fmt::Display for KeyCombination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        
        if self.modifiers.contains(KeyModifiers::CONTROL) {
            parts.push("Ctrl");
        }
        if self.modifiers.contains(KeyModifiers::ALT) {
            parts.push("Alt");
        }
        if self.modifiers.contains(KeyModifiers::SHIFT) {
            parts.push("Shift");
        }
        
        let key_str = match self.key {
            KeyCode::Esc => "Escape",
            KeyCode::Enter => "Enter",
            KeyCode::Tab => "Tab",
            KeyCode::Backspace => "Backspace",
            KeyCode::Delete => "Delete",
            KeyCode::Home => "Home",
            KeyCode::End => "End",
            KeyCode::PageUp => "PageUp",
            KeyCode::PageDown => "PageDown",
            KeyCode::Up => "Up",
            KeyCode::Down => "Down",
            KeyCode::Left => "Left",
            KeyCode::Right => "Right",
            KeyCode::Insert => "Insert",
            KeyCode::F(n) => return write!(f, "{}F{}", parts.join("+"), n),
            KeyCode::Char(' ') => "Space",
            KeyCode::Char(c) => return write!(f, "{}{}", if parts.is_empty() { "" } else { &format!("{}+", parts.join("+")) }, c.to_uppercase()),
            _ => "Unknown",
        };
        
        if parts.is_empty() {
            write!(f, "{}", key_str)
        } else {
            write!(f, "{}+{}", parts.join("+"), key_str)
        }
    }
}

/// UI context where shortcuts are active
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyboardContext {
    /// Global shortcuts (active everywhere)
    Global,
    /// Normal email interface
    Email,
    /// Email composition
    Compose,
    /// Calendar interface
    Calendar,
    /// Event creation/editing
    EventForm,
    /// Search interface
    Search,
    /// Draft management
    DraftList,
    /// Start page
    StartPage,
    /// Email viewer (full screen)
    EmailViewer,
    /// Invitation viewer
    InvitationViewer,
    /// Keyboard shortcuts help
    KeyboardShortcuts,
    /// Migration interface
    Migration,
    /// Custom context defined by plugins
    Custom(String),
}

impl fmt::Display for KeyboardContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeyboardContext::Global => write!(f, "Global"),
            KeyboardContext::Email => write!(f, "Email"),
            KeyboardContext::Compose => write!(f, "Compose"),
            KeyboardContext::Calendar => write!(f, "Calendar"),
            KeyboardContext::EventForm => write!(f, "Event Form"),
            KeyboardContext::Search => write!(f, "Search"),
            KeyboardContext::DraftList => write!(f, "Draft List"),
            KeyboardContext::StartPage => write!(f, "Start Page"),
            KeyboardContext::EmailViewer => write!(f, "Email Viewer"),
            KeyboardContext::InvitationViewer => write!(f, "Invitation Viewer"),
            KeyboardContext::KeyboardShortcuts => write!(f, "Keyboard Shortcuts"),
            KeyboardContext::Migration => write!(f, "Migration"),
            KeyboardContext::Custom(name) => write!(f, "Custom: {}", name),
        }
    }
}

/// Priority level for key bindings (higher priority wins conflicts)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum KeyBindingPriority {
    /// Lowest priority - default bindings
    Default = 0,
    /// Plugin-defined bindings
    Plugin = 1,
    /// User-customized bindings
    User = 2,
    /// System overrides (highest priority)
    System = 3,
}

/// Represents an action that can be triggered by a key binding
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyboardAction {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub context: KeyboardContext,
    pub default_binding: Option<KeyCombination>,
    pub is_customizable: bool,
}

impl KeyboardAction {
    /// Create a new keyboard action
    pub fn new(
        id: &str,
        name: &str,
        description: &str,
        category: &str,
        context: KeyboardContext,
    ) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            category: category.to_string(),
            context,
            default_binding: None,
            is_customizable: true,
        }
    }
    
    /// Set default key binding
    pub fn with_default_binding(mut self, binding: KeyCombination) -> Self {
        self.default_binding = Some(binding);
        self
    }
    
    /// Mark as non-customizable (system actions)
    pub fn non_customizable(mut self) -> Self {
        self.is_customizable = false;
        self
    }
}

/// A key binding mapping a key combination to an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBinding {
    pub id: Uuid,
    pub action_id: String,
    pub key_combination: KeyCombination,
    pub context: KeyboardContext,
    pub priority: KeyBindingPriority,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
}

impl KeyBinding {
    /// Create a new key binding
    pub fn new(
        action_id: String,
        key_combination: KeyCombination,
        context: KeyboardContext,
        priority: KeyBindingPriority,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            action_id,
            key_combination,
            context,
            priority,
            enabled: true,
            created_at: now,
            modified_at: now,
        }
    }
    
    /// Check if this binding matches a key event in a given context
    pub fn matches(&self, key: KeyCode, modifiers: KeyModifiers, context: &KeyboardContext) -> bool {
        if !self.enabled {
            return false;
        }
        
        // Check context match (global context matches all)
        let context_matches = self.context == KeyboardContext::Global || 
                             self.context == *context;
        
        context_matches && self.key_combination.matches(key, modifiers)
    }
}

/// Keyboard customization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardConfig {
    pub version: String,
    pub name: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub actions: Vec<KeyboardAction>,
    pub bindings: Vec<KeyBinding>,
}

impl Default for KeyboardConfig {
    fn default() -> Self {
        Self {
            version: "1.0.0".to_string(),
            name: "Default Configuration".to_string(),
            description: Some("Default keyboard configuration".to_string()),
            author: None,
            created_at: Utc::now(),
            modified_at: Utc::now(),
            actions: Vec::new(),
            bindings: Vec::new(),
        }
    }
}

/// Main keyboard customization manager
pub struct KeyboardCustomizationManager {
    config: KeyboardConfig,
    config_path: PathBuf,
    actions_registry: HashMap<String, KeyboardAction>,
    context_bindings: HashMap<KeyboardContext, HashMap<KeyCombination, Vec<KeyBinding>>>,
    conflict_resolution: ConflictResolution,
}

/// How to handle key binding conflicts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictResolution {
    /// Reject conflicting bindings
    Reject,
    /// Allow conflicts, highest priority wins
    Priority,
    /// Allow conflicts, most recent wins
    Latest,
    /// Allow conflicts, prompt user for resolution
    Prompt,
}

impl KeyboardCustomizationManager {
    /// Create a new keyboard customization manager
    pub fn new(config_path: PathBuf) -> KeyboardResult<Self> {
        let mut manager = Self {
            config: KeyboardConfig::default(),
            config_path,
            actions_registry: HashMap::new(),
            context_bindings: HashMap::new(),
            conflict_resolution: ConflictResolution::Priority,
        };
        
        // Initialize with default actions
        manager.register_default_actions();
        
        // Load existing configuration
        if manager.config_path.exists() {
            manager.load_config()?;
        } else {
            manager.create_default_bindings();
            manager.save_config()?;
        }
        
        Ok(manager)
    }
    
    /// Register default keyboard actions
    fn register_default_actions(&mut self) {
        let actions = vec![
            // Global actions
            KeyboardAction::new("quit", "Quit Application", "Exit the application", "Global", KeyboardContext::Global)
                .with_default_binding(KeyCombination::ctrl(KeyCode::Char('q'))),
            KeyboardAction::new("help", "Show Help", "Display keyboard shortcuts", "Global", KeyboardContext::Global)
                .with_default_binding(KeyCombination::from_key(KeyCode::F(1))),
            
            // Navigation
            KeyboardAction::new("next_pane", "Next Pane", "Switch to next pane", "Navigation", KeyboardContext::Email)
                .with_default_binding(KeyCombination::from_key(KeyCode::Tab)),
            KeyboardAction::new("prev_pane", "Previous Pane", "Switch to previous pane", "Navigation", KeyboardContext::Email)
                .with_default_binding(KeyCombination::shift(KeyCode::Tab)),
            KeyboardAction::new("move_up", "Move Up", "Move selection up", "Navigation", KeyboardContext::Email)
                .with_default_binding(KeyCombination::from_key(KeyCode::Up)),
            KeyboardAction::new("move_down", "Move Down", "Move selection down", "Navigation", KeyboardContext::Email)
                .with_default_binding(KeyCombination::from_key(KeyCode::Down)),
            
            // Email actions
            KeyboardAction::new("compose", "Compose Email", "Start composing a new email", "Email", KeyboardContext::Email)
                .with_default_binding(KeyCombination::from_key(KeyCode::Char('c'))),
            KeyboardAction::new("reply", "Reply", "Reply to current message", "Email", KeyboardContext::Email)
                .with_default_binding(KeyCombination::from_key(KeyCode::Char('r'))),
            KeyboardAction::new("reply_all", "Reply All", "Reply to all recipients", "Email", KeyboardContext::Email)
                .with_default_binding(KeyCombination::shift(KeyCode::Char('r'))),
            KeyboardAction::new("forward", "Forward", "Forward current message", "Email", KeyboardContext::Email)
                .with_default_binding(KeyCombination::from_key(KeyCode::Char('f'))),
            KeyboardAction::new("open_email", "Open Email", "Open email in viewer", "Email", KeyboardContext::Email)
                .with_default_binding(KeyCombination::from_key(KeyCode::Enter)),
            
            // Search actions
            KeyboardAction::new("search", "Search", "Open search interface", "Search", KeyboardContext::Email)
                .with_default_binding(KeyCombination::from_key(KeyCode::Char('/'))),
            KeyboardAction::new("search_next", "Next Result", "Go to next search result", "Search", KeyboardContext::Search)
                .with_default_binding(KeyCombination::from_key(KeyCode::Down)),
            KeyboardAction::new("search_prev", "Previous Result", "Go to previous search result", "Search", KeyboardContext::Search)
                .with_default_binding(KeyCombination::from_key(KeyCode::Up)),
            
            // Calendar actions
            KeyboardAction::new("show_calendar", "Show Calendar", "Open calendar interface", "Calendar", KeyboardContext::Email)
                .with_default_binding(KeyCombination::alt(KeyCode::Char('c'))),
            KeyboardAction::new("create_event", "Create Event", "Create new calendar event", "Calendar", KeyboardContext::Calendar)
                .with_default_binding(KeyCombination::from_key(KeyCode::Char('c'))),
            KeyboardAction::new("today", "Go to Today", "Navigate to today in calendar", "Calendar", KeyboardContext::Calendar)
                .with_default_binding(KeyCombination::from_key(KeyCode::Char('t'))),
            
            // Compose actions
            KeyboardAction::new("send_email", "Send Email", "Send the composed email", "Compose", KeyboardContext::Compose)
                .with_default_binding(KeyCombination::ctrl(KeyCode::Enter)),
            KeyboardAction::new("save_draft", "Save Draft", "Save email as draft", "Compose", KeyboardContext::Compose)
                .with_default_binding(KeyCombination::ctrl(KeyCode::Char('s'))),
            KeyboardAction::new("cancel_compose", "Cancel Compose", "Cancel email composition", "Compose", KeyboardContext::Compose)
                .with_default_binding(KeyCombination::from_key(KeyCode::Esc)),
            
            // Draft management
            KeyboardAction::new("show_drafts", "Show Drafts", "Open draft list", "Drafts", KeyboardContext::Email)
                .with_default_binding(KeyCombination::alt(KeyCode::Char('d'))),
            KeyboardAction::new("delete_draft", "Delete Draft", "Delete selected draft", "Drafts", KeyboardContext::DraftList)
                .with_default_binding(KeyCombination::from_key(KeyCode::Delete)),
        ];
        
        for action in actions {
            self.actions_registry.insert(action.id.clone(), action);
        }
    }
    
    /// Create default key bindings from registered actions
    fn create_default_bindings(&mut self) {
        for action in self.actions_registry.values() {
            if let Some(ref default_binding) = action.default_binding {
                let binding = KeyBinding::new(
                    action.id.clone(),
                    default_binding.clone(),
                    action.context.clone(),
                    KeyBindingPriority::Default,
                );
                self.add_binding_internal(binding);
            }
        }
    }
    
    /// Add a key binding (internal method without conflict checking)
    fn add_binding_internal(&mut self, binding: KeyBinding) {
        let context_map = self.context_bindings
            .entry(binding.context.clone())
            .or_insert_with(HashMap::new);
        
        let key_bindings = context_map
            .entry(binding.key_combination.clone())
            .or_insert_with(Vec::new);
        
        key_bindings.push(binding.clone());
        
        // Sort by priority (highest first)
        key_bindings.sort_by(|a, b| b.priority.cmp(&a.priority));
        
        self.config.bindings.push(binding);
    }
    
    /// Add a new key binding with conflict checking
    pub fn add_binding(
        &mut self,
        action_id: &str,
        key_combination: KeyCombination,
        context: KeyboardContext,
        priority: KeyBindingPriority,
    ) -> KeyboardResult<()> {
        // Check if action exists
        if !self.actions_registry.contains_key(action_id) {
            return Err(KeyboardCustomizationError::ActionNotFound(action_id.to_string()));
        }
        
        // Check for conflicts
        if let Some(conflict) = self.find_conflict(&key_combination, &context, priority) {
            match self.conflict_resolution {
                ConflictResolution::Reject => {
                    return Err(KeyboardCustomizationError::KeyBindingConflict {
                        key: key_combination,
                        existing_action: conflict.action_id,
                        context: context.to_string(),
                    });
                }
                ConflictResolution::Priority => {
                    if conflict.priority >= priority {
                        return Err(KeyboardCustomizationError::KeyBindingConflict {
                            key: key_combination,
                            existing_action: conflict.action_id,
                            context: context.to_string(),
                        });
                    }
                }
                ConflictResolution::Latest => {
                    // Remove existing binding
                    self.remove_binding(&conflict.id)?;
                }
                ConflictResolution::Prompt => {
                    // This would need to be handled by the UI layer
                    return Err(KeyboardCustomizationError::KeyBindingConflict {
                        key: key_combination,
                        existing_action: conflict.action_id,
                        context: context.to_string(),
                    });
                }
            }
        }
        
        let binding = KeyBinding::new(action_id.to_string(), key_combination, context, priority);
        self.add_binding_internal(binding);
        self.config.modified_at = Utc::now();
        
        Ok(())
    }
    
    /// Find conflicting key binding
    fn find_conflict(
        &self,
        key_combination: &KeyCombination,
        context: &KeyboardContext,
        _priority: KeyBindingPriority,
    ) -> Option<&KeyBinding> {
        // Check direct context conflicts
        if let Some(context_map) = self.context_bindings.get(context) {
            if let Some(bindings) = context_map.get(key_combination) {
                if let Some(binding) = bindings.first() {
                    if binding.enabled {
                        return Some(binding);
                    }
                }
            }
        }
        
        // Check global context conflicts
        if *context != KeyboardContext::Global {
            if let Some(global_map) = self.context_bindings.get(&KeyboardContext::Global) {
                if let Some(bindings) = global_map.get(key_combination) {
                    if let Some(binding) = bindings.first() {
                        if binding.enabled {
                            return Some(binding);
                        }
                    }
                }
            }
        }
        
        None
    }
    
    /// Remove a key binding
    pub fn remove_binding(&mut self, binding_id: &Uuid) -> KeyboardResult<()> {
        // Find and remove from config
        let binding_index = self.config.bindings
            .iter()
            .position(|b| b.id == *binding_id)
            .ok_or_else(|| KeyboardCustomizationError::ActionNotFound(binding_id.to_string()))?;
        
        let binding = self.config.bindings.remove(binding_index);
        
        // Remove from context bindings
        if let Some(context_map) = self.context_bindings.get_mut(&binding.context) {
            if let Some(key_bindings) = context_map.get_mut(&binding.key_combination) {
                key_bindings.retain(|b| b.id != *binding_id);
                if key_bindings.is_empty() {
                    context_map.remove(&binding.key_combination);
                }
            }
        }
        
        self.config.modified_at = Utc::now();
        Ok(())
    }
    
    /// Find action for key combination in context
    pub fn find_action(
        &self,
        key: KeyCode,
        modifiers: KeyModifiers,
        context: &KeyboardContext,
    ) -> Option<&str> {
        let key_combo = KeyCombination::new(key, modifiers);
        
        // Check context-specific bindings first
        if let Some(context_map) = self.context_bindings.get(context) {
            if let Some(bindings) = context_map.get(&key_combo) {
                for binding in bindings {
                    if binding.enabled {
                        return Some(&binding.action_id);
                    }
                }
            }
        }
        
        // Check global bindings
        if *context != KeyboardContext::Global {
            if let Some(global_map) = self.context_bindings.get(&KeyboardContext::Global) {
                if let Some(bindings) = global_map.get(&key_combo) {
                    for binding in bindings {
                        if binding.enabled {
                            return Some(&binding.action_id);
                        }
                    }
                }
            }
        }
        
        None
    }
    
    /// Get all actions for a context
    pub fn get_actions_for_context(&self, context: &KeyboardContext) -> Vec<&KeyboardAction> {
        self.actions_registry
            .values()
            .filter(|action| action.context == *context || action.context == KeyboardContext::Global)
            .collect()
    }
    
    /// Get all bindings for an action
    pub fn get_bindings_for_action(&self, action_id: &str) -> Vec<&KeyBinding> {
        self.config.bindings
            .iter()
            .filter(|binding| binding.action_id == action_id && binding.enabled)
            .collect()
    }
    
    /// Register a new action (for plugins)
    pub fn register_action(&mut self, action: KeyboardAction) -> KeyboardResult<()> {
        if self.actions_registry.contains_key(&action.id) {
            return Err(KeyboardCustomizationError::ValidationFailed(
                format!("Action {} already exists", action.id)
            ));
        }
        
        self.actions_registry.insert(action.id.clone(), action.clone());
        self.config.actions.push(action);
        self.config.modified_at = Utc::now();
        
        Ok(())
    }
    
    /// Load configuration from file
    pub fn load_config(&mut self) -> KeyboardResult<()> {
        let content = std::fs::read_to_string(&self.config_path)?;
        let config: KeyboardConfig = serde_json::from_str(&content)?;
        
        self.validate_config(&config)?;
        
        self.config = config;
        self.rebuild_context_bindings();
        
        Ok(())
    }
    
    /// Save configuration to file
    pub fn save_config(&self) -> KeyboardResult<()> {
        let content = serde_json::to_string_pretty(&self.config)?;
        std::fs::write(&self.config_path, content)?;
        Ok(())
    }
    
    /// Validate configuration
    fn validate_config(&self, config: &KeyboardConfig) -> KeyboardResult<()> {
        // Check for duplicate action IDs
        let mut action_ids = HashSet::new();
        for action in &config.actions {
            if !action_ids.insert(&action.id) {
                return Err(KeyboardCustomizationError::ValidationFailed(
                    format!("Duplicate action ID: {}", action.id)
                ));
            }
        }
        
        // Check that all bindings reference valid actions
        for binding in &config.bindings {
            if !self.actions_registry.contains_key(&binding.action_id) &&
               !config.actions.iter().any(|a| a.id == binding.action_id) {
                return Err(KeyboardCustomizationError::ValidationFailed(
                    format!("Binding references unknown action: {}", binding.action_id)
                ));
            }
        }
        
        Ok(())
    }
    
    /// Rebuild context bindings from configuration
    fn rebuild_context_bindings(&mut self) {
        self.context_bindings.clear();
        
        for binding in &self.config.bindings {
            if binding.enabled {
                let context_map = self.context_bindings
                    .entry(binding.context.clone())
                    .or_insert_with(HashMap::new);
                
                let key_bindings = context_map
                    .entry(binding.key_combination.clone())
                    .or_insert_with(Vec::new);
                
                key_bindings.push(binding.clone());
            }
        }
        
        // Sort all bindings by priority
        for context_map in self.context_bindings.values_mut() {
            for key_bindings in context_map.values_mut() {
                key_bindings.sort_by(|a, b| b.priority.cmp(&a.priority));
            }
        }
    }
    
    /// Export configuration to file
    pub fn export_config(&self, path: &Path) -> KeyboardResult<()> {
        let content = serde_json::to_string_pretty(&self.config)?;
        std::fs::write(path, content)?;
        Ok(())
    }
    
    /// Import configuration from file
    pub fn import_config(&mut self, path: &Path, merge: bool) -> KeyboardResult<()> {
        let content = std::fs::read_to_string(path)?;
        let imported_config: KeyboardConfig = serde_json::from_str(&content)
            .map_err(|e| KeyboardCustomizationError::ImportFailed(e.to_string()))?;
        
        self.validate_config(&imported_config)?;
        
        if merge {
            // Merge imported actions and bindings
            for action in imported_config.actions {
                if !self.actions_registry.contains_key(&action.id) {
                    self.actions_registry.insert(action.id.clone(), action.clone());
                    self.config.actions.push(action);
                }
            }
            
            for binding in imported_config.bindings {
                if !self.config.bindings.iter().any(|b| b.id == binding.id) {
                    self.config.bindings.push(binding);
                }
            }
        } else {
            // Replace configuration
            self.config = imported_config;
            
            // Update actions registry
            self.actions_registry.clear();
            self.register_default_actions();
            for action in &self.config.actions {
                self.actions_registry.insert(action.id.clone(), action.clone());
            }
        }
        
        self.rebuild_context_bindings();
        self.config.modified_at = Utc::now();
        
        Ok(())
    }
    
    /// Reset to default configuration
    pub fn reset_to_defaults(&mut self) -> KeyboardResult<()> {
        self.config = KeyboardConfig::default();
        self.context_bindings.clear();
        self.actions_registry.clear();
        
        self.register_default_actions();
        self.create_default_bindings();
        
        Ok(())
    }
    
    /// Get configuration metadata
    pub fn get_config_info(&self) -> &KeyboardConfig {
        &self.config
    }
    
    /// Set conflict resolution strategy
    pub fn set_conflict_resolution(&mut self, resolution: ConflictResolution) {
        self.conflict_resolution = resolution;
    }
    
    /// Get all registered actions
    pub fn get_all_actions(&self) -> Vec<&KeyboardAction> {
        self.actions_registry.values().collect()
    }
    
    /// Get all current bindings
    pub fn get_all_bindings(&self) -> &[KeyBinding] {
        &self.config.bindings
    }
    
    /// Enable/disable a binding
    pub fn set_binding_enabled(&mut self, binding_id: &Uuid, enabled: bool) -> KeyboardResult<()> {
        let binding = self.config.bindings
            .iter_mut()
            .find(|b| b.id == *binding_id)
            .ok_or_else(|| KeyboardCustomizationError::ActionNotFound(binding_id.to_string()))?;
        
        binding.enabled = enabled;
        binding.modified_at = Utc::now();
        
        self.rebuild_context_bindings();
        self.config.modified_at = Utc::now();
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_key_combination_parsing() {
        assert_eq!(
            KeyCombination::parse("Ctrl+C").unwrap(),
            KeyCombination::ctrl(KeyCode::Char('c'))
        );
        
        assert_eq!(
            KeyCombination::parse("Alt+F4").unwrap(),
            KeyCombination::alt(KeyCode::F(4))
        );
        
        assert_eq!(
            KeyCombination::parse("Escape").unwrap(),
            KeyCombination::from_key(KeyCode::Esc)
        );
        
        assert_eq!(
            KeyCombination::parse("Shift+Tab").unwrap(),
            KeyCombination::shift(KeyCode::Tab)
        );
    }

    #[test]
    fn test_key_combination_display() {
        assert_eq!(
            KeyCombination::ctrl(KeyCode::Char('c')).to_string(),
            "Ctrl+C"
        );
        
        assert_eq!(
            KeyCombination::alt(KeyCode::F(4)).to_string(),
            "Alt+F4"
        );
        
        assert_eq!(
            KeyCombination::from_key(KeyCode::Esc).to_string(),
            "Escape"
        );
    }

    #[test]
    fn test_key_combination_matching() {
        let combo = KeyCombination::ctrl(KeyCode::Char('c'));
        
        assert!(combo.matches(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(!combo.matches(KeyCode::Char('c'), KeyModifiers::NONE));
        assert!(!combo.matches(KeyCode::Char('v'), KeyModifiers::CONTROL));
    }

    #[tokio::test]
    async fn test_keyboard_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("keyboard.json");
        
        let manager = KeyboardCustomizationManager::new(config_path).unwrap();
        
        // Should have default actions registered
        assert!(!manager.get_all_actions().is_empty());
        
        // Should find default quit action
        let action = manager.find_action(
            KeyCode::Char('q'),
            KeyModifiers::CONTROL,
            &KeyboardContext::Global
        );
        assert_eq!(action, Some("quit"));
    }

    #[tokio::test]
    async fn test_adding_custom_binding() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("keyboard.json");
        
        let mut manager = KeyboardCustomizationManager::new(config_path).unwrap();
        
        // Add custom binding
        let result = manager.add_binding(
            "compose",
            KeyCombination::ctrl(KeyCode::Char('n')),
            KeyboardContext::Email,
            KeyBindingPriority::User,
        );
        
        assert!(result.is_ok());
        
        // Should find the new binding
        let action = manager.find_action(
            KeyCode::Char('n'),
            KeyModifiers::CONTROL,
            &KeyboardContext::Email
        );
        assert_eq!(action, Some("compose"));
    }

    #[test]
    fn test_conflict_detection() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("keyboard.json");
        
        let mut manager = KeyboardCustomizationManager::new(config_path).unwrap();
        
        // Add first binding
        manager.add_binding(
            "compose",
            KeyCombination::ctrl(KeyCode::Char('n')),
            KeyboardContext::Email,
            KeyBindingPriority::User,
        ).unwrap();
        
        // Try to add conflicting binding (should fail with default conflict resolution)
        let result = manager.add_binding(
            "reply",
            KeyCombination::ctrl(KeyCode::Char('n')),
            KeyboardContext::Email,
            KeyBindingPriority::User,
        );
        
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KeyboardCustomizationError::KeyBindingConflict { .. }));
    }
}