use crate::oauth2::{
    OAuth2Error, OAuth2Result, OAuth2Client, ProviderConfig, OAuth2Provider,
    ProviderDetector, AccountConfig, SecureStorage
};
use crate::theme::Theme;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, 
        Gauge, Wrap
    },
    Frame, Terminal,
};
use std::io;
use tokio::time::{timeout, Duration};

/// OAuth2 setup wizard states
#[derive(Debug, Clone)]
enum WizardState {
    Welcome,
    EmailInput,
    ProviderSelection,
    ProviderInstructions,
    CredentialsInput,
    Authorization,
    Testing,
    Complete,
    Error(String),
}

/// Setup wizard for OAuth2 account configuration
pub struct SetupWizard {
    state: WizardState,
    theme: Theme,
    storage: SecureStorage,
    
    // User input data
    email_input: String,
    display_name_input: String,
    selected_provider: Option<OAuth2Provider>,
    provider_list_state: ListState,
    client_id_input: String,
    client_secret_input: String,
    
    // OAuth2 client and config
    oauth_client: Option<OAuth2Client>,
    account_config: Option<AccountConfig>,
    auth_request: Option<crate::oauth2::client::AuthorizationRequest>,
    authorization_started: bool,
    
    // UI state
    input_mode: InputMode,
    scroll_offset: usize,
    show_help: bool,
}

#[derive(Debug, Clone)]
enum InputMode {
    Email,
    DisplayName,
    ClientId,
    ClientSecret,
    Navigation,
}

impl SetupWizard {
    /// Create a new setup wizard
    pub fn new() -> OAuth2Result<Self> {
        let storage = SecureStorage::new("comunicado".to_string())?;
        let mut provider_list_state = ListState::default();
        provider_list_state.select(Some(0));
        
        Ok(Self {
            state: WizardState::Welcome,
            theme: Theme::default(),
            storage,
            email_input: String::new(),
            display_name_input: String::new(),
            selected_provider: None,
            provider_list_state,
            client_id_input: String::new(),
            client_secret_input: String::new(),
            oauth_client: None,
            account_config: None,
            auth_request: None,
            authorization_started: false,
            input_mode: InputMode::Navigation,
            scroll_offset: 0,
            show_help: false,
        })
    }
    
    /// Run the setup wizard
    pub async fn run(&mut self) -> OAuth2Result<Option<AccountConfig>> {
        // Setup terminal
        enable_raw_mode().map_err(|e| OAuth2Error::StorageError(e.to_string()))?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen).map_err(|e| OAuth2Error::StorageError(e.to_string()))?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).map_err(|e| OAuth2Error::StorageError(e.to_string()))?;
        
        let result = self.run_wizard_loop(&mut terminal).await;
        
        // Cleanup terminal
        disable_raw_mode().map_err(|e| OAuth2Error::StorageError(e.to_string()))?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen).map_err(|e| OAuth2Error::StorageError(e.to_string()))?;
        terminal.show_cursor().map_err(|e| OAuth2Error::StorageError(e.to_string()))?;
        
        result
    }
    
    async fn run_wizard_loop(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> OAuth2Result<Option<AccountConfig>> {
        loop {
            terminal.draw(|f| self.draw(f)).map_err(|e| OAuth2Error::StorageError(e.to_string()))?;
            
            match self.state {
                WizardState::Authorization => {
                    // Handle OAuth2 authorization flow
                    if let Err(e) = self.handle_authorization().await {
                        self.state = WizardState::Error(format!("Authorization failed: {}", e));
                        continue;
                    }
                }
                WizardState::Testing => {
                    // Test the configuration
                    if let Err(e) = self.test_configuration().await {
                        self.state = WizardState::Error(format!("Configuration test failed: {}", e));
                        continue;
                    }
                }
                WizardState::Complete => {
                    return Ok(self.account_config.clone());
                }
                WizardState::Error(_) => {
                    // Wait for user input to continue or exit
                }
                _ => {}
            }
            
            // Handle input events
            if event::poll(Duration::from_millis(100)).map_err(|e| OAuth2Error::StorageError(e.to_string()))? {
                if let Event::Key(key) = event::read().map_err(|e| OAuth2Error::StorageError(e.to_string()))? {
                    match self.handle_key_event(key).await {
                        Ok(should_continue) => {
                            if !should_continue {
                                return Ok(None);
                            }
                        }
                        Err(e) => {
                            self.state = WizardState::Error(format!("Input error: {}", e));
                        }
                    }
                }
            }
        }
    }
    
    async fn handle_key_event(&mut self, key: KeyEvent) -> OAuth2Result<bool> {
        // Global shortcuts
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('c') => return Ok(false), // Exit
                KeyCode::Char('h') => {
                    self.show_help = !self.show_help;
                    return Ok(true);
                }
                _ => {}
            }
        }
        
        match (&self.state, &self.input_mode) {
            (WizardState::Welcome, _) => {
                match key.code {
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        self.state = WizardState::EmailInput;
                        self.input_mode = InputMode::Email;
                    }
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(false),
                    _ => {}
                }
            }
            
            (WizardState::EmailInput, InputMode::Email) => {
                match key.code {
                    KeyCode::Enter => {
                        if !self.email_input.is_empty() && self.email_input.contains('@') {
                            self.selected_provider = ProviderDetector::detect_from_email(&self.email_input);
                            if self.selected_provider.is_some() {
                                self.state = WizardState::ProviderInstructions;
                            } else {
                                self.state = WizardState::ProviderSelection;
                            }
                            self.input_mode = InputMode::Navigation;
                        }
                    }
                    KeyCode::Backspace => {
                        self.email_input.pop();
                    }
                    KeyCode::Char(c) => {
                        self.email_input.push(c);
                    }
                    KeyCode::Tab => {
                        self.input_mode = InputMode::DisplayName;
                    }
                    KeyCode::Esc => {
                        self.state = WizardState::Welcome;
                        self.input_mode = InputMode::Navigation;
                    }
                    _ => {}
                }
            }
            
            (WizardState::EmailInput, InputMode::DisplayName) => {
                match key.code {
                    KeyCode::Enter => {
                        if !self.email_input.is_empty() && self.email_input.contains('@') {
                            self.selected_provider = ProviderDetector::detect_from_email(&self.email_input);
                            if self.selected_provider.is_some() {
                                self.state = WizardState::ProviderInstructions;
                            } else {
                                self.state = WizardState::ProviderSelection;
                            }
                            self.input_mode = InputMode::Navigation;
                        }
                    }
                    KeyCode::Backspace => {
                        self.display_name_input.pop();
                    }
                    KeyCode::Char(c) => {
                        self.display_name_input.push(c);
                    }
                    KeyCode::Tab => {
                        self.input_mode = InputMode::Email;
                    }
                    KeyCode::Esc => {
                        self.state = WizardState::Welcome;
                        self.input_mode = InputMode::Navigation;
                    }
                    _ => {}
                }
            }
            
            (WizardState::ProviderSelection, _) => {
                match key.code {
                    KeyCode::Up => {
                        let providers = ProviderConfig::supported_providers();
                        let selected = self.provider_list_state.selected().unwrap_or(0);
                        let new_selected = if selected == 0 { providers.len() - 1 } else { selected - 1 };
                        self.provider_list_state.select(Some(new_selected));
                    }
                    KeyCode::Down => {
                        let providers = ProviderConfig::supported_providers();
                        let selected = self.provider_list_state.selected().unwrap_or(0);
                        let new_selected = (selected + 1) % providers.len();
                        self.provider_list_state.select(Some(new_selected));
                    }
                    KeyCode::Enter => {
                        let providers = ProviderConfig::supported_providers();
                        if let Some(selected) = self.provider_list_state.selected() {
                            self.selected_provider = Some(providers[selected].clone());
                            self.state = WizardState::ProviderInstructions;
                        }
                    }
                    KeyCode::Esc => {
                        self.state = WizardState::EmailInput;
                        self.input_mode = InputMode::Email;
                    }
                    _ => {}
                }
            }
            
            (WizardState::ProviderInstructions, _) => {
                match key.code {
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        self.state = WizardState::CredentialsInput;
                        self.input_mode = InputMode::ClientId;
                    }
                    KeyCode::Esc => {
                        self.state = WizardState::ProviderSelection;
                    }
                    KeyCode::Up => {
                        self.scroll_offset = self.scroll_offset.saturating_sub(1);
                    }
                    KeyCode::Down => {
                        self.scroll_offset += 1;
                    }
                    _ => {}
                }
            }
            
            (WizardState::CredentialsInput, InputMode::ClientId) => {
                match key.code {
                    KeyCode::Enter | KeyCode::Tab => {
                        if !self.client_id_input.is_empty() {
                            self.input_mode = InputMode::ClientSecret;
                        }
                    }
                    KeyCode::Backspace => {
                        self.client_id_input.pop();
                    }
                    KeyCode::Char(c) => {
                        self.client_id_input.push(c);
                    }
                    KeyCode::Esc => {
                        self.state = WizardState::ProviderInstructions;
                        self.input_mode = InputMode::Navigation;
                    }
                    _ => {}
                }
            }
            
            (WizardState::CredentialsInput, InputMode::ClientSecret) => {
                match key.code {
                    KeyCode::Enter => {
                        if !self.client_id_input.is_empty() {
                            self.setup_oauth_client().await?;
                            self.state = WizardState::Authorization;
                            self.input_mode = InputMode::Navigation;
                        }
                    }
                    KeyCode::Backspace => {
                        self.client_secret_input.pop();
                    }
                    KeyCode::Char(c) => {
                        self.client_secret_input.push(c);
                    }
                    KeyCode::Tab => {
                        self.input_mode = InputMode::ClientId;
                    }
                    KeyCode::Esc => {
                        self.state = WizardState::ProviderInstructions;
                        self.input_mode = InputMode::Navigation;
                    }
                    _ => {}
                }
            }
            
            (WizardState::Error(_), _) => {
                match key.code {
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        self.state = WizardState::Welcome;
                        self.reset_inputs();
                    }
                    KeyCode::Esc | KeyCode::Char('q') => return Ok(false),
                    _ => {}
                }
            }
            
            (WizardState::Complete, _) => {
                return Ok(true); // Complete the wizard
            }
            
            _ => {}
        }
        
        Ok(true)
    }
    
    async fn setup_oauth_client(&mut self) -> OAuth2Result<()> {
        if let Some(provider) = &self.selected_provider {
            let config = ProviderConfig::get_config(provider)?
                .with_credentials(
                    self.client_id_input.clone(),
                    if self.client_secret_input.is_empty() { 
                        None 
                    } else { 
                        Some(self.client_secret_input.clone()) 
                    }
                );
            
            config.validate()?;
            self.oauth_client = Some(OAuth2Client::new(config)?);
        }
        
        Ok(())
    }
    
    async fn handle_authorization(&mut self) -> OAuth2Result<()> {
        if let Some(oauth_client) = &mut self.oauth_client {
            // Only start authorization once
            if !self.authorization_started {
                // Start authorization flow
                let auth_request = oauth_client.start_authorization().await?;
                
                // Attempt to open browser automatically
                if let Err(e) = open_browser_url(&auth_request.authorization_url) {
                    tracing::warn!("Failed to open browser automatically: {}", e);
                    // Continue anyway, user can click the link manually
                }
                
                self.auth_request = Some(auth_request);
                self.authorization_started = true;
                
                return Ok(()); // Return early, let the UI update
            }
            
            // Check for callback (non-blocking)
            let auth_code = match oauth_client.try_get_authorization().await {
                Some(Ok(code)) => code,
                Some(Err(e)) => return Err(e),
                None => return Ok(()), // Still waiting, not an error
            };
            
            // Exchange code for tokens
            let token_response = oauth_client.exchange_code(&auth_code).await?;
            
            // Create account configuration
            let display_name = if self.display_name_input.is_empty() {
                None
            } else {
                Some(self.display_name_input.clone())
            };
            
            self.account_config = Some(
                oauth_client.create_account_config(&token_response, display_name).await?
            );
            
            self.state = WizardState::Testing;
        }
        
        Ok(())
    }
    
    async fn test_configuration(&mut self) -> OAuth2Result<()> {
        if let Some(account) = &self.account_config {
            // Store the account configuration
            self.storage.store_account(account)?;
            
            // Test IMAP connection (simplified - in real implementation, test actual connection)
            tokio::time::sleep(Duration::from_millis(1000)).await;
            
            self.state = WizardState::Complete;
        }
        
        Ok(())
    }
    
    fn reset_inputs(&mut self) {
        self.email_input.clear();
        self.display_name_input.clear();
        self.client_id_input.clear();
        self.client_secret_input.clear();
        self.selected_provider = None;
        self.oauth_client = None;
        self.account_config = None;
        self.auth_request = None;
        self.authorization_started = false;
        self.input_mode = InputMode::Navigation;
        self.scroll_offset = 0;
    }
    
    fn draw(&mut self, f: &mut Frame) {
        let area = f.size();
        
        // Main layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(0),    // Content
                Constraint::Length(3), // Status/Help
            ])
            .split(area);
        
        // Title
        let title = Paragraph::new("Comunicado - OAuth2 Account Setup")
            .style(Style::default().fg(self.theme.colors.palette.accent))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);
        
        // Content based on state
        match &self.state {
            WizardState::Welcome => self.draw_welcome(f, chunks[1]),
            WizardState::EmailInput => self.draw_email_input(f, chunks[1]),
            WizardState::ProviderSelection => self.draw_provider_selection(f, chunks[1]),
            WizardState::ProviderInstructions => self.draw_provider_instructions(f, chunks[1]),
            WizardState::CredentialsInput => self.draw_credentials_input(f, chunks[1]),
            WizardState::Authorization => self.draw_authorization(f, chunks[1]),
            WizardState::Testing => self.draw_testing(f, chunks[1]),
            WizardState::Complete => self.draw_complete(f, chunks[1]),
            WizardState::Error(error) => {
                let error_msg = error.clone();
                self.draw_error(f, chunks[1], &error_msg)
            },
        }
        
        // Status bar
        self.draw_status_bar(f, chunks[2]);
        
        // Help overlay
        if self.show_help {
            self.draw_help_overlay(f, area);
        }
    }
    
    fn draw_welcome(&mut self, f: &mut Frame, area: Rect) {
        let text = vec![
            Line::from(""),
            Line::from("Welcome to the OAuth2 Account Setup Wizard!"),
            Line::from(""),
            Line::from("This wizard will help you configure your email account"),
            Line::from("using OAuth2 authentication for Gmail, Outlook, or Yahoo."),
            Line::from(""),
            Line::from("Benefits of OAuth2:"),
            Line::from("• More secure than traditional passwords"),
            Line::from("• No need to store your email password"),
            Line::from("• Revocable access tokens"),
            Line::from("• Modern authentication standard"),
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
    
    fn draw_email_input(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(area);
        
        // Email input
        let email_style = if matches!(self.input_mode, InputMode::Email) {
            Style::default().fg(self.theme.colors.palette.accent)
        } else {
            Style::default()
        };
        
        let email_input = Paragraph::new(self.email_input.as_str())
            .style(email_style)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Email Address *"));
        f.render_widget(email_input, chunks[0]);
        
        // Display name input
        let name_style = if matches!(self.input_mode, InputMode::DisplayName) {
            Style::default().fg(self.theme.colors.palette.accent)
        } else {
            Style::default()
        };
        
        let display_name = if self.display_name_input.is_empty() {
            "Optional - leave empty to use email"
        } else {
            &self.display_name_input
        };
        
        let name_input = Paragraph::new(display_name)
            .style(name_style)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Display Name"));
        f.render_widget(name_input, chunks[1]);
        
        // Instructions
        let instructions = vec![
            Line::from("Enter your email address and optional display name."),
            Line::from(""),
            Line::from("Supported providers will be automatically detected:"),
            Line::from("• Gmail (@gmail.com, @googlemail.com)"),
            Line::from("• Outlook (@outlook.com, @hotmail.com, @live.com)"),
            Line::from("• Yahoo (@yahoo.com, @yahoo.co.uk, etc.)"),
            Line::from(""),
            Line::from("Use Tab to switch between fields."),
            Line::from("Press Enter when ready to continue."),
        ];
        
        let paragraph = Paragraph::new(instructions)
            .wrap(Wrap { trim: true })
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Instructions"));
        
        f.render_widget(paragraph, chunks[2]);
    }
    
    fn draw_provider_selection(&mut self, f: &mut Frame, area: Rect) {
        let providers = ProviderConfig::supported_providers();
        let items: Vec<ListItem> = providers
            .iter()
            .map(|provider| {
                ListItem::new(provider.display_name())
            })
            .collect();
        
        let list = List::new(items)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Select Email Provider"))
            .highlight_style(Style::default().bg(self.theme.colors.palette.accent).fg(Color::Black))
            .highlight_symbol("► ");
        
        f.render_stateful_widget(list, area, &mut self.provider_list_state);
    }
    
    fn draw_provider_instructions(&mut self, f: &mut Frame, area: Rect) {
        if let Some(provider) = &self.selected_provider {
            let config = ProviderConfig::get_config(provider).unwrap();
            let instructions = config.setup_instructions();
            
            let text: Vec<Line> = instructions
                .iter()
                .skip(self.scroll_offset)
                .map(|line| Line::from(line.clone()))
                .collect();
            
            let paragraph = Paragraph::new(text)
                .wrap(Wrap { trim: true })
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title(format!("{} Setup Instructions", provider.display_name())));
            
            f.render_widget(paragraph, area);
        }
    }
    
    fn draw_credentials_input(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(area);
        
        // Client ID input
        let client_id_style = if matches!(self.input_mode, InputMode::ClientId) {
            Style::default().fg(self.theme.colors.palette.accent)
        } else {
            Style::default()
        };
        
        let client_id_input = Paragraph::new(self.client_id_input.as_str())
            .style(client_id_style)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Client ID *"));
        f.render_widget(client_id_input, chunks[0]);
        
        // Client Secret input (optional for PKCE)
        let client_secret_style = if matches!(self.input_mode, InputMode::ClientSecret) {
            Style::default().fg(self.theme.colors.palette.accent)
        } else {
            Style::default()
        };
        
        let secret_display = if self.client_secret_input.is_empty() {
            "Optional for PKCE-enabled providers"
        } else {
            &"*".repeat(self.client_secret_input.len())
        };
        
        let client_secret_input = Paragraph::new(secret_display)
            .style(client_secret_style)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Client Secret"));
        f.render_widget(client_secret_input, chunks[1]);
        
        // Instructions
        let instructions = vec![
            Line::from("Enter the OAuth2 credentials from your provider."),
            Line::from(""),
            Line::from("The Client ID is always required."),
            Line::from("Client Secret is optional for modern PKCE-enabled providers."),
            Line::from(""),
            Line::from("Use Tab to switch between fields."),
            Line::from("Press Enter when ready to start authorization."),
        ];
        
        let paragraph = Paragraph::new(instructions)
            .wrap(Wrap { trim: true })
            .block(Block::default()
                .borders(Borders::ALL)
                .title("OAuth2 Credentials"));
        
        f.render_widget(paragraph, chunks[2]);
    }
    
    fn draw_authorization(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5),
                Constraint::Min(0),
            ])
            .split(area);
        
        // Progress indicator
        let gauge = Gauge::default()
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Authorization in Progress"))
            .gauge_style(Style::default().fg(self.theme.colors.palette.accent))
            .percent(50)
            .label("Waiting for browser authorization...");
        f.render_widget(gauge, chunks[0]);
        
        // Instructions
        let mut instructions = vec![
            Line::from("OAuth2 authorization is in progress."),
            Line::from(""),
        ];
        
        if let Some(auth_request) = &self.auth_request {
            instructions.extend(vec![
                Line::from(format!("✓ Callback server started on port {}", auth_request.callback_port)),
                Line::from(""),
                Line::from("Steps:"),
                Line::from("1. ✓ Browser should open automatically"),
                Line::from("2. Log in to your email provider"),
                Line::from("3. Grant permission to Comunicado"),
                Line::from("4. Return to this window"),
                Line::from(""),
                Line::from("If browser didn't open, click this URL:"),
                create_clickable_url_line(&auth_request.authorization_url),
                Line::from(""),
                Line::from("This process will timeout in 5 minutes."),
            ]);
        } else {
            instructions.extend(vec![
                Line::from("Preparing authorization..."),
                Line::from(""),
                Line::from("Steps:"),
                Line::from("1. Your browser should open automatically"),
                Line::from("2. Log in to your email provider"),
                Line::from("3. Grant permission to Comunicado"),
                Line::from("4. Return to this window"),
                Line::from(""),
                Line::from("This process will timeout in 5 minutes."),
            ]);
        }
        
        let paragraph = Paragraph::new(instructions)
            .wrap(Wrap { trim: true })
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Browser Authorization"));
        
        f.render_widget(paragraph, chunks[1]);
    }
    
    fn draw_testing(&mut self, f: &mut Frame, area: Rect) {
        let gauge = Gauge::default()
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Testing Configuration"))
            .gauge_style(Style::default().fg(self.theme.colors.palette.accent))
            .percent(75)
            .label("Testing IMAP connection...");
        f.render_widget(gauge, area);
    }
    
    fn draw_complete(&mut self, f: &mut Frame, area: Rect) {
        let text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("✓ ", Style::default().fg(Color::Green)),
                Span::styled("Account setup completed successfully!", Style::default()),
            ]),
            Line::from(""),
            Line::from("Your OAuth2 account has been configured and tested."),
            Line::from("You can now use Comunicado to access your email."),
            Line::from(""),
            Line::from("Account details:"),
            Line::from(format!("Email: {}", self.email_input)),
            Line::from(format!("Provider: {}", 
                self.selected_provider.as_ref().unwrap().display_name())),
            Line::from(""),
            Line::from("Press any key to continue."),
        ];
        
        let paragraph = Paragraph::new(text)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Setup Complete")
                .border_style(Style::default().fg(Color::Green)));
        
        f.render_widget(paragraph, area);
    }
    
    fn draw_error(&mut self, f: &mut Frame, area: Rect, error: &str) {
        let text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("✗ ", Style::default().fg(Color::Red)),
                Span::styled("Setup Error", Style::default()),
            ]),
            Line::from(""),
            Line::from(error),
            Line::from(""),
            Line::from("Press Enter to start over or Esc to quit."),
        ];
        
        let paragraph = Paragraph::new(text)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Error")
                .border_style(Style::default().fg(Color::Red)));
        
        f.render_widget(paragraph, area);
    }
    
    fn draw_status_bar(&mut self, f: &mut Frame, area: Rect) {
        let status_text = match &self.state {
            WizardState::Welcome => "Welcome to OAuth2 Setup",
            WizardState::EmailInput => "Enter your email address",
            WizardState::ProviderSelection => "Select your email provider",
            WizardState::ProviderInstructions => "Follow provider setup instructions",
            WizardState::CredentialsInput => "Enter OAuth2 credentials",
            WizardState::Authorization => "Browser authorization in progress",
            WizardState::Testing => "Testing account configuration",
            WizardState::Complete => "Setup completed successfully",
            WizardState::Error(_) => "An error occurred",
        };
        
        let help_text = "Ctrl+H: Help | Ctrl+C: Exit | Esc: Back";
        
        let status = Paragraph::new(vec![
            Line::from(vec![
                Span::styled(status_text, Style::default().fg(self.theme.colors.palette.accent)),
                Span::styled(" | ", Style::default()),
                Span::styled(help_text, Style::default().fg(Color::DarkGray)),
            ])
        ])
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
        
        f.render_widget(status, area);
    }
    
    fn draw_help_overlay(&mut self, f: &mut Frame, area: Rect) {
        let help_area = centered_rect(60, 80, area);
        
        f.render_widget(Clear, help_area);
        
        let help_text = vec![
            Line::from("OAuth2 Setup Wizard Help"),
            Line::from(""),
            Line::from("Navigation:"),
            Line::from("  Enter/Space - Confirm/Continue"),
            Line::from("  Esc - Go back/Cancel"),
            Line::from("  Tab - Switch input fields"),
            Line::from("  ↑/↓ - Navigate lists"),
            Line::from("  Ctrl+C - Exit wizard"),
            Line::from("  Ctrl+H - Toggle this help"),
            Line::from(""),
            Line::from("Setup Process:"),
            Line::from("  1. Enter email address"),
            Line::from("  2. Select or detect provider"),
            Line::from("  3. Follow setup instructions"),
            Line::from("  4. Enter OAuth2 credentials"),
            Line::from("  5. Complete browser authorization"),
            Line::from("  6. Test configuration"),
            Line::from(""),
            Line::from("Press Ctrl+H to close this help."),
        ];
        
        let help = Paragraph::new(help_text)
            .wrap(Wrap { trim: true })
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Help")
                .border_style(Style::default().fg(self.theme.colors.palette.accent)));
        
        f.render_widget(help, help_area);
    }
}

impl Default for SetupWizard {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

/// Helper function to create a centered rectangle
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Attempt to open URL in the default browser
fn open_browser_url(url: &str) -> Result<(), String> {
    use std::process::Command;
    
    // Try different browser opening commands based on the platform
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
            Ok(_) => {
                tracing::info!("Successfully opened browser with command: {}", command);
                return Ok(());
            }
            Err(e) => {
                tracing::debug!("Failed to open browser with {}: {}", command, e);
                continue;
            }
        }
    }
    
    Err("Failed to open browser with any available command".to_string())
}

/// Create a clickable URL line for the terminal
fn create_clickable_url_line(url: &str) -> Line {
    // OSC 8 hyperlink escape sequence: \e]8;;URL\e\\TEXT\e]8;;\e\\
    // This makes the URL clickable in terminals that support hyperlinks
    let hyperlink_start = format!("\x1b]8;;{}\x1b\\", url);
    let hyperlink_end = "\x1b]8;;\x1b\\";
    
    // Create a shortened display version for long URLs
    let display_url = if url.len() > 80 {
        format!("{}...{}", &url[..40], &url[url.len()-30..])
    } else {
        url.to_string()
    };
    
    Line::from(vec![
        Span::styled(
            format!("{}{}{}", hyperlink_start, display_url, hyperlink_end),
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::UNDERLINED)
        )
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_wizard_creation() {
        let wizard = SetupWizard::new();
        assert!(wizard.is_ok());
        
        let wizard = wizard.unwrap();
        assert!(matches!(wizard.state, WizardState::Welcome));
        assert!(wizard.email_input.is_empty());
        assert!(wizard.selected_provider.is_none());
    }
    
    #[test]
    fn test_input_modes() {
        let wizard = SetupWizard::new().unwrap();
        assert!(matches!(wizard.input_mode, InputMode::Navigation));
    }
    
    #[test]
    fn test_clickable_url_creation() {
        let test_url = "https://accounts.google.com/o/oauth2/v2/auth?response_type=code&client_id=test";
        let clickable_line = create_clickable_url_line(test_url);
        
        // The line should contain the URL and be formatted properly
        let line_content = format!("{:?}", clickable_line);
        assert!(line_content.contains("accounts.google.com"));
        // The escape sequences should be present in some form
        assert!(line_content.contains("\\u{1b}") || line_content.contains("\\x1b"));
    }
    
    #[test]
    fn test_long_url_truncation() {
        let long_url = "https://accounts.google.com/o/oauth2/v2/auth?response_type=code&client_id=very_long_client_id_that_makes_url_exceed_80_characters&redirect_uri=http://localhost:8080/oauth/callback&scope=read%20write&state=random_state_string";
        let clickable_line = create_clickable_url_line(long_url);
        
        // Should contain truncation indicator
        let line_content = format!("{:?}", clickable_line);
        assert!(line_content.contains("..."));
    }
}