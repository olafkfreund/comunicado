use clipboard::{ClipboardProvider, ClipboardContext};
use std::error::Error;

/// Clipboard utilities for copy/paste functionality in TUI
pub struct ClipboardManager {
    ctx: Option<ClipboardContext>,
}

impl ClipboardManager {
    pub fn new() -> Self {
        let ctx = ClipboardProvider::new().ok();
        if ctx.is_none() {
            tracing::warn!("Failed to initialize clipboard - copy/paste functionality will be limited");
        }
        Self { ctx }
    }
    
    /// Copy text to clipboard
    pub fn copy(&mut self, text: &str) -> Result<(), Box<dyn Error>> {
        if let Some(ref mut ctx) = self.ctx {
            ctx.set_contents(text.to_string())?;
            tracing::info!("Copied {} characters to clipboard", text.len());
            Ok(())
        } else {
            Err("Clipboard not available".into())
        }
    }
    
    /// Paste text from clipboard
    pub fn paste(&mut self) -> Result<String, Box<dyn Error>> {
        if let Some(ref mut ctx) = self.ctx {
            let content = ctx.get_contents()?;
            tracing::info!("Pasted {} characters from clipboard", content.len());
            Ok(content)
        } else {
            Err("Clipboard not available".into())
        }
    }
    
    /// Check if clipboard is available
    pub fn is_available(&self) -> bool {
        self.ctx.is_some()
    }
}

impl Default for ClipboardManager {
    fn default() -> Self {
        Self::new()
    }
}