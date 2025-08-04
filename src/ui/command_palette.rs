/// Command Palette System for TUI Navigation
/// 
/// Provides a discoverable, context-aware command interface that works reliably
/// across all terminal environments. Activated with Ctrl+D, shows available
/// commands with letter shortcuts (a, b, c, etc.) for easy selection.

use crate::theme::Theme;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Style},
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
    Contacts,
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
    // Global actions
    ToggleAIAssistant,
    OpenSettings,
    ComposeEmail,
    SearchEmails,
    ToggleCalendar,
    ToggleContacts,
    SyncEmails,
    ShowHelp,
    ToggleFolder,
    ShowKeyboardShortcuts,
    ChangeTheme,
    ExportData,
    ImportData,
    
    // Email actions
    MarkAsRead,
    MarkAsUnread,
    DeleteEmail,
    ReplyEmail,
    ReplyAllEmail,
    ForwardEmail,
    ArchiveEmail,
    FlagEmail,
    MoveEmail,
    
    // Calendar actions
    CreateEvent,
    ViewEvent,
    EditEvent,
    DeleteEvent,
    CreateTodo,
    ViewWeek,
    ViewMonth,
    ViewDay,
    SyncCalendar,
    
    // Contact actions
    CreateContact,
    EditContact,
    DeleteContact,
    ViewContact,
    SearchContacts,
    ImportContacts,
    ExportContacts,
    AddToFavorites,
    
    // Compose actions
    SendEmail,
    SaveDraft,
    AttachFile,
    ToggleFormat,
    InsertSignature,
    
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
            Command {
                id: "calendar".to_string(),
                name: "Calendar".to_string(),
                description: "Open calendar view".to_string(),
                shortcut: 'l',
                context: CommandContext::Global,
                action: CommandAction::ToggleCalendar,
            },
            Command {
                id: "contacts".to_string(),
                name: "Contacts".to_string(),
                description: "Open contacts view".to_string(),
                shortcut: 'o',
                context: CommandContext::Global,
                action: CommandAction::ToggleContacts,
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
            Command {
                id: "archive".to_string(),
                name: "Archive Email".to_string(),
                description: "Archive selected email".to_string(),
                shortcut: 'e',
                context: CommandContext::EmailList,
                action: CommandAction::ArchiveEmail,
            },
            Command {
                id: "flag".to_string(),
                name: "Flag Email".to_string(),
                description: "Flag selected email".to_string(),
                shortcut: 'g',
                context: CommandContext::EmailList,
                action: CommandAction::FlagEmail,
            },
            Command {
                id: "move".to_string(),
                name: "Move Email".to_string(),
                description: "Move selected email to folder".to_string(),
                shortcut: 'm',
                context: CommandContext::EmailList,
                action: CommandAction::MoveEmail,
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
                id: "reply_all".to_string(),
                name: "Reply All".to_string(),
                description: "Reply to all recipients".to_string(),
                shortcut: 'e',
                context: CommandContext::EmailViewer,
                action: CommandAction::ReplyAllEmail,
            },
            Command {
                id: "forward".to_string(),
                name: "Forward".to_string(),
                description: "Forward this email".to_string(),
                shortcut: 'w',
                context: CommandContext::EmailViewer,
                action: CommandAction::ForwardEmail,
            },
            Command {
                id: "delete_viewer".to_string(),
                name: "Delete".to_string(),
                description: "Delete this email".to_string(),
                shortcut: 'd',
                context: CommandContext::EmailViewer,
                action: CommandAction::DeleteEmail,
            },
            Command {
                id: "archive_viewer".to_string(),
                name: "Archive".to_string(),
                description: "Archive this email".to_string(),
                shortcut: 'v',
                context: CommandContext::EmailViewer,
                action: CommandAction::ArchiveEmail,
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
            Command {
                id: "edit_event".to_string(),
                name: "Edit Event".to_string(),
                description: "Edit selected calendar event".to_string(),
                shortcut: 'e',
                context: CommandContext::Calendar,
                action: CommandAction::EditEvent,
            },
            Command {
                id: "delete_event".to_string(),
                name: "Delete Event".to_string(),
                description: "Delete selected calendar event".to_string(),
                shortcut: 'd',
                context: CommandContext::Calendar,
                action: CommandAction::DeleteEvent,
            },
            Command {
                id: "create_todo".to_string(),
                name: "Create Todo".to_string(),
                description: "Create a new todo item".to_string(),
                shortcut: 't',
                context: CommandContext::Calendar,
                action: CommandAction::CreateTodo,
            },
            Command {
                id: "view_day".to_string(),
                name: "Day View".to_string(),
                description: "Switch to day view".to_string(),
                shortcut: 'j',
                context: CommandContext::Calendar,
                action: CommandAction::ViewDay,
            },
            Command {
                id: "view_week".to_string(),
                name: "Week View".to_string(),
                description: "Switch to week view".to_string(),
                shortcut: 'w',
                context: CommandContext::Calendar,
                action: CommandAction::ViewWeek,
            },
            Command {
                id: "view_month".to_string(),
                name: "Month View".to_string(),
                description: "Switch to month view".to_string(),
                shortcut: 'm',
                context: CommandContext::Calendar,
                action: CommandAction::ViewMonth,
            },
            Command {
                id: "sync_calendar".to_string(),
                name: "Sync Calendar".to_string(),
                description: "Synchronize calendar data".to_string(),
                shortcut: 'y',
                context: CommandContext::Calendar,
                action: CommandAction::SyncCalendar,
            },
        ];
        
        // Contacts specific commands
        let contacts_commands = vec![
            Command {
                id: "create_contact".to_string(),
                name: "Create Contact".to_string(),
                description: "Create a new contact".to_string(),
                shortcut: 'n',
                context: CommandContext::Contacts,
                action: CommandAction::CreateContact,
            },
            Command {
                id: "edit_contact".to_string(),
                name: "Edit Contact".to_string(),
                description: "Edit selected contact".to_string(),
                shortcut: 'e',
                context: CommandContext::Contacts,
                action: CommandAction::EditContact,
            },
            Command {
                id: "delete_contact".to_string(),
                name: "Delete Contact".to_string(),
                description: "Delete selected contact".to_string(),
                shortcut: 'd',
                context: CommandContext::Contacts,
                action: CommandAction::DeleteContact,
            },
            Command {
                id: "view_contact".to_string(),
                name: "View Contact".to_string(),
                description: "View contact details".to_string(),
                shortcut: 'v',
                context: CommandContext::Contacts,
                action: CommandAction::ViewContact,
            },
            Command {
                id: "search_contacts".to_string(),
                name: "Search Contacts".to_string(),
                description: "Search through contacts".to_string(),
                shortcut: 'f',
                context: CommandContext::Contacts,
                action: CommandAction::SearchContacts,
            },
            Command {
                id: "import_contacts".to_string(),
                name: "Import Contacts".to_string(),
                description: "Import contacts from file".to_string(),
                shortcut: 'i',
                context: CommandContext::Contacts,
                action: CommandAction::ImportContacts,
            },
            Command {
                id: "export_contacts".to_string(),
                name: "Export Contacts".to_string(),
                description: "Export contacts to file".to_string(),
                shortcut: 'x',
                context: CommandContext::Contacts,
                action: CommandAction::ExportContacts,
            },
            Command {
                id: "add_favorite".to_string(),
                name: "Add to Favorites".to_string(),
                description: "Add contact to favorites".to_string(),
                shortcut: 'g',
                context: CommandContext::Contacts,
                action: CommandAction::AddToFavorites,
            },
        ];
        
        // Compose specific commands
        let compose_commands = vec![
            Command {
                id: "send_email".to_string(),
                name: "Send Email".to_string(),
                description: "Send the composed email".to_string(),
                shortcut: 's',
                context: CommandContext::Compose,
                action: CommandAction::SendEmail,
            },
            Command {
                id: "save_draft".to_string(),
                name: "Save Draft".to_string(),
                description: "Save email as draft".to_string(),
                shortcut: 'd',
                context: CommandContext::Compose,
                action: CommandAction::SaveDraft,
            },
            Command {
                id: "attach_file".to_string(),
                name: "Attach File".to_string(),
                description: "Attach a file to email".to_string(),
                shortcut: 'a',
                context: CommandContext::Compose,
                action: CommandAction::AttachFile,
            },
            Command {
                id: "toggle_format".to_string(),
                name: "Toggle Format".to_string(),
                description: "Toggle between HTML and plain text".to_string(),
                shortcut: 't',
                context: CommandContext::Compose,
                action: CommandAction::ToggleFormat,
            },
            Command {
                id: "insert_signature".to_string(),
                name: "Insert Signature".to_string(),
                description: "Insert email signature".to_string(),
                shortcut: 'g',
                context: CommandContext::Compose,
                action: CommandAction::InsertSignature,
            },
        ];
        
        self.commands.insert(CommandContext::Global, global_commands);
        self.commands.insert(CommandContext::EmailList, email_list_commands);
        self.commands.insert(CommandContext::EmailViewer, email_viewer_commands);
        self.commands.insert(CommandContext::Calendar, calendar_commands);
        self.commands.insert(CommandContext::Contacts, contacts_commands);
        self.commands.insert(CommandContext::Compose, compose_commands);
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
        
        // Add context-specific commands first
        if let Some(context_cmds) = self.commands.get(&self.current_context) {
            commands.extend(context_cmds.clone());
        }
        
        // Add global commands only if we're not already in Global context
        // This prevents duplicates when current_context is Global
        if self.current_context != CommandContext::Global {
            if let Some(global_cmds) = self.commands.get(&CommandContext::Global) {
                commands.extend(global_cmds.clone());
            }
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
        
        // Create the main block - context menu style
        let block = Block::default()
            .title(" Commands ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.colors.palette.accent))
            .style(Style::default().bg(theme.colors.palette.background));
            
        frame.render_widget(block, popup_area);
        
        // Layout for content - more compact context menu style
        let inner_area = popup_area.inner(&Margin::new(1, 1));
        
        // Skip header/footer for compact context menu - just show commands
        if inner_area.height > 6 {
            // If we have enough space, show minimal header and commands
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // Minimal header
                    Constraint::Min(3),    // Command list (most space)
                    Constraint::Length(1), // Minimal footer
                ])
                .split(inner_area);
            
            self.render_compact_header(frame, chunks[0], theme);
            self.render_command_list(frame, chunks[1], theme);
            self.render_compact_footer(frame, chunks[2], theme);
        } else {
            // Very compact - just show commands
            self.render_command_list(frame, inner_area, theme);
        }
    }
    
    /// Render compact header for context menu
    fn render_compact_header(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let context_name = match self.current_context {
            CommandContext::EmailList => "Email",
            CommandContext::EmailViewer => "View", 
            CommandContext::Calendar => "Calendar",
            CommandContext::Contacts => "Contacts",
            CommandContext::Compose => "Compose",
            CommandContext::Settings => "Settings",
            CommandContext::Global => "Global",
        };
        
        let text = format!("{} ({})", context_name, self.filtered_commands.len());
        
        let paragraph = Paragraph::new(text)
            .style(Style::default()
                .fg(theme.colors.palette.text_secondary)
                .add_modifier(Modifier::DIM))
            .alignment(Alignment::Center);
            
        frame.render_widget(paragraph, area);
    }

    
    /// Render the command list - compact context menu style
    fn render_command_list(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let items: Vec<ListItem> = self.filtered_commands
            .iter()
            .take(area.height.saturating_sub(1) as usize) // Limit to visible area
            .map(|command| {
                let shortcut_style = Style::default()
                    .fg(theme.colors.palette.accent)
                    .add_modifier(Modifier::BOLD);
                    
                let name_style = Style::default()
                    .fg(theme.colors.palette.text_primary);
                
                // More compact format - shorter descriptions for context menu
                let desc_preview = if command.description.len() > 20 {
                    format!("{}…", &command.description[..17])
                } else {
                    command.description.clone()
                };
                
                let desc_style = Style::default()
                    .fg(theme.colors.palette.text_secondary)
                    .add_modifier(Modifier::DIM);
                
                let line = Line::from(vec![
                    Span::styled(format!("{}", command.shortcut.to_uppercase()), shortcut_style),
                    Span::raw(" "),
                    Span::styled(format!("{}", command.name), name_style),
                    Span::raw(" "),
                    Span::styled(format!("{}", desc_preview), desc_style),
                ]);
                
                ListItem::new(line)
            })
            .collect();
        
        let list = List::new(items)
            .highlight_style(Style::default()
                .bg(theme.colors.palette.accent)
                .fg(theme.colors.palette.background)
                .add_modifier(Modifier::BOLD))
            .highlight_symbol("▶ ");
            
        frame.render_stateful_widget(list, area, &mut self.list_state);
    }
    
    /// Render compact footer for context menu
    fn render_compact_footer(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let text = "Key: select • ↑↓: nav • Esc: close";
        
        let paragraph = Paragraph::new(text)
            .style(Style::default()
                .fg(theme.colors.palette.text_secondary)
                .add_modifier(Modifier::DIM))
            .alignment(Alignment::Center);
            
        frame.render_widget(paragraph, area);
    }

    
    /// Calculate popup area for context menu style (smaller, positioned contextually)
    fn get_popup_area(&self, area: Rect) -> Rect {
        // Context menu style - much smaller and more compact
        let max_commands = self.filtered_commands.len().min(12); // Show max 12 commands
        let content_height = max_commands + 3; // Commands + header + footer + borders
        
        // Width based on longest command name, but with reasonable limits
        let max_command_width = self.filtered_commands
            .iter()
            .map(|cmd| cmd.name.len() + cmd.description.len() + 8) // 8 for formatting
            .max()
            .unwrap_or(30);
        
        let popup_width = ((max_command_width + 4).min(60).max(35) as u16).min(area.width); // 35-60 char width
        let popup_height = ((content_height + 2).min(16).max(8) as u16).min(area.height); // 8-16 rows height
        
        // Position slightly off-center, more like a context menu
        // Place it in upper-right area to feel more contextual
        let x = area.x + (area.width.saturating_mul(60) / 100).saturating_sub(popup_width);
        let y = area.y + (area.height / 6); // Upper portion of screen
        
        // Ensure it fits within screen bounds
        let x = x.min(area.width.saturating_sub(popup_width));
        let y = y.min(area.height.saturating_sub(popup_height));
        
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