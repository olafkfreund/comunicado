//! Basic AI integration test using public APIs only

use comunicado::email::EmailSummary;
use tokio::sync::mpsc;
use std::time::{Duration, Instant};

/// Basic test to ensure AI types can be constructed
#[test]
fn test_email_summary_creation() {
    // Use the EmailCategory from the AI service module which is public
    use comunicado::ai::service::EmailCategory;
    
    let summary = EmailSummary {
        summary: "Test email summary".to_string(),
        key_points: vec!["Point 1".to_string(), "Point 2".to_string()],
        category: EmailCategory::Work,
        action_items: vec!["Action 1".to_string()],
        confidence: 0.85,
    };

    assert_eq!(summary.summary, "Test email summary");
    assert_eq!(summary.key_points.len(), 2);
    assert_eq!(summary.action_items.len(), 1);
    assert!((0.0..=1.0).contains(&summary.confidence));
}

/// Test that different email categories work
#[test]
fn test_email_categories() {
    use comunicado::ai::service::EmailCategory;
    
    let categories = vec![
        EmailCategory::Work,
        EmailCategory::Personal,
        EmailCategory::Newsletter,
        EmailCategory::Promotional,
        EmailCategory::Social,
        EmailCategory::Uncategorized,
    ];

    // Test that we can create summaries with different categories
    for category in categories {
        let summary = EmailSummary {
            summary: format!("Summary for {:?} category", category),
            key_points: vec!["Point".to_string()],
            category,
            action_items: vec![],
            confidence: 0.8,
        };
        
        assert!(!summary.summary.is_empty());
    }
}

/// Test async channel communication (core functionality)
#[tokio::test]
async fn test_async_communication() {
    use comunicado::ai::service::EmailCategory;
    
    let (tx, mut rx) = mpsc::unbounded_channel::<EmailSummary>();

    // Create test summary
    let summary = EmailSummary {
        summary: "Async test summary".to_string(),
        key_points: vec!["Async point".to_string()],
        category: EmailCategory::Work,
        action_items: vec![],
        confidence: 0.9,
    };

    // Send through channel
    tx.send(summary.clone()).expect("Failed to send through channel");

    // Receive and verify
    match rx.recv().await {
        Some(received) => {
            assert_eq!(received.summary, summary.summary);
            assert_eq!(received.key_points, summary.key_points);
        }
        None => panic!("Failed to receive from channel"),
    }
}

/// Test performance of summary operations
#[test]
fn test_summary_performance() {
    use comunicado::ai::service::EmailCategory;
    
    let start = Instant::now();

    // Create many summaries to test performance
    let mut summaries = Vec::new();
    for i in 0..1000 {
        let summary = EmailSummary {
            summary: format!("Performance test summary {}", i),
            key_points: vec![format!("Point {}", i)],
            category: EmailCategory::Work,
            action_items: vec![],
            confidence: 0.8,
        };
        summaries.push(summary);
    }

    let duration = start.elapsed();
    println!("Creating 1000 summaries took: {:?}", duration);

    // Should be very fast (less than 10ms)
    assert!(duration < Duration::from_millis(10), "Summary creation should be fast");
    assert_eq!(summaries.len(), 1000);
}

/// Test confidence score validation
#[test]
fn test_confidence_scores() {
    use comunicado::ai::service::EmailCategory;
    
    let test_scores = vec![0.0, 0.25, 0.5, 0.75, 1.0];

    for score in test_scores {
        let summary = EmailSummary {
            summary: "Confidence test".to_string(),
            key_points: vec!["Point".to_string()],
            category: EmailCategory::Work,
            action_items: vec![],
            confidence: score,
        };
        
        assert!((0.0..=1.0).contains(&summary.confidence));
        assert_eq!(summary.confidence, score);
    }
}

/// Test that we can handle empty fields gracefully
#[test]
fn test_empty_fields() {
    use comunicado::ai::service::EmailCategory;
    
    let summary = EmailSummary {
        summary: String::new(), // Empty summary
        key_points: vec![], // No key points
        category: EmailCategory::Uncategorized,
        action_items: vec![], // No action items
        confidence: 0.0, // Zero confidence
    };

    // Should not panic and should handle empty fields
    assert_eq!(summary.summary.len(), 0);
    assert_eq!(summary.key_points.len(), 0);
    assert_eq!(summary.action_items.len(), 0);
    assert_eq!(summary.confidence, 0.0);
}

/// Test large content handling
#[test]
fn test_large_content() {
    use comunicado::ai::service::EmailCategory;
    
    // Create summary with large content
    let large_summary = "A".repeat(10000); // 10KB summary
    let many_points: Vec<String> = (0..100).map(|i| format!("Point {}", i)).collect();
    let many_actions: Vec<String> = (0..50).map(|i| format!("Action {}", i)).collect();

    let summary = EmailSummary {
        summary: large_summary.clone(),
        key_points: many_points.clone(),
        category: EmailCategory::Work,
        action_items: many_actions.clone(),
        confidence: 0.95,
    };

    assert_eq!(summary.summary.len(), 10000);
    assert_eq!(summary.key_points.len(), 100);
    assert_eq!(summary.action_items.len(), 50);
}

/// Test concurrent access to summaries
#[tokio::test]
async fn test_concurrent_access() {
    use comunicado::ai::service::EmailCategory;
    use std::sync::Arc;
    
    let summary = Arc::new(EmailSummary {
        summary: "Concurrent test".to_string(),
        key_points: vec!["Concurrent point".to_string()],
        category: EmailCategory::Work,
        action_items: vec!["Concurrent action".to_string()],
        confidence: 0.9,
    });

    // Spawn multiple tasks that access the summary
    let mut handles = vec![];
    for i in 0..10 {
        let summary_clone = Arc::clone(&summary);
        let handle = tokio::spawn(async move {
            // Simulate some work
            tokio::time::sleep(Duration::from_millis(1)).await;
            
            // Access summary data
            assert_eq!(summary_clone.summary, "Concurrent test");
            assert_eq!(summary_clone.key_points.len(), 1);
            assert_eq!(summary_clone.action_items.len(), 1);
            
            i
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.expect("Task should complete successfully");
    }
}

/// Manual test runner for verifying AI integration
pub fn run_basic_ai_tests() -> Result<(), String> {
    println!("🧪 Running Basic AI Integration Tests...\n");

    let mut passed = 0;
    let mut total = 0;

    // Test 1: Email Summary Creation
    total += 1;
    match std::panic::catch_unwind(|| test_email_summary_creation()) {
        Ok(_) => {
            println!("✅ Email Summary Creation");
            passed += 1;
        }
        Err(_) => {
            println!("❌ Email Summary Creation");
        }
    }

    // Test 2: Email Categories
    total += 1;
    match std::panic::catch_unwind(|| test_email_categories()) {
        Ok(_) => {
            println!("✅ Email Categories");
            passed += 1;
        }
        Err(_) => {
            println!("❌ Email Categories");
        }
    }

    // Test 3: Confidence Scores
    total += 1;
    match std::panic::catch_unwind(|| test_confidence_scores()) {
        Ok(_) => {
            println!("✅ Confidence Scores");
            passed += 1;
        }
        Err(_) => {
            println!("❌ Confidence Scores");
        }
    }

    // Test 4: Empty Fields
    total += 1;
    match std::panic::catch_unwind(|| test_empty_fields()) {
        Ok(_) => {
            println!("✅ Empty Fields Handling");
            passed += 1;
        }
        Err(_) => {
            println!("❌ Empty Fields Handling");
        }
    }

    // Test 5: Performance
    total += 1;
    match std::panic::catch_unwind(|| test_summary_performance()) {
        Ok(_) => {
            println!("✅ Performance Test");
            passed += 1;
        }
        Err(_) => {
            println!("❌ Performance Test");
        }
    }

    println!("\n📊 Test Results:");
    println!("   Passed: {}/{}", passed, total);
    println!("   Success Rate: {:.1}%", (passed as f64 / total as f64) * 100.0);

    if passed == total {
        println!("\n🎉 All basic AI tests passed!");
        println!("   The core AI data structures are working correctly.");
        Ok(())
    } else {
        Err(format!("{} out of {} tests failed", total - passed, total))
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn run_all_basic_tests() {
        run_basic_ai_tests().expect("All basic AI tests should pass");
    }
}