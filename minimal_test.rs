#!/usr/bin/env cargo run --bin minimal_test --features modular-ui
//! Minimal test to validate ModularUI architecture

use comunicado::ui::components::{ModularUI, AppMode};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Minimal ModularUI Test");
    println!("========================\n");
    
    // Test 1: Basic creation
    println!("📋 Test 1: Basic Creation");
    let mut ui = ModularUI::new()?;
    println!("✅ ModularUI created successfully");
    
    // Test 2: Mode switching
    println!("\n📋 Test 2: Mode Switching");
    assert_eq!(ui.current_mode(), AppMode::Email);
    println!("✅ Initial mode is Email");
    
    ui.switch_mode(AppMode::Calendar)?;
    assert_eq!(ui.current_mode(), AppMode::Calendar);
    println!("✅ Switched to Calendar mode");
    
    ui.switch_mode(AppMode::Contacts)?;
    assert_eq!(ui.current_mode(), AppMode::Contacts);
    println!("✅ Switched to Contacts mode");
    
    // Test 3: Performance metrics
    println!("\n📋 Test 3: Performance Metrics");
    let metrics = ui.performance_metrics();
    println!("📊 Components: {}", metrics.total_components);
    println!("📊 Events processed: {}", metrics.total_events_processed);
    println!("📊 Layout cache hit rate: {:.1}%", metrics.layout_cache_hit_rate * 100.0);
    
    // Test 4: Minimal initialization
    println!("\n📋 Test 4: Minimal Initialization");
    ui.initialize(None, None, None, None, None, None, None).await?;
    println!("✅ Minimal initialization completed");
    assert!(ui.is_initialized());
    
    let final_metrics = ui.performance_metrics();
    println!("📊 Final components: {}", final_metrics.total_components);
    
    println!("\n🎉 All tests passed! ModularUI architecture is working correctly.");
    println!("✨ The modular component system is ready for use! 🚀");
    
    Ok(())
}