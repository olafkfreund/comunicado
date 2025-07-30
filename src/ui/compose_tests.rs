//! Tests for the ComposeUI component
//! Testing keyboard handling and basic functionality through public interface

use super::compose::*;
use crate::contacts::{ContactsDatabase, ContactsManager};
use crate::oauth2::TokenManager;
use crate::theme::Theme;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, layout::Rect, Terminal};
use std::sync::Arc;
use tokio;

/// Create a test contacts manager for testing
async fn create_test_contacts_manager() -> Arc<ContactsManager> {
    let db = ContactsDatabase::new_in_memory();
    let token_manager = Arc::new(TokenManager::new());
    ContactsManager::new(db, token_manager).await.expect("Failed to create contacts manager")
}

#[tokio::test]
async fn test_compose_ui_creation() {
    let contacts_manager = create_test_contacts_manager().await;
    let compose_ui = ComposeUI::new(contacts_manager);
    
    // Test that UI can be created and basic state is initialized
    assert!(!compose_ui.is_modified());
    assert_eq!(compose_ui.current_draft_id(), None);
}

#[tokio::test]
async fn test_compose_ui_reply_creation() {
    let contacts_manager = create_test_contacts_manager().await;
    let original_message = crate::email::StoredMessage {
        id: 1,
        subject: "Test Subject".to_string(),
        sender_name: "Test Sender".to_string(),
        sender_email: "sender@example.com".to_string(),
        preview: "Test preview".to_string(),
        body: "Test body".to_string(),
        date: chrono::Utc::now(),
        is_read: true,
        is_flagged: false,
        folder: "INBOX".to_string(),
        uid: 1,
        size: 100,
        has_attachments: false,
        thread_id: None,
    };

    let compose_ui = ComposeUI::new_reply(contacts_manager, &original_message);
    
    // Test that reply UI is created with proper initial state
    assert!(!compose_ui.is_modified());
}

#[tokio::test]
async fn test_compose_ui_forward_creation() {
    let contacts_manager = create_test_contacts_manager().await;
    let original_message = crate::email::StoredMessage {
        id: 1,
        subject: "Test Subject".to_string(),
        sender_name: "Test Sender".to_string(),
        sender_email: "sender@example.com".to_string(),
        preview: "Test preview".to_string(),
        body: "Test body".to_string(),
        date: chrono::Utc::now(),
        is_read: true,
        is_flagged: false,
        folder: "INBOX".to_string(),
        uid: 1,
        size: 100,
        has_attachments: false,
        thread_id: None,
    };

    let compose_ui = ComposeUI::new_forward(contacts_manager, &original_message);
    
    // Test that forward UI is created with proper initial state
    assert!(!compose_ui.is_modified());
}

#[tokio::test]
async fn test_keyboard_event_handling() {
    let contacts_manager = create_test_contacts_manager().await;
    let mut compose_ui = ComposeUI::new(contacts_manager);
    
    // Test basic keyboard shortcuts
    let cancel_key = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
    let action = compose_ui.handle_key(cancel_key).await;
    assert_eq!(action, ComposeAction::Cancel);
    
    let send_key = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL);
    let action = compose_ui.handle_key(send_key).await;
    assert_eq!(action, ComposeAction::Send);
    
    let draft_key = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL);
    let action = compose_ui.handle_key(draft_key).await;
    assert_eq!(action, ComposeAction::SaveDraft);
}

#[tokio::test]
async fn test_email_data_generation() {
    let contacts_manager = create_test_contacts_manager().await;
    let compose_ui = ComposeUI::new(contacts_manager);
    
    // Test email data generation
    let email_data = compose_ui.get_email_data();
    assert_eq!(email_data.to, "");
    assert_eq!(email_data.cc, "");
    assert_eq!(email_data.bcc, "");
    assert_eq!(email_data.subject, "");
    assert_eq!(email_data.body, "");
}

#[tokio::test]
async fn test_auto_save_functionality() {
    let contacts_manager = create_test_contacts_manager().await;
    let compose_ui = ComposeUI::new(contacts_manager);
    
    // Test auto-save properties
    assert!(!compose_ui.should_auto_save());
    assert_eq!(compose_ui.auto_save_interval_secs(), 30); // Default interval
    assert_eq!(compose_ui.check_auto_save(), None);
}

#[tokio::test]
async fn test_draft_management() {
    let contacts_manager = create_test_contacts_manager().await;
    let mut compose_ui = ComposeUI::new(contacts_manager);
    
    // Test draft ID management
    assert_eq!(compose_ui.current_draft_id(), None);
    
    compose_ui.set_current_draft_id(Some("test-draft-id".to_string()));
    assert_eq!(compose_ui.current_draft_id(), Some(&"test-draft-id".to_string()));
    
    compose_ui.set_current_draft_id(None);
    assert_eq!(compose_ui.current_draft_id(), None);
}

#[tokio::test]
async fn test_modification_tracking() {
    let contacts_manager = create_test_contacts_manager().await;
    let mut compose_ui = ComposeUI::new(contacts_manager);
    
    // Initially not modified
    assert!(!compose_ui.is_modified());
    
    // Clear modification flag
    compose_ui.clear_modified();
    assert!(!compose_ui.is_modified());
}

#[tokio::test]
async fn test_email_address_parsing() {
    let addresses = ComposeUI::parse_addresses("test1@example.com, test2@example.com");
    assert_eq!(addresses.len(), 2);
    assert_eq!(addresses[0], "test1@example.com");
    assert_eq!(addresses[1], "test2@example.com");
    
    let single_address = ComposeUI::parse_addresses("single@example.com");
    assert_eq!(single_address.len(), 1);
    assert_eq!(single_address[0], "single@example.com");
    
    let empty_addresses = ComposeUI::parse_addresses("");
    assert_eq!(empty_addresses.len(), 0);
}

#[tokio::test]
async fn test_rendering_without_panic() {
    let contacts_manager = create_test_contacts_manager().await;
    let mut compose_ui = ComposeUI::new(contacts_manager);
    let theme = Theme::default();
    
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    
    // Test that rendering doesn't panic
    terminal.draw(|f| {
        let area = Rect::new(0, 0, 80, 24);
        compose_ui.render(f, area, &theme);
    }).unwrap();
}

#[tokio::test]
async fn test_auto_save_interval_setting() {
    let contacts_manager = create_test_contacts_manager().await;
    let mut compose_ui = ComposeUI::new(contacts_manager);
    
    // Test setting auto-save interval
    compose_ui.set_auto_save_interval(60);
    assert_eq!(compose_ui.auto_save_interval_secs(), 60);
    
    compose_ui.set_auto_save_interval(120);
    assert_eq!(compose_ui.auto_save_interval_secs(), 120);
}

#[tokio::test]
async fn test_validation() {
    let contacts_manager = create_test_contacts_manager().await;
    let compose_ui = ComposeUI::new(contacts_manager);
    
    // Test validation of empty compose (should fail due to empty recipients)
    let validation_result = compose_ui.validate();
    assert!(validation_result.is_err());
}