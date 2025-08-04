# Settings Implementation Plan

## Current State Analysis

**Status**: Only 9 of 47 settings have working functionality (19% complete)

**Working Settings**:
- General: Sync interval, Max concurrent syncs, Default folder (3/10)
- UI: Font size (1/7) 
- Performance: Cache size, Max concurrent operations, Cleanup interval (3/6)
- AI: Provider (1/8) ← Just implemented

**Major Issues**:
1. **No Visual Feedback**: Setting displays show hardcoded values, not actual config
2. **No Persistence**: Changes don't save to configuration files
3. **Missing Implementations**: 38 settings have no edit functionality
4. **Poor UX**: Edit mode only shows input in footer, not inline

## Implementation Strategy

### Phase 1: Foundation (High Priority)
**Goal**: Create robust settings infrastructure

#### 1.1 Configuration Management System
```rust
// Create src/config/mod.rs
pub struct AppConfig {
    pub general: GeneralConfig,
    pub ui: UIConfig, 
    pub accounts: AccountsConfig,
    pub keyboard: KeyboardConfig,
    pub performance: PerformanceConfig,
    pub privacy: PrivacyConfig,
    pub ai: AIConfig,
    pub advanced: AdvancedConfig,
}

impl AppConfig {
    pub fn load() -> Result<Self, ConfigError>;
    pub fn save(&self) -> Result<(), ConfigError>;
    pub fn default() -> Self;
}
```

#### 1.2 Settings Data Model
```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub auto_sync: bool,
    pub sync_interval_minutes: u64,
    pub fetch_on_startup: bool,
    pub incremental_sync: bool,
    pub max_concurrent_syncs: u32,
    pub default_folder: String,
    pub confirm_delete: bool,
    pub show_notifications: bool,
    pub thread_grouping: ThreadGrouping,
    pub mark_read_on_reply: bool,
}
```

#### 1.3 Settings UI State Management
- Replace hardcoded display strings with actual config values
- Add real-time config updates
- Implement proper validation and error handling

### Phase 2: UI Enhancement (Medium Priority)
**Goal**: Improve user experience and visual feedback

#### 2.1 Enhanced Edit Mode
- **Inline Editing**: Show input field directly in setting item
- **Visual Indicators**: Highlight edited items, show validation status
- **Better Navigation**: Tab between editable fields

#### 2.2 Setting Type System
```rust
pub enum SettingType {
    Boolean(bool),
    Text(String),
    Number { value: f64, min: f64, max: f64 },
    Choice { options: Vec<String>, selected: usize },
    Action(Box<dyn Fn() -> Result<(), SettingError>>),
}
```

#### 2.3 Improved Rendering
- Show current actual values instead of hardcoded strings
- Visual feedback for modified settings
- Input validation indicators
- Better status messages

### Phase 3: Complete Implementation (Medium Priority)
**Goal**: Implement all missing settings functionality

#### 3.1 Missing Edit Functions

**Accounts Tab** (all missing):
```rust
fn apply_accounts_edit(&mut self, value: String) {
    match self.state.selected_index {
        // Implement account name editing, server settings, etc.
    }
}
```

**Keyboard Tab** (all missing):
```rust
fn apply_keyboard_edit(&mut self, value: String) {
    match self.state.selected_index {
        // Implement shortcut key binding editing
    }
}
```

**Privacy Tab** (all missing):
```rust
fn apply_privacy_edit(&mut self, value: String) {
    match self.state.selected_index {
        // Implement data retention periods, etc.
    }
}
```

**Advanced Tab** (all missing):
```rust
fn apply_advanced_edit(&mut self, value: String) {
    match self.state.selected_index {
        // Implement log levels, debug settings, etc.
    }
}
```

#### 3.2 Enhanced AI Settings
Expand beyond just provider:
```rust
fn apply_ai_edit(&mut self, value: String) {
    match self.state.selected_index {
        1 => self.edit_ai_provider(value),
        2 => self.edit_ai_model(value),
        3 => self.edit_ai_api_key(value),
        4 => self.edit_ai_endpoint(value),
        // ... more AI settings
    }
}
```

### Phase 4: Advanced Features (Low Priority)
**Goal**: Polish and advanced functionality

#### 4.1 Import/Export
- Configuration backup/restore
- Settings migration between versions
- Profile management

#### 4.2 Validation & Error Handling
- Comprehensive input validation
- Connection testing for network settings
- Rollback on invalid configurations

#### 4.3 Advanced UI Features
- Search/filter settings
- Setting categories and groups
- Context-sensitive help
- Undo/redo functionality

## Implementation Roadmap

### Sprint 1: Configuration Foundation (1-2 weeks)
- [ ] Create configuration data structures
- [ ] Implement config file loading/saving
- [ ] Add TOML/JSON serialization
- [ ] Create configuration validation
- [ ] Add default configuration generation

### Sprint 2: UI Infrastructure (1 week)
- [ ] Replace hardcoded strings with actual config values
- [ ] Implement real-time config binding
- [ ] Add proper error handling and status messages
- [ ] Create setting type system

### Sprint 3: Missing Functionality (2-3 weeks)
- [ ] Implement all missing apply_*_edit functions
- [ ] Add validation for each setting type
- [ ] Create comprehensive test coverage
- [ ] Add setting-specific help text

### Sprint 4: Enhanced UX (1-2 weeks) 
- [ ] Implement inline editing
- [ ] Add visual feedback and indicators
- [ ] Improve navigation and keyboard shortcuts
- [ ] Add setting search/filter

### Sprint 5: Polish & Testing (1 week)
- [ ] Comprehensive integration testing
- [ ] Performance optimization
- [ ] Documentation updates
- [ ] User experience refinement

## File Structure

```
src/
├── config/
│   ├── mod.rs              # Main config types and loading
│   ├── defaults.rs         # Default configuration values  
│   ├── validation.rs       # Input validation and sanitization
│   └── persistence.rs      # File I/O and serialization
├── ui/
│   ├── settings_ui.rs      # Enhanced with real config binding
│   └── settings/
│       ├── edit_mode.rs    # Improved edit mode handling
│       ├── validators.rs   # UI input validation
│       └── renderers.rs    # Setting-specific render functions
└── settings/
    ├── general.rs          # General settings implementation
    ├── accounts.rs         # Account settings implementation  
    ├── ai.rs              # AI settings implementation
    └── ...                # Other setting categories
```

## Success Metrics

- [ ] **100% Functional Coverage**: All 47 settings have working functionality
- [ ] **Real Configuration**: All settings read/write actual config files
- [ ] **Visual Feedback**: Users see current values and changes immediately  
- [ ] **Proper Validation**: Invalid inputs are caught with helpful error messages
- [ ] **Persistence**: Settings survive application restarts
- [ ] **Performance**: Settings UI remains responsive with large configurations
- [ ] **Test Coverage**: >90% test coverage for settings functionality

## Priority Assessment

**Critical (Must Have)**:
- Configuration persistence (Phase 1)
- Visual feedback showing actual values (Phase 2)
- AI settings completion (Phase 3)

**Important (Should Have)**:
- All missing edit functions (Phase 3)
- Better input validation (Phases 2-3)
- Improved UX (Phase 4)

**Nice to Have**:
- Advanced features like import/export (Phase 4)
- Setting search/filter (Phase 4)
- Undo/redo (Phase 4)

This plan transforms the settings system from a mostly non-functional mockup into a complete, professional configuration interface.