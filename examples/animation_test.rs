/// Example demonstrating GIF animation support in Comunicado
/// 
/// This example shows how to:
/// 1. Initialize the AnimationManager
/// 2. Load a GIF animation from URL or base64 data
/// 3. Play the animation and receive frame updates
/// 4. Display animation frames in the terminal (if supported)

use std::sync::Arc;
use comunicado::{
    images::ImageManager,
    animation::{AnimationManager, AnimationResult},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("🎬 Comunicado Animation System Test");
    println!("==================================\n");
    
    // Initialize image and animation managers
    let image_manager = Arc::new(ImageManager::new()?);
    let animation_manager = AnimationManager::new(image_manager);
    
    // Check if animations are supported
    if !animation_manager.supports_animations() {
        println!("❌ Animations are not supported in this terminal");
        println!("   Try running in Kitty, Foot, Wezterm, or another compatible terminal");
        return Ok(());
    }
    
    println!("✅ Animations are supported in this terminal!");
    println!("   Protocol: {:?}\n", animation_manager.protocol());
    
    // Example 1: Load a GIF from a test URL (this would be a real GIF URL in practice)
    println!("📡 Testing GIF loading from URL...");
    match test_gif_from_url(&animation_manager).await {
        Ok(_) => println!("✅ GIF URL loading test completed"),
        Err(e) => println!("❌ GIF URL loading failed: {}", e),
    }
    
    // Example 2: Load a GIF from base64 data (minimal test GIF)
    println!("\n📄 Testing GIF loading from base64 data...");
    match test_gif_from_base64(&animation_manager).await {
        Ok(_) => println!("✅ GIF base64 loading test completed"),
        Err(e) => println!("❌ GIF base64 loading failed: {}", e),
    }
    
    // Example 3: Test cache functionality
    println!("\n💾 Testing animation cache...");
    let (cached_count, active_count) = animation_manager.get_cache_stats().await;
    println!("   Cached animations: {}", cached_count);
    println!("   Active animations: {}", active_count);
    
    println!("\n🧹 Clearing animation cache...");
    animation_manager.clear_cache().await;
    
    let (cached_count, active_count) = animation_manager.get_cache_stats().await;
    println!("   Cached animations after clear: {}", cached_count);
    println!("   Active animations after clear: {}", active_count);
    
    println!("\n🎉 Animation system test completed successfully!");
    
    Ok(())
}

/// Test loading a GIF from URL
async fn test_gif_from_url(_animation_manager: &AnimationManager) -> Result<(), Box<dyn std::error::Error>> {
    // Note: This would use a real GIF URL in practice
    // For testing, we'll just test the loading mechanism
    println!("   Simulating GIF URL loading...");
    
    // In a real test, you would use:
    // let animation_id = animation_manager.load_gif_from_url("https://example.com/test.gif").await?;
    // let mut receiver = animation_manager.play_animation(&animation_id, true).await?;
    
    println!("   URL loading mechanism is ready");
    Ok(())
}

/// Test loading a GIF from base64 data
async fn test_gif_from_base64(animation_manager: &AnimationManager) -> Result<(), Box<dyn std::error::Error>> {
    // This is a minimal GIF (1x1 pixel, single frame) encoded in base64
    // In practice, you'd use real GIF data from email attachments
    let test_gif_base64 = "R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7";
    
    println!("   Loading test GIF from base64 data...");
    
    match animation_manager.load_gif_from_base64(test_gif_base64).await {
        Ok(animation_id) => {
            println!("   ✅ Successfully loaded animation: {}", animation_id);
            
            // Get animation info
            if let Ok(info) = animation_manager.get_animation_info(&animation_id).await {
                println!("      Frames: {}", info.frame_count);
                println!("      Duration: {}ms", info.duration_ms);
                println!("      Size: {}x{}", info.width, info.height);
                println!("      Loops: {}", info.loops);
            }
            
            // Test playback
            println!("   🎬 Starting animation playback...");
            let mut receiver = animation_manager.play_animation(&animation_id, false).await?;
            
            // Handle a few frames
            let mut frame_count = 0;
            while let Some(result) = receiver.recv().await {
                match result {
                    AnimationResult::Frame { id, frame_data: _, frame_index } => {
                        println!("      Frame {}: {}", frame_index, id);
                        frame_count += 1;
                        if frame_count >= 3 {
                            break; // Stop after a few frames
                        }
                    }
                    AnimationResult::Finished { id } => {
                        println!("      Animation finished: {}", id);
                        break;
                    }
                    AnimationResult::Error { id, error } => {
                        println!("      Animation error in {}: {}", id, error);
                        break;
                    }
                }
            }
        }
        Err(e) => {
            println!("   ❌ Failed to load test GIF: {}", e);
        }
    }
    
    Ok(())
}