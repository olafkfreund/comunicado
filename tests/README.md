# Comunicado TUI Testing Framework

This directory contains comprehensive testing tools for the Comunicado TUI interface, providing both automated and manual testing capabilities.

## 🧪 Testing Approach

We use a multi-layered testing strategy:

1. **Unit Tests** - Individual component testing with ratatui TestBackend
2. **Integration Tests** - End-to-end workflow testing
3. **Manual Testing** - Guided interactive testing sessions
4. **Performance Testing** - Startup time and responsiveness validation

## 📁 Test Files

### Core Testing Framework

- **`tui_test_framework.rs`** - Main testing framework with `TuiTestRunner`
- **`mod.rs`** - Module declarations and re-exports

### Legacy Test Files (Reference)

- **`tui_integration_tests.rs`** - External terminal testing (requires xdotool)
- **`tui_unit_tests.rs`** - Direct component unit tests (WIP)
- **`tui_e2e_tests.rs`** - End-to-end testing framework (WIP)

## 🚀 Quick Start

### Run All TUI Tests

```bash
# Run the core TUI test suite
cargo test tui_test_framework --lib

# Run specific test category
cargo test test_keyboard_shortcuts --lib
cargo test test_email_operations --lib
cargo test test_calendar_operations --lib
```

### Automated Testing Scripts

```bash
# Comprehensive automated testing
./scripts/run_tui_tests.sh

# Manual testing with guided scenarios
./scripts/run_manual_tests.sh
```

## 🔧 Test Framework Usage

### Basic Test Creation

```rust
use comunicado::tests::tui_test_framework::*;

#[tokio::test]
async fn my_custom_tui_test() {
    let runner = TuiTestRunner::new(120, 40);
    
    let sequence = TestSequence::new()
        .add_keystroke("Navigate to email", KeyCode::Char('1'))
        .add_ctrl_key("Open command palette", 'd')
        .add_text_input("Type command", "search")
        .add_keystroke("Execute", KeyCode::Enter);
    
    let result = runner.run_test_sequence("my_test", sequence).await;
    result.print_summary();
    assert!(result.passed);
}
```

### Advanced Test Scenarios

```rust
#[tokio::test]
async fn test_complex_workflow() {
    let runner = TuiTestRunner::new(120, 40)
        .with_timeout(Duration::from_secs(10));
    
    let sequence = TestSequence::new()
        .add_step(TestStep::new("Multi-key operation")
            .with_key(KeyCode::Char('1'), KeyModifiers::NONE)
            .with_key(KeyCode::Down, KeyModifiers::NONE)
            .with_key(KeyCode::Enter, KeyModifiers::NONE)
            .expect_text("Email selected"));
    
    let result = runner.run_test_sequence("complex_workflow", sequence).await;
    assert!(result.passed);
}
```

## 📋 Manual Testing Guide

### Available Test Scenarios

The manual testing script (`scripts/run_manual_tests.sh`) provides:

1. **Basic Startup & Navigation** - Core functionality verification
2. **Keyboard Shortcuts** - Complete shortcut testing
3. **Email Operations** - Email-specific functionality
4. **Calendar Operations** - Calendar feature testing
5. **Search Functionality** - Search across all content
6. **UI Responsiveness** - Performance and smoothness
7. **Full Feature Walkthrough** - Comprehensive end-to-end test
8. **Error Handling** - Edge case and error scenario testing

### Running Manual Tests

```bash
# Start interactive testing session
./scripts/run_manual_tests.sh

# Quick specific tests
./scripts/run_tui_tests.sh --startup
./scripts/run_tui_tests.sh --keyboard
./scripts/run_tui_tests.sh --performance
```

## ⌨️ Key Shortcuts Tested

### Global Navigation
- `1` - Email view
- `2` - Calendar view  
- `3` - Contacts view
- `Ctrl+D` - Command palette
- `Ctrl+A` - Account manager
- `/` - Global search
- `F1` - Help

### Email Operations
- `r` - Mark as read
- `u` - Mark as unread
- `f` - Toggle flag
- `R` - Reply
- `Delete` - Delete email

### Calendar Operations
- `n` - New event
- `d` - Day view
- `w` - Week view
- `m` - Month view

### Universal
- Arrow keys - Navigation
- `Enter` - Select/Confirm
- `Escape` - Cancel/Back
- `Tab` - Next field

## 📊 Test Categories

### 1. Functionality Tests
- Application startup and shutdown
- Navigation between views
- Keyboard shortcut responsiveness
- Command palette operations
- Search functionality

### 2. Performance Tests
- Startup time measurement
- Memory usage monitoring
- UI responsiveness under load
- Rapid navigation testing

### 3. Reliability Tests
- Error handling validation
- Edge case scenarios
- Invalid input handling
- Recovery from failures

### 4. User Experience Tests
- Keyboard-only navigation
- Visual feedback validation
- Help system accessibility
- Workflow completeness

## 🛠️ Test Infrastructure

### Dependencies

The testing framework requires:

```toml
[dev-dependencies]
assert_cmd = "2.0"      # Command line testing
predicates = "3.0"      # Assertions for command testing
tempfile = "3.8"        # Temporary files for testing
serial_test = "3.0"     # Sequential test execution
proptest = "1.4"        # Property-based testing
```

### Optional External Tools

For complete testing functionality:

- **expect** - Interactive keyboard testing (Linux/macOS)
- **xdotool** - External terminal automation (Linux)
- **timeout** - Command timeout handling (Usually available)

Install on Ubuntu/Debian:
```bash
sudo apt-get install expect xdotool
```

Install on macOS:
```bash
brew install expect
```

## 📈 Performance Benchmarks

Expected performance targets:

- **Startup Time**: < 2 seconds
- **Memory Usage**: < 100MB during normal operation
- **Keyboard Response**: < 100ms latency
- **View Switching**: < 50ms transition time

## 🔍 Troubleshooting

### Common Issues

1. **Tests timeout**: Increase timeout in `TuiTestRunner::with_timeout()`
2. **Terminal size issues**: Ensure terminal is at least 120x40 characters
3. **External tool missing**: Install expect/xdotool for full functionality
4. **Permission errors**: Make scripts executable with `chmod +x`

### Debug Mode

Run tests with debug information:

```bash
RUST_LOG=debug cargo test tui_test_framework --lib
```

### Test Data Cleanup

Tests create temporary data in `/tmp/comunicado_test_data`. This is automatically cleaned up, but can be manually removed:

```bash
rm -rf /tmp/comunicado_test_data
```

## 🚦 Continuous Integration

For CI environments, use the automated test script:

```bash
# CI-friendly testing (no interactive components)
./scripts/run_tui_tests.sh --startup --performance
```

## 📝 Contributing

When adding new TUI features:

1. Add corresponding tests to `tui_test_framework.rs`
2. Update keyboard shortcut lists in this README
3. Add manual test scenarios to `run_manual_tests.sh`
4. Ensure all tests pass before committing

## 📚 Further Reading

- [Ratatui Testing Documentation](https://ratatui.rs/how-to/test/)
- [TUI Testing Best Practices](https://github.com/ratatui-org/ratatui/blob/main/docs/src/howto/test.md)
- [Crossterm Event Handling](https://docs.rs/crossterm/latest/crossterm/event/index.html)

---

**Note**: This testing framework is designed to provide comprehensive coverage of the Comunicado TUI interface while being maintainable and easy to extend as new features are added.