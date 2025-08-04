//! Modular UI System
//!
//! Replaces the monolithic UI struct with a component-based architecture.

use super::{
    ComponentRegistry, UIServices, LayoutManager, EmailComponent, CalendarComponent, 
    ContactsComponent, ComponentId, UIComponent, ComponentResult,
    UIEvent, EventResult, ComponentMetrics, LayoutSpec, LayoutTemplate,
};
use crate::{
    theme::Theme,
    email::EmailDatabase,
    calendar::CalendarManager,
    contacts::ContactsManager,
    notifications::UnifiedNotificationManager,
    oauth2::SecureStorage,
    imap::ImapAccountManager,
    smtp::SmtpService,
};
use ratatui::{layout::Rect, Frame, style::{Style, Modifier}};
use std::sync::Arc;
use std::collections::HashMap;
use crossterm::event::KeyCode;

/// Application modes for the modular UI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Email,
    Calendar,
    Contacts,
    Settings,
    Help,
}

impl AppMode {
    pub fn name(&self) -> &'static str {
        match self {
            AppMode::Email => "Email",
            AppMode::Calendar => "Calendar", 
            AppMode::Contacts => "Contacts",
            AppMode::Settings => "Settings",
            AppMode::Help => "Help",
        }
    }

    pub fn all() -> &'static [AppMode] {
        &[
            AppMode::Email,
            AppMode::Calendar,
            AppMode::Contacts,
            AppMode::Settings,
            AppMode::Help,
        ]
    }
}

/// Modular UI system that manages components and application state
pub struct ModularUI {
    // Core systems
    component_registry: ComponentRegistry,
    ui_services: UIServices,
    layout_manager: LayoutManager,
    
    // Component IDs for direct access
    email_component_id: Option<ComponentId>,
    calendar_component_id: Option<ComponentId>,
    contacts_component_id: Option<ComponentId>,
    
    // Application state
    current_mode: AppMode,
    previous_mode: Option<AppMode>,
    mode_history: Vec<AppMode>,
    
    // Global UI state
    is_initialized: bool,
    show_help: bool,
    show_settings: bool,
    global_focus: bool,
    
    // Performance tracking
    frame_count: u64,
    last_performance_check: std::time::Instant,
    average_frame_time: std::time::Duration,
}

impl ModularUI {
    /// Create a new modular UI system
    pub fn new() -> ComponentResult<Self> {
        Ok(Self {
            component_registry: ComponentRegistry::new(),
            ui_services: UIServices::new(),
            layout_manager: LayoutManager::default(),
            email_component_id: None,
            calendar_component_id: None,
            contacts_component_id: None,
            current_mode: AppMode::Email,
            previous_mode: None,
            mode_history: Vec::new(),
            is_initialized: false,
            show_help: false,
            show_settings: false,
            global_focus: true,
            frame_count: 0,
            last_performance_check: std::time::Instant::now(),
            average_frame_time: std::time::Duration::ZERO,
        })
    }
    
    /// Initialize the modular UI with all services and components
    pub async fn initialize(
        &mut self,
        database: Option<Arc<EmailDatabase>>,
        imap_manager: Option<Arc<ImapAccountManager>>,
        smtp_service: Option<SmtpService>,
        calendar_manager: Option<Arc<CalendarManager>>,
        contacts_manager: Option<Arc<ContactsManager>>,
        notification_manager: Option<Arc<UnifiedNotificationManager>>,
        secure_storage: Option<SecureStorage>,
    ) -> ComponentResult<()> {
        // Initialize UI services
        self.ui_services.initialize(
            database.clone(),
            imap_manager,
            smtp_service,
            calendar_manager.clone(),
            contacts_manager.clone(),
            notification_manager,
            secure_storage,
        ).await.map_err(|e| super::ComponentError::InitializationFailed(e.to_string()))?;
        
        // Create and register email component
        let mut email_component = EmailComponent::new()
            .with_services(database, None /* sender_recognition service */);
        email_component.initialize()?;
        let email_id = self.component_registry.register(email_component)?;
        self.email_component_id = Some(email_id);
        
        // Create and register calendar component
        let mut calendar_component = CalendarComponent::new();
        if let Some(cal_manager) = calendar_manager {
            calendar_component = calendar_component.with_calendar_manager(cal_manager);
        }
        calendar_component.initialize()?;
        let calendar_id = self.component_registry.register(calendar_component)?;
        self.calendar_component_id = Some(calendar_id);
        
        // Create and register contacts component
        let mut contacts_component = ContactsComponent::new()
            .with_services(contacts_manager, None /* sender_recognition service */);
        contacts_component.initialize()?;
        let contacts_id = self.component_registry.register(contacts_component)?;
        self.contacts_component_id = Some(contacts_id);
        
        // Set initial focus to email component
        self.component_registry.set_focus(Some(email_id))?;
        
        // Register custom layouts for different modes
        self.register_custom_layouts();
        
        self.is_initialized = true;
        Ok(())
    }
    
    /// Register custom layouts for different application modes
    fn register_custom_layouts(&mut self) {
        // Email mode layout - three pane with folder tree, message list, and preview
        let email_layout = LayoutSpec::new(
            "email_mode".to_string(),
            LayoutTemplate::ThreePane {
                left: 0.25,   // Folder tree
                center: 0.35, // Message list  
                right: 0.40,  // Email preview
            },
        )
        .with_responsive_rule(
            super::ResponsiveRule::when_width_lt(120)
                .use_two_pane(0.4, 0.6) // Collapse to list + preview
        )
        .with_responsive_rule(
            super::ResponsiveRule::when_width_lt(80)
                .use_full_screen() // Single pane on small screens
        )
        .with_min_size(60, 20);
        
        self.layout_manager.register_layout(email_layout);
        
        // Calendar mode layout - header/footer with main calendar view
        let calendar_layout = LayoutSpec::new(
            "calendar_mode".to_string(),
            LayoutTemplate::HeaderFooter {
                header_height: 4,
                footer_height: 1,
            },
        )
        .with_responsive_rule(
            super::ResponsiveRule::when_height_lt(20)
                .use_full_screen()
        );
        
        self.layout_manager.register_layout(calendar_layout);
        
        // Contacts mode layout - sidebar with contact list and details
        let contacts_layout = LayoutSpec::new(
            "contacts_mode".to_string(),
            LayoutTemplate::TwoPane {
                left: 0.6,   // Contact list
                right: 0.4,  // Contact details
            },
        )
        .with_responsive_rule(
            super::ResponsiveRule::when_width_lt(100)
                .use_full_screen()
        );
        
        self.layout_manager.register_layout(contacts_layout);
    }
    
    /// Get the current application mode
    pub fn current_mode(&self) -> AppMode {
        self.current_mode
    }
    
    /// Switch to a different application mode
    pub fn switch_mode(&mut self, mode: AppMode) -> ComponentResult<()> {
        if self.current_mode != mode {
            // Store previous mode
            self.previous_mode = Some(self.current_mode);
            self.mode_history.push(self.current_mode);
            
            // Limit history size
            if self.mode_history.len() > 10 {
                self.mode_history.remove(0);
            }
            
            // Update current mode
            self.current_mode = mode;
            
            // Update component focus based on mode
            let component_id = match mode {
                AppMode::Email => self.email_component_id,
                AppMode::Calendar => self.calendar_component_id,
                AppMode::Contacts => self.contacts_component_id,
                AppMode::Settings | AppMode::Help => None, // These don't have dedicated components yet
            };
            
            if let Some(id) = component_id {
                self.component_registry.set_focus(Some(id))?;
            }
        }
        
        Ok(())
    }
    
    /// Go back to the previous mode
    pub fn go_back(&mut self) -> ComponentResult<()> {
        if let Some(prev_mode) = self.previous_mode {
            self.switch_mode(prev_mode)?;
        }
        Ok(())
    }
    
    /// Toggle help overlay
    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }
    
    /// Toggle settings overlay  
    pub fn toggle_settings(&mut self) {
        self.show_settings = !self.show_settings;
    }
    
    /// Check if the UI is initialized
    pub fn is_initialized(&self) -> bool {
        self.is_initialized
    }
    
    /// Get direct access to email component
    pub fn email_component(&mut self) -> Option<&mut super::ComponentHandle> {
        if let Some(id) = self.email_component_id {
            self.component_registry.get_mut(id)
        } else {
            None
        }
    }
    
    /// Get direct access to calendar component
    pub fn calendar_component(&mut self) -> Option<&mut super::ComponentHandle> {
        if let Some(id) = self.calendar_component_id {
            self.component_registry.get_mut(id)
        } else {
            None
        }
    }
    
    /// Get direct access to contacts component
    pub fn contacts_component(&mut self) -> Option<&mut super::ComponentHandle> {
        if let Some(id) = self.contacts_component_id {
            self.component_registry.get_mut(id)
        } else {
            None
        }
    }
    
    /// Render the entire modular UI
    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) -> ComponentResult<()> {
        let start_time = std::time::Instant::now();
        
        if !self.is_initialized {
            // Render loading screen
            self.render_loading_screen(frame, area, theme);
            return Ok(());
        }
        
        // Get layout for current mode
        let layout_id = match self.current_mode {
            AppMode::Email => "email_mode",
            AppMode::Calendar => "calendar_mode", 
            AppMode::Contacts => "contacts_mode",
            AppMode::Settings | AppMode::Help => "fullscreen", // Use default fullscreen
        };
        
        let layout_areas = self.layout_manager.calculate_layout(layout_id, area)?;
        let main_area = layout_areas.get(0).copied().unwrap_or(area);
        
        // Render components based on current mode 
        // TODO: Fix borrowing issues with component rendering
        match self.current_mode {
            AppMode::Settings => {
                self.render_settings_placeholder(frame, main_area, theme);
            }
            AppMode::Help => {
                self.render_help_placeholder(frame, main_area, theme);
            }
            _ => {
                // Render a placeholder for now while we work on the borrowing issues
                use ratatui::widgets::{Block, Borders, Paragraph};
                use ratatui::style::{Style, Color};
                
                let mode_name = match self.current_mode {
                    AppMode::Email => "Email",
                    AppMode::Calendar => "Calendar", 
                    AppMode::Contacts => "Contacts",
                    _ => "Unknown",
                };
                
                let placeholder = Paragraph::new(format!("{} Component - Under Development", mode_name))
                    .block(Block::default().borders(Borders::ALL).title(format!("{} Mode", mode_name)))
                    .style(Style::default().fg(Color::Yellow));
                    
                frame.render_widget(placeholder, main_area);
            }
        }
        
        // Render mode tabs at the top
        self.render_mode_tabs(frame, area, theme)?;
        
        // Render overlays
        if self.show_help {
            self.render_help_overlay(frame, area, theme);
        }
        
        if self.show_settings {
            self.render_settings_overlay(frame, area, theme);
        }
        
        // Update performance metrics
        let frame_time = start_time.elapsed();
        self.frame_count += 1;
        
        // Update average frame time (exponential moving average)
        let weight = 0.05; // 5% weight to new frame
        self.average_frame_time = std::time::Duration::from_nanos(
            (self.average_frame_time.as_nanos() as f64 * (1.0 - weight) +
             frame_time.as_nanos() as f64 * weight) as u64
        );
        
        Ok(())
    }
    
    /// Handle global UI events
    pub fn handle_event(&mut self, event: UIEvent) -> ComponentResult<EventResult> {
        // Handle global shortcuts first
        if let UIEvent::Key(key) = &event {
            match key.code {
                KeyCode::F(1) => {
                    self.toggle_help();
                    return Ok(EventResult::Consumed);
                }
                KeyCode::F(2) => {
                    self.toggle_settings();
                    return Ok(EventResult::Consumed);
                }
                KeyCode::F(3) => {
                    self.switch_mode(AppMode::Email)?;
                    return Ok(EventResult::Consumed);
                }
                KeyCode::F(4) => {
                    self.switch_mode(AppMode::Calendar)?;
                    return Ok(EventResult::Consumed);
                }
                KeyCode::F(5) => {
                    self.switch_mode(AppMode::Contacts)?;
                    return Ok(EventResult::Consumed);
                }
                KeyCode::Esc => {
                    if self.show_help {
                        self.show_help = false;
                        return Ok(EventResult::Consumed);
                    }
                    if self.show_settings {
                        self.show_settings = false;
                        return Ok(EventResult::Consumed);
                    }
                    // Let components handle escape if no overlays are shown
                }
                _ => {}
            }
        }
        
        // Route event to active component
        self.component_registry.handle_event(&event)
    }
    
    /// Get comprehensive performance metrics
    pub fn performance_metrics(&self) -> ModularUIMetrics {
        let registry_metrics = self.component_registry.performance_metrics();
        let (cache_hits, cache_misses, cache_hit_rate) = self.layout_manager.cache_stats();
        
        ModularUIMetrics {
            total_components: registry_metrics.total_components,
            total_render_time: registry_metrics.total_render_time,
            total_events_processed: registry_metrics.total_events_processed,
            focused_component: registry_metrics.focused_component,
            frame_count: self.frame_count,
            average_frame_time: self.average_frame_time,
            layout_cache_hits: cache_hits,
            layout_cache_misses: cache_misses,
            layout_cache_hit_rate: cache_hit_rate,
            current_mode: self.current_mode,
            component_metrics: registry_metrics.component_metrics,
        }
    }
    
    /// Render loading screen
    fn render_loading_screen(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        use ratatui::widgets::{Block, Borders, Paragraph};
        use ratatui::layout::Alignment;
        use ratatui::style::{Style, Modifier};
        
        let loading_text = "🚀 Loading Comunicado...\n\nInitializing components and services...";
        let loading_widget = Paragraph::new(loading_text)
            .block(
                Block::default()
                    .title("Comunicado")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.colors.palette.accent))
            )
            .style(
                Style::default()
                    .fg(theme.colors.palette.text_primary)
                    .add_modifier(Modifier::BOLD)
            )
            .alignment(Alignment::Center);
            
        frame.render_widget(loading_widget, area);
    }
    
    /// Render mode tabs
    fn render_mode_tabs(&self, frame: &mut Frame, area: Rect, theme: &Theme) -> ComponentResult<()> {
        use ratatui::widgets::Tabs;
        use ratatui::text::Line;
        use ratatui::layout::{Layout, Constraint, Direction};
        
        // Create small area at top for tabs
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(area);
        
        let tab_titles: Vec<Line> = AppMode::all()
            .iter()
            .map(|mode| Line::from(mode.name()))
            .collect();
        
        let current_tab_index = AppMode::all()
            .iter()
            .position(|&mode| mode == self.current_mode)
            .unwrap_or(0);
        
        let tabs = Tabs::new(tab_titles)
            .highlight_style(
                Style::default()
                    .fg(theme.colors.palette.accent)
                    .add_modifier(Modifier::BOLD)
            )
            .select(current_tab_index);
        
        frame.render_widget(tabs, chunks[0]);
        Ok(())
    }
    
    /// Render settings placeholder
    fn render_settings_placeholder(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        use ratatui::widgets::{Block, Borders, Paragraph};
        use ratatui::layout::Alignment;
        
        let settings_text = "⚙️ Settings\n\nSettings interface coming soon...";
        let settings_widget = Paragraph::new(settings_text)
            .block(
                Block::default()
                    .title("Settings")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.colors.palette.border))
            )
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.colors.palette.text_muted));
            
        frame.render_widget(settings_widget, area);
    }
    
    /// Render help placeholder
    fn render_help_placeholder(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        use ratatui::widgets::{Block, Borders, Paragraph};
        use ratatui::layout::Alignment;
        
        let help_text = "❓ Help\n\nHelp system coming soon...";
        let help_widget = Paragraph::new(help_text)
            .block(
                Block::default()
                    .title("Help")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.colors.palette.border))
            )
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.colors.palette.text_muted));
            
        frame.render_widget(help_widget, area);
    }
    
    /// Render help overlay
    fn render_help_overlay(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        use ratatui::widgets::{Block, Borders, Paragraph, Clear};
        use ratatui::layout::{Alignment, Margin};
        
        let popup_area = area.inner(&Margin { vertical: 4, horizontal: 8 });
        
        frame.render_widget(Clear, popup_area);
        
        let help_text = "🔥 Comunicado Keyboard Shortcuts\n\n\
            F1: Toggle this help\n\
            F2: Settings\n\
            F3: Email mode\n\
            F4: Calendar mode\n\
            F5: Contacts mode\n\
            Esc: Close overlays/go back\n\
            Tab: Cycle through panes\n\
            Enter: Select/Open\n\
            ↑↓: Navigate lists\n\
            ←→: Navigate periods (calendar)\n\n\
            Press Esc to close this help";
        
        let help_widget = Paragraph::new(help_text)
            .block(
                Block::default()
                    .title("Help")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.colors.palette.accent))
            )
            .style(Style::default().fg(theme.colors.palette.text_primary))
            .alignment(Alignment::Left);
            
        frame.render_widget(help_widget, popup_area);
    }
    
    /// Render settings overlay
    fn render_settings_overlay(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        use ratatui::widgets::{Block, Borders, Paragraph, Clear};
        use ratatui::layout::{Alignment, Margin};
        
        let popup_area = area.inner(&Margin { vertical: 4, horizontal: 8 });
        
        frame.render_widget(Clear, popup_area);
        
        let metrics = self.performance_metrics();
        let settings_text = format!(
            "⚙️ Comunicado Settings & Stats\n\n\
            Performance:\n\
            • Components: {}\n\
            • Frames rendered: {}\n\
            • Avg frame time: {:.2}ms\n\
            • Events processed: {}\n\
            • Layout cache hit rate: {:.1}%\n\n\
            Current Mode: {}\n\n\
            Press Esc to close settings",
            metrics.total_components,
            metrics.frame_count,
            metrics.average_frame_time.as_secs_f64() * 1000.0,
            metrics.total_events_processed,
            metrics.layout_cache_hit_rate * 100.0,
            metrics.current_mode.name()
        );
        
        let settings_widget = Paragraph::new(settings_text)
            .block(
                Block::default()
                    .title("Settings")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.colors.palette.accent))
            )
            .style(Style::default().fg(theme.colors.palette.text_primary))
            .alignment(Alignment::Left);
            
        frame.render_widget(settings_widget, popup_area);
    }
}

/// Performance metrics for the modular UI system
#[derive(Debug, Clone)]
pub struct ModularUIMetrics {
    pub total_components: usize,
    pub total_render_time: std::time::Duration,
    pub total_events_processed: u64,
    pub focused_component: Option<ComponentId>,
    pub frame_count: u64,
    pub average_frame_time: std::time::Duration,
    pub layout_cache_hits: u64,
    pub layout_cache_misses: u64,
    pub layout_cache_hit_rate: f64,
    pub current_mode: AppMode,
    pub component_metrics: HashMap<ComponentId, ComponentMetrics>,
}

impl Default for ModularUI {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            // Fallback implementation if creation fails
            Self {
                component_registry: ComponentRegistry::new(),
                ui_services: UIServices::new(),
                layout_manager: LayoutManager::default(),
                email_component_id: None,
                calendar_component_id: None,
                contacts_component_id: None,
                current_mode: AppMode::Email,
                previous_mode: None,
                mode_history: Vec::new(),
                is_initialized: false,
                show_help: false,
                show_settings: false,
                global_focus: true,
                frame_count: 0,
                last_performance_check: std::time::Instant::now(),
                average_frame_time: std::time::Duration::ZERO,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_modular_ui_creation() {
        let ui = ModularUI::new().unwrap();
        assert!(!ui.is_initialized());
        assert_eq!(ui.current_mode(), AppMode::Email);
    }
    
    #[test]
    fn test_mode_switching() {
        let mut ui = ModularUI::new().unwrap();
        
        // Switch to calendar mode
        ui.switch_mode(AppMode::Calendar).unwrap();
        assert_eq!(ui.current_mode(), AppMode::Calendar);
        assert_eq!(ui.previous_mode, Some(AppMode::Email));
        
        // Switch to contacts mode
        ui.switch_mode(AppMode::Contacts).unwrap();
        assert_eq!(ui.current_mode(), AppMode::Contacts);
        assert_eq!(ui.previous_mode, Some(AppMode::Calendar));
        
        // Go back to previous mode
        ui.go_back().unwrap();
        assert_eq!(ui.current_mode(), AppMode::Calendar);
    }
    
    #[test]
    fn test_help_and_settings_toggle() {
        let mut ui = ModularUI::new().unwrap();
        
        assert!(!ui.show_help);
        ui.toggle_help();
        assert!(ui.show_help);
        
        assert!(!ui.show_settings);
        ui.toggle_settings();
        assert!(ui.show_settings);
    }
    
    #[test]
    fn test_performance_metrics() {
        let ui = ModularUI::new().unwrap();
        let metrics = ui.performance_metrics();
        
        assert_eq!(metrics.total_components, 0); // No components registered yet
        assert_eq!(metrics.frame_count, 0);
        assert_eq!(metrics.current_mode, AppMode::Email);
    }
}