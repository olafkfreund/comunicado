//! Comprehensive tests for the ComposeUI component
//! Testing keyboard handling, field validation, contact autocomplete integration

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
    let database = ContactsDatabase::new_in_memory().await.unwrap();
    let token_manager = TokenManager::new();
    let manager = ContactsManager::new(database, token_manager).await.unwrap();
    Arc::new(manager)
}

#[tokio::test]
async fn test_compose_ui_creation() {
    let contacts_manager = create_test_contacts_manager().await;
    let compose_ui = ComposeUI::new(contacts_manager);
    
    assert_eq!(compose_ui.current_field, ComposeField::To);
    assert!(!compose_ui.is_modified());
    assert_eq!(compose_ui.to_field, "");
    assert_eq!(compose_ui.subject_field, "");
    assert_eq!(compose_ui.body_lines, vec![String::new()]);
}

#[tokio::test]
async fn test_compose_ui_reply_creation() {
    let contacts_manager = create_test_contacts_manager().await;
    let compose_ui = ComposeUI::new_reply(
        contacts_manager,
        "sender@example.com",
        "Original Subject"
    );
    
    assert_eq!(compose_ui.to_field, "sender@example.com");
    assert_eq!(compose_ui.subject_field, "Re: Original Subject");
    assert_eq!(compose_ui.current_field, ComposeField::Body);
    
    // Test that "Re: " prefix is not duplicated
    let compose_ui2 = ComposeUI::new_reply(
        create_test_contacts_manager().await,
        "sender@example.com",
        "Re: Already prefixed"
    );
    assert_eq!(compose_ui2.subject_field, "Re: Already prefixed");
}

#[tokio::test]
async fn test_compose_ui_forward_creation() {
    let contacts_manager = create_test_contacts_manager().await;
    let original_body = "This is the original message body.";
    let compose_ui = ComposeUI::new_forward(
        contacts_manager,
        "Original Subject",
        original_body
    );
    
    assert_eq!(compose_ui.subject_field, "Fwd: Original Subject");
    assert!(compose_ui.body_text.contains("--- Forwarded Message ---"));
    assert!(compose_ui.body_text.contains(original_body));
    assert_eq!(compose_ui.current_field, ComposeField::To);
}

#[tokio::test]
async fn test_field_navigation() {
    let contacts_manager = create_test_contacts_manager().await;
    let mut compose_ui = ComposeUI::new(contacts_manager);
    
    // Test forward navigation
    assert_eq!(compose_ui.current_field, ComposeField::To);
    
    compose_ui.next_field();
    assert_eq!(compose_ui.current_field, ComposeField::Cc);
    
    compose_ui.next_field();
    assert_eq!(compose_ui.current_field, ComposeField::Bcc);
    
    compose_ui.next_field();
    assert_eq!(compose_ui.current_field, ComposeField::Subject);
    
    compose_ui.next_field();
    assert_eq!(compose_ui.current_field, ComposeField::Body);
    
    compose_ui.next_field();
    assert_eq!(compose_ui.current_field, ComposeField::To); // Wraps around
    
    // Test backward navigation
    compose_ui.previous_field();
    assert_eq!(compose_ui.current_field, ComposeField::Body);
    
    compose_ui.previous_field();
    assert_eq!(compose_ui.current_field, ComposeField::Subject);
}

#[tokio::test]
async fn test_text_input_and_editing() {
    let contacts_manager = create_test_contacts_manager().await;
    let mut compose_ui = ComposeUI::new(contacts_manager);
    
    // Test character insertion in To field
    compose_ui.insert_char('t');
    compose_ui.insert_char('e');
    compose_ui.insert_char('s');
    compose_ui.insert_char('t');
    compose_ui.insert_char('@');
    compose_ui.insert_char('e');
    compose_ui.insert_char('x');
    compose_ui.insert_char('a');
    compose_ui.insert_char('m');
    compose_ui.insert_char('p');
    compose_ui.insert_char('l');
    compose_ui.insert_char('e');
    compose_ui.insert_char('.');
    compose_ui.insert_char('c');
    compose_ui.insert_char('o');
    compose_ui.insert_char('m');
    
    assert_eq!(compose_ui.to_field, "test@example.com");
    assert_eq!(compose_ui.to_cursor, 16);
    assert!(compose_ui.is_modified());
    
    // Test backspace
    compose_ui.delete_char();
    compose_ui.delete_char();
    compose_ui.delete_char();
    assert_eq!(compose_ui.to_field, "test@example.");
    assert_eq!(compose_ui.to_cursor, 13);
    
    // Test cursor movement
    compose_ui.move_cursor_left();
    compose_ui.move_cursor_left();
    assert_eq!(compose_ui.to_cursor, 11);
    
    compose_ui.move_cursor_right();
    assert_eq!(compose_ui.to_cursor, 12);
}

#[tokio::test]
async fn test_body_text_handling() {
    let contacts_manager = create_test_contacts_manager().await;
    let mut compose_ui = ComposeUI::new(contacts_manager);
    
    // Move to body field
    compose_ui.current_field = ComposeField::Body;
    
    // Test multi-line text input
    compose_ui.insert_char('H');
    compose_ui.insert_char('e');
    compose_ui.insert_char('l');
    compose_ui.insert_char('l');
    compose_ui.insert_char('o');
    
    assert_eq!(compose_ui.body_lines[0], "Hello");
    assert_eq!(compose_ui.body_cursor, 5);
    
    // Test newline insertion
    compose_ui.insert_newline();
    assert_eq!(compose_ui.body_lines.len(), 2);
    assert_eq!(compose_ui.body_line_index, 1);
    assert_eq!(compose_ui.body_cursor, 0);
    assert_eq!(compose_ui.body_lines[0], "Hello");
    assert_eq!(compose_ui.body_lines[1], "");
    
    // Test text on second line
    compose_ui.insert_char('W');
    compose_ui.insert_char('o');
    compose_ui.insert_char('r');
    compose_ui.insert_char('l');
    compose_ui.insert_char('d');
    
    assert_eq!(compose_ui.body_lines[1], "World");
    
    // Test cursor movement between lines
    compose_ui.move_cursor_up();
    assert_eq!(compose_ui.body_line_index, 0);
    assert_eq!(compose_ui.body_cursor, 5); // Should be at end of "Hello"
    
    compose_ui.move_cursor_down();
    assert_eq!(compose_ui.body_line_index, 1);
    assert_eq!(compose_ui.body_cursor, 5); // Should maintain cursor position
}

#[tokio::test]
async fn test_email_data_generation() {
    let contacts_manager = create_test_contacts_manager().await;
    let mut compose_ui = ComposeUI::new(contacts_manager);
    
    // Set up test data
    compose_ui.to_field = "recipient@example.com".to_string();
    compose_ui.cc_field = "cc@example.com".to_string();
    compose_ui.subject_field = "Test Subject".to_string();
    compose_ui.body_lines = vec![
        "Line 1".to_string(),
        "Line 2".to_string(),
        "Line 3".to_string(),
    ];
    
    let email_data = compose_ui.get_email_data();
    
    assert_eq!(email_data.to, "recipient@example.com");
    assert_eq!(email_data.cc, "cc@example.com");
    assert_eq!(email_data.subject, "Test Subject");
    assert_eq!(email_data.body, "Line 1\nLine 2\nLine 3");
}

#[tokio::test]
async fn test_keyboard_event_handling() {
    let contacts_manager = create_test_contacts_manager().await;
    let mut compose_ui = ComposeUI::new(contacts_manager);
    
    // Test Tab key for field navigation
    let action = compose_ui.handle_key(KeyCode::Tab).await;
    assert_eq!(action, ComposeAction::Continue);
    assert_eq!(compose_ui.current_field, ComposeField::Cc);
    
    // Test Escape key
    let action = compose_ui.handle_key(KeyCode::Esc).await;
    assert_eq!(action, ComposeAction::Cancel);
    
    // Test F1 key for send
    let action = compose_ui.handle_key(KeyCode::F(1)).await;
    assert_eq!(action, ComposeAction::Send);
    
    // Test F2 key for save draft
    let action = compose_ui.handle_key(KeyCode::F(2)).await;
    assert_eq!(action, ComposeAction::SaveDraft);
    
    // Test character input
    let action = compose_ui.handle_key(KeyCode::Char('a')).await;
    assert_eq!(action, ComposeAction::Continue);
    assert_eq!(compose_ui.cc_field, "a");
}

#[tokio::test]
async fn test_autocomplete_integration() {
    let contacts_manager = create_test_contacts_manager().await;
    let mut compose_ui = ComposeUI::new(contacts_manager);
    
    // Test autocomplete trigger
    compose_ui.current_field = ComposeField::To;
    compose_ui.to_field = "test".to_string();
    compose_ui.to_cursor = 4;
    
    // Trigger autocomplete
    compose_ui.update_autocomplete().await;
    
    // Autocomplete should be triggered for fields with 2+ characters
    // (actual contact matching would require database setup)
    
    // Test @ symbol for contact lookup trigger
    let key_event = crossterm::event::KeyEvent::new(KeyCode::Char('@'), crossterm::event::KeyModifiers::empty());
    let action = compose_ui.handle_key(key_event).await;
    assert_eq!(action, ComposeAction::Continue);
    assert_eq!(compose_ui.to_field, "test@");
}

#[tokio::test]
async fn test_auto_save_functionality() {
    let contacts_manager = create_test_contacts_manager().await;
    let mut compose_ui = ComposeUI::new(contacts_manager);
    
    // Initially should not need auto-save
    assert!(!compose_ui.should_auto_save());
    
    // Make some changes
    compose_ui.insert_char('t');
    assert!(compose_ui.is_modified());
    
    // Set a very short auto-save interval for testing
    compose_ui.set_auto_save_interval(0);
    
    // Should now trigger auto-save
    assert!(compose_ui.should_auto_save());
    
    // Mark as auto-saved
    compose_ui.mark_auto_saved();
    assert!(!compose_ui.should_auto_save());
    
    // Test auto-save action generation
    let action = compose_ui.check_auto_save();
    assert!(action.is_none()); // No auto-save needed after marking as saved
}

#[tokio::test]
async fn test_draft_loading() {
    let contacts_manager = create_test_contacts_manager().await;
    let mut compose_ui = ComposeUI::new(contacts_manager);
    
    // Create test draft data
    let draft_data = EmailComposeData {
        to: "draft@example.com".to_string(),
        cc: "cc@example.com".to_string(),
        bcc: "bcc@example.com".to_string(),
        subject: "Draft Subject".to_string(),
        body: "Draft body\nSecond line".to_string(),
    };
    
    let draft_id = "test-draft-123".to_string();
    
    // Load the draft
    compose_ui.load_from_draft(draft_data, draft_id.clone());
    
    // Verify all fields are loaded correctly
    let email_data = compose_ui.get_email_data();
    assert_eq!(email_data.to, "draft@example.com");
    assert_eq!(email_data.cc, "cc@example.com");
    assert_eq!(email_data.bcc, "bcc@example.com");
    assert_eq!(email_data.subject, "Draft Subject");
    assert_eq!(email_data.body, "Draft body\nSecond line");
    assert_eq!(compose_ui.current_draft_id(), Some(&draft_id));
    assert!(!compose_ui.is_modified()); // Should not be marked as modified after loading
}

#[tokio::test]
async fn test_email_compose_data_validation() {
    // Test valid email data
    let valid_data = EmailComposeData {
        to: "valid@example.com".to_string(),
        cc: "".to_string(),
        bcc: "".to_string(),
        subject: "Valid Subject".to_string(),
        body: "Valid body".to_string(),
    };
    
    assert!(valid_data.validate().is_ok());
    
    // Test missing To field
    let invalid_to = EmailComposeData {
        to: "".to_string(),
        cc: "".to_string(),
        bcc: "".to_string(),
        subject: "Subject".to_string(),
        body: "Body".to_string(),
    };
    
    assert!(invalid_to.validate().is_err());
    assert_eq!(invalid_to.validate().unwrap_err(), "To field is required");
    
    // Test missing subject
    let invalid_subject = EmailComposeData {
        to: "test@example.com".to_string(),
        cc: "".to_string(),
        bcc: "".to_string(),
        subject: "".to_string(),
        body: "Body".to_string(),
    };
    
    assert!(invalid_subject.validate().is_err());
    assert_eq!(invalid_subject.validate().unwrap_err(), "Subject is required");
    
    // Test invalid email address
    let invalid_email = EmailComposeData {
        to: "invalid-email".to_string(),
        cc: "".to_string(),
        bcc: "".to_string(),
        subject: "Subject".to_string(),
        body: "Body".to_string(),
    };
    
    assert!(invalid_email.validate().is_err());
    assert!(invalid_email.validate().unwrap_err().contains("Invalid email address"));
}

#[tokio::test]
async fn test_email_address_parsing() {
    // Test simple email
    let addresses = EmailComposeData::parse_addresses("test@example.com");
    assert_eq!(addresses, vec!["test@example.com"]);
    
    // Test multiple emails
    let addresses = EmailComposeData::parse_addresses("first@example.com, second@example.com");
    assert_eq!(addresses, vec!["first@example.com", "second@example.com"]);
    
    // Test "Name <email>" format
    let addresses = EmailComposeData::parse_addresses("John Doe <john@example.com>");
    assert_eq!(addresses, vec!["john@example.com"]);
    
    // Test mixed formats
    let addresses = EmailComposeData::parse_addresses("john@example.com, Jane Doe <jane@example.com>, bob@example.com");
    assert_eq!(addresses, vec!["john@example.com", "jane@example.com", "bob@example.com"]);
    
    // Test empty and whitespace
    let addresses = EmailComposeData::parse_addresses("");
    assert_eq!(addresses, Vec::<String>::new());
    
    let addresses = EmailComposeData::parse_addresses("  ,  ,  ");
    assert_eq!(addresses, Vec::<String>::new());
}

#[tokio::test]
async fn test_recipient_aggregation() {
    let data = EmailComposeData {
        to: "to1@example.com, to2@example.com".to_string(),
        cc: "cc1@example.com".to_string(),
        bcc: "bcc1@example.com, bcc2@example.com".to_string(),
        subject: "Test".to_string(),
        body: "Test body".to_string(),
    };
    
    let all_recipients = data.get_all_recipients();
    assert_eq!(all_recipients.len(), 5);
    assert!(all_recipients.contains(&"to1@example.com".to_string()));
    assert!(all_recipients.contains(&"to2@example.com".to_string()));
    assert!(all_recipients.contains(&"cc1@example.com".to_string()));
    assert!(all_recipients.contains(&"bcc1@example.com".to_string()));
    assert!(all_recipients.contains(&"bcc2@example.com".to_string()));
}

#[tokio::test]
async fn test_rendering_without_panic() {
    let contacts_manager = create_test_contacts_manager().await;
    let mut compose_ui = ComposeUI::new(contacts_manager);
    
    // Set up some test data using public API
    let test_data = crate::ui::EmailComposeData {
        to: "test@example.com".to_string(),
        cc: "".to_string(),
        bcc: "".to_string(),
        subject: "Test Subject".to_string(),
        body: "Test body line 1\nTest body line 2".to_string(),
    };
    compose_ui.load_from_draft(test_data, "test-draft-id".to_string());
    
    // Create a test terminal
    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let theme = Theme::default();
    
    // Test that rendering doesn't panic
    terminal.draw(|f| {
        let area = Rect::new(0, 0, 100, 30);
        compose_ui.render(f, area, &theme);
    }).unwrap();
    
    // Test rendering with autocomplete visible (would need public API)
    // TODO: Add public method to show/hide autocomplete for testing
    // compose_ui.show_contact_autocomplete();
    terminal.draw(|f| {
        let area = Rect::new(0, 0, 100, 30);
        compose_ui.render(f, area, &theme);
    }).unwrap();
}

#[test]
fn test_compose_field_enum() {
    // Test PartialEq implementation
    assert_eq!(ComposeField::To, ComposeField::To);
    assert_ne!(ComposeField::To, ComposeField::Cc);
    
    // Test Clone implementation
    let field = ComposeField::Subject;
    let field_clone = field.clone();
    assert_eq!(field, field_clone);
    
    // Test Debug implementation
    let debug_str = format!("{:?}", ComposeField::Body);
    assert_eq!(debug_str, "Body");
}

#[test]
fn test_compose_action_enum() {
    // Test PartialEq implementation
    assert_eq!(ComposeAction::Send, ComposeAction::Send);
    assert_ne!(ComposeAction::Send, ComposeAction::Cancel);
    
    // Test Clone implementation
    let action = ComposeAction::SaveDraft;
    let action_clone = action.clone();
    assert_eq!(action, action_clone);
    
    // Test Debug implementation
    let debug_str = format!("{:?}", ComposeAction::AutoSave);
    assert_eq!(debug_str, "AutoSave");
}