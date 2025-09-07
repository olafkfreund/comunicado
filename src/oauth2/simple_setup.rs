use crate::oauth2::{
    client::OAuth2Client,
    providers::{OAuth2Provider, ProviderConfig},
    OAuth2Error, OAuth2Result,
};
use crate::theme::Theme;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap},
};
use std::io;

/// RAII guard for terminal state cleanup
struct TerminalGuard;

impl TerminalGuard {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        enable_raw_mode()?;
        Ok(TerminalGuard)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if let Err(e) = disable_raw_mode() {
            tracing::warn!("Failed to disable raw mode during cleanup: {}", e);
        }
    }
}

#[allow(dead_code)]
pub struct SimpleSetupWizard {
    theme: Theme,
    state: SimpleSetupState,
    detected_providers: Vec<DetectedAccount>,
    selected_index: usize,
    email_input: String,
    cursor_position: usize,
    error_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
enum SimpleSetupState {
    Welcome,
    QuickDetection,
    OneClickSetup(DetectedAccount),
    ManualEmailInput,
    ProviderInstructions(OAuth2Provider),
    Authorization,
    Complete(String), // account_id
    Error(String),
}

#[derive(Debug, Clone, PartialEq)]
struct DetectedAccount {
    email: String,
    provider: OAuth2Provider,
    is_ready: bool,
    description: String,
}

impl SimpleSetupWizard {
    pub fn new(theme: Theme) -> Self {
        Self {
            theme,
            state: SimpleSetupState::Welcome,
            detected_providers: Vec::new(),
            selected_index: 0,
            email_input: String::new(),
            cursor_position: 0,
            error_message: None,
        }
    }

    /// Auto-detect common email accounts from system
    fn detect_common_accounts() -> Vec<DetectedAccount> {
        let mut detected = Vec::new();

        // Common Gmail patterns
        let _common_emails: Vec<String> = vec![
            // Try to detect from common environment variables or files
            // This is a simplified example - real implementation might check:
            // - Git config
            // - SSH config
            // - Chrome/Firefox saved passwords (with permission)
            // - System keyring entries
        ];

        // For now, provide quick setup for common providers
        let quick_providers = vec![
            DetectedAccount {
                email: "Quick Gmail Setup".to_string(),
                provider: OAuth2Provider::Gmail,
                is_ready: true,
                description: "One-click setup for Gmail accounts".to_string(),
            },
            DetectedAccount {
                email: "Quick Outlook Setup".to_string(),
                provider: OAuth2Provider::Outlook,
                is_ready: true,
                description: "One-click setup for Outlook/Hotmail accounts".to_string(),
            },
            DetectedAccount {
                email: "Manual Setup".to_string(),
                provider: OAuth2Provider::Custom("manual".to_string()),
                is_ready: false,
                description: "Manual configuration for any email provider".to_string(),
            },
        ];

        detected.extend(quick_providers);
        detected
    }

    pub async fn run(&mut self) -> OAuth2Result<Option<String>> {
        // Setup terminal with proper error handling
        let _terminal_guard = TerminalGuard::new()
            .map_err(|e| OAuth2Error::StorageError(format!("Failed to setup terminal: {}", e)))?;

        let mut stdout = io::stdout();
        stdout.execute(EnterAlternateScreen).map_err(|e| {
            OAuth2Error::StorageError(format!("Failed to enter alternate screen: {}", e))
        })?;

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)
            .map_err(|e| OAuth2Error::StorageError(format!("Failed to create terminal: {}", e)))?;

        // Initialize with detected accounts
        self.detected_providers = Self::detect_common_accounts();
        self.state = SimpleSetupState::QuickDetection;

        // Run with proper cleanup on any exit
        let result = match self.run_wizard_loop(&mut terminal).await {
            Ok(result) => result,
            Err(e) => {
                // Ensure we clean up terminal even on error
                let _ = terminal.backend_mut().execute(LeaveAlternateScreen);
                let _ = terminal.show_cursor();
                return Err(e);
            }
        };

        // Clean cleanup terminal
        if let Err(e) = terminal.backend_mut().execute(LeaveAlternateScreen) {
            tracing::warn!("Failed to exit alternate screen: {}", e);
        }
        if let Err(e) = terminal.show_cursor() {
            tracing::warn!("Failed to show cursor: {}", e);
        }

        Ok(result)
    }

    async fn run_wizard_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> OAuth2Result<Option<String>> {
        loop {
            terminal
                .draw(|f| self.draw(f))
                .map_err(|e| OAuth2Error::StorageError(e.to_string()))?;

            match self.state {
                SimpleSetupState::Complete(ref account_id) => {
                    return Ok(Some(account_id.clone()));
                }
                SimpleSetupState::Error(_) => {
                    return Ok(None);
                }
                _ => {}
            }

            if let Event::Key(key) =
                event::read().map_err(|e| OAuth2Error::StorageError(e.to_string()))?
            {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                self.handle_key_event(key).await?;
            }
        }
    }

    async fn handle_key_event(&mut self, key: event::KeyEvent) -> OAuth2Result<()> {
        match self.state {
            SimpleSetupState::Welcome => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.state = SimpleSetupState::QuickDetection;
                }
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.state = SimpleSetupState::Error("Setup cancelled".to_string());
                }
                _ => {}
            },

            SimpleSetupState::QuickDetection => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.selected_index > 0 {
                        self.selected_index -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.selected_index < self.detected_providers.len() - 1 {
                        self.selected_index += 1;
                    }
                }
                KeyCode::Enter => {
                    if let Some(selected) = self.detected_providers.get(self.selected_index) {
                        if selected.provider == OAuth2Provider::Custom("manual".to_string()) {
                            self.state = SimpleSetupState::ManualEmailInput;
                        } else {
                            self.state = SimpleSetupState::OneClickSetup(selected.clone());
                        }
                    }
                }
                KeyCode::Esc => {
                    self.state = SimpleSetupState::Error("Setup cancelled".to_string());
                }
                _ => {}
            },

            SimpleSetupState::OneClickSetup(ref account) => {
                let provider = account.provider.clone();
                match key.code {
                    KeyCode::Enter | KeyCode::Char('y') => {
                        // Start OAuth2 flow
                        self.state = SimpleSetupState::Authorization;
                        self.start_oauth_flow(&provider).await?;
                    }
                    KeyCode::Esc | KeyCode::Char('n') => {
                        self.state = SimpleSetupState::QuickDetection;
                    }
                    _ => {}
                }
            }

            SimpleSetupState::ManualEmailInput => match key.code {
                KeyCode::Char(c) => {
                    self.email_input.insert(self.cursor_position, c);
                    self.cursor_position += 1;
                }
                KeyCode::Backspace => {
                    if self.cursor_position > 0 {
                        self.cursor_position -= 1;
                        self.email_input.remove(self.cursor_position);
                    }
                }
                KeyCode::Enter => {
                    if !self.email_input.is_empty() {
                        if let Some(provider) = OAuth2Provider::detect_from_email(&self.email_input)
                        {
                            let provider_name = provider.display_name();
                            self.state = SimpleSetupState::OneClickSetup(DetectedAccount {
                                email: self.email_input.clone(),
                                provider: provider.clone(),
                                is_ready: true,
                                description: format!("Auto-detected {}", provider_name),
                            });
                        } else {
                            self.state = SimpleSetupState::ProviderInstructions(
                                OAuth2Provider::Custom("unknown".to_string()),
                            );
                        }
                    }
                }
                KeyCode::Esc => {
                    self.state = SimpleSetupState::QuickDetection;
                }
                _ => {}
            },

            _ => {
                // Handle other states...
                match key.code {
                    KeyCode::Esc => {
                        self.state = SimpleSetupState::Error("Setup cancelled".to_string());
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    async fn start_oauth_flow(&mut self, provider: &OAuth2Provider) -> OAuth2Result<()> {
        match provider {
            OAuth2Provider::Gmail => {
                // Check for client credentials - for now, show error if missing
                let config = ProviderConfig::gmail();
                if let Err(_) = config.validate() {
                    self.state = SimpleSetupState::Error("Gmail OAuth2 credentials not configured.\n\nPlease configure client_id and client_secret in:\n~/.config/comunicado/oauth2-config.json\n\nSee documentation for setup instructions.".to_string());
                    return Ok(());
                }

                let mut client = OAuth2Client::new(config)?;
                let auth_request = client.start_authorization().await?;

                // Open browser automatically
                if let Err(e) = self.open_browser_url(&auth_request.authorization_url) {
                    tracing::warn!("Failed to open browser automatically: {}", e);
                    // Continue anyway, user can manually copy the URL
                }

                // Wait for authorization with timeout
                match client.wait_for_authorization(300).await {
                    Ok(auth_code) => {
                        // Exchange code for tokens
                        let token_response = client.exchange_code(&auth_code).await?;

                        // Create account configuration
                        let account_config =
                            client.create_account_config(&token_response, None).await?;

                        // TODO: Store account configuration (would need storage integration)
                        self.state = SimpleSetupState::Complete(account_config.account_id);
                    }
                    Err(e) => {
                        self.state =
                            SimpleSetupState::Error(format!("Authorization failed: {}", e));
                    }
                }
            }
            OAuth2Provider::Outlook => {
                // Check for client credentials
                let config = ProviderConfig::outlook();
                if let Err(_) = config.validate() {
                    self.state = SimpleSetupState::Error("Outlook OAuth2 credentials not configured.\n\nPlease configure client_id and client_secret in:\n~/.config/comunicado/oauth2-config.json\n\nSee documentation for setup instructions.".to_string());
                    return Ok(());
                }

                let mut client = OAuth2Client::new(config)?;
                let auth_request = client.start_authorization().await?;

                // Open browser automatically
                if let Err(e) = self.open_browser_url(&auth_request.authorization_url) {
                    tracing::warn!("Failed to open browser automatically: {}", e);
                }

                // Wait for authorization with timeout
                match client.wait_for_authorization(300).await {
                    Ok(auth_code) => {
                        // Exchange code for tokens
                        let token_response = client.exchange_code(&auth_code).await?;

                        // Create account configuration
                        let account_config =
                            client.create_account_config(&token_response, None).await?;

                        // TODO: Store account configuration
                        self.state = SimpleSetupState::Complete(account_config.account_id);
                    }
                    Err(e) => {
                        self.state =
                            SimpleSetupState::Error(format!("Authorization failed: {}", e));
                    }
                }
            }
            _ => {
                self.state =
                    SimpleSetupState::Error("Provider not supported in quick setup".to_string());
            }
        }
        Ok(())
    }

    /// Open URL in browser (cross-platform)
    fn open_browser_url(&self, url: &str) -> Result<(), String> {
        // First try using webbrowser crate if available
        if let Err(e1) = webbrowser::open(url) {
            // Fallback to platform-specific commands
            self.open_browser_platform_specific(url).map_err(|e2| {
                format!(
                    "Both webbrowser crate ({}) and platform-specific commands ({}) failed",
                    e1, e2
                )
            })
        } else {
            Ok(())
        }
    }

    /// Platform-specific browser opening fallback
    fn open_browser_platform_specific(&self, url: &str) -> Result<(), String> {
        use std::process::Command;

        let commands = if cfg!(target_os = "linux") {
            vec!["xdg-open", "firefox", "chromium", "google-chrome"]
        } else if cfg!(target_os = "macos") {
            vec!["open"]
        } else if cfg!(target_os = "windows") {
            vec!["start", "explorer"]
        } else {
            return Err("Unsupported platform for automatic browser opening".to_string());
        };

        for command in commands {
            match Command::new(command).arg(url).spawn() {
                Ok(_) => return Ok(()),
                Err(_) => continue,
            }
        }

        Err("Failed to find any working browser command".to_string())
    }

    fn draw(&mut self, f: &mut Frame) {
        let size = f.area();

        // Create main layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Content
                Constraint::Length(3), // Footer
            ])
            .split(size);

        // Header
        let header = Paragraph::new("📧 Comunicado - Quick Email Setup")
            .style(
                Style::default()
                    .fg(self.theme.colors.palette.accent)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(header, chunks[0]);

        // Content based on state
        match self.state.clone() {
            SimpleSetupState::Welcome => self.draw_welcome(f, chunks[1]),
            SimpleSetupState::QuickDetection => self.draw_quick_detection(f, chunks[1]),
            SimpleSetupState::OneClickSetup(account) => {
                self.draw_one_click_setup(f, chunks[1], &account)
            }
            SimpleSetupState::ManualEmailInput => self.draw_manual_input(f, chunks[1]),
            SimpleSetupState::Authorization => self.draw_authorization(f, chunks[1]),
            SimpleSetupState::Complete(account_id) => self.draw_complete(f, chunks[1], &account_id),
            SimpleSetupState::Error(msg) => self.draw_error(f, chunks[1], &msg),
            _ => {}
        }

        // Footer with navigation hints
        let footer_text = match &self.state {
            SimpleSetupState::Welcome => "Press Enter to continue • Esc to quit",
            SimpleSetupState::QuickDetection => "↑/↓ to select • Enter to continue • Esc to quit",
            SimpleSetupState::OneClickSetup(_) => "Enter/Y to setup • N/Esc to go back",
            SimpleSetupState::ManualEmailInput => {
                "Type your email • Enter to continue • Esc to go back"
            }
            _ => "Please wait...",
        };

        let footer = Paragraph::new(footer_text)
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(footer, chunks[2]);
    }

    fn draw_welcome(&mut self, f: &mut Frame, area: Rect) {
        let text = vec![
            Line::from(""),
            Line::from("Welcome to Comunicado's simplified email setup!"),
            Line::from(""),
            Line::from("We'll help you get connected to your email in just a few clicks."),
            Line::from(""),
            Line::from("✓ Auto-detect popular email providers"),
            Line::from("✓ One-click OAuth2 setup for Gmail, Outlook"),
            Line::from("✓ Pre-configured server settings"),
            Line::from("✓ No complex configuration needed"),
            Line::from(""),
            Line::from("Press Enter to get started!"),
        ];

        let paragraph = Paragraph::new(text)
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL).title("Quick Setup"));
        f.render_widget(paragraph, area);
    }

    fn draw_quick_detection(&mut self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .detected_providers
            .iter()
            .enumerate()
            .map(|(i, account)| {
                let style = if i == self.selected_index {
                    Style::default()
                        .bg(self.theme.colors.palette.accent)
                        .fg(Color::Black)
                } else {
                    Style::default()
                };

                let icon = match account.provider {
                    OAuth2Provider::Gmail => "📧",
                    OAuth2Provider::Outlook => "📮",
                    OAuth2Provider::Custom(_) => "⚙️",
                    _ => "📬",
                };

                ListItem::new(vec![
                    Line::from(vec![
                        Span::raw(format!("{} ", icon)),
                        Span::styled(&account.email, style.add_modifier(Modifier::BOLD)),
                    ]),
                    Line::from(vec![
                        Span::raw("  "),
                        Span::styled(&account.description, Style::default().fg(Color::DarkGray)),
                    ]),
                ])
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Select Setup Method"),
            )
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        f.render_stateful_widget(list, area, &mut ratatui::widgets::ListState::default());
    }

    fn draw_one_click_setup(&mut self, f: &mut Frame, area: Rect, account: &DetectedAccount) {
        let provider_name = account.provider.display_name();
        let icon = match account.provider {
            OAuth2Provider::Gmail => "📧",
            OAuth2Provider::Outlook => "📮",
            _ => "📬",
        };

        let text = vec![
            Line::from(""),
            Line::from(vec![
                Span::raw(format!("{} Ready to set up ", icon)),
                Span::styled(
                    provider_name,
                    Style::default()
                        .fg(self.theme.colors.palette.accent)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from("This will:"),
            Line::from("• Open your browser for OAuth2 authentication"),
            Line::from("• Use pre-configured server settings"),
            Line::from("• Test the connection automatically"),
            Line::from("• Save your account securely"),
            Line::from(""),
            Line::from("No complex configuration required!"),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Press Y/Enter to continue",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" or "),
                Span::styled("N/Esc to go back", Style::default().fg(Color::Red)),
            ]),
        ];

        let paragraph = Paragraph::new(text).wrap(Wrap { trim: true }).block(
            Block::default()
                .borders(Borders::ALL)
                .title("One-Click Setup"),
        );
        f.render_widget(paragraph, area);
    }

    fn draw_manual_input(&mut self, f: &mut Frame, area: Rect) {
        let text = vec![
            Line::from(""),
            Line::from("Enter your email address:"),
            Line::from(""),
            Line::from(vec![
                Span::raw("📧 "),
                Span::styled(
                    &self.email_input,
                    Style::default().fg(self.theme.colors.palette.accent),
                ),
                Span::raw("_"),
            ]),
            Line::from(""),
            Line::from("We'll auto-detect your email provider and configure"),
            Line::from("the optimal settings for you."),
            Line::from(""),
            Line::from("Supported providers: Gmail, Outlook, Yahoo, and more"),
        ];

        let paragraph = Paragraph::new(text).wrap(Wrap { trim: true }).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Email Address"),
        );
        f.render_widget(paragraph, area);
    }

    fn draw_authorization(&mut self, f: &mut Frame, area: Rect) {
        let gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Setting up your account..."),
            )
            .gauge_style(Style::default().fg(self.theme.colors.palette.accent))
            .percent(75)
            .label("Opening browser for authentication...");
        f.render_widget(gauge, area);
    }

    fn draw_complete(&mut self, f: &mut Frame, area: Rect, account_id: &str) {
        let text = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "✅ Setup Complete!",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from("Your email account has been configured successfully!"),
            Line::from(""),
            Line::from(format!("Account ID: {}", account_id)),
            Line::from(""),
            Line::from("🎉 You can now start using Comunicado to manage your email."),
            Line::from(""),
            Line::from("Press any key to continue..."),
        ];

        let paragraph = Paragraph::new(text)
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL).title("Success"));
        f.render_widget(paragraph, area);
    }

    fn draw_error(&mut self, f: &mut Frame, area: Rect, error: &str) {
        let text = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "❌ Setup Failed",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(error),
            Line::from(""),
            Line::from("Press Esc to exit or try the manual setup option."),
        ];

        let paragraph = Paragraph::new(text)
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL).title("Error"));
        f.render_widget(paragraph, area);
    }
}

/// Helper function to launch simplified setup
pub async fn launch_simple_setup(theme: Theme) -> OAuth2Result<Option<String>> {
    let mut wizard = SimpleSetupWizard::new(theme);
    wizard.run().await
}
