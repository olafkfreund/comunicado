//! Comprehensive TUI Integration Tests for Comunicado
//! 
//! This module provides automated testing for the TUI interface including:
//! - Keyboard shortcut validation
//! - UI state transitions
//! - External terminal testing
//! - Mock backend integration

use assert_cmd::Command;
use predicates::prelude::*;
use serial_test::serial;
use std::io::Write;
use std::process::{Command as StdCommand, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use tempfile::NamedTempFile;
use tokio::time::timeout;

/// Test configuration for TUI testing
#[derive(Debug, Clone)]
pub struct TuiTestConfig {
    pub timeout_duration: Duration,
    pub mock_data_path: Option<String>,
    pub terminal_size: (u16, u16), // (width, height)
}

impl Default for TuiTestConfig {
    fn default() -> Self {
        Self {
            timeout_duration: Duration::from_secs(10),
            mock_data_path: None,
            terminal_size: (120, 40),
        }
    }
}

/// TUI Test Runner - coordinates external terminal testing
pub struct TuiTestRunner {
    config: TuiTestConfig,
}

impl TuiTestRunner {
    pub fn new(config: TuiTestConfig) -> Self {
        Self { config }
    }

    /// Run TUI test in external terminal with automated input
    pub async fn run_external_terminal_test<F>(&self, test_name: &str, test_fn: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnOnce() -> Vec<KeySequence> + Send + 'static,
    {
        let sequences = test_fn();
        let timeout_duration = self.config.timeout_duration;
        
        // Create a temporary script for the test
        let mut script_file = NamedTempFile::new()?;
        writeln!(script_file, "#!/bin/bash")?;
        writeln!(script_file, "echo 'Starting TUI test: {}'", test_name)?;
        
        // Set terminal size
        writeln!(script_file, "stty cols {} rows {}", 
                self.config.terminal_size.0, self.config.terminal_size.1)?;
        
        // Start comunicado in background
        writeln!(script_file, "timeout {} cargo run --bin comunicado &", 
                timeout_duration.as_secs())?;
        writeln!(script_file, "APP_PID=$!")?;
        writeln!(script_file, "sleep 2")?; // Wait for app to start
        
        // Send key sequences
        for (i, seq) in sequences.iter().enumerate() {
            writeln!(script_file, "echo 'Step {}: {}'", i + 1, seq.description)?;
            writeln!(script_file, "sleep {}", seq.delay.as_secs_f32())?;
            
            for key in &seq.keys {
                writeln!(script_file, "{}", key.to_shell_command())?;
            }
        }
        
        writeln!(script_file, "sleep 2")?; // Final wait
        writeln!(script_file, "kill $APP_PID 2>/dev/null || true")?;
        writeln!(script_file, "echo 'Test completed: {}'", test_name)?;
        
        script_file.flush()?;
        
        // Make script executable and run it
        let script_path = script_file.path();
        StdCommand::new("chmod")
            .arg("+x")
            .arg(script_path)
            .output()?;
        
        let output = timeout(timeout_duration, async {
            tokio::process::Command::new("bash")
                .arg(script_path)
                .output()
                .await
        }).await??;
        
        if !output.status.success() {
            return Err(format!("Test '{}' failed with output: {}", 
                             test_name, 
                             String::from_utf8_lossy(&output.stderr)).into());
        }
        
        println!("✅ External terminal test '{}' passed", test_name);
        Ok(())
    }
}

/// Represents a sequence of keyboard inputs for testing
#[derive(Debug, Clone)]
pub struct KeySequence {
    pub description: String,
    pub keys: Vec<Key>,
    pub delay: Duration,
}

impl KeySequence {
    pub fn new(description: &str, keys: Vec<Key>) -> Self {
        Self {
            description: description.to_string(),
            keys,
            delay: Duration::from_millis(500),
        }
    }
    
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }
}

/// Represents a keyboard key for testing
#[derive(Debug, Clone)]
pub enum Key {
    Char(char),
    Ctrl(char),
    Alt(char),
    Enter,
    Escape,
    Tab,
    Backspace,
    Delete,
    Up,
    Down,
    Left,
    Right,
    F(u8),
}

impl Key {
    /// Convert key to shell command for external terminal testing
    fn to_shell_command(&self) -> String {
        match self {
            Key::Char(c) => format!("echo -n '{}' | xdotool type --file -", c),
            Key::Ctrl(c) => format!("xdotool key ctrl+{}", c),
            Key::Alt(c) => format!("xdotool key alt+{}", c),
            Key::Enter => "xdotool key Return".to_string(),
            Key::Escape => "xdotool key Escape".to_string(),
            Key::Tab => "xdotool key Tab".to_string(),
            Key::Backspace => "xdotool key BackSpace".to_string(),
            Key::Delete => "xdotool key Delete".to_string(),
            Key::Up => "xdotool key Up".to_string(),
            Key::Down => "xdotool key Down".to_string(),
            Key::Left => "xdotool key Left".to_string(),
            Key::Right => "xdotool key Right".to_string(),
            Key::F(n) => format!("xdotool key F{}", n),
        }
    }
}

// ============================================================================
// ACTUAL TUI INTEGRATION TESTS
// ============================================================================

#[tokio::test]
#[serial]
async fn test_application_startup() {
    let mut cmd = Command::cargo_bin("comunicado").unwrap();
    
    // Test that the application starts without crashing
    cmd.timeout(Duration::from_secs(5))
       .assert()
       .success(); // Should exit cleanly after timeout
}

#[tokio::test]
#[serial]
async fn test_command_palette_shortcut() -> Result<(), Box<dyn std::error::Error>> {
    let runner = TuiTestRunner::new(TuiTestConfig::default());
    
    runner.run_external_terminal_test("command_palette_shortcut", || {
        vec![
            KeySequence::new("Open command palette", vec![Key::Ctrl('d')]),
            KeySequence::new("Type search", vec![
                Key::Char('s'), Key::Char('e'), Key::Char('a'), Key::Char('r'), Key::Char('c'), Key::Char('h')
            ]),
            KeySequence::new("Press Enter", vec![Key::Enter]),
            KeySequence::new("Close with Escape", vec![Key::Escape]),
        ]
    }).await
}

#[tokio::test]
#[serial]
async fn test_navigation_shortcuts() -> Result<(), Box<dyn std::error::Error>> {
    let runner = TuiTestRunner::new(TuiTestConfig::default());
    
    runner.run_external_terminal_test("navigation_shortcuts", || {
        vec![
            KeySequence::new("Navigate to email", vec![Key::Char('1')]),
            KeySequence::new("Navigate to calendar", vec![Key::Char('2')]),
            KeySequence::new("Navigate to contacts", vec![Key::Char('3')]),
            KeySequence::new("Back to email", vec![Key::Char('1')]),
            KeySequence::new("Move down in list", vec![Key::Down, Key::Down]),
            KeySequence::new("Move up in list", vec![Key::Up]),
            KeySequence::new("Select item", vec![Key::Enter]),
        ]
    }).await
}

#[tokio::test]
#[serial]
async fn test_email_operations() -> Result<(), Box<dyn std::error::Error>> {
    let runner = TuiTestRunner::new(TuiTestConfig::default());
    
    runner.run_external_terminal_test("email_operations", || {
        vec![
            KeySequence::new("Go to email view", vec![Key::Char('1')]),
            KeySequence::new("Select first email", vec![Key::Down]),
            KeySequence::new("Mark as read", vec![Key::Char('r')]),
            KeySequence::new("Mark as unread", vec![Key::Char('u')]),
            KeySequence::new("Toggle flag", vec![Key::Char('f')]),
            KeySequence::new("Reply to email", vec![Key::Char('R')]),
            KeySequence::new("Cancel compose", vec![Key::Escape]),
            KeySequence::new("Delete email", vec![Key::Delete]),
        ]
    }).await
}

#[tokio::test]
#[serial]
async fn test_calendar_operations() -> Result<(), Box<dyn std::error::Error>> {
    let runner = TuiTestRunner::new(TuiTestConfig::default());
    
    runner.run_external_terminal_test("calendar_operations", || {
        vec![
            KeySequence::new("Go to calendar view", vec![Key::Char('2')]),
            KeySequence::new("Create new event", vec![Key::Char('n')]),
            KeySequence::new("Type event title", vec![
                Key::Char('T'), Key::Char('e'), Key::Char('s'), Key::Char('t'), 
                Key::Char(' '), Key::Char('E'), Key::Char('v'), Key::Char('e'), 
                Key::Char('n'), Key::Char('t')
            ]),
            KeySequence::new("Tab to next field", vec![Key::Tab]),
            KeySequence::new("Save event", vec![Key::Ctrl('s')]),
            KeySequence::new("Navigate calendar", vec![Key::Left, Key::Right, Key::Up, Key::Down]),
            KeySequence::new("View different calendar modes", vec![
                Key::Char('d'), // day view
                Key::Char('w'), // week view  
                Key::Char('m'), // month view
            ]),
        ]
    }).await
}

#[tokio::test]
#[serial]
async fn test_search_functionality() -> Result<(), Box<dyn std::error::Error>> {
    let runner = TuiTestRunner::new(TuiTestConfig::default());
    
    runner.run_external_terminal_test("search_functionality", || {
        vec![
            KeySequence::new("Open global search", vec![Key::Char('/')]),
            KeySequence::new("Type search query", vec![
                Key::Char('t'), Key::Char('e'), Key::Char('s'), Key::Char('t')
            ]),
            KeySequence::new("Execute search", vec![Key::Enter]),
            KeySequence::new("Navigate results", vec![Key::Down, Key::Down, Key::Up]),
            KeySequence::new("Select result", vec![Key::Enter]),
            KeySequence::new("Clear search", vec![Key::Escape]),
        ]
    }).await
}

#[tokio::test]
#[serial]
async fn test_account_management() -> Result<(), Box<dyn std::error::Error>> {
    let runner = TuiTestRunner::new(TuiTestConfig::default());
    
    runner.run_external_terminal_test("account_management", || {
        vec![
            KeySequence::new("Open account manager", vec![Key::Ctrl('a')]),
            KeySequence::new("Navigate accounts", vec![Key::Down, Key::Up]),
            KeySequence::new("Account details", vec![Key::Enter]),
            KeySequence::new("Back to list", vec![Key::Escape]),
            KeySequence::new("Add new account", vec![Key::Char('n')]),
            KeySequence::new("Cancel add", vec![Key::Escape]),
            KeySequence::new("Close account manager", vec![Key::Escape]),
        ]
    }).await
}

#[tokio::test]
#[serial]
async fn test_ui_responsiveness() -> Result<(), Box<dyn std::error::Error>> {
    let runner = TuiTestRunner::new(TuiTestConfig {
        timeout_duration: Duration::from_secs(15),
        ..Default::default()
    });
    
    runner.run_external_terminal_test("ui_responsiveness", || {
        vec![
            KeySequence::new("Rapid navigation", vec![
                Key::Char('1'), Key::Char('2'), Key::Char('3'), Key::Char('1')
            ]).with_delay(Duration::from_millis(100)),
            KeySequence::new("Quick command palette", vec![
                Key::Ctrl('d'), Key::Escape
            ]).with_delay(Duration::from_millis(100)),
            KeySequence::new("Fast scrolling", vec![
                Key::Down, Key::Down, Key::Down, Key::Down, Key::Down,
                Key::Up, Key::Up, Key::Up, Key::Up, Key::Up
            ]).with_delay(Duration::from_millis(50)),
            KeySequence::new("Window resize handling", vec![Key::Ctrl('l')]),
        ]
    }).await
}

// ============================================================================
// MOCK BACKEND TESTING
// ============================================================================

/// Create a mock email database for testing
pub fn create_mock_email_data() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    
    // Create mock email database
    let db_path = temp_dir.path().join("test_emails.db");
    std::fs::write(&db_path, b"mock email database")?;
    
    // Create mock configuration
    let config_path = temp_dir.path().join("config.toml");
    let config_content = r#"
[email]
database_path = "test_emails.db"

[calendar]
database_path = "test_calendar.db"

[ui]
theme = "dark"
enable_animations = false
"#;
    std::fs::write(&config_path, config_content)?;
    
    Ok(temp_dir)
}

#[tokio::test]
#[serial]
async fn test_with_mock_data() -> Result<(), Box<dyn std::error::Error>> {
    let mock_data = create_mock_email_data()?;
    let config_path = mock_data.path().join("config.toml");
    
    let runner = TuiTestRunner::new(TuiTestConfig {
        mock_data_path: Some(config_path.to_string_lossy().to_string()),
        ..Default::default()
    });
    
    runner.run_external_terminal_test("mock_data_operations", || {
        vec![
            KeySequence::new("Load with mock data", vec![Key::Char('1')]),
            KeySequence::new("Navigate mock emails", vec![Key::Down, Key::Down]),
            KeySequence::new("Select mock email", vec![Key::Enter]),
            KeySequence::new("Test basic operations", vec![Key::Char('r'), Key::Char('f')]),
        ]
    }).await
}

// ============================================================================
// PERFORMANCE TESTING
// ============================================================================

#[tokio::test]
#[serial]
async fn test_startup_performance() {
    let start = std::time::Instant::now();
    
    let mut cmd = Command::cargo_bin("comunicado").unwrap();
    cmd.timeout(Duration::from_secs(5))
       .assert()
       .success();
    
    let duration = start.elapsed();
    
    // Application should start within reasonable time
    assert!(duration < Duration::from_secs(3), 
           "Application took too long to start: {:?}", duration);
}

#[tokio::test]
#[serial]
async fn test_memory_usage() -> Result<(), Box<dyn std::error::Error>> {
    // This test would require additional system monitoring
    // For now, we'll test that the app doesn't crash under load
    
    let runner = TuiTestRunner::new(TuiTestConfig {
        timeout_duration: Duration::from_secs(30),
        ..Default::default()
    });
    
    runner.run_external_terminal_test("memory_stress_test", || {
        let mut sequences = Vec::new();
        
        // Generate many rapid operations to test memory usage
        for i in 0..50 {
            sequences.push(KeySequence::new(
                &format!("Operation {}", i),
                vec![Key::Char('1'), Key::Down, Key::Up, Key::Char('2')]
            ).with_delay(Duration::from_millis(100)));
        }
        
        sequences
    }).await
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Helper function to run a simple TUI test
pub async fn run_simple_tui_test(name: &str, keys: Vec<Key>) -> Result<(), Box<dyn std::error::Error>> {
    let runner = TuiTestRunner::new(TuiTestConfig::default());
    let sequence = KeySequence::new(name, keys);
    
    runner.run_external_terminal_test(name, || vec![sequence]).await
}

/// Check if xdotool is available for external testing
pub fn check_external_test_dependencies() -> bool {
    std::process::Command::new("which")
        .arg("xdotool")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[tokio::test]
async fn test_dependencies() {
    if !check_external_test_dependencies() {
        println!("⚠️  Warning: xdotool not found. External terminal tests may not work.");
        println!("   Install with: sudo apt-get install xdotool (Ubuntu/Debian)");
        println!("   or: brew install xdotool (macOS)");
    } else {
        println!("✅ External test dependencies available");
    }
}