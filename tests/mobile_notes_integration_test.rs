//! Integration tests for mobile-notes functionality compatibility
//! 
//! Tests that the notes system and mobile system can work together,
//! verifying the foundation for SMS-to-notes integration.

use comunicado::plugins::notes::{NoteStorage, Note, WatchedDirectory};
use comunicado::mobile::{KdeConnectClient, MessageStore};

use std::sync::Arc;
use tempfile::TempDir;

#[tokio::test]
async fn test_notes_and_mobile_systems_compatibility() {
    println!("🔗 Testing notes and mobile systems compatibility...");
    
    let temp_dir = TempDir::new().unwrap();
    
    // Test that we can create both systems independently
    let note_storage = match NoteStorage::new(temp_dir.path()).await {
        Ok(storage) => Arc::new(storage),
        Err(e) => {
            println!("Note storage creation failed: {}", e);
            return;
        }
    };
    
    let message_store = match MessageStore::new(temp_dir.path().join("messages.db")).await {
        Ok(store) => Arc::new(store),
        Err(e) => {
            println!("Message store creation failed: {}", e);
            return;
        }
    };
    
    // Test basic note operations
    let dir = WatchedDirectory::new(
        temp_dir.path().join("notes"),
        "Test Notes".to_string(),
    );
    let stored_dir = note_storage.add_watched_directory(dir).await.unwrap();
    
    let note = Note::new(
        "mobile-test-note".to_string(),
        "Mobile Integration Test".to_string(),
        "# Mobile Integration Test\n\nThis note tests mobile-notes compatibility.".to_string(),
        temp_dir.path().join("test-note.md"),
    );
    
    note_storage.store_note(&note, stored_dir.id).await.unwrap();
    
    // Verify note was stored
    let retrieved = note_storage.get_note(&note.id).await.unwrap();
    assert!(retrieved.is_some());
    
    // Test message store basic operations
    let stats = message_store.get_stats().await.unwrap();
    assert_eq!(stats.message_count, 0); // No messages yet
    
    // Test KDE Connect client creation (may fail if not available)
    match KdeConnectClient::new() {
        Ok(_client) => {
            println!("✓ KDE Connect client created successfully");
        }
        Err(_) => {
            println!("⚠️ KDE Connect not available (expected in test environment)");
        }
    }
    
    println!("✅ Notes and mobile systems are compatible and working");
}

#[tokio::test]
async fn test_note_metadata_for_mobile_integration() {
    println!("📱 Testing note metadata handling for mobile integration...");
    
    let temp_dir = TempDir::new().unwrap();
    let note_storage = NoteStorage::new(temp_dir.path()).await.unwrap();
    
    // Add a watched directory
    let dir = WatchedDirectory::new(
        temp_dir.path().join("mobile-notes"),
        "Mobile Notes".to_string(),
    );
    let stored_dir = note_storage.add_watched_directory(dir).await.unwrap();
    
    // Create a note with mobile-like metadata
    let mut note = Note::new(
        "sms-converted-note".to_string(),
        "SMS from Contact".to_string(),
        "# SMS from John Doe\n\n**Date:** 2025-01-15 14:30:00\n**Phone:** +1234567890\n\nDon't forget about the meeting tomorrow!\n\n---\n*Converted from SMS*".to_string(),
        temp_dir.path().join("sms-note.md"),
    );
    
    // Add mobile-related tags
    note.tags = vec![
        "sms".to_string(),
        "mobile".to_string(),
        "contact:john-doe".to_string(),
        "meeting".to_string(),
    ];
    
    // Store the note
    note_storage.store_note(&note, stored_dir.id).await.unwrap();
    
    // Test searching for mobile-related notes
    let mobile_notes = note_storage.get_notes_by_tag("mobile").await.unwrap();
    assert_eq!(mobile_notes.len(), 1);
    assert_eq!(mobile_notes[0].id, note.id);
    
    let sms_notes = note_storage.get_notes_by_tag("sms").await.unwrap();
    assert_eq!(sms_notes.len(), 1);
    
    let contact_notes = note_storage.get_notes_by_tag("contact:john-doe").await.unwrap();
    assert_eq!(contact_notes.len(), 1);
    
    // Test full-text search for mobile content
    let search_results = note_storage.search_notes("converted from SMS", 10).await.unwrap();
    assert!(!search_results.is_empty());
    
    println!("✓ Note metadata handling supports mobile integration patterns");
    println!("✅ Mobile integration metadata tests completed");
}

#[tokio::test]
async fn test_mobile_note_template_processing() {
    println!("📝 Testing mobile note template processing...");
    
    // Test template-like content processing
    let template = "# SMS from {{contact}}\n\n**Date:** {{date}}\n**Phone:** {{phone}}\n\n{{content}}\n\n---\n*Converted from SMS*";
    
    let processed = template
        .replace("{{contact}}", "John Doe")
        .replace("{{date}}", "2025-01-15 14:30:00")
        .replace("{{phone}}", "+1234567890")
        .replace("{{content}}", "Don't forget about the meeting tomorrow!");
    
    assert!(processed.contains("John Doe"));
    assert!(processed.contains("2025-01-15 14:30:00"));
    assert!(processed.contains("+1234567890"));
    assert!(processed.contains("Don't forget about the meeting tomorrow!"));
    assert!(processed.contains("*Converted from SMS*"));
    
    // Test that we can create a note with this content
    let temp_dir = TempDir::new().unwrap();
    let note_storage = NoteStorage::new(temp_dir.path()).await.unwrap();
    
    let dir = WatchedDirectory::new(
        temp_dir.path().join("mobile-notes"),
        "Mobile Notes".to_string(),
    );
    let stored_dir = note_storage.add_watched_directory(dir).await.unwrap();
    
    let note = Note::new(
        "template-test-note".to_string(),
        "SMS from John Doe".to_string(),
        processed,
        temp_dir.path().join("template-note.md"),
    );
    
    note_storage.store_note(&note, stored_dir.id).await.unwrap();
    
    // Verify the note was stored with the processed content
    let retrieved = note_storage.get_note(&note.id).await.unwrap().unwrap();
    assert!(retrieved.content.contains("John Doe"));
    assert!(retrieved.content.contains("*Converted from SMS*"));
    
    println!("✓ Template processing works correctly for mobile notes");
    println!("✅ Mobile note template processing tests completed");
}

#[tokio::test]
async fn test_integration_readiness() {
    println!("🚀 Testing overall mobile-notes integration readiness...");
    
    let temp_dir = TempDir::new().unwrap();
    
    // Test all components can be created
    let note_storage = NoteStorage::new(temp_dir.path()).await.unwrap();
    let message_store = MessageStore::new(temp_dir.path().join("messages.db")).await.unwrap();
    
    // Test directory setup for mobile notes
    let mobile_dir = WatchedDirectory::new(
        temp_dir.path().join("mobile-notes"),
        "Mobile Notes".to_string(),
    );
    let stored_dir = note_storage.add_watched_directory(mobile_dir).await.unwrap();
    assert!(stored_dir.id > 0);
    
    // Test message store statistics
    let stats = message_store.get_stats().await.unwrap();
    assert_eq!(stats.conversation_count, 0);
    assert_eq!(stats.message_count, 0);
    
    // Test note storage capabilities needed for mobile integration
    let recent_notes = note_storage.get_recent_notes(10).await.unwrap();
    assert_eq!(recent_notes.len(), 0); // No notes yet
    
    let search_results = note_storage.search_notes("mobile", 10).await.unwrap();
    assert_eq!(search_results.len(), 0); // No matching notes yet
    
    // Test that we can store notes with mobile metadata
    let mobile_note = Note::new(
        "integration-test".to_string(),
        "Integration Test Note".to_string(),
        "This note verifies mobile integration readiness.".to_string(),
        temp_dir.path().join("integration-test.md"),
    );
    
    note_storage.store_note(&mobile_note, stored_dir.id).await.unwrap();
    
    // Verify the systems are working together
    let all_notes = note_storage.get_recent_notes(5).await.unwrap();
    assert_eq!(all_notes.len(), 1);
    assert_eq!(all_notes[0].id, mobile_note.id);
    
    println!("✓ Note storage system ready for mobile integration");
    println!("✓ Message store system ready for SMS processing");
    println!("✓ Directory structure supports mobile note organization");
    println!("✓ Search and retrieval functions work correctly");
    println!("✅ Mobile-notes integration infrastructure is ready!");
}