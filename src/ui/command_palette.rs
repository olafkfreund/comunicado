/// Command Palette System for TUI Navigation
/// 
/// Provides a discoverable, context-aware command interface that works reliably
/// across all terminal environments. Activated with Ctrl+D, shows available
/// commands with letter shortcuts (a, b, c, etc.) for easy selection.

use crate::theme::Theme;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};
use std::collections::HashMap;

/// Command palette visibility and state
#[derive(Debug, Clone, PartialEq)]
pub enum PaletteState {
    Hidden,
    Visible,
    Executing(String), // Command being executed
}

/// Available command categories based on current context
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CommandContext {
    EmailList,
    EmailViewer,
    Calendar,
    Compose,
    Settings,
    Global, // Always available commands
}

/// Individual command definition
#[derive(Debug, Clone)]
pub struct Command {
    pub id: String,
    pub name: String,
    pub description: String,
    pub shortcut: char, // Single letter shortcut (a, b, c, etc.)
    pub context: CommandContext,
    pub action: CommandAction,
}

/// Actions that commands can perform
#[derive(Debug, Clone)]
pub enum CommandAction {
    ToggleAIAssistant,
    OpenSettings,
    ComposeEmail,
    SearchEmails,
    ToggleCalendar,
    SyncEmails,
    ShowHelp,
    ToggleFolder,
    MarkAsRead,
    MarkAsUnread,
    DeleteEmail,
    ReplyEmail,
    ForwardEmail,
    CreateEvent,
    ViewEvent,
    ExportData,
    ImportData,
    ChangeTheme,
    ShowKeyboardShortcuts,
    Custom(String), // For extensibility
}

/// Command Palette Manager
pub struct CommandPalette {
    state: PaletteState,
    current_context: CommandContext,
    commands: HashMap<CommandContext, Vec<Command>>,
    selected_index: usize,
    list_state: ListState,
    search_query: String,
    filtered_commands: Vec<Command>,
}

impl Default for CommandPalette {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandPalette {
    /// Create a new command palette
    pub fn new() -> Self {
        let mut palette = Self {
            state: PaletteState::Hidden,
            current_context: CommandContext::Global,
            commands: HashMap::new(),
            selected_index: 0,
            list_state: ListState::default(),
            search_query: String::new(),
            filtered_commands: Vec::new(),
        };
        
        palette.initialize_commands();
        palette
    }
    
    /// Initialize all available commands
    fn initialize_commands(&mut self) {
        // Global commands (always available)
        let global_commands = vec![
            Command {
                id: "ai_assistant".to_string(),
                name: "AI Assistant".to_string(),
                description: "Toggle AI Assistant panel".to_string(),
                shortcut: 'a',
                context: CommandContext::Global,
                action: CommandAction::ToggleAIAssistant,
            },
            Command {
                id: "settings".to_string(),
                name: "Settings".to_string(),
                description: "Open application settings".to_string(),
                shortcut: 's',
                context: CommandContext::Global,
                action: CommandAction::OpenSettings,
            },
            Command {
                id: "help".to_string(),
                name: "Help".to_string(),
                description: "Show help and keyboard shortcuts".to_string(),
                shortcut: 'h',
                context: CommandContext::Global,
                action: CommandAction::ShowHelp,
            },
            Command {
                id: "sync".to_string(),
                name: "Sync Emails".to_string(),
                description: "Synchronize all email accounts".to_string(),
                shortcut: 'y',
                context: CommandContext::Global,
                action: CommandAction::SyncEmails,
            },
            Command {
                id: "compose".to_string(),
                name: "Compose Email".to_string(),
                description: "Create a new email".to_string(),
                shortcut: 'c',
                context: CommandContext::Global,
                action: CommandAction::ComposeEmail,
            },
            Command {
                id: "search".to_string(),
                name: "Search Emails".to_string(),
                description: "Search through all emails".to_string(),
                shortcut: 'f',
                context: CommandContext::Global,
                action: CommandAction::SearchEmails,
            },
        ];
        
        // Email list specific commands
        let email_list_commands = vec![
            Command {
                id: "mark_read".to_string(),
                name: "Mark as Read".to_string(),
                description: "Mark selected email as read".to_string(),
                shortcut: 'r',
                context: CommandContext::EmailList,
                action: CommandAction::MarkAsRead,
            },
            Command {
                id: "mark_unread".to_string(),
                name: "Mark as Unread".to_string(),
                description: "Mark selected email as unread".to_string(),
                shortcut: 'u',
                context: CommandContext::EmailList,
                action: CommandAction::MarkAsUnread,
            },
            Command {
                id: "delete".to_string(),
                name: "Delete Email".to_string(),
                description: "Delete selected email".to_string(),
                shortcut: 'd',
                context: CommandContext::EmailList,
                action: CommandAction::DeleteEmail,
            },
        ];
        
        // Email viewer specific commands
        let email_viewer_commands = vec![
            Command {
                id: "reply".to_string(),
                name: "Reply".to_string(),
                description: "Reply to this email".to_string(),
                shortcut: 'r',
                context: CommandContext::EmailViewer,
                action: CommandAction::ReplyEmail,
            },
            Command {
                id: "forward".to_string(),
                name: "Forward".to_string(),
                description: "Forward this email".to_string(),
                shortcut: 'w',
                context: CommandContext::EmailViewer,
                action: CommandAction::ForwardEmail,
            },
        ];
        
        // Calendar specific commands
        let calendar_commands = vec![
            Command {
                id: "create_event".to_string(),
                name: "Create Event".to_string(),
                description: "Create a new calendar event".to_string(),
                shortcut: 'n',
                context: CommandContext::Calendar,
                action: CommandAction::CreateEvent,
            },
            Command {
                id: "view_event".to_string(),
                name: "View Event".to_string(),
                description: "View selected calendar event".to_string(),
                shortcut: 'v',
                context: CommandContext::Calendar,
                action: CommandAction::ViewEvent,
            },
        ];
        
        self.commands.insert(CommandContext::Global, global_commands);
        self.commands.insert(CommandContext::EmailList, email_list_commands);
        self.commands.insert(CommandContext::EmailViewer, email_viewer_commands);
        self.commands.insert(CommandContext::Calendar, calendar_commands);
    }
    
    /// Show the command palette
    pub fn show(&mut self, context: CommandContext) {
        self.state = PaletteState::Visible;
        self.current_context = context;
        self.selected_index = 0;
        self.search_query.clear();
        self.update_filtered_commands();
        self.list_state.select(Some(0));
    }
    
    /// Hide the command palette  
    pub fn hide(&mut self) {
        self.state = PaletteState::Hidden;
        self.search_query.clear();
        self.selected_index = 0;
        self.list_state.select(None);
    }
    
    /// Check if palette is visible
    pub fn is_visible(&self) -> bool {
        matches!(self.state, PaletteState::Visible)
    }
    
    /// Update filtered commands based on current context and search
    fn update_filtered_commands(&mut self) {
        let mut commands = Vec::new();
        
        // Always include global commands
        if let Some(global_cmds) = self.commands.get(&CommandContext::Global) {
            commands.extend(global_cmds.clone());
        }
        
        // Add context-specific commands
        if let Some(context_cmds) = self.commands.get(&self.current_context) {
            commands.extend(context_cmds.clone());
        }
        
        // Filter by search query if present
        if !self.search_query.is_empty() {
            commands.retain(|cmd| {
                cmd.name.to_lowercase().contains(&self.search_query.to_lowercase()) ||
                cmd.description.to_lowercase().contains(&self.search_query.to_lowercase())
            });
        }
        
        self.filtered_commands = commands;
        
        // Reset selection if needed
        if self.selected_index >= self.filtered_commands.len() && !self.filtered_commands.is_empty() {
            self.selected_index = 0;
            self.list_state.select(Some(0));
        }
    }
    
    /// Handle character input for command selection
    pub fn handle_char(&mut self, c: char) -> Option<CommandAction> {
        if !self.is_visible() {
            return None;
        }
        
        // Look for command with matching shortcut
        for command in &self.filtered_commands {
            if command.shortcut == c.to_ascii_lowercase() {
                let action = command.action.clone();
                self.hide(); // Close palette after selection
                return Some(action);
            }
        }
        
        None
    }
    
    /// Move selection up
    pub fn select_previous(&mut self) {
        if !self.filtered_commands.is_empty() {
            if self.selected_index > 0 {
                self.selected_index -= 1;
            } else {
                self.selected_index = self.filtered_commands.len() - 1;
            }
            self.list_state.select(Some(self.selected_index));
        }
    }
    
    /// Move selection down
    pub fn select_next(&mut self) {
        if !self.filtered_commands.is_empty() {
            if self.selected_index < self.filtered_commands.len() - 1 {
                self.selected_index += 1;
            } else {
                self.selected_index = 0;
            }
            self.list_state.select(Some(self.selected_index));
        }
    }
    
    /// Execute currently selected command
    pub fn execute_selected(&mut self) -> Option<CommandAction> {
        if self.is_visible() && self.selected_index < self.filtered_commands.len() {
            let action = self.filtered_commands[self.selected_index].action.clone();
            self.hide();
            Some(action)
        } else {
            None
        }
    }
    
    /// Render the command palette
    pub fn render(&mut self, frame: &mut Frame, theme: &Theme) {
        if !self.is_visible() {
            return;
        }
        
        let area = frame.size();
        
        // Create a centered popup area
        let popup_area = self.get_popup_area(area);
        
        // Clear the background
        frame.render_widget(Clear, popup_area);
        
        // Create the main block
        let block = Block::default()
            .title(" Command Palette (Ctrl+D) ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.colors.palette.accent))
            .style(Style::default().bg(theme.colors.palette.background));
            
        frame.render_widget(block, popup_area);
        
        // Layout for content
        let inner_area = popup_area.inner(&Margin::new(1, 1));
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Header
                Constraint::Min(5),    // Command list
                Constraint::Length(2), // Footer
            ])
            .split(inner_area);
        
        // Render header
        self.render_header(frame, chunks[0], theme);
        
        // Render command list
        self.render_command_list(frame, chunks[1], theme);
        
        // Render footer
        self.render_footer(frame, chunks[2], theme);
    }
    
    /// Render the header section
    fn render_header(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let context_name = match self.current_context {
            CommandContext::EmailList => "Email List",
            CommandContext::EmailViewer => "Email Viewer", 
            CommandContext::Calendar => "Calendar",
            CommandContext::Compose => "Compose",
            CommandContext::Settings => "Settings",
            CommandContext::Global => "Global",
        };
        
        let text = format!("Context: {} | {} commands available", 
                          context_name, self.filtered_commands.len());
        
        let paragraph = Paragraph::new(text)
            .style(Style::default()
                .fg(theme.colors.palette.text_secondary)
                .add_modifier(Modifier::ITALIC))
            .alignment(Alignment::Center);
            
        frame.render_widget(paragraph, area);
    }
    
    /// Render the command list
    fn render_command_list(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let items: Vec<ListItem> = self.filtered_commands
            .iter()
            .map(|command| {
                let shortcut_style = Style::default()
                    .fg(theme.colors.palette.accent)
                    .add_modifier(Modifier::BOLD);
                    
                let name_style = Style::default()
                    .fg(theme.colors.palette.text_primary);
                    
                let desc_style = Style::default()
                    .fg(theme.colors.palette.text_secondary);
                
                let line = Line::from(vec![
                    Span::styled(format!(" {} ", command.shortcut.to_uppercase()), shortcut_style),
                    Span::styled(format!("{}", command.name), name_style),
                    Span::styled(format!(" - {}", command.description), desc_style),
                ]);
                
                ListItem::new(line)
            })
            .collect();
        
        let list = List::new(items)
            .highlight_style(Style::default()
                .bg(theme.colors.palette.accent)
                .fg(theme.colors.palette.background)
                .add_modifier(Modifier::BOLD))
            .highlight_symbol("► ");
            
        frame.render_stateful_widget(list, area, &mut self.list_state);
    }
    
    /// Render the footer section
    fn render_footer(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let text = "Press letter key to execute • ↑↓ to navigate • Enter to execute • Esc to close";
        
        let paragraph = Paragraph::new(text)
            .style(Style::default()
                .fg(theme.colors.palette.text_secondary)
                .add_modifier(Modifier::ITALIC))
            .alignment(Alignment::Center);
            
        frame.render_widget(paragraph, area);
    }
    
    /// Calculate popup area (centered, 60% width, 70% height)
    fn get_popup_area(&self, area: Rect) -> Rect {
        let popup_width = area.width.saturating_mul(60) / 100;
        let popup_height = area.height.saturating_mul(70) / 100;
        
        let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
        let y = area.y + (area.height.saturating_sub(popup_height)) / 2;
        
        Rect::new(x, y, popup_width, popup_height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_command_palette_creation() {
        let palette = CommandPalette::new();
        assert_eq!(palette.state, PaletteState::Hidden);
        assert!(!palette.is_visible());
    }
    
    #[test]
    fn test_show_hide_palette() {
        let mut palette = CommandPalette::new();
        
        palette.show(CommandContext::EmailList);
        assert!(palette.is_visible());
        assert_eq!(palette.current_context, CommandContext::EmailList);
        
        palette.hide();
        assert!(!palette.is_visible());
    }
    
    #[test]
    fn test_command_shortcuts() {
        let mut palette = CommandPalette::new();
        palette.show(CommandContext::Global);
        
        // Test AI assistant shortcut
        let action = palette.handle_char('a');
        assert!(matches!(action, Some(CommandAction::ToggleAIAssistant)));
        assert!(!palette.is_visible()); // Should hide after selection
    }
    
    #[test]
    fn test_navigation() {
        let mut palette = CommandPalette::new();
        palette.show(CommandContext::Global);
        
        let initial_index = palette.selected_index;
        palette.select_next();
        assert_ne!(palette.selected_index, initial_index);
        
        palette.select_previous();
        assert_eq!(palette.selected_index, initial_index);
    }
}