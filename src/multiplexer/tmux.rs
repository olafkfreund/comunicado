//! Tmux integration implementation
//!
//! Provides comprehensive tmux integration including:
//! - Session and window management
//! - Status line integration
//! - Clipboard synchronization
//! - Key binding setup
//! - Pane management

use super::{MultiplexerError, MultiplexerResult, SessionInfo, MultiplexerType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::{Command, Stdio};
// use uuid::Uuid;

/// Tmux integration handler
pub struct TmuxIntegration {
    session_info: Option<SessionInfo>,
    windows: HashMap<String, TmuxWindow>,
    panes: HashMap<String, TmuxPane>,
    config: TmuxConfig,
}

/// Tmux configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmuxConfig {
    pub status_enabled: bool,
    pub status_position: StatusPosition,
    pub status_format: String,
    pub mouse_enabled: bool,
    pub clipboard_enabled: bool,
    pub escape_time: u32,
    pub prefix_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StatusPosition {
    Top,
    Bottom,
}

/// Tmux session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmuxSession {
    pub name: String,
    pub id: String,
    pub created: chrono::DateTime<chrono::Utc>,
    pub attached: bool,
    pub windows: Vec<TmuxWindow>,
}

/// Tmux window information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmuxWindow {
    pub id: String,
    pub index: u32,
    pub name: String,
    pub active: bool,
    pub panes: Vec<TmuxPane>,
    pub layout: WindowLayout,
}

/// Tmux pane information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmuxPane {
    pub id: String,
    pub index: u32,
    pub active: bool,
    pub width: u16,
    pub height: u16,
    pub top: u16,
    pub left: u16,
}

/// Window layout types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WindowLayout {
    EvenHorizontal,
    EvenVertical,
    MainHorizontal,
    MainVertical,
    Tiled,
    Custom(String),
}

impl TmuxIntegration {
    pub fn new() -> MultiplexerResult<Self> {
        Ok(Self {
            session_info: None,
            windows: HashMap::new(),
            panes: HashMap::new(),
            config: TmuxConfig::default(),
        })
    }

    /// Initialize tmux integration
    pub fn initialize(&mut self) -> MultiplexerResult<()> {
        // Verify tmux is available
        self.check_tmux_available()?;
        
        // Get current session info
        self.update_session_info()?;
        
        // Load current windows and panes
        self.refresh_windows()?;
        
        Ok(())
    }

    /// Check if tmux is available and accessible
    pub fn check_tmux_available(&self) -> MultiplexerResult<()> {
        Command::new("tmux")
            .arg("list-sessions")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| MultiplexerError::NotFound(format!("tmux not found: {}", e)))?;
        
        Ok(())
    }

    /// Update session information
    pub fn update_session_info(&mut self) -> MultiplexerResult<()> {
        let session_name = self.run_tmux_command(&["display-message", "-p", "#S"])?;
        let session_id = self.run_tmux_command(&["display-message", "-p", "#{session_id}"])?;
        
        self.session_info = Some(SessionInfo {
            multiplexer: MultiplexerType::Tmux,
            session_name: session_name.trim().to_string(),
            window_id: Some(self.run_tmux_command(&["display-message", "-p", "#I"])?),
            pane_id: Some(self.run_tmux_command(&["display-message", "-p", "#P"])?),
            attached: true,
            remote: std::env::var("SSH_CONNECTION").is_ok(),
            socket_path: std::env::var("TMUX").ok().map(std::path::PathBuf::from),
        });
        
        Ok(())
    }

    /// Refresh windows and panes information
    pub fn refresh_windows(&mut self) -> MultiplexerResult<()> {
        let windows_output = self.run_tmux_command(&[
            "list-windows", 
            "-F", 
            "#{window_id}|#{window_index}|#{window_name}|#{window_active}|#{window_layout}"
        ])?;

        self.windows.clear();
        
        for line in windows_output.lines() {
            if let Some(window) = self.parse_window_info(line)? {
                // Get panes for this window
                let panes = self.get_window_panes(&window.id)?;
                let mut window = window;
                window.panes = panes;
                self.windows.insert(window.id.clone(), window);
            }
        }

        Ok(())
    }

    /// Get panes for a specific window
    pub fn get_window_panes(&mut self, window_id: &str) -> MultiplexerResult<Vec<TmuxPane>> {
        let panes_output = self.run_tmux_command(&[
            "list-panes",
            "-t", window_id,
            "-F", "#{pane_id}|#{pane_index}|#{pane_active}|#{pane_width}|#{pane_height}|#{pane_top}|#{pane_left}"
        ])?;

        let mut panes = Vec::new();
        for line in panes_output.lines() {
            if let Some(pane) = self.parse_pane_info(line)? {
                panes.push(pane);
            }
        }

        Ok(panes)
    }

    /// Create a new window
    pub fn create_window(&mut self, name: &str) -> MultiplexerResult<String> {
        let output = self.run_tmux_command(&[
            "new-window",
            "-d", // Don't switch to the new window
            "-n", name,
            "-P", // Print window info
            "-F", "#{window_id}"
        ])?;

        let window_id = output.trim().to_string();
        self.refresh_windows()?;
        Ok(window_id)
    }

    /// Split current pane
    pub fn split_pane(&mut self, vertical: bool) -> MultiplexerResult<String> {
        let split_arg = if vertical { "-v" } else { "-h" };
        
        let output = self.run_tmux_command(&[
            "split-window",
            split_arg,
            "-d", // Don't switch to new pane
            "-P", // Print pane info
            "-F", "#{pane_id}"
        ])?;

        let pane_id = output.trim().to_string();
        self.refresh_windows()?;
        Ok(pane_id)
    }

    /// Configure tmux status line
    pub fn configure_status_line(&mut self) -> MultiplexerResult<()> {
        if !self.config.status_enabled {
            return Ok(());
        }

        let status_position = match self.config.status_position {
            StatusPosition::Top => "top",
            StatusPosition::Bottom => "bottom",
        };

        // Set status position
        self.run_tmux_command(&["set-option", "-g", "status-position", status_position])?;
        
        // Set status format
        self.run_tmux_command(&[
            "set-option", "-g", "status-right", 
            "#{?client_prefix,#[reverse]<Prefix>#[noreverse] ,}📧 Comunicado #[fg=blue]%H:%M %d-%b-%y"
        ])?;
        
        // Configure status style
        self.run_tmux_command(&["set-option", "-g", "status-style", "bg=colour234,fg=colour137"])?;
        
        Ok(())
    }

    /// Set up tmux key bindings
    pub fn setup_keybindings(&mut self) -> MultiplexerResult<()> {
        // Set up prefix key if specified
        if !self.config.prefix_key.is_empty() {
            self.run_tmux_command(&[
                "set-option", "-g", "prefix", &self.config.prefix_key
            ])?;
        }

        // Comunicado-specific key bindings
        let bindings = [
            // Quick email actions
            ("bind-key", "-T", "prefix", "m", "display-message", "'📧 New Email'"),
            ("bind-key", "-T", "prefix", "c", "display-message", "'📅 Calendar'"),
            ("bind-key", "-T", "prefix", "s", "display-message", "'⚙️  Settings'"),
            
            // Window management for Comunicado
            ("bind-key", "-T", "prefix", "E", "new-window", "-n Email"),
            ("bind-key", "-T", "prefix", "C", "new-window", "-n Calendar"),
        ];

        for binding in &bindings {
            let args: &[&str] = &[binding.0, binding.1, binding.2, binding.3, binding.4, binding.5];
            self.run_tmux_command(args)?;
        }

        Ok(())
    }

    /// Enable mouse support
    pub fn enable_mouse_support(&mut self) -> MultiplexerResult<()> {
        if self.config.mouse_enabled {
            self.run_tmux_command(&["set-option", "-g", "mouse", "on"])?;
        }
        Ok(())
    }

    /// Configure clipboard integration
    pub fn setup_clipboard(&mut self) -> MultiplexerResult<()> {
        if !self.config.clipboard_enabled {
            return Ok(());
        }

        // Set up clipboard commands
        let copy_command = if cfg!(target_os = "macos") {
            "pbcopy"
        } else {
            "xclip -selection clipboard"
        };

        let paste_command = if cfg!(target_os = "macos") {
            "pbpaste"
        } else {
            "xclip -selection clipboard -o"
        };

        self.run_tmux_command(&[
            "set-option", "-g", "set-clipboard", "on"
        ])?;

        self.run_tmux_command(&[
            "bind-key", "-T", "copy-mode-vi", "y", "send-keys", "-X", 
            &format!("copy-pipe-and-cancel \"{}\"", copy_command)
        ])?;

        Ok(())
    }

    /// Send notification to tmux
    pub fn send_notification(&self, message: &str) -> MultiplexerResult<()> {
        self.run_tmux_command(&["display-message", message])?;
        Ok(())
    }

    /// Update window name
    pub fn update_window_name(&self, window_id: &str, name: &str) -> MultiplexerResult<()> {
        self.run_tmux_command(&[
            "rename-window", "-t", window_id, name
        ])?;
        Ok(())
    }

    /// Get current session
    pub fn current_session(&self) -> Option<&SessionInfo> {
        self.session_info.as_ref()
    }

    /// List all tmux sessions
    pub fn list_sessions(&self) -> MultiplexerResult<Vec<TmuxSession>> {
        let output = self.run_tmux_command(&[
            "list-sessions", 
            "-F", "#{session_name}|#{session_id}|#{session_created}|#{session_attached}"
        ])?;

        let mut sessions = Vec::new();
        for line in output.lines() {
            if let Some(session) = self.parse_session_info(line)? {
                sessions.push(session);
            }
        }

        Ok(sessions)
    }

    /// Attach to a session
    pub fn attach_session(&self, session_name: &str) -> MultiplexerResult<()> {
        self.run_tmux_command(&["attach-session", "-t", session_name])?;
        Ok(())
    }

    /// Detach from current session
    pub fn detach_session(&self) -> MultiplexerResult<()> {
        self.run_tmux_command(&["detach-client"])?;
        Ok(())
    }

    // Private helper methods
    fn run_tmux_command(&self, args: &[&str]) -> MultiplexerResult<String> {
        let output = Command::new("tmux")
            .args(args)
            .output()
            .map_err(|e| MultiplexerError::CommandFailed(format!("tmux command failed: {}", e)))?;

        if !output.status.success() {
            return Err(MultiplexerError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string()
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn parse_window_info(&self, line: &str) -> MultiplexerResult<Option<TmuxWindow>> {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 5 {
            return Ok(None);
        }

        let layout = match parts[4] {
            "even-horizontal" => WindowLayout::EvenHorizontal,
            "even-vertical" => WindowLayout::EvenVertical,
            "main-horizontal" => WindowLayout::MainHorizontal,
            "main-vertical" => WindowLayout::MainVertical,
            "tiled" => WindowLayout::Tiled,
            other => WindowLayout::Custom(other.to_string()),
        };

        Ok(Some(TmuxWindow {
            id: parts[0].to_string(),
            index: parts[1].parse().unwrap_or(0),
            name: parts[2].to_string(),
            active: parts[3] == "1",
            panes: Vec::new(), // Will be filled later
            layout,
        }))
    }

    fn parse_pane_info(&self, line: &str) -> MultiplexerResult<Option<TmuxPane>> {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 7 {
            return Ok(None);
        }

        Ok(Some(TmuxPane {
            id: parts[0].to_string(),
            index: parts[1].parse().unwrap_or(0),
            active: parts[2] == "1",
            width: parts[3].parse().unwrap_or(0),
            height: parts[4].parse().unwrap_or(0),
            top: parts[5].parse().unwrap_or(0),
            left: parts[6].parse().unwrap_or(0),
        }))
    }

    fn parse_session_info(&self, line: &str) -> MultiplexerResult<Option<TmuxSession>> {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 4 {
            return Ok(None);
        }

        let created = chrono::DateTime::parse_from_str(parts[2], "%s")
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());

        Ok(Some(TmuxSession {
            name: parts[0].to_string(),
            id: parts[1].to_string(),
            created,
            attached: parts[3] != "0",
            windows: Vec::new(), // Would need separate call to populate
        }))
    }
}

impl Default for TmuxConfig {
    fn default() -> Self {
        Self {
            status_enabled: true,
            status_position: StatusPosition::Bottom,
            status_format: "#{?client_prefix,#[reverse]<Prefix>#[noreverse] ,}📧 Comunicado #[fg=blue]%H:%M %d-%b-%y".to_string(),
            mouse_enabled: true,
            clipboard_enabled: true,
            escape_time: 10,
            prefix_key: "C-a".to_string(),
        }
    }
}