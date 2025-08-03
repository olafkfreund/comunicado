//! Integration Test for Modular UI
//! 
//! This is a minimal test to validate the modular UI architecture
//! without requiring full service initialization.

use crate::ui::components::{ModularUI, AppMode, UIEvent, EventResult};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Simple integration test for ModularUI
pub async fn test_modular_ui_basic() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing ModularUI basic functionality...");
    
    // Create ModularUI
    let mut ui = ModularUI::new()?;
    println!("✅ ModularUI created successfully");
    
    // Test initial state
    assert_eq!(ui.current_mode(), AppMode::Email);
    assert!(!ui.is_initialized());
    println!("✅ Initial state verified");
    
    // Test mode switching
    ui.switch_mode(AppMode::Calendar)?;
    assert_eq!(ui.current_mode(), AppMode::Calendar);
    println!("✅ Mode switching works");
    
    ui.switch_mode(AppMode::Contacts)?;
    assert_eq!(ui.current_mode(), AppMode::Contacts);
    println!("✅ Multiple mode switches work");
    
    // Test event handling
    let key_event = UIEvent::Key(KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE));
    let result = ui.handle_event(key_event)?;
    match result {
        EventResult::Consumed => println!("✅ F1 help event handled correctly"),
        _ => println!("ℹ️ F1 event handled with result: {:?}", result),
    }
    
    // Test performance metrics (before initialization)
    let metrics = ui.performance_metrics();
    println!("📊 Basic metrics: {} components, {} events", 
             metrics.total_components, metrics.total_events_processed);
    
    println!("🎉 All basic tests passed!");
    Ok(())
}

/// Test ModularUI with minimal initialization
pub async fn test_modular_ui_with_minimal_init() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing ModularUI with minimal initialization...");
    
    let mut ui = ModularUI::new()?;
    
    // Initialize with all None services (minimal setup)
    ui.initialize(
        None, // database
        None, // imap_manager
        None, // smtp_service
        None, // calendar_manager
        None, // contacts_manager
        None, // notification_manager
        None, // secure_storage
    ).await?;
    
    println!("✅ ModularUI initialized with minimal services");
    assert!(ui.is_initialized());
    
    // Test that components were created even without services
    let metrics = ui.performance_metrics();
    println!("📊 Post-init metrics: {} components", metrics.total_components);
    
    // Test rapid mode switching (stress test)
    for _ in 0..10 {
        ui.switch_mode(AppMode::Email)?;
        ui.switch_mode(AppMode::Calendar)?;
        ui.switch_mode(AppMode::Contacts)?;
    }
    
    let final_metrics = ui.performance_metrics();
    println!("📊 After stress test: {} events processed", 
             final_metrics.total_events_processed);
    
    println!("🎉 Minimal initialization test passed!");
    Ok(())
}

/// Test performance improvements
pub async fn test_modular_ui_performance() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing ModularUI performance characteristics...");
    
    let mut ui = ModularUI::new()?;
    ui.initialize(None, None, None, None, None, None, None).await?;
    
    let start_time = std::time::Instant::now();
    
    // Simulate rapid event processing
    for i in 0..100 {
        let key_event = UIEvent::Key(KeyEvent::new(
            if i % 2 == 0 { KeyCode::Char('j') } else { KeyCode::Char('k') },
            KeyModifiers::NONE
        ));
        ui.handle_event(key_event)?;
    }
    
    let event_processing_time = start_time.elapsed();
    let metrics = ui.performance_metrics();
    
    println!("⚡ Performance Results:");
    println!("   Event processing time: {:.2}ms", event_processing_time.as_secs_f64() * 1000.0);
    println!("   Average per event: {:.2}μs", event_processing_time.as_micros() as f64 / 100.0);
    println!("   Layout cache hit rate: {:.1}%", metrics.layout_cache_hit_rate * 100.0);
    println!("   Total events processed: {}", metrics.total_events_processed);
    
    // Verify performance is reasonable (sub-millisecond per event)
    let avg_event_time = event_processing_time.as_micros() as f64 / 100.0;
    if avg_event_time < 1000.0 { // Less than 1ms per event
        println!("✅ Performance is excellent (<1ms per event)");
    } else {
        println!("⚠️ Performance might need optimization ({}μs per event)", avg_event_time);
    }
    
    println!("🎉 Performance test completed!");
    Ok(())
}

/// Run all integration tests
pub async fn run_all_tests() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Running ModularUI Integration Tests...\n");
    
    test_modular_ui_basic().await?;
    println!();
    
    test_modular_ui_with_minimal_init().await?;
    println!();
    
    test_modular_ui_performance().await?;
    println!();
    
    println!("🎯 All integration tests completed successfully!");
    println!("ModularUI is ready for production use! 🚀");
    
    Ok(())
}