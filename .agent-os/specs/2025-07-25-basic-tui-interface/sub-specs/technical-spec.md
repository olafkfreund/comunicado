# Technical Specification

This is the technical specification for the spec detailed in @.agent-os/specs/2025-07-25-basic-tui-interface/spec.md

> Created: 2025-07-25
> Version: 1.0.0

## Technical Requirements

- **Application Framework**: Rust binary using Ratatui for TUI rendering
- **Event Handling**: Crossterm for cross-platform terminal input and output
- **Panel Layout**: Three-column layout with adjustable panel sizes
- **Keyboard Input**: Vim-style navigation keybindings (h/j/k/l, Tab, q)
- **Visual Feedback**: Clear focus indicators showing active panel
- **Terminal Compatibility**: Support for modern terminals with proper fallback
- **Performance**: Responsive event loop with <16ms input response time
- **State Management**: Clean application state structure for future feature extension

## Approach Options

**Option A:** Single main.rs with inline widget definitions
- Pros: Simple initial implementation, fast development
- Cons: Hard to maintain as features grow, poor separation of concerns

**Option B:** Modular architecture with separate widget modules (Selected)
- Pros: Clean separation of concerns, easy to extend, maintainable structure
- Cons: More initial setup, slightly more complex

**Option C:** Full MVC architecture with separate model/view/controller
- Pros: Perfect separation, very maintainable
- Cons: Over-engineering for current scope, unnecessarily complex

**Rationale:** Option B provides the right balance of maintainability and simplicity. It sets up a clean foundation for future features while remaining approachable for the initial implementation.

## External Dependencies

- **ratatui** - Modern TUI framework for Rust with rich widget support
- **Justification:** Core requirement for terminal UI, actively maintained, excellent documentation

- **crossterm** - Cross-platform terminal manipulation library
- **Justification:** Provides consistent input/output handling across different terminals and operating systems

- **tokio** - Async runtime for future IMAP/network operations
- **Justification:** Foundation for async operations needed in later phases, better to establish early

## Architecture Design

### Project Structure
```
src/
├── main.rs              # Application entry point and event loop
├── app.rs               # Main application state and logic
├── ui/
│   ├── mod.rs          # UI module exports
│   ├── layout.rs       # Panel layout and sizing logic
│   ├── folder_tree.rs  # Left panel folder tree widget
│   ├── message_list.rs # Center panel message list widget
│   └── preview.rs      # Right panel message preview widget
└── events.rs           # Keyboard event handling and mapping
```

### State Management
```rust
pub struct App {
    pub active_panel: ActivePanel,
    pub folder_tree: FolderTreeState,
    pub message_list: MessageListState,
    pub preview: PreviewState,
    pub should_quit: bool,
}

pub enum ActivePanel {
    FolderTree,
    MessageList,
    Preview,
}
```

### Event Loop Design
- Use crossterm's event polling for non-blocking input
- 60 FPS refresh rate target for smooth visual updates
- Immediate response to keyboard input for panel switching
- Proper cleanup on application exit

### Terminal Compatibility Strategy
- Test primary support on Kitty, Foot, and Wezterm
- Graceful fallback for legacy terminals without advanced features
- Consistent color scheme that works in light and dark themes
- Proper handling of terminal resize events