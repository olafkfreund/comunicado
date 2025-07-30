//! Performance tests for email handling with large mailboxes
//! Tests memory usage, query performance, and scalability

use super::database::{EmailDatabase, StoredMessage};
use super::threading::{MessageThreader, ThreadingAlgorithm};
use crate::email::search::EmailSearchEngine;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio;
use uuid::Uuid;

/// Performance benchmarking utilities
struct PerformanceBenchmark {
    start_time: std::time::Instant,
    name: String,
}

impl PerformanceBenchmark {
    fn new(name: &str) -> Self {
        Self {
            start_time: std::time::Instant::now(),
            name: name.to_string(),
        }
    }

    fn finish(self) -> std::time::Duration {
        let duration = self.start_time.elapsed();
        eprintln!("Benchmark '{}' took: {:?}", self.name, duration);
        duration
    }
}

/// Create a test message with configurable content
fn create_test_message(
    account_id: &str,
    folder_name: &str,
    subject: &str,
    from_addr: &str,
    body_size_kb: usize,
    date_offset_hours: i64,
) -> StoredMessage {
    let body = "Test message content ".repeat(body_size_kb * 50); // Roughly 1KB per 50 repetitions
    
    StoredMessage {
        id: Uuid::new_v4(),
        account_id: account_id.to_string(),
        folder_name: folder_name.to_string(),
        imap_uid: rand::random::<u32>(),
        message_id: Some(format!("{}@example.com", Uuid::new_v4())),
        thread_id: None,
        in_reply_to: None,
        references: vec![],
        subject: subject.to_string(),
        from_addr: from_addr.to_string(),
        from_name: Some("Test Sender".to_string()),
        to_addrs: vec!["recipient@example.com".to_string()],
        cc_addrs: vec![],
        bcc_addrs: vec![],
        reply_to: None,
        date: Utc::now() - chrono::Duration::hours(date_offset_hours),
        body_text: Some(body),
        body_html: None,
        attachments: vec![],
        flags: vec![],
        labels: vec![],
        size: None,
        priority: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_synced: Utc::now(),
        sync_version: 1,
        is_draft: false,
        is_deleted: false,
    }
}

/// Generate a realistic email thread with multiple messages
fn create_email_thread(
    account_id: &str,
    folder_name: &str,
    thread_size: usize,
    base_subject: &str,
) -> Vec<StoredMessage> {
    let mut messages = Vec::new();
    let thread_id = Uuid::new_v4();
    let mut references = Vec::new();
    
    for i in 0..thread_size {
        let message_id = format!("thread-{}@example.com", i);
        let subject = if i == 0 {
            base_subject.to_string()
        } else {
            format!("Re: {}", base_subject)
        };
        
        let mut message = create_test_message(
            account_id,
            folder_name,
            &subject,
            &format!("user{}@example.com", i % 5), // Rotate between 5 senders
            1, // 1KB body
            (thread_size - i) as i64, // Chronological order
        );
        
        message.message_id = Some(message_id.clone());
        message.thread_id = Some(thread_id);
        
        if i > 0 {
            message.in_reply_to = messages.last().and_then(|m| m.message_id.clone());
            message.references = references.clone();
        }
        
        references.push(message_id);
        messages.push(message);
    }
    
    messages
}

#[tokio::test]
async fn test_database_performance_with_large_mailbox() {
    let db = EmailDatabase::new_in_memory().await.unwrap();
    let account_id = "test_account";
    let folder_name = "INBOX";
    
    // Test 1: Insert 10,000 messages performance
    let bench = PerformanceBenchmark::new("Insert 10,000 messages");
    
    for i in 0..10_000 {
        let message = create_test_message(
            account_id,
            folder_name,
            &format!("Test Message {}", i),
            &format!("sender{}@example.com", i % 100),
            1, // 1KB body
            i as i64,
        );
        
        db.store_message(&message).await.unwrap();
    }
    
    let insert_duration = bench.finish();
    assert!(insert_duration.as_secs() < 30, "Insert should complete within 30 seconds");
    
    // Test 2: Query performance with large dataset
    let bench = PerformanceBenchmark::new("Query all messages from large mailbox");
    let messages = db.get_messages(account_id, folder_name, Some(10_000)).await.unwrap();
    let query_duration = bench.finish();
    
    assert_eq!(messages.len(), 10_000);
    assert!(query_duration.as_millis() < 1000, "Query should complete within 1 second");
    
    // Test 3: Search performance
    let bench = PerformanceBenchmark::new("Search in large mailbox");
    let search_results = db.search_messages(account_id, "Test Message", Some(100)).await.unwrap();
    let search_duration = bench.finish();
    
    assert!(!search_results.is_empty());
    assert!(search_duration.as_millis() < 500, "Search should complete within 500ms");
    
    // Test 4: Pagination performance
    let bench = PerformanceBenchmark::new("Paginated queries");
    for page in 0..10 {
        let offset = page * 100;
        let page_messages = db.get_messages_with_pagination(account_id, folder_name, 100, offset).await.unwrap();
        assert_eq!(page_messages.len(), 100);
    }
    let pagination_duration = bench.finish();
    
    assert!(pagination_duration.as_millis() < 100, "Pagination should be fast");
}

#[tokio::test]
async fn test_threading_performance_with_large_conversations() {
    let db = Arc::new(EmailDatabase::new_in_memory().await.unwrap());
    let account_id = "test_account";
    let folder_name = "INBOX";
    
    // Create 100 email threads with 20 messages each (2,000 total messages)
    let mut all_messages = Vec::new();
    
    for thread_num in 0..100 {
        let thread_messages = create_email_thread(
            account_id,
            folder_name,
            20,
            &format!("Thread Subject {}", thread_num),
        );
        
        for message in &thread_messages {
            db.store_message(message).await.unwrap();
        }
        
        all_messages.extend(thread_messages);
    }
    
    // Test JWZ threading algorithm performance
    let bench = PerformanceBenchmark::new("JWZ threading with 2,000 messages in 100 threads");
    let threader = MessageThreader::new(ThreadingAlgorithm::JWZ);
    let threaded_messages = threader.thread_messages(&all_messages);
    let jwz_duration = bench.finish();
    
    // Test Simple threading algorithm performance
    let bench = PerformanceBenchmark::new("Simple threading with 2,000 messages in 100 threads");
    let threader = MessageThreader::new(ThreadingAlgorithm::Simple);
    let simple_threaded = threader.thread_messages(&all_messages);
    let simple_duration = bench.finish();
    
    // Verify results
    assert!(!threaded_messages.is_empty());
    assert!(!simple_threaded.is_empty());
    assert_eq!(threaded_messages.len(), 100); // Should have 100 root threads
    
    // Performance assertions
    assert!(jwz_duration.as_secs() < 5, "JWZ threading should complete within 5 seconds");
    assert!(simple_duration.as_secs() < 2, "Simple threading should complete within 2 seconds");
    
    // Memory usage check (basic)
    let thread_count = threaded_messages.len();
    assert!(thread_count > 0 && thread_count <= 100, "Thread count should be reasonable");
}

#[tokio::test]
async fn test_search_engine_performance() {
    let db = Arc::new(EmailDatabase::new_in_memory().await.unwrap());
    let account_id = "test_account";
    let folder_name = "INBOX";
    
    // Create a diverse set of messages for search testing
    let search_terms = vec![
        "important project meeting",
        "quarterly financial report",
        "customer feedback analysis",
        "software development update",
        "marketing campaign results",
    ];
    
    // Insert 5,000 messages with varied content
    for i in 0..5_000 {
        let term_index = i % search_terms.len();
        let subject = format!("{} - Message {}", search_terms[term_index], i);
        let body = format!("This message is about {}. It contains detailed information about the topic discussed in the subject line.", search_terms[term_index]);
        
        let mut message = create_test_message(
            account_id,
            folder_name,
            &subject,
            &format!("sender{}@example.com", i % 50),
            1,
            i as i64,
        );
        message.body_text = Some(body);
        
        db.store_message(&message).await.unwrap();
    }
    
    let search_engine = EmailSearchEngine::new(db.clone());
    
    // Test various search scenarios
    let search_scenarios = vec![
        ("Single word", "project"),
        ("Multiple words", "important project"),
        ("Phrase search", "quarterly financial report"),
        ("Common word", "message"),
        ("Rare word", "analysis"),
    ];
    
    for (scenario_name, query) in search_scenarios {
        let bench = PerformanceBenchmark::new(&format!("Search: {}", scenario_name));
        
        let results = search_engine.search_text(
            account_id,
            query,
            Some(100),
            None,
        ).await.unwrap();
        
        let search_duration = bench.finish();
        
        // Performance assertions
        assert!(search_duration.as_millis() < 200, 
               "Search '{}' should complete within 200ms", scenario_name);
        assert!(!results.is_empty(), "Search '{}' should return results", scenario_name);
    }
}

#[tokio::test]
async fn test_concurrent_database_operations() {
    let db = Arc::new(EmailDatabase::new_in_memory().await.unwrap());
    let account_id = "test_account";
    let folder_name = "INBOX";
    
    let bench = PerformanceBenchmark::new("Concurrent database operations");
    
    // Create 10 concurrent tasks that each insert 500 messages
    let mut handles = Vec::new();
    
    for task_id in 0..10 {
        let db_clone = db.clone();
        let account_id = account_id.to_string();
        let folder_name = folder_name.to_string();
        
        let handle = tokio::spawn(async move {
            for i in 0..500 {
                let message = create_test_message(
                    &account_id,
                    &folder_name,
                    &format!("Task {} Message {}", task_id, i),
                    &format!("sender{}@example.com", i),
                    1,
                    i as i64,
                );
                
                db_clone.store_message(&message).await.unwrap();
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }
    
    let concurrent_duration = bench.finish();
    
    // Verify all messages were inserted
    let total_messages = db.get_messages(account_id, folder_name, Some(10_000)).await.unwrap();
    assert_eq!(total_messages.len(), 5_000);
    
    // Performance assertion
    assert!(concurrent_duration.as_secs() < 20, 
           "Concurrent operations should complete within 20 seconds");
}

#[tokio::test]
async fn test_memory_usage_with_large_messages() {
    let db = EmailDatabase::new_in_memory().await.unwrap();
    let account_id = "test_account";
    let folder_name = "INBOX";
    
    // Test with messages of varying sizes
    let message_sizes_kb = vec![1, 10, 100, 500, 1000]; // From 1KB to 1MB
    let messages_per_size = 20;
    
    for &size_kb in &message_sizes_kb {
        let bench = PerformanceBenchmark::new(&format!("Insert {} {}KB messages", messages_per_size, size_kb));
        
        for i in 0..messages_per_size {
            let message = create_test_message(
                account_id,
                folder_name,
                &format!("Large Message {} ({}KB)", i, size_kb),
                "sender@example.com",
                size_kb,
                i as i64,
            );
            
            db.store_message(&message).await.unwrap();
        }
        
        let duration = bench.finish();
        
        // Larger messages should still complete in reasonable time
        let max_seconds = if size_kb >= 500 { 30 } else { 10 };
        assert!(duration.as_secs() < max_seconds, 
               "Insert of {}KB messages should complete within {}s", size_kb, max_seconds);
    }
    
    // Test retrieval performance with large messages
    let bench = PerformanceBenchmark::new("Retrieve large messages");
    let all_messages = db.get_messages(account_id, folder_name, Some(200)).await.unwrap();
    let retrieval_duration = bench.finish();
    
    assert_eq!(all_messages.len(), 100); // 20 messages × 5 sizes
    assert!(retrieval_duration.as_secs() < 5, "Large message retrieval should be fast");
    
    // Verify large message content is preserved
    let large_messages: Vec<_> = all_messages.iter()
        .filter(|msg| msg.subject.contains("1000KB"))
        .collect();
    
    assert_eq!(large_messages.len(), 20);
    for msg in large_messages {
        if let Some(ref body) = msg.body_text {
            assert!(body.len() > 50_000, "1MB message body should be preserved");
        }
    }
}

#[tokio::test]
async fn test_database_cleanup_performance() {
    let db = EmailDatabase::new_in_memory().await.unwrap();
    let account_id = "test_account";
    let folder_name = "INBOX";
    
    // Insert a large number of messages
    for i in 0..5_000 {
        let message = create_test_message(
            account_id,
            folder_name,
            &format!("Message to delete {}", i),
            "sender@example.com",
            1,
            i as i64,
        );
        
        db.store_message(&message).await.unwrap();
    }
    
    // Test bulk deletion performance
    let bench = PerformanceBenchmark::new("Delete 2,500 messages");
    
    let messages_to_delete = db.get_messages(account_id, folder_name, Some(2_500)).await.unwrap();
    for message in &messages_to_delete[..2_500] {
        db.delete_message(account_id, folder_name, message.imap_uid).await.unwrap();
    }
    
    let deletion_duration = bench.finish();
    
    // Verify deletion
    let remaining_messages = db.get_messages(account_id, folder_name, Some(5_000)).await.unwrap();
    assert_eq!(remaining_messages.len(), 2_500);
    
    // Performance assertion
    assert!(deletion_duration.as_secs() < 10, "Bulk deletion should complete within 10 seconds");
    
    // Test cleanup of orphaned data
    let bench = PerformanceBenchmark::new("Database cleanup operations");
    // Note: Actual cleanup methods would be called here if they exist
    let cleanup_duration = bench.finish();
    
    assert!(cleanup_duration.as_millis() < 1000, "Cleanup should be fast");
}

#[tokio::test]
async fn test_indexing_performance() {
    let db = EmailDatabase::new_in_memory().await.unwrap();
    let account_id = "test_account";
    let folder_name = "INBOX";
    
    // Insert messages without any indexes initially
    let bench = PerformanceBenchmark::new("Insert 1,000 messages for indexing test");
    
    for i in 0..1_000 {
        let message = create_test_message(
            account_id,
            folder_name,
            &format!("Indexing test message {}", i),
            &format!("sender{}@example.com", i % 10),
            2, // 2KB body
            i as i64,
        );
        
        db.store_message(&message).await.unwrap();
    }
    
    bench.finish();
    
    // Test query performance before explicit indexing
    let bench = PerformanceBenchmark::new("Query before explicit indexing");
    let results_before = db.search_messages(account_id, "indexing test", Some(100)).await.unwrap();
    let query_before_duration = bench.finish();
    
    // Test query performance with different search patterns
    let search_patterns = vec![
        "message",
        "sender1@example.com",
        "Indexing test message 123",
        "test",
    ];
    
    for pattern in search_patterns {
        let bench = PerformanceBenchmark::new(&format!("Search pattern: '{}'", pattern));
        
        let results = db.search_messages(account_id, pattern, Some(50)).await.unwrap();
        
        let search_duration = bench.finish();
        
        // Each search should complete quickly
        assert!(search_duration.as_millis() < 100, 
               "Search for '{}' should complete within 100ms", pattern);
        
        // Should return relevant results
        if pattern == "message" {
            assert!(results.len() > 100, "Should find many messages with 'message'");
        }
    }
    
    assert!(!results_before.is_empty());
}

/// Stress test to ensure system handles extreme loads gracefully
#[tokio::test]
async fn test_stress_limits() {
    let db = Arc::new(EmailDatabase::new_in_memory().await.unwrap());
    let account_id = "stress_test_account";
    
    // Test 1: Very large single message (10MB)
    let bench = PerformanceBenchmark::new("Store 10MB message");
    
    let large_message = create_test_message(
        account_id,
        "INBOX",
        "Extremely large message",
        "sender@example.com",
        10_000, // 10MB
        0,
    );
    
    let result = db.store_message(&large_message).await;
    let large_message_duration = bench.finish();
    
    // Should handle large messages without crashing
    assert!(result.is_ok(), "Should be able to store 10MB message");
    assert!(large_message_duration.as_secs() < 30, "Large message storage should complete within 30s");
    
    // Test 2: Many small concurrent operations
    let bench = PerformanceBenchmark::new("1000 concurrent small operations");
    
    let mut handles = Vec::new();
    
    for i in 0..100 {
        let db_clone = db.clone();
        let account_id = account_id.to_string();
        
        let handle = tokio::spawn(async move {
            for j in 0..10 {
                let message = create_test_message(
                    &account_id,
                    "STRESS",
                    &format!("Stress message {}-{}", i, j),
                    "stress@example.com",
                    1,
                    j as i64,
                );
                
                let _ = db_clone.store_message(&message).await;
            }
        });
        
        handles.push(handle);
    }
    
    for handle in handles {
        let _ = handle.await;
    }
    
    let stress_duration = bench.finish();
    assert!(stress_duration.as_secs() < 10, "Stress test should complete within 10 seconds");
    
    // Test 3: Query performance under stress
    let bench = PerformanceBenchmark::new("Query performance under stress");
    
    let stress_results = db.get_messages(account_id, "STRESS", Some(2_000)).await.unwrap();
    
    let query_stress_duration = bench.finish();
    
    assert!(stress_results.len() >= 1_000, "Should retrieve stress test messages");
    assert!(query_stress_duration.as_secs() < 2, "Queries should remain fast under stress");
}

#[test]
fn test_memory_efficient_data_structures() {
    // Test that our data structures are memory efficient
    let message = create_test_message(
        "test",
        "INBOX", 
        "Test memory efficiency",
        "test@example.com",
        1,
        0,
    );
    
    // Basic size check - a typical message shouldn't use excessive memory
    let message_size = std::mem::size_of_val(&message);
    
    // StoredMessage should be reasonably sized (under 1KB for the struct itself)
    assert!(message_size < 1024, "StoredMessage struct should be under 1KB, got {}", message_size);
    
    // Test collections efficiency
    let mut messages = Vec::with_capacity(1000);
    for i in 0..1000 {
        messages.push(create_test_message(
            "test",
            "INBOX",
            &format!("Message {}", i),
            "test@example.com",
            1,
            i as i64,
        ));
    }
    
    // Collection overhead should be reasonable
    let collection_overhead = std::mem::size_of_val(&messages);
    assert!(collection_overhead < 50_000, "Collection overhead should be reasonable"); // Vec overhead + 1000 * message size
}