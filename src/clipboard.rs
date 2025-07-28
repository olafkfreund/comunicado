use arboard::Clipboard;
use std::error::Error;

/// Clipboard utilities for copy/paste functionality in TUI
pub struct ClipboardManager {
    clipboard: Option<Clipboard>,
}

impl ClipboardManager {
    pub fn new() -> Self {
        let clipboard = match Clipboard::new() {
            Ok(cb) => {
                tracing::info!("Clipboard initialized successfully");
                Some(cb)
            }
            Err(e) => {
                tracing::warn!("Failed to initialize clipboard: {} - copy/paste functionality will be limited", e);
                None
            }
        };
        Self { clipboard }
    }
    
    /// Copy text to clipboard
    pub fn copy(&mut self, text: &str) -> Result<(), Box<dyn Error>> {
        if let Some(ref mut clipboard) = self.clipboard {
            clipboard.set_text(text)?;
            tracing::info!("Copied {} characters to clipboard", text.len());
            Ok(())
        } else {
            Err("Clipboard not available".into())
        }
    }
    
    /// Paste text from clipboard
    pub fn paste(&mut self) -> Result<String, Box<dyn Error>> {
        if let Some(ref mut clipboard) = self.clipboard {
            let content = clipboard.get_text()?;
            tracing::info!("Pasted {} characters from clipboard", content.len());
            Ok(content)
        } else {
            Err("Clipboard not available".into())
        }
    }
    
    /// Check if clipboard is available
    pub fn is_available(&self) -> bool {
        self.clipboard.is_some()
    }
}

impl Default for ClipboardManager {
    fn default() -> Self {
        Self::new()
    }
}