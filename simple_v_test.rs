// Simple test to check V key shortcut
// Run with: cargo run --bin simple-v-test

use comunicado::keyboard::{KeyboardShortcut, KeyboardAction};
use crossterm::event::{KeyCode, KeyModifiers};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Simple V key test");
    println!("===================");
    
    // Directly create the shortcut we expect
    let v_shortcut = KeyboardShortcut::simple(KeyCode::Char('V'));
    
    println!("✅ V shortcut created: {:?}", v_shortcut);
    println!("   Key: {:?}", v_shortcut.key);
    println!("   Modifiers: {:?}", v_shortcut.modifiers);
    
    // Compare with what should be the mapping
    if let KeyCode::Char(c) = v_shortcut.key {
        println!("   Character: '{}'", c);
        println!("   Is uppercase V: {}", c == 'V');
        println!("   Is no modifiers: {}", v_shortcut.modifiers == KeyModifiers::NONE);
    }
    
    println!("\n📋 Expected mapping: V -> OpenEmailViewer");
    println!("📋 Action enum value: {:?}", KeyboardAction::OpenEmailViewer);
    
    Ok(())
}