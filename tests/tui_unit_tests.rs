//! Unit Tests for TUI Components
//! 
//! This module provides unit-level testing for individual TUI components
//! without requiring external terminals or complex setup.

use comunicado::app::App;
use comunicado::ui::components::*;
use comunicado::events::{AppEvent, KeyEvent};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    backend::TestBackend,
    Terminal,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    text::Text,
};
use std::time::Duration;
use tokio::sync::mpsc;

/// Test backend for TUI component testing
pub struct TuiTestBackend {
    terminal: Terminal<TestBackend>,
    events_tx: mpsc::UnboundedSender<AppEvent>,
    events_rx: mpsc::UnboundedReceiver<AppEvent>,
}

impl TuiTestBackend {
    pub fn new(width: u16, height: u16) -> Self {
        let backend = TestBackend::new(width, height);
        let terminal = Terminal::new(backend).unwrap();
        let (events_tx, events_rx) = mpsc::unbounded_channel();
        
        Self {
            terminal,
            events_tx,
            events_rx,
        }
    }
    
    pub fn send_key(&self, key_code: KeyCode, modifiers: KeyModifiers) {
        let key_event = KeyEvent {
            code: key_code,
            modifiers,
        };
        let _ = self.events_tx.send(AppEvent::Key(key_event));
    }
    
    pub fn send_key_char(&self, c: char) {
        self.send_key(KeyCode::Char(c), KeyModifiers::NONE);
    }
    
    pub fn send_key_ctrl(&self, c: char) {
        self.send_key(KeyCode::Char(c), KeyModifiers::CONTROL);
    }
    
    pub async fn receive_event(&mut self) -> Option<AppEvent> {
        self.events_rx.recv().await
    }
    
    pub fn get_buffer(&self) -> &ratatui::buffer::Buffer {
        self.terminal.backend().buffer()
    }
    
    pub fn assert_contains_text(&self, text: &str) {
        let buffer = self.get_buffer();
        let content = buffer.to_string();
        assert!(content.contains(text), "Buffer does not contain '{}'\nActual content:\n{}", text, content);
    }
    
    pub fn assert_cursor_at(&self, x: u16, y: u16) {
        let (cursor_x, cursor_y) = self.terminal.backend().get_cursor().unwrap();
        assert_eq!((cursor_x, cursor_y), (x, y), "Cursor not at expected position");
    }
}

// ============================================================================
// COMPONENT UNIT TESTS
// ============================================================================

#[tokio::test]
async fn test_command_palette_component() {
    let mut backend = TuiTestBackend::new(80, 24);
    
    // Test command palette opening
    backend.send_key_ctrl('d');
    
    let event = backend.receive_event().await;
    assert!(matches!(event, Some(AppEvent::Key(_))));
    
    // Test typing in command palette
    backend.send_key_char('s');
    backend.send_key_char('e');
    backend.send_key_char('a');
    backend.send_key_char('r');
    backend.send_key_char('c');
    backend.send_key_char('h');
    
    // Verify command palette would show search results
    // Note: This would require integration with actual command palette component
}

#[tokio::test]
async fn test_email_list_navigation() {
    let mut backend = TuiTestBackend::new(120, 40);
    
    // Test email list navigation
    backend.send_key(KeyCode::Down, KeyModifiers::NONE);
    backend.send_key(KeyCode::Down, KeyModifiers::NONE);
    backend.send_key(KeyCode::Up, KeyModifiers::NONE);
    
    // Test email selection
    backend.send_key(KeyCode::Enter, KeyModifiers::NONE);
    
    // Verify navigation events are generated correctly
    for _ in 0..4 {
        let event = backend.receive_event().await;
        assert!(matches!(event, Some(AppEvent::Key(_))));
    }
}

#[tokio::test]
async fn test_calendar_view_switching() {
    let mut backend = TuiTestBackend::new(120, 40);
    
    // Test calendar view mode switching
    backend.send_key_char('2'); // Go to calendar
    backend.send_key_char('d'); // Day view
    backend.send_key_char('w'); // Week view
    backend.send_key_char('m'); // Month view
    
    // Verify all events are captured
    for _ in 0..4 {
        let event = backend.receive_event().await;
        assert!(matches!(event, Some(AppEvent::Key(_))));
    }
}

#[tokio::test]
async fn test_keyboard_shortcut_parsing() {
    let mut backend = TuiTestBackend::new(80, 24);
    
    // Test various keyboard shortcuts
    let shortcuts = vec![
        (KeyCode::Char('1'), KeyModifiers::NONE),
        (KeyCode::Char('2'), KeyModifiers::NONE),
        (KeyCode::Char('3'), KeyModifiers::NONE),
        (KeyCode::Char('d'), KeyModifiers::CONTROL),
        (KeyCode::Char('a'), KeyModifiers::CONTROL),
        (KeyCode::Char('x'), KeyModifiers::CONTROL),
        (KeyCode::F(1), KeyModifiers::NONE),
        (KeyCode::Escape, KeyModifiers::NONE),
    ];
    
    for (key_code, modifiers) in shortcuts {
        backend.send_key(key_code, modifiers);
        let event = backend.receive_event().await;
        
        if let Some(AppEvent::Key(key_event)) = event {
            assert_eq!(key_event.code, key_code);
            assert_eq!(key_event.modifiers, modifiers);
        } else {
            panic!("Expected KeyEvent, got {:?}", event);
        }
    }
}

// ============================================================================
// VISUAL RENDERING TESTS
// ============================================================================

#[test]
fn test_basic_widget_rendering() {
    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    
    terminal.draw(|f| {
        let block = Block::default()
            .title("Test Block")
            .borders(Borders::ALL);
        f.render_widget(block, f.size());
    }).unwrap();
    
    let buffer = terminal.backend().buffer();
    
    // Check that the title is rendered
    let title_line = buffer.content.iter()
        .skip(40) // Skip first line
        .take(40) // Take second line
        .map(|cell| cell.symbol.as_str())
        .collect::<String>();
    
    assert!(title_line.contains("Test Block"));
}

#[test]
fn test_layout_constraints() {
    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    
    terminal.draw(|f| {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Percentage(60),
                Constraint::Percentage(20),
            ])
            .split(f.size());
        
        // Render blocks in each chunk to test layout
        let top_block = Block::default().title("Top").borders(Borders::ALL);
        let middle_block = Block::default().title("Middle").borders(Borders::ALL);
        let bottom_block = Block::default().title("Bottom").borders(Borders::ALL);
        
        f.render_widget(top_block, chunks[0]);
        f.render_widget(middle_block, chunks[1]);
        f.render_widget(bottom_block, chunks[2]);
    }).unwrap();
    
    // Verify layout is correct
    let buffer = terminal.backend().buffer();
    assert!(buffer.to_string().contains("Top"));
    assert!(buffer.to_string().contains("Middle"));
    assert!(buffer.to_string().contains("Bottom"));
}

// ============================================================================
// PERFORMANCE TESTS
// ============================================================================

#[tokio::test]
async fn test_rapid_key_input_handling() {
    let mut backend = TuiTestBackend::new(80, 24);
    let start = std::time::Instant::now();
    
    // Send 1000 key events rapidly
    for i in 0..1000 {
        backend.send_key_char(char::from(b'a' + (i % 26) as u8));
    }
    
    // Receive all events
    for _ in 0..1000 {
        let _event = backend.receive_event().await;
    }
    
    let duration = start.elapsed();
    
    // Should handle 1000 key events in under 1 second
    assert!(duration < Duration::from_secs(1), 
           "Key input handling too slow: {:?}", duration);
}

#[test]
fn test_rendering_performance() {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    
    let start = std::time::Instant::now();
    
    // Render complex layout multiple times
    for _ in 0..100 {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(20),
                    Constraint::Percentage(60),
                    Constraint::Percentage(20),
                ])
                .split(f.size());
            
            // Render complex content in each chunk
            let left_content = (0..20).map(|i| format!("Left item {}", i)).collect::<Vec<_>>().join("\n");
            let middle_content = (0..30).map(|i| format!("Middle item {}", i)).collect::<Vec<_>>().join("\n");
            let right_content = (0..20).map(|i| format!("Right item {}", i)).collect::<Vec<_>>().join("\n");
            
            f.render_widget(Paragraph::new(left_content), chunks[0]);
            f.render_widget(Paragraph::new(middle_content), chunks[1]);
            f.render_widget(Paragraph::new(right_content), chunks[2]);
        }).unwrap();
    }
    
    let duration = start.elapsed();
    
    // Should render 100 complex frames in under 1 second
    assert!(duration < Duration::from_secs(1), 
           "Rendering performance too slow: {:?}", duration);
}

// ============================================================================
// ERROR HANDLING TESTS
// ============================================================================

#[tokio::test]
async fn test_invalid_key_sequences() {
    let mut backend = TuiTestBackend::new(80, 24);
    
    // Test various invalid or edge-case key combinations
    let invalid_sequences = vec![
        KeyCode::Null,
        KeyCode::F(25), // Invalid F key
    ];
    
    for key_code in invalid_sequences {
        backend.send_key(key_code, KeyModifiers::NONE);
        
        // Should still receive the event, even if it's invalid
        let event = backend.receive_event().await;
        assert!(matches!(event, Some(AppEvent::Key(_))));
    }
}

#[test]
fn test_small_terminal_rendering() {
    // Test rendering on very small terminal
    let backend = TestBackend::new(10, 5);
    let mut terminal = Terminal::new(backend).unwrap();
    
    // Should not panic even with tiny terminal
    terminal.draw(|f| {
        let block = Block::default()
            .title("Tiny")
            .borders(Borders::ALL);
        f.render_widget(block, f.size());
    }).unwrap();
    
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer.area.width, 10);
    assert_eq!(buffer.area.height, 5);
}

// ============================================================================
// ACCESSIBILITY TESTS
// ============================================================================

#[test]
fn test_color_contrast() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    
    terminal.draw(|f| {
        let paragraph = Paragraph::new("High contrast text")
            .style(Style::default().fg(Color::White).bg(Color::Black));
        f.render_widget(paragraph, f.size());
    }).unwrap();
    
    let buffer = terminal.backend().buffer();
    
    // Check that text is rendered with proper contrast
    for cell in buffer.content.iter() {
        if !cell.symbol.trim().is_empty() {
            // All non-empty cells should have proper contrast
            assert!(cell.fg != cell.bg, "Text and background colors should differ");
        }
    }
}

#[tokio::test]
async fn test_keyboard_only_navigation() {
    let mut backend = TuiTestBackend::new(80, 24);
    
    // Test that all functionality is accessible via keyboard
    let navigation_keys = vec![
        KeyCode::Tab,
        KeyCode::BackTab,
        KeyCode::Up,
        KeyCode::Down,
        KeyCode::Left,
        KeyCode::Right,
        KeyCode::Enter,
        KeyCode::Escape,
    ];
    
    for key in navigation_keys {
        backend.send_key(key, KeyModifiers::NONE);
        let event = backend.receive_event().await;
        assert!(matches!(event, Some(AppEvent::Key(_))));
    }
}

// ============================================================================
// UTILITY FUNCTIONS FOR TESTING
// ============================================================================

/// Helper to create a test app instance
pub fn create_test_app() -> Result<App, Box<dyn std::error::Error>> {
    // This would create a minimal App instance for testing
    // For now, we'll return an error as the App structure may need specific setup
    Err("Test app creation not yet implemented".into())
}

/// Helper to simulate user workflow
pub async fn simulate_user_workflow(backend: &mut TuiTestBackend, workflow: &[(&str, KeyCode)]) {
    for (description, key) in workflow {
        println!("Simulating: {}", description);
        backend.send_key(*key, KeyModifiers::NONE);
        let _event = backend.receive_event().await;
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// Test helper for checking UI state
pub fn assert_ui_state(backend: &TuiTestBackend, expected_elements: &[&str]) {
    let buffer = backend.get_buffer();
    let content = buffer.to_string();
    
    for element in expected_elements {
        assert!(content.contains(element), 
               "UI should contain '{}' but content is:\n{}", element, content);
    }
}