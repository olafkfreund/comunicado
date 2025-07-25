# Spec Tasks

These are the tasks to be completed for the spec detailed in @.agent-os/specs/2025-07-25-basic-tui-interface/spec.md

> Created: 2025-07-25
> Status: Ready for Implementation

## Tasks

- [x] 1. Project Foundation and Dependencies
  - [x] 1.1 Write tests for basic project structure and dependency integration
  - [x] 1.2 Initialize Rust project with Cargo.toml and required dependencies (ratatui, crossterm, tokio)
  - [x] 1.3 Set up basic project structure with src/ modules as per technical spec
  - [x] 1.4 Create placeholder module files (app.rs, events.rs, ui/mod.rs, etc.)
  - [x] 1.5 Verify all tests pass and dependencies resolve correctly

- [ ] 2. Application State Management
  - [ ] 2.1 Write tests for App state structure and ActivePanel enum
  - [ ] 2.2 Implement App struct with all required state fields
  - [ ] 2.3 Implement ActivePanel enum with FolderTree, MessageList, Preview variants
  - [ ] 2.4 Add state transition methods for panel switching logic
  - [ ] 2.5 Verify all state management tests pass

- [ ] 3. Event System and Keyboard Handling
  - [ ] 3.1 Write tests for keyboard event mapping and processing
  - [ ] 3.2 Implement event handling module with crossterm integration
  - [ ] 3.3 Add vim-style keyboard navigation (h/j/k/l keys)
  - [ ] 3.4 Implement tab cycling and quit functionality
  - [ ] 3.5 Verify all event handling tests pass

- [ ] 4. UI Layout and Panel Structure
  - [ ] 4.1 Write tests for three-panel layout calculations
  - [ ] 4.2 Implement layout module with responsive panel sizing
  - [ ] 4.3 Create folder tree widget with placeholder content
  - [ ] 4.4 Create message list widget with placeholder content
  - [ ] 4.5 Create preview panel widget with placeholder content
  - [ ] 4.6 Verify all layout tests pass

- [ ] 5. Main Application Loop and Integration
  - [ ] 5.1 Write integration tests for complete application lifecycle
  - [ ] 5.2 Implement main.rs with terminal initialization and cleanup
  - [ ] 5.3 Create main event loop with ratatui rendering
  - [ ] 5.4 Integrate all modules into working application
  - [ ] 5.5 Add terminal resize handling and focus indicators
  - [ ] 5.6 Verify all integration tests pass and application runs successfully