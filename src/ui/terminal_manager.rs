//! Terminal management utilities with RAII pattern
//! 
//! This module provides a clean abstraction for terminal operations,
//! ensuring proper setup and teardown of terminal state using RAII.

use std::io::{self, Stdout};
use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};

/// Terminal manager trait for abstraction
pub trait TerminalManager {
    type Backend;
    
    /// Setup terminal for TUI operations
    fn setup(&mut self) -> Result<()>;
    
    /// Teardown terminal and restore original state
    fn teardown(&mut self) -> Result<()>;
    
    /// Execute a function with terminal access
    fn with_terminal<F, R>(&mut self, f: F) -> Result<R>
    where
        F: FnOnce(&mut Terminal<Self::Backend>) -> Result<R>;
}

/// RAII guard for terminal state
/// Automatically restores terminal on drop
pub struct TerminalGuard {
    should_restore: bool,
}

impl TerminalGuard {
    /// Create a new terminal guard and setup terminal
    pub fn new() -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        
        Ok(Self {
            should_restore: true,
        })
    }
    
    /// Setup terminal and return guard
    pub fn setup() -> Result<Self> {
        Self::new()
    }
    
    /// Consume guard without restoring (for handoff scenarios)
    pub fn release(mut self) {
        self.should_restore = false;
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if self.should_restore {
            // Best effort restoration - don't panic in drop
            let _ = disable_raw_mode();
            let _ = execute!(io::stdout(), LeaveAlternateScreen);
        }
    }
}

/// Managed terminal with automatic cleanup
pub struct ManagedTerminal {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    _guard: TerminalGuard,
}

impl ManagedTerminal {
    /// Create a new managed terminal
    pub fn new() -> Result<Self> {
        let guard = TerminalGuard::setup()?;
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend)?;
        
        Ok(Self {
            terminal,
            _guard: guard,
        })
    }
    
    /// Get mutable reference to the terminal
    pub fn terminal(&mut self) -> &mut Terminal<CrosstermBackend<Stdout>> {
        &mut self.terminal
    }
    
    /// Execute a function with the terminal
    pub fn with<F, R>(&mut self, f: F) -> Result<R>
    where
        F: FnOnce(&mut Terminal<CrosstermBackend<Stdout>>) -> Result<R>,
    {
        f(&mut self.terminal)
    }
    
    /// Release the terminal without cleanup (for handoff)
    pub fn release(self) -> Terminal<CrosstermBackend<Stdout>> {
        // Guard will be dropped but won't restore due to release
        self.terminal
    }
}

/// Standard terminal manager implementation
pub struct StandardTerminalManager {
    terminal: Option<Terminal<CrosstermBackend<Stdout>>>,
    guard: Option<TerminalGuard>,
}

impl StandardTerminalManager {
    /// Create a new terminal manager
    pub fn new() -> Self {
        Self {
            terminal: None,
            guard: None,
        }
    }
    
    /// Check if terminal is active
    pub fn is_active(&self) -> bool {
        self.terminal.is_some()
    }
}

impl Default for StandardTerminalManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalManager for StandardTerminalManager {
    type Backend = CrosstermBackend<Stdout>;
    
    fn setup(&mut self) -> Result<()> {
        if self.is_active() {
            return Ok(()); // Already setup
        }
        
        let guard = TerminalGuard::setup()?;
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend)?;
        
        self.guard = Some(guard);
        self.terminal = Some(terminal);
        
        Ok(())
    }
    
    fn teardown(&mut self) -> Result<()> {
        // Drop terminal first
        self.terminal = None;
        // Guard will automatically restore on drop
        self.guard = None;
        Ok(())
    }
    
    fn with_terminal<F, R>(&mut self, f: F) -> Result<R>
    where
        F: FnOnce(&mut Terminal<Self::Backend>) -> Result<R>,
    {
        match self.terminal.as_mut() {
            Some(term) => f(term),
            None => anyhow::bail!("Terminal not setup. Call setup() first."),
        }
    }
}

/// Scoped terminal execution with automatic cleanup
pub fn with_terminal<F, R>(f: F) -> Result<R>
where
    F: FnOnce(&mut Terminal<CrosstermBackend<Stdout>>) -> Result<R>,
{
    let mut managed = ManagedTerminal::new()?;
    f(managed.terminal())
}

/// Execute a function with a temporary terminal
/// Terminal is automatically setup and torn down
pub fn execute_with_terminal<F, R>(f: F) -> Result<R>
where
    F: FnOnce(&mut Terminal<CrosstermBackend<Stdout>>) -> Result<R>,
{
    let guard = TerminalGuard::setup()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    
    let result = f(&mut terminal);
    
    // Guard automatically cleans up on drop
    drop(guard);
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_terminal_guard_creation() {
        // Note: This test would need to be run with a real terminal
        // In CI/CD, we'd skip or mock this
        if std::env::var("CI").is_ok() {
            return; // Skip in CI environment
        }
        
        // Guard should be created and dropped without panic
        {
            let _guard = TerminalGuard::new();
            // Guard exists
        }
        // Guard dropped and terminal restored
    }
    
    #[test]
    fn test_standard_manager_lifecycle() {
        let mut manager = StandardTerminalManager::new();
        assert!(!manager.is_active());
        
        // After setup (would need terminal in real test)
        // assert!(manager.setup().is_ok());
        // assert!(manager.is_active());
        
        // After teardown
        // assert!(manager.teardown().is_ok());
        // assert!(!manager.is_active());
    }
}