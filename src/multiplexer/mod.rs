//! Terminal multiplexer integration for tmux and screen
//!
//! This module provides seamless integration with terminal multiplexers:
//! - Session persistence and restoration
//! - Window and pane management
//! - Status line integration
//! - Clipboard synchronization
//! - Remote session support
//! - Notification forwarding

pub mod tmux;
pub mod screen;
pub mod session;
pub mod window_manager;
pub mod status_integration;
pub mod clipboard_sync;
pub mod remote_session;

pub use tmux::{TmuxIntegration, TmuxSession, TmuxWindow, TmuxPane};
pub use screen::{ScreenIntegration, ScreenSession, ScreenWindow};
pub use session::{SessionManager, SessionState, SessionConfig, SessionResult};
pub use window_manager::{WindowManager, WindowLayout, PaneArrangement};
pub use status_integration::{StatusLineProvider, StatusFormat, StatusUpdate};
pub use clipboard_sync::{ClipboardSync, ClipboardMode};
pub use remote_session::{RemoteSession, SSHSession, MoshSession};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;
// use uuid::Uuid;

/// Multiplexer integration errors
#[derive(Error, Debug)]
pub enum MultiplexerError {
    #[error("Multiplexer not found: {0}")]
    NotFound(String),
    
    #[error("Session error: {0}")]
    SessionError(String),
    
    #[error("Command failed: {0}")]
    CommandFailed(String),
    
    #[error("Connection error: {0}")]
    ConnectionError(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type MultiplexerResult<T> = Result<T, MultiplexerError>;

/// Supported terminal multiplexers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MultiplexerType {
    Tmux,
    Screen,
    Zellij,
    None,
}

/// Multiplexer detection and integration
pub struct MultiplexerDetector {
    detected_type: Option<MultiplexerType>,
    session_info: Option<SessionInfo>,
}

/// Session information from multiplexer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub multiplexer: MultiplexerType,
    pub session_name: String,
    pub window_id: Option<String>,
    pub pane_id: Option<String>,
    pub attached: bool,
    pub remote: bool,
    pub socket_path: Option<PathBuf>,
}

impl MultiplexerDetector {
    pub fn new() -> Self {
        Self {
            detected_type: None,
            session_info: None,
        }
    }

    /// Detect the current terminal multiplexer
    pub fn detect(&mut self) -> MultiplexerResult<MultiplexerType> {
        // Check for tmux
        if std::env::var("TMUX").is_ok() {
            self.detected_type = Some(MultiplexerType::Tmux);
            self.session_info = Some(self.get_tmux_info()?);
            return Ok(MultiplexerType::Tmux);
        }

        // Check for screen
        if std::env::var("STY").is_ok() {
            self.detected_type = Some(MultiplexerType::Screen);
            self.session_info = Some(self.get_screen_info()?);
            return Ok(MultiplexerType::Screen);
        }

        // Check for Zellij
        if std::env::var("ZELLIJ").is_ok() {
            self.detected_type = Some(MultiplexerType::Zellij);
            return Ok(MultiplexerType::Zellij);
        }

        self.detected_type = Some(MultiplexerType::None);
        Ok(MultiplexerType::None)
    }

    /// Get current session information
    pub fn session_info(&self) -> Option<&SessionInfo> {
        self.session_info.as_ref()
    }

    fn get_tmux_info(&self) -> MultiplexerResult<SessionInfo> {
        let session_name = std::process::Command::new("tmux")
            .args(&["display-message", "-p", "#S"])
            .output()
            .map_err(|e| MultiplexerError::CommandFailed(e.to_string()))?;

        let window_id = std::process::Command::new("tmux")
            .args(&["display-message", "-p", "#I"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string());

        let pane_id = std::process::Command::new("tmux")
            .args(&["display-message", "-p", "#P"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string());

        Ok(SessionInfo {
            multiplexer: MultiplexerType::Tmux,
            session_name: String::from_utf8(session_name.stdout)
                .unwrap_or_default()
                .trim()
                .to_string(),
            window_id,
            pane_id,
            attached: true,
            remote: std::env::var("SSH_CONNECTION").is_ok(),
            socket_path: std::env::var("TMUX").ok().map(PathBuf::from),
        })
    }

    fn get_screen_info(&self) -> MultiplexerResult<SessionInfo> {
        let sty = std::env::var("STY")
            .map_err(|_| MultiplexerError::SessionError("STY not set".to_string()))?;
        
        let parts: Vec<&str> = sty.split('.').collect();
        let session_name = if parts.len() > 1 {
            parts[1].to_string()
        } else {
            sty.clone()
        };

        Ok(SessionInfo {
            multiplexer: MultiplexerType::Screen,
            session_name,
            window_id: std::env::var("WINDOW").ok(),
            pane_id: None,
            attached: true,
            remote: std::env::var("SSH_CONNECTION").is_ok(),
            socket_path: None,
        })
    }
}

/// Multiplexer integration manager
pub struct MultiplexerManager {
    detector: MultiplexerDetector,
    tmux: Option<TmuxIntegration>,
    screen: Option<ScreenIntegration>,
    session_manager: SessionManager,
    window_manager: WindowManager,
    status_provider: Option<Box<dyn StatusLineProvider>>,
    clipboard_sync: Option<ClipboardSync>,
}

impl MultiplexerManager {
    pub fn new() -> MultiplexerResult<Self> {
        let mut detector = MultiplexerDetector::new();
        let multiplexer_type = detector.detect()?;

        let (tmux, screen) = match multiplexer_type {
            MultiplexerType::Tmux => (Some(TmuxIntegration::new()?), None),
            MultiplexerType::Screen => (None, Some(ScreenIntegration::new()?)),
            _ => (None, None),
        };

        Ok(Self {
            detector,
            tmux,
            screen,
            session_manager: SessionManager::new()?,
            window_manager: WindowManager::new()?,
            status_provider: None,
            clipboard_sync: None,
        })
    }

    /// Initialize multiplexer integration
    pub fn initialize(&mut self) -> MultiplexerResult<()> {
        match self.detector.detected_type {
            Some(MultiplexerType::Tmux) => {
                if let Some(ref mut tmux) = self.tmux {
                    tmux.initialize()?;
                    self.setup_tmux_integration()?;
                }
            }
            Some(MultiplexerType::Screen) => {
                if let Some(ref mut screen) = self.screen {
                    screen.initialize()?;
                    self.setup_screen_integration()?;
                }
            }
            _ => {}
        }

        // Initialize clipboard sync if available
        if self.is_multiplexer_active() {
            self.clipboard_sync = Some(ClipboardSync::new()?);
        }

        Ok(())
    }

    /// Check if running inside a multiplexer
    pub fn is_multiplexer_active(&self) -> bool {
        matches!(
            self.detector.detected_type,
            Some(MultiplexerType::Tmux) | Some(MultiplexerType::Screen) | Some(MultiplexerType::Zellij)
        )
    }

    /// Get current multiplexer type
    pub fn multiplexer_type(&self) -> Option<MultiplexerType> {
        self.detector.detected_type.clone()
    }

    /// Save current session state
    pub fn save_session(&self) -> MultiplexerResult<SessionState> {
        self.session_manager.save_current_state()
    }

    /// Restore session state
    pub fn restore_session(&mut self, state: SessionState) -> MultiplexerResult<()> {
        self.session_manager.restore_state(state)
    }

    /// Create a new window
    pub fn create_window(&mut self, name: &str) -> MultiplexerResult<String> {
        match self.detector.detected_type {
            Some(MultiplexerType::Tmux) => {
                if let Some(ref mut tmux) = self.tmux {
                    tmux.create_window(name)
                } else {
                    Err(MultiplexerError::NotFound("Tmux".to_string()))
                }
            }
            Some(MultiplexerType::Screen) => {
                if let Some(ref mut screen) = self.screen {
                    screen.create_window(name)
                } else {
                    Err(MultiplexerError::NotFound("Screen".to_string()))
                }
            }
            _ => Err(MultiplexerError::NotFound("No multiplexer".to_string())),
        }
    }

    /// Split current pane/window
    pub fn split_pane(&mut self, vertical: bool) -> MultiplexerResult<String> {
        match self.detector.detected_type {
            Some(MultiplexerType::Tmux) => {
                if let Some(ref mut tmux) = self.tmux {
                    tmux.split_pane(vertical)
                } else {
                    Err(MultiplexerError::NotFound("Tmux".to_string()))
                }
            }
            _ => Err(MultiplexerError::NotFound("Split not supported".to_string())),
        }
    }

    /// Update status line
    pub fn update_status(&mut self, status: String) -> MultiplexerResult<()> {
        if let Some(ref mut provider) = self.status_provider {
            provider.update_status(status)
        } else {
            Ok(())
        }
    }

    /// Sync clipboard with multiplexer
    pub fn sync_clipboard(&mut self) -> MultiplexerResult<()> {
        if let Some(ref mut sync) = self.clipboard_sync {
            sync.synchronize()
        } else {
            Ok(())
        }
    }

    /// Handle remote session
    pub fn handle_remote_session(&mut self) -> MultiplexerResult<()> {
        if let Some(ref info) = self.detector.session_info {
            if info.remote {
                // Special handling for remote sessions
                self.configure_for_remote()?;
            }
        }
        Ok(())
    }

    fn setup_tmux_integration(&mut self) -> MultiplexerResult<()> {
        // Set up tmux-specific features
        if let Some(ref mut tmux) = self.tmux {
            // Configure status line
            tmux.configure_status_line()?;
            
            // Set up key bindings
            tmux.setup_keybindings()?;
            
            // Enable mouse support
            tmux.enable_mouse_support()?;
        }
        Ok(())
    }

    fn setup_screen_integration(&mut self) -> MultiplexerResult<()> {
        // Set up screen-specific features
        if let Some(ref mut screen) = self.screen {
            // Configure hardstatus line
            screen.configure_hardstatus()?;
            
            // Set up key bindings
            screen.setup_keybindings()?;
        }
        Ok(())
    }

    fn configure_for_remote(&mut self) -> MultiplexerResult<()> {
        // Optimize for remote sessions
        // Reduce update frequency, handle latency, etc.
        Ok(())
    }
}

impl Default for MultiplexerManager {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            detector: MultiplexerDetector::new(),
            tmux: None,
            screen: None,
            session_manager: SessionManager::new().unwrap(),
            window_manager: WindowManager::new().unwrap(),
            status_provider: None,
            clipboard_sync: None,
        })
    }
}

/// Multiplexer hook for application events
pub trait MultiplexerHook {
    fn on_startup(&mut self) -> MultiplexerResult<()>;
    fn on_shutdown(&mut self) -> MultiplexerResult<()>;
    fn on_focus_gained(&mut self) -> MultiplexerResult<()>;
    fn on_focus_lost(&mut self) -> MultiplexerResult<()>;
    fn on_resize(&mut self, width: u16, height: u16) -> MultiplexerResult<()>;
}

impl MultiplexerHook for MultiplexerManager {
    fn on_startup(&mut self) -> MultiplexerResult<()> {
        self.initialize()?;
        self.handle_remote_session()?;
        Ok(())
    }

    fn on_shutdown(&mut self) -> MultiplexerResult<()> {
        // Save session state
        let state = self.save_session()?;
        self.session_manager.persist_state(&state)?;
        Ok(())
    }

    fn on_focus_gained(&mut self) -> MultiplexerResult<()> {
        self.sync_clipboard()?;
        Ok(())
    }

    fn on_focus_lost(&mut self) -> MultiplexerResult<()> {
        // Minimal updates when not focused
        Ok(())
    }

    fn on_resize(&mut self, _width: u16, _height: u16) -> MultiplexerResult<()> {
        // Handle terminal resize
        Ok(())
    }
}