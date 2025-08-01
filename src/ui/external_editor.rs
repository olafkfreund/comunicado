//! External editor integration for rich text editing in email composition
//!
//! This module provides functionality to launch external editors like nvim, vim, nano, etc.
//! for editing email content within the TUI application.

use anyhow::{anyhow, Result};
use std::env;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use tempfile::NamedTempFile;
use tokio::fs;

/// External editor manager for launching editors like nvim
pub struct ExternalEditor {
    editor_command: String,
    temp_dir: PathBuf,
}

impl ExternalEditor {
    /// Create a new external editor manager
    pub fn new() -> Result<Self> {
        let editor_command = Self::detect_editor();
        let temp_dir = Self::get_temp_directory()?;

        Ok(Self {
            editor_command,
            temp_dir,
        })
    }

    /// Launch external editor with content and return edited content
    pub async fn edit_content(&self, content: &str) -> Result<String> {
        // Create temporary file with initial content
        let temp_file = self.create_temp_file(content).await?;
        let temp_path = temp_file.path().to_path_buf();

        // Store file path for editor command
        let file_path = temp_path.to_string_lossy().to_string();

        // Suspend the TUI before launching editor
        self.suspend_tui()?;

        // Launch editor and wait for completion
        let exit_status = self.launch_editor(&file_path)?;

        // Resume the TUI after editor closes
        self.resume_tui()?;

        // Check if editor exited successfully
        if !exit_status.success() {
            return Err(anyhow!("Editor exited with error code: {:?}", exit_status.code()));
        }

        // Read the edited content back from file
        let edited_content = fs::read_to_string(&temp_path).await?;

        Ok(edited_content)
    }

    /// Launch external editor for email body editing
    pub async fn edit_email_body(&self, current_body: &str) -> Result<String> {
        let email_template = self.create_email_template(current_body);
        let edited_content = self.edit_content(&email_template).await?;
        
        // Extract just the body content (remove template headers if any)
        self.extract_body_from_template(&edited_content)
    }

    /// Detect which editor to use based on environment variables and availability
    fn detect_editor() -> String {
        // Check environment variables in order of preference
        if let Ok(editor) = env::var("VISUAL") {
            return editor;
        }
        
        if let Ok(editor) = env::var("EDITOR") {
            return editor;
        }

        // Check for common editors in order of preference
        let editors = ["nvim", "vim", "nano", "emacs", "code", "gedit"];
        
        for editor in &editors {
            if Self::command_exists(editor) {
                return editor.to_string();
            }
        }

        // Fallback to nano (most universally available)
        "nano".to_string()
    }

    /// Check if a command exists in the system PATH
    fn command_exists(command: &str) -> bool {
        Command::new("which")
            .arg(command)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Get temporary directory for editor files
    fn get_temp_directory() -> Result<PathBuf> {
        let temp_dir = env::temp_dir().join("comunicado").join("editor");
        std::fs::create_dir_all(&temp_dir)?;
        Ok(temp_dir)
    }

    /// Create temporary file with content
    async fn create_temp_file(&self, content: &str) -> Result<NamedTempFile> {
        let mut temp_file = NamedTempFile::new_in(&self.temp_dir)?;
        temp_file.write_all(content.as_bytes())?;
        temp_file.flush()?;
        Ok(temp_file)
    }

    /// Create email template with headers for better editing experience
    fn create_email_template(&self, body: &str) -> String {
        let template = format!(
            "# Email Body - Edit below this line\n\
             # Lines starting with # are comments and will be removed\n\
             # Save and exit your editor when done\n\
             #\n\
             # Editor: {}\n\
             #\n\
             \n\
             {}",
            self.editor_command,
            body
        );
        template
    }

    /// Extract body content from template, removing comment lines
    fn extract_body_from_template(&self, content: &str) -> Result<String> {
        let lines: Vec<&str> = content.lines().collect();
        let mut body_lines = Vec::new();
        let mut in_body = false;

        for line in lines {
            if line.starts_with('#') {
                // Skip comment lines
                continue;
            }
            
            // Once we hit the first non-comment line, we're in the body
            if !in_body && !line.trim().is_empty() {
                in_body = true;
            }
            
            if in_body || line.trim().is_empty() {
                body_lines.push(line);
            }
        }

        // Join lines and trim trailing whitespace
        let body = body_lines.join("\n").trim_end().to_string();
        Ok(body)
    }

    /// Launch the editor with the specified file
    fn launch_editor(&self, file_path: &str) -> Result<std::process::ExitStatus> {
        // Parse editor command (might have arguments)
        let parts: Vec<&str> = self.editor_command.split_whitespace().collect();
        if parts.is_empty() {
            return Err(anyhow!("Invalid editor command"));
        }

        let editor = parts[0];
        let mut args: Vec<&str> = parts[1..].to_vec();
        args.push(file_path);

        // Special handling for common editors
        match editor {
            "nvim" | "vim" => {
                // For vim/nvim, ensure we start in insert mode for better UX
                let mut cmd = Command::new(editor);
                cmd.args(&args);
                
                // Add vim-specific options for email editing
                if editor == "vim" || editor == "nvim" {
                    cmd.arg("-c").arg("set textwidth=72"); // Standard email line width
                    cmd.arg("-c").arg("set spell");        // Enable spell checking
                    cmd.arg("-c").arg("startinsert");      // Start in insert mode
                }
                
                let status = cmd.status()?;
                Ok(status)
            }
            "nano" => {
                // Nano options for better email editing
                let mut cmd = Command::new("nano");
                cmd.arg("-w");  // Don't wrap long lines automatically
                cmd.arg("-T").arg("4");  // Set tab size to 4
                cmd.args(&args);
                
                let status = cmd.status()?;
                Ok(status)
            }
            "emacs" => {
                // Emacs in terminal mode
                let mut cmd = Command::new("emacs");
                cmd.arg("-nw");  // No window (terminal mode)
                cmd.args(&args);
                
                let status = cmd.status()?;
                Ok(status)
            }
            _ => {
                // Generic editor launch
                let mut cmd = Command::new(editor);
                cmd.args(&args);
                
                let status = cmd.status()?;
                Ok(status)
            }
        }
    }

    /// Suspend the TUI application before launching editor
    fn suspend_tui(&self) -> Result<()> {
        // Disable raw mode and show cursor
        crossterm::terminal::disable_raw_mode()?;
        crossterm::execute!(
            std::io::stdout(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::cursor::Show
        )?;
        
        Ok(())
    }

    /// Resume the TUI application after editor closes
    fn resume_tui(&self) -> Result<()> {
        // Re-enable raw mode and hide cursor
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(
            std::io::stdout(),
            crossterm::terminal::EnterAlternateScreen,
            crossterm::cursor::Hide
        )?;
        
        Ok(())
    }

    /// Get the name of the current editor
    pub fn editor_name(&self) -> &str {
        self.editor_command
            .split_whitespace()
            .next()
            .unwrap_or("unknown")
    }

    /// Check if the current editor supports syntax highlighting
    pub fn supports_syntax_highlighting(&self) -> bool {
        matches!(self.editor_name(), "nvim" | "vim" | "emacs" | "code")
    }

    /// Check if the current editor supports spell checking
    pub fn supports_spell_checking(&self) -> bool {
        matches!(self.editor_name(), "nvim" | "vim" | "emacs")
    }

    /// Get editor-specific configuration for email editing
    pub fn get_editor_config(&self) -> EditorConfig {
        match self.editor_name() {
            "nvim" | "vim" => EditorConfig {
                name: self.editor_name().to_string(),
                supports_syntax: true,
                supports_spell: true,
                line_wrap: 72,
                features: vec![
                    "Syntax highlighting".to_string(),
                    "Spell checking".to_string(),
                    "Auto-indentation".to_string(),
                    "Multiple undo/redo".to_string(),
                ],
            },
            "nano" => EditorConfig {
                name: "nano".to_string(),
                supports_syntax: true,
                supports_spell: false,
                line_wrap: 72,
                features: vec![
                    "Simple interface".to_string(),
                    "Search and replace".to_string(),
                    "Auto-indentation".to_string(),
                ],
            },
            "emacs" => EditorConfig {
                name: "emacs".to_string(),
                supports_syntax: true,
                supports_spell: true,
                line_wrap: 72,
                features: vec![
                    "Advanced editing".to_string(),
                    "Spell checking".to_string(),
                    "Multiple buffers".to_string(),
                    "Extensible".to_string(),
                ],
            },
            _ => EditorConfig {
                name: self.editor_name().to_string(),
                supports_syntax: false,
                supports_spell: false,
                line_wrap: 0,
                features: vec!["Basic text editing".to_string()],
            },
        }
    }
}

/// Configuration information for an external editor
#[derive(Debug, Clone)]
pub struct EditorConfig {
    pub name: String,
    pub supports_syntax: bool,
    pub supports_spell: bool,
    pub line_wrap: usize,
    pub features: Vec<String>,
}

impl Default for ExternalEditor {
    fn default() -> Self {
        Self::new().expect("Failed to create default ExternalEditor")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_detection() {
        let editor = ExternalEditor::detect_editor();
        // Should detect some editor
        assert!(!editor.is_empty());
    }

    #[test]
    fn test_email_template_creation() {
        let editor = ExternalEditor::new().unwrap();
        let body = "Hello, this is a test email.";
        let template = editor.create_email_template(body);
        
        assert!(template.contains("# Email Body"));
        assert!(template.contains(body));
    }

    #[test]
    fn test_template_extraction() {
        let editor = ExternalEditor::new().unwrap();
        let template = "# This is a comment\n# Another comment\n\nHello world\nThis is the body.";
        let extracted = editor.extract_body_from_template(template).unwrap();
        
        assert_eq!(extracted, "Hello world\nThis is the body.");
        assert!(!extracted.contains('#'));
    }

    #[test]
    fn test_command_exists() {
        // Test with a command that should exist on most systems
        assert!(ExternalEditor::command_exists("ls") || ExternalEditor::command_exists("dir"));
        
        // Test with a command that shouldn't exist
        assert!(!ExternalEditor::command_exists("nonexistent_command_12345"));
    }

    #[test]
    fn test_editor_config() {
        let editor = ExternalEditor::new().unwrap();
        let config = editor.get_editor_config();
        
        assert!(!config.name.is_empty());
        assert!(!config.features.is_empty());
    }
}