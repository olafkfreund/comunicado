//! User onboarding system for first-time Comunicado users
//!
//! This module provides a comprehensive onboarding experience that guides new users
//! through the initial setup and key features of Comunicado. It includes:
//! - Welcome screens with feature overview
//! - Account setup assistance
//! - Key shortcut tutorials
//! - Theme and UI preference setup
//! - Plugin introduction

use crate::theme::Theme;
use super::account_setup::AccountSetupManager;
use super::config_manager::{ConfigurationManager, AppConfigAdapter};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;
use anyhow::Result;
use super::terminal_manager::ManagedTerminal;

/// Onboarding flow states
#[derive(Debug, Clone, PartialEq)]
pub enum OnboardingState {
    Welcome,
    Features,
    Shortcuts,
    AccountSetup,
    ThemeSelection,
    PluginIntro,
    Complete,
}

/// Main onboarding system
pub struct OnboardingFlow {
    state: OnboardingState,
    current_step: usize,
    total_steps: usize,
    theme: Theme,
    config_manager: Box<dyn ConfigurationManager>,
    account_setup: AccountSetupManager,
    selected_theme: String,
    show_shortcuts: bool,
    shortcuts_page: usize,
    max_shortcuts_pages: usize,
}

impl OnboardingFlow {
    /// Create a new onboarding flow with dependency injection
    pub fn new_with_dependencies(
        config_manager: Box<dyn ConfigurationManager>,
        account_setup: AccountSetupManager,
        theme: Theme,
    ) -> Result<Self> {
        Ok(Self {
            state: OnboardingState::Welcome,
            current_step: 1,
            total_steps: 6,
            theme,
            config_manager,
            account_setup,
            selected_theme: "Dark".to_string(),
            show_shortcuts: false,
            shortcuts_page: 0,
            max_shortcuts_pages: 3,
        })
    }
    
    /// Create a new onboarding flow with default dependencies
    pub fn new() -> Result<Self> {
        let config_manager = Box::new(AppConfigAdapter::load_default()?);
        let account_setup = AccountSetupManager::with_defaults()?;
        let theme = Theme::default();
        
        Self::new_with_dependencies(config_manager, account_setup, theme)
    }

    /// Run the complete onboarding flow
    pub async fn run(&mut self) -> Result<bool> {
        let mut managed_terminal = ManagedTerminal::new()?;
        self.run_onboarding_loop(managed_terminal.terminal()).await
    }

    async fn run_onboarding_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<bool> {
        loop {
            terminal.draw(|f| self.draw(f))?;

            // Handle input events
            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match self.handle_key_event(key).await {
                        Ok(should_continue) => {
                            if !should_continue {
                                return Ok(false); // User cancelled
                            }
                            if self.state == OnboardingState::Complete {
                                return Ok(true); // Onboarding completed
                            }
                        }
                        Err(_) => return Ok(false),
                    }
                }
            }
        }
    }

    async fn handle_key_event(&mut self, key: KeyEvent) -> Result<bool> {
        // Global shortcuts
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('c') => return Ok(false), // Exit
                _ => {}
            }
        }

        match self.state {
            OnboardingState::Welcome => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => self.next_step(),
                KeyCode::Char('q') | KeyCode::Esc => return Ok(false),
                _ => {}
            },
            
            OnboardingState::Features => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => self.next_step(),
                KeyCode::Esc => self.previous_step(),
                _ => {}
            },
            
            OnboardingState::Shortcuts => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => self.next_step(),
                KeyCode::Left | KeyCode::Char('h') => {
                    if self.shortcuts_page > 0 {
                        self.shortcuts_page -= 1;
                    }
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    if self.shortcuts_page < self.max_shortcuts_pages - 1 {
                        self.shortcuts_page += 1;
                    }
                }
                KeyCode::Esc => self.previous_step(),
                _ => {}
            },
            
            OnboardingState::AccountSetup => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    // Launch account setup through abstraction
                    match self.account_setup.setup_with_fallback().await {
                        Ok(Some(_)) => {
                            // Account setup successful, continue onboarding
                            self.next_step();
                        }
                        Ok(None) => {
                            // User cancelled setup, but allow to continue
                            self.next_step();
                        }
                        Err(_) => {
                            // Setup failed, allow user to skip or retry
                            self.next_step();
                        }
                    }
                }
                KeyCode::Char('s') => {
                    // Skip account setup for now
                    self.next_step();
                }
                KeyCode::Esc => self.previous_step(),
                _ => {}
            },
            
            OnboardingState::ThemeSelection => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    // Cycle through available themes
                    self.selected_theme = match self.selected_theme.as_str() {
                        "Light" => "Dark",
                        "Dark" => "Gruvbox",
                        "Gruvbox" => "Solarized",
                        "Solarized" => "Nord",
                        _ => "Light",
                    }.to_string();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.selected_theme = match self.selected_theme.as_str() {
                        "Dark" => "Light",
                        "Gruvbox" => "Dark",
                        "Solarized" => "Gruvbox",
                        "Nord" => "Solarized",
                        _ => "Dark",
                    }.to_string();
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    // Apply selected theme and continue
                    let _ = self.config_manager.set_value_json("ui_theme", serde_json::Value::String(self.selected_theme.clone()));
                    self.next_step();
                }
                KeyCode::Esc => self.previous_step(),
                _ => {}
            },
            
            OnboardingState::PluginIntro => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => self.next_step(),
                KeyCode::Esc => self.previous_step(),
                _ => {}
            },
            
            OnboardingState::Complete => {
                return Ok(true); // Complete onboarding
            }
        }

        Ok(true)
    }

    fn next_step(&mut self) {
        match self.state {
            OnboardingState::Welcome => {
                self.state = OnboardingState::Features;
                self.current_step = 2;
            }
            OnboardingState::Features => {
                self.state = OnboardingState::Shortcuts;
                self.current_step = 3;
            }
            OnboardingState::Shortcuts => {
                self.state = OnboardingState::AccountSetup;
                self.current_step = 4;
            }
            OnboardingState::AccountSetup => {
                self.state = OnboardingState::ThemeSelection;
                self.current_step = 5;
            }
            OnboardingState::ThemeSelection => {
                self.state = OnboardingState::PluginIntro;
                self.current_step = 6;
            }
            OnboardingState::PluginIntro => {
                self.state = OnboardingState::Complete;
                self.complete_onboarding();
            }
            OnboardingState::Complete => {}
        }
    }

    fn previous_step(&mut self) {
        match self.state {
            OnboardingState::Welcome => {}
            OnboardingState::Features => {
                self.state = OnboardingState::Welcome;
                self.current_step = 1;
            }
            OnboardingState::Shortcuts => {
                self.state = OnboardingState::Features;
                self.current_step = 2;
            }
            OnboardingState::AccountSetup => {
                self.state = OnboardingState::Shortcuts;
                self.current_step = 3;
            }
            OnboardingState::ThemeSelection => {
                self.state = OnboardingState::AccountSetup;
                self.current_step = 4;
            }
            OnboardingState::PluginIntro => {
                self.state = OnboardingState::ThemeSelection;
                self.current_step = 5;
            }
            OnboardingState::Complete => {
                self.state = OnboardingState::PluginIntro;
                self.current_step = 6;
            }
        }
    }

    fn complete_onboarding(&mut self) {
        // Mark onboarding as completed in config
        let _ = self.config_manager.mark_onboarding_completed();
        
        // Save the updated configuration
        let _ = self.config_manager.save();
    }

    fn draw(&mut self, f: &mut Frame) {
        let area = f.size();
        
        // Main layout with progress bar at top
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Progress bar
                Constraint::Min(0),    // Content
                Constraint::Length(3), // Navigation help
            ])
            .split(area);

        // Draw progress bar
        self.draw_progress_bar(f, chunks[0]);
        
        // Draw content based on state
        match self.state {
            OnboardingState::Welcome => self.draw_welcome(f, chunks[1]),
            OnboardingState::Features => self.draw_features(f, chunks[1]),
            OnboardingState::Shortcuts => self.draw_shortcuts(f, chunks[1]),
            OnboardingState::AccountSetup => self.draw_account_setup(f, chunks[1]),
            OnboardingState::ThemeSelection => self.draw_theme_selection(f, chunks[1]),
            OnboardingState::PluginIntro => self.draw_plugin_intro(f, chunks[1]),
            OnboardingState::Complete => self.draw_complete(f, chunks[1]),
        }
        
        // Draw navigation help
        self.draw_navigation_help(f, chunks[2]);
    }

    fn draw_progress_bar(&self, f: &mut Frame, area: Rect) {
        let progress = (self.current_step as f64 / self.total_steps as f64 * 100.0) as u16;
        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("Onboarding Progress"))
            .gauge_style(Style::default().fg(self.theme.colors.palette.accent))
            .percent(progress)
            .label(format!("Step {} of {}", self.current_step, self.total_steps));
        f.render_widget(gauge, area);
    }

    fn draw_welcome(&self, f: &mut Frame, area: Rect) {
        let text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("Welcome to ", Style::default()),
                Span::styled("Comunicado", Style::default().fg(self.theme.colors.palette.accent).add_modifier(Modifier::BOLD)),
                Span::styled("!", Style::default()),
            ]),
            Line::from(""),
            Line::from("A modern terminal-based email and calendar client"),
            Line::from("designed for power users who value privacy and efficiency."),
            Line::from(""),
            Line::from("This onboarding will help you:"),
            Line::from("• Learn about key features"),
            Line::from("• Set up your email account"),
            Line::from("• Discover keyboard shortcuts"),
            Line::from("• Customize your experience"),
            Line::from("• Explore available plugins"),
            Line::from(""),
            Line::from("Let's get started! 🚀"),
            Line::from(""),
            Line::from(vec![
                Span::styled("Press ", Style::default()),
                Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(" to continue or ", Style::default()),
                Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(" to quit", Style::default()),
            ]),
        ];

        let paragraph = Paragraph::new(text)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL).title("Welcome"));

        f.render_widget(paragraph, area);
    }

    fn draw_features(&self, f: &mut Frame, area: Rect) {
        let text = vec![
            Line::from("Key Features of Comunicado"),
            Line::from(""),
            Line::from("📧 Modern Email Experience"),
            Line::from("  • Native HTML rendering with images and animations"),
            Line::from("  • Intelligent email threading and organization"),
            Line::from("  • Multi-account support with OAuth2 authentication"),
            Line::from("  • Powerful search across all your messages"),
            Line::from(""),
            Line::from("📅 Integrated Calendar"),
            Line::from("  • CalDAV synchronization with popular providers"),
            Line::from("  • Meeting invitation handling with RSVP"),
            Line::from("  • Multiple calendar views (day, week, month)"),
            Line::from("  • Shared calendar support for team collaboration"),
            Line::from(""),
            Line::from("🔐 Privacy & Security"),
            Line::from("  • Local email storage in Maildir format"),
            Line::from("  • Optional email encryption with GPG"),
            Line::from("  • No tracking or external analytics"),
            Line::from("  • Secure token storage for OAuth2"),
            Line::from(""),
            Line::from("🔌 Extensible Architecture"),
            Line::from("  • Plugin system for custom functionality"),
            Line::from("  • AI assistant integration (local and cloud)"),
            Line::from("  • Customizable themes and layouts"),
            Line::from("  • Vim-style keyboard shortcuts"),
        ];

        let paragraph = Paragraph::new(text)
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL).title("What Makes Comunicado Special"));

        f.render_widget(paragraph, area);
    }

    fn draw_shortcuts(&self, f: &mut Frame, area: Rect) {
        let shortcuts = self.get_shortcuts_for_page(self.shortcuts_page);
        
        let text: Vec<Line> = shortcuts.into_iter().map(|s| Line::from(s)).collect();

        let title = format!("Essential Shortcuts (Page {} of {})", 
                           self.shortcuts_page + 1, 
                           self.max_shortcuts_pages);

        let paragraph = Paragraph::new(text)
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL).title(title));

        f.render_widget(paragraph, area);
    }

    fn get_shortcuts_for_page(&self, page: usize) -> Vec<String> {
        match page {
            0 => vec![
                "Navigation Shortcuts".to_string(),
                "".to_string(),
                "j, ↓        - Move down".to_string(),
                "k, ↑        - Move up".to_string(),
                "h, ←        - Go back / Left panel".to_string(),
                "l, →        - Go forward / Right panel".to_string(),
                "g           - Go to top".to_string(),
                "G           - Go to bottom".to_string(),
                "Tab         - Switch between panels".to_string(),
                "/           - Search".to_string(),
                "n           - Next search result".to_string(),
                "N           - Previous search result".to_string(),
                "".to_string(),
                "Use ← → or h/l to navigate between pages".to_string(),
            ],
            1 => vec![
                "Email Management".to_string(),
                "".to_string(),
                "Enter       - Open/read email".to_string(),
                "r           - Reply to email".to_string(),
                "R           - Reply all".to_string(),
                "f           - Forward email".to_string(),
                "c           - Compose new email".to_string(),
                "d           - Delete email".to_string(),
                "u           - Mark as unread".to_string(),
                "s           - Star/flag email".to_string(),
                "a           - Archive email".to_string(),
                "m           - Move to folder".to_string(),
                "".to_string(),
                "Use ← → or h/l to navigate between pages".to_string(),
            ],
            2 => vec![
                "Application Control".to_string(),
                "".to_string(),
                "q           - Quit application".to_string(),
                "Ctrl+c      - Force quit".to_string(),
                "?           - Show help".to_string(),
                ":           - Command mode".to_string(),
                "Ctrl+r      - Refresh/sync".to_string(),
                "Ctrl+s      - Save/sync now".to_string(),
                "Ctrl+f      - Search".to_string(),
                "Esc         - Cancel/go back".to_string(),
                "".to_string(),
                "Calendar Shortcuts".to_string(),
                "t           - Today view".to_string(),
                "w           - Week view".to_string(),
                "M           - Month view".to_string(),
                "".to_string(),
                "Use ← → or h/l to navigate between pages".to_string(),
            ],
            _ => vec!["No more pages".to_string()],
        }
    }

    fn draw_account_setup(&self, f: &mut Frame, area: Rect) {
        let text = vec![
            Line::from("Email Account Setup"),
            Line::from(""),
            Line::from("Comunicado supports modern OAuth2 authentication"),
            Line::from("for secure access to your email accounts."),
            Line::from(""),
            Line::from("Supported Providers:"),
            Line::from("• Gmail (@gmail.com)"),
            Line::from("• Outlook (@outlook.com, @hotmail.com, @live.com)"),
            Line::from("• Yahoo (@yahoo.com)"),
            Line::from("• Custom IMAP servers"),
            Line::from(""),
            Line::from("Benefits of OAuth2:"),
            Line::from("• No need to store your password"),
            Line::from("• Revocable access tokens"),
            Line::from("• Modern security standards"),
            Line::from("• Two-factor authentication support"),
            Line::from(""),
            Line::from("Ready to set up your first account?"),
            Line::from(""),
            Line::from(vec![
                Span::styled("Press ", Style::default()),
                Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(" to start setup or ", Style::default()),
                Span::styled("s", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(" to skip for now", Style::default()),
            ]),
        ];

        let paragraph = Paragraph::new(text)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL).title("Account Setup"));

        f.render_widget(paragraph, area);
    }

    fn draw_theme_selection(&self, f: &mut Frame, area: Rect) {
        let themes = ["Light", "Dark", "Gruvbox", "Solarized", "Nord"];
        
        let items: Vec<ListItem> = themes
            .iter()
            .map(|theme| {
                let style = if *theme == self.selected_theme {
                    Style::default()
                        .bg(self.theme.colors.palette.accent)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(*theme).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Choose Your Theme"))
            .highlight_style(
                Style::default()
                    .bg(self.theme.colors.palette.accent)
                    .add_modifier(Modifier::BOLD),
            );

        // Create a centered area for the list
        let centered_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Length(themes.len() as u16 + 4),
                Constraint::Percentage(20),
            ])
            .split(area)[1];

        let centered_area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(50),
                Constraint::Percentage(25),
            ])
            .split(centered_area)[1];

        f.render_widget(list, centered_area);

        // Add instructions below
        let instructions_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(70),
                Constraint::Min(3),
            ])
            .split(area)[1];

        let instructions = Paragraph::new(vec![
            Line::from("Use ↑/↓ or j/k to select a theme"),
            Line::from("Press Enter to apply your choice"),
        ])
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Instructions"));

        f.render_widget(instructions, instructions_area);
    }

    fn draw_plugin_intro(&self, f: &mut Frame, area: Rect) {
        let text = vec![
            Line::from("Plugins & Extensions"),
            Line::from(""),
            Line::from("Comunicado features a powerful plugin system"),
            Line::from("that extends functionality with additional features."),
            Line::from(""),
            Line::from("Built-in Plugins:"),
            Line::from(""),
            Line::from("📝 Notes Integration"),
            Line::from("  • Create and manage notes within Comunicado"),
            Line::from("  • Link notes to emails and calendar events"),
            Line::from("  • Markdown support with live preview"),
            Line::from(""),
            Line::from("📱 KDE Connect"),
            Line::from("  • Sync notifications with your mobile device"),
            Line::from("  • Share files between desktop and phone"),
            Line::from("  • Remote control capabilities"),
            Line::from(""),
            Line::from("🤖 AI Assistant"),
            Line::from("  • Email summarization and smart replies"),
            Line::from("  • Calendar event suggestions"),
            Line::from("  • Local (Ollama) or cloud-based AI"),
            Line::from(""),
            Line::from("You can enable these plugins later in Settings."),
            Line::from(""),
            Line::from("Ready to complete the onboarding? 🎉"),
        ];

        let paragraph = Paragraph::new(text)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL).title("Extend Your Experience"));

        f.render_widget(paragraph, area);
    }

    fn draw_complete(&self, f: &mut Frame, area: Rect) {
        let text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("🎉 Welcome to Comunicado! 🎉", 
                    Style::default().fg(self.theme.colors.palette.accent).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from("You're all set to start using Comunicado!"),
            Line::from(""),
            Line::from("Next Steps:"),
            Line::from("• Press Enter to start the application"),
            Line::from("• Use ? for help anytime"),
            Line::from("• Visit Settings to customize further"),
            Line::from("• Check out the documentation online"),
            Line::from(""),
            Line::from("Remember:"),
            Line::from("• Use vim-style navigation (hjkl)"),
            Line::from("• Press Tab to switch between panels"),
            Line::from("• Use / to search"),
            Line::from("• Press q to quit"),
            Line::from(""),
            Line::from("Thank you for choosing Comunicado!"),
            Line::from(""),
            Line::from("Press any key to begin..."),
        ];

        let paragraph = Paragraph::new(text)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Onboarding Complete")
                    .border_style(Style::default().fg(Color::Green)),
            );

        f.render_widget(paragraph, area);
    }

    fn draw_navigation_help(&self, f: &mut Frame, area: Rect) {
        let help_text = match self.state {
            OnboardingState::Welcome => "Enter: Continue • q: Quit",
            OnboardingState::Features => "Enter: Continue • Esc: Back",
            OnboardingState::Shortcuts => "Enter: Continue • ←→/hl: Pages • Esc: Back",
            OnboardingState::AccountSetup => "Enter: Setup • s: Skip • Esc: Back",
            OnboardingState::ThemeSelection => "↑↓/jk: Select • Enter: Apply • Esc: Back",
            OnboardingState::PluginIntro => "Enter: Continue • Esc: Back",
            OnboardingState::Complete => "Any key: Start application",
        };

        let help = Paragraph::new(help_text)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::DarkGray));

        f.render_widget(help, area);
    }
}

impl Default for OnboardingFlow {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

/// Check if the user needs onboarding
pub fn should_show_onboarding() -> bool {
    match AppConfigAdapter::load_default() {
        Ok(config) => config.is_first_run() || !config.is_onboarding_completed(),
        Err(_) => true, // If config can't be loaded, assume first run
    }
}

/// Run the onboarding flow if needed
pub async fn maybe_run_onboarding() -> Result<bool> {
    if should_show_onboarding() {
        let mut flow = OnboardingFlow::new()?;
        flow.run().await
    } else {
        Ok(true) // Onboarding not needed, continue normally
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_onboarding_state_transitions() {
        let mut flow = OnboardingFlow::new().unwrap();
        assert_eq!(flow.state, OnboardingState::Welcome);
        assert_eq!(flow.current_step, 1);

        flow.next_step();
        assert_eq!(flow.state, OnboardingState::Features);
        assert_eq!(flow.current_step, 2);

        flow.previous_step();
        assert_eq!(flow.state, OnboardingState::Welcome);
        assert_eq!(flow.current_step, 1);
    }

    #[test] 
    fn test_shortcuts_pagination() {
        let flow = OnboardingFlow::new().unwrap();
        let page_0 = flow.get_shortcuts_for_page(0);
        let page_1 = flow.get_shortcuts_for_page(1);
        
        assert!(!page_0.is_empty());
        assert!(!page_1.is_empty());
        assert_ne!(page_0[0], page_1[0]); // Different content
    }

    #[test]
    fn test_should_show_onboarding_default() {
        // Should return true when no config exists or first_run is true
        // This test depends on the current system state, so we just verify it doesn't panic
        let _ = should_show_onboarding();
    }
}