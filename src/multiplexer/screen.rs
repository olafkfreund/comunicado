//! GNU Screen integration implementation
//!
//! Provides screen integration including:
//! - Session and window management
//! - Hardstatus line integration
//! - Key binding setup
//! - Multi-user session support

use super::{MultiplexerError, MultiplexerResult, SessionInfo, MultiplexerType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::{Command, Stdio};

/// Screen integration handler
pub struct ScreenIntegration {
    session_info: Option<SessionInfo>,
    windows: HashMap<String, ScreenWindow>,
    config: ScreenConfig,
}

/// Screen configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenConfig {
    pub hardstatus_enabled: bool,
    pub hardstatus_format: String,
    pub escape_char: char,
    pub bell_msg: String,
    pub activity_msg: String,
}

/// Screen session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenSession {
    pub name: String,
    pub pid: u32,
    pub status: SessionStatus,
    pub created: chrono::DateTime<chrono::Utc>,
    pub windows: Vec<ScreenWindow>,
}

/// Screen window information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenWindow {
    pub id: String,
    pub number: u32,
    pub title: String,
    pub active: bool,
    pub flags: Vec<WindowFlag>,
}

/// Screen session status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionStatus {
    Attached,
    Detached,
    Dead,
}

/// Screen window flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WindowFlag {
    Active,      // *
    Previous,    // -
    Bell,        // !
    Activity,    // @
    Monitor,     // #
    Silence,     // $
    Zombie,      // Z
}

impl ScreenIntegration {
    pub fn new() -> MultiplexerResult<Self> {
        Ok(Self {
            session_info: None,
            windows: HashMap::new(),
            config: ScreenConfig::default(),
        })
    }

    /// Initialize screen integration
    pub fn initialize(&mut self) -> MultiplexerResult<()> {
        // Verify screen is available
        self.check_screen_available()?;
        
        // Get current session info
        self.update_session_info()?;
        
        // Load current windows
        self.refresh_windows()?;
        
        Ok(())
    }

    /// Check if screen is available
    pub fn check_screen_available(&self) -> MultiplexerResult<()> {
        Command::new("screen")
            .arg("-ls")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| MultiplexerError::NotFound(format!("screen not found: {}", e)))?;
        
        Ok(())
    }

    /// Update session information from environment
    pub fn update_session_info(&mut self) -> MultiplexerResult<()> {
        let sty = std::env::var("STY")
            .map_err(|_| MultiplexerError::SessionError("Not running in screen session".to_string()))?;
        
        // Parse STY format: "pid.session_name"
        let parts: Vec<&str> = sty.split('.').collect();
        let session_name = if parts.len() > 1 {
            parts[1].to_string()
        } else {
            sty.clone()
        };

        self.session_info = Some(SessionInfo {
            multiplexer: MultiplexerType::Screen,
            session_name,
            window_id: std::env::var("WINDOW").ok(),
            pane_id: None, // Screen doesn't have panes
            attached: true,
            remote: std::env::var("SSH_CONNECTION").is_ok(),
            socket_path: None,
        });

        Ok(())
    }

    /// Refresh window information
    pub fn refresh_windows(&mut self) -> MultiplexerResult<()> {
        // Get current window list
        let output = self.run_screen_command(&["-Q", "windows"])?;
        
        self.windows.clear();
        self.parse_windows(&output)?;
        
        Ok(())
    }

    /// Create a new window
    pub fn create_window(&mut self, name: &str) -> MultiplexerResult<String> {
        // Create new screen window
        self.run_screen_command(&["-X", "screen", "-t", name])?;
        
        // Get the new window number
        let windows_output = self.run_screen_command(&["-Q", "windows"])?;
        
        // Parse to find the newly created window
        if let Some(window) = self.find_window_by_name(&windows_output, name)? {
            self.refresh_windows()?;
            Ok(window.id)
        } else {
            Err(MultiplexerError::CommandFailed("Failed to create window".to_string()))
        }
    }

    /// Configure hardstatus line
    pub fn configure_hardstatus(&mut self) -> MultiplexerResult<()> {
        if !self.config.hardstatus_enabled {
            return Ok(());
        }

        // Set hardstatus line
        self.run_screen_command(&[
            "-X", "hardstatus", "alwayslastline",
            &self.config.hardstatus_format
        ])?;

        // Configure caption (window list)
        self.run_screen_command(&[
            "-X", "caption", "always",
            "%{= kw}%-w%{= BW}%n %t%{-}%+w %= 📧 Comunicado | %c %d/%m/%y"
        ])?;

        Ok(())
    }

    /// Set up screen key bindings
    pub fn setup_keybindings(&mut self) -> MultiplexerResult<()> {
        // Set escape character if configured
        if self.config.escape_char != '\x01' { // Default is Ctrl-A
            self.run_screen_command(&[
                "-X", "escape", 
                &format!("{}{}", self.config.escape_char, self.config.escape_char)
            ])?;
        }

        // Comunicado-specific key bindings
        let bindings = [
            // Quick email actions
            ("bind", "m", "stuff", "📧 New Email^M"),
            ("bind", "c", "stuff", "📅 Calendar^M"),
            ("bind", "s", "stuff", "⚙️ Settings^M"),
            
            // Window management
            ("bind", "E", "screen", "-t", "Email"),
            ("bind", "C", "screen", "-t", "Calendar"),
        ];

        for binding in &bindings {
            self.run_screen_command(&["-X", binding.0, binding.1, binding.2, binding.3])?;
        }

        Ok(())
    }

    /// Send notification via bell/message
    pub fn send_notification(&self, message: &str) -> MultiplexerResult<()> {
        // Send message to hardstatus
        self.run_screen_command(&["-X", "echo", message])?;
        
        // Optional bell
        self.run_screen_command(&["-X", "bell_msg", message])?;
        
        Ok(())
    }

    /// Switch to window by number
    pub fn switch_to_window(&self, window_number: u32) -> MultiplexerResult<()> {
        self.run_screen_command(&["-X", "select", &window_number.to_string()])?;
        Ok(())
    }

    /// Rename current window
    pub fn rename_window(&self, name: &str) -> MultiplexerResult<()> {
        self.run_screen_command(&["-X", "title", name])?;
        Ok(())
    }

    /// List all screen sessions
    pub fn list_sessions(&self) -> MultiplexerResult<Vec<ScreenSession>> {
        let output = self.run_command(&["screen", "-ls"])?;
        self.parse_sessions(&output)
    }

    /// Detach from current session
    pub fn detach_session(&self) -> MultiplexerResult<()> {
        self.run_screen_command(&["-X", "detach"])?;
        Ok(())
    }

    /// Kill current session
    pub fn kill_session(&self) -> MultiplexerResult<()> {
        self.run_screen_command(&["-X", "quit"])?;
        Ok(())
    }

    /// Get current session info
    pub fn current_session(&self) -> Option<&SessionInfo> {
        self.session_info.as_ref()
    }

    /// Configure monitoring for a window
    pub fn set_monitor(&self, window_number: u32, enabled: bool) -> MultiplexerResult<()> {
        let command = if enabled { "monitor on" } else { "monitor off" };
        self.run_screen_command(&["-X", "select", &window_number.to_string()])?;
        self.run_screen_command(&["-X", command])?;
        Ok(())
    }

    // Private helper methods
    fn run_screen_command(&self, args: &[&str]) -> MultiplexerResult<String> {
        let session_name = self.get_session_name()?;
        let mut cmd_args = vec!["screen", "-S", &session_name];
        cmd_args.extend_from_slice(args);
        self.run_command(&cmd_args)
    }

    fn run_command(&self, args: &[&str]) -> MultiplexerResult<String> {
        let output = Command::new(args[0])
            .args(&args[1..])
            .output()
            .map_err(|e| MultiplexerError::CommandFailed(format!("Command failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.is_empty() {
                return Err(MultiplexerError::CommandFailed(stderr.to_string()));
            }
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn get_session_name(&self) -> MultiplexerResult<String> {
        if let Some(ref info) = self.session_info {
            Ok(info.session_name.clone())
        } else {
            Err(MultiplexerError::SessionError("No session info available".to_string()))
        }
    }

    fn parse_windows(&mut self, output: &str) -> MultiplexerResult<()> {
        // Parse screen windows format: "0*$ bash  1-$ htop  2$ vim"
        for part in output.split_whitespace() {
            if let Some(window) = self.parse_window_part(part)? {
                self.windows.insert(window.id.clone(), window);
            }
        }
        Ok(())
    }

    fn parse_window_part(&self, part: &str) -> MultiplexerResult<Option<ScreenWindow>> {
        // Extract window number and flags
        let mut chars: Vec<char> = part.chars().collect();
        if chars.is_empty() {
            return Ok(None);
        }

        let mut flags = Vec::new();
        let mut number_end = 0;

        // Find where number ends and flags begin
        for (i, &ch) in chars.iter().enumerate() {
            if ch.is_ascii_digit() {
                number_end = i + 1;
            } else {
                break;
            }
        }

        if number_end == 0 {
            return Ok(None);
        }

        let number_str: String = chars[0..number_end].iter().collect();
        let number: u32 = number_str.parse().unwrap_or(0);

        // Parse flags
        for &ch in chars[number_end..].iter() {
            match ch {
                '*' => flags.push(WindowFlag::Active),
                '-' => flags.push(WindowFlag::Previous),
                '!' => flags.push(WindowFlag::Bell),
                '@' => flags.push(WindowFlag::Activity),
                '#' => flags.push(WindowFlag::Monitor),
                '$' => flags.push(WindowFlag::Silence),
                'Z' => flags.push(WindowFlag::Zombie),
                _ => {} // Ignore other characters
            }
        }

        Ok(Some(ScreenWindow {
            id: number.to_string(),
            number,
            title: format!("Window {}", number),
            active: flags.contains(&WindowFlag::Active),
            flags,
        }))
    }

    fn find_window_by_name(&self, output: &str, name: &str) -> MultiplexerResult<Option<ScreenWindow>> {
        // This is a simplified implementation
        // In practice, you'd need to parse the full window list with titles
        Ok(None)
    }

    fn parse_sessions(&self, output: &str) -> MultiplexerResult<Vec<ScreenSession>> {
        let mut sessions = Vec::new();
        
        for line in output.lines() {
            if let Some(session) = self.parse_session_line(line)? {
                sessions.push(session);
            }
        }
        
        Ok(sessions)
    }

    fn parse_session_line(&self, line: &str) -> MultiplexerResult<Option<ScreenSession>> {
        // Parse screen -ls output format
        // Example: "1234.session_name	(Detached)"
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            return Ok(None);
        }

        let session_part = parts[0];
        let status_part = parts[1];

        // Parse session name and PID
        let session_parts: Vec<&str> = session_part.split('.').collect();
        if session_parts.len() < 2 {
            return Ok(None);
        }

        let pid: u32 = session_parts[0].parse().unwrap_or(0);
        let name = session_parts[1].to_string();

        let status = match status_part {
            "(Attached)" => SessionStatus::Attached,
            "(Detached)" => SessionStatus::Detached,
            "(Dead" => SessionStatus::Dead,
            _ => SessionStatus::Detached,
        };

        Ok(Some(ScreenSession {
            name,
            pid,
            status,
            created: chrono::Utc::now(), // Screen doesn't provide creation time easily
            windows: Vec::new(), // Would need separate call to populate
        }))
    }
}

impl Default for ScreenConfig {
    fn default() -> Self {
        Self {
            hardstatus_enabled: true,
            hardstatus_format: "%{= kG}[ %{G}%H %{g}][%= %{= kw}%?%-Lw%?%{r}(%{W}%n*%f%t%?(%u)%?%{r})%{w}%?%+Lw%?%?%= %{g}][%{B} 📧 Comunicado %{W}%c %{g}]".to_string(),
            escape_char: '\x01', // Ctrl-A
            bell_msg: "Bell in window %n".to_string(),
            activity_msg: "Activity in window %n".to_string(),
        }
    }
}