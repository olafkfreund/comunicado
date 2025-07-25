use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState},
    Frame,
};
use crate::theme::Theme;

#[derive(Debug, Clone)]
pub struct MessageItem {
    pub subject: String,
    pub sender: String,
    pub date: String,
    pub is_read: bool,
    pub is_important: bool,
    pub has_attachments: bool,
}

impl MessageItem {
    pub fn new(subject: String, sender: String, date: String) -> Self {
        Self {
            subject,
            sender,
            date,
            is_read: true,
            is_important: false,
            has_attachments: false,
        }
    }

    pub fn unread(mut self) -> Self {
        self.is_read = false;
        self
    }

    pub fn important(mut self) -> Self {
        self.is_important = true;
        self
    }

    pub fn with_attachments(mut self) -> Self {
        self.has_attachments = true;
        self
    }
}

pub struct MessageList {
    messages: Vec<MessageItem>,
    state: ListState,
}

impl MessageList {
    pub fn new() -> Self {
        let mut list = Self {
            messages: Vec::new(),
            state: ListState::default(),
        };
        
        // Initialize with sample messages
        list.initialize_sample_messages();
        list.state.select(Some(0));
        
        list
    }

    fn initialize_sample_messages(&mut self) {
        self.messages = vec![
            MessageItem::new(
                "Welcome to Comunicado!".to_string(),
                "Comunicado Team".to_string(),
                "Today 10:30".to_string(),
            ).unread().important(),
            
            MessageItem::new(
                "Project Update: Q1 Planning".to_string(),
                "Alice Johnson".to_string(),
                "Today 09:15".to_string(),
            ).with_attachments(),
            
            MessageItem::new(
                "Re: Meeting Notes from Yesterday".to_string(),
                "Bob Smith".to_string(),
                "Yesterday 16:45".to_string(),
            ),
            
            MessageItem::new(
                "Monthly Newsletter - Tech Updates".to_string(),
                "TechNews Daily".to_string(),
                "Yesterday 14:20".to_string(),
            ).unread(),
            
            MessageItem::new(
                "Invitation: Team Lunch Tomorrow".to_string(),
                "Carol Davis".to_string(),
                "Mon 11:30".to_string(),
            ).important(),
            
            MessageItem::new(
                "Security Alert: Password Change Required".to_string(),
                "IT Security".to_string(),
                "Mon 09:00".to_string(),
            ).unread().important(),
            
            MessageItem::new(
                "Vacation Photos from Hawaii".to_string(),
                "family@example.com".to_string(),
                "Sun 18:22".to_string(),
            ).with_attachments(),
            
            MessageItem::new(
                "Re: Budget Proposal Review".to_string(),
                "David Wilson".to_string(),
                "Fri 15:30".to_string(),
            ),
            
            MessageItem::new(
                "Weekend Plans - Anyone up for hiking?".to_string(),
                "Adventure Club".to_string(),
                "Thu 20:15".to_string(),
            ),
            
            MessageItem::new(
                "Reminder: Dentist Appointment Tomorrow".to_string(),
                "Dr. Smith's Office".to_string(),
                "Wed 12:00".to_string(),
            ).unread(),
        ];
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, block: Block, _is_focused: bool, theme: &Theme) {
        let items: Vec<ListItem> = self.messages
            .iter()
            .enumerate()
            .map(|(i, message)| {
                let is_selected = self.state.selected() == Some(i);
                
                // Style based on message state
                let subject_style = if is_selected {
                    theme.styles.get_selected_style("message_list", &theme.colors)
                } else if !message.is_read {
                    Style::default()
                        .fg(theme.colors.message_list.subject_unread)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.colors.message_list.subject_read)
                };

                let sender_style = if is_selected {
                    theme.styles.get_selected_style("message_list", &theme.colors)
                } else {
                    Style::default().fg(theme.colors.message_list.sender)
                };

                let date_style = if is_selected {
                    theme.styles.get_selected_style("message_list", &theme.colors)
                } else {
                    Style::default().fg(theme.colors.message_list.date)
                };

                // Create indicators (professional, text-based)
                let mut indicators = String::new();
                if message.is_important {
                    indicators.push('!');
                }
                if message.has_attachments {
                    indicators.push('@');
                }
                if !message.is_read {
                    indicators.push('•');
                }
                if !indicators.is_empty() {
                    indicators.push(' ');
                }

                // Format the message line
                let subject_truncated = if message.subject.len() > 35 {
                    format!("{}...", &message.subject[..32])
                } else {
                    message.subject.clone()
                };

                let sender_truncated = if message.sender.len() > 20 {
                    format!("{}...", &message.sender[..17])
                } else {
                    message.sender.clone()
                };

                let line = Line::from(vec![
                    Span::raw(indicators),
                    Span::styled(subject_truncated, subject_style),
                    Span::raw("\n  "),
                    Span::styled(format!("From: {}", sender_truncated), sender_style),
                    Span::raw(" • "),
                    Span::styled(message.date.clone(), date_style),
                ]);

                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        frame.render_stateful_widget(list, area, &mut self.state.clone());
    }

    pub fn handle_up(&mut self) {
        let selected = match self.state.selected() {
            Some(i) => {
                if i > 0 {
                    Some(i - 1)
                } else {
                    Some(self.messages.len() - 1)
                }
            }
            None => Some(0),
        };
        self.state.select(selected);
    }

    pub fn handle_down(&mut self) {
        let selected = match self.state.selected() {
            Some(i) => {
                if i < self.messages.len() - 1 {
                    Some(i + 1)
                } else {
                    Some(0)
                }
            }
            None => Some(0),
        };
        self.state.select(selected);
    }

    pub fn handle_enter(&mut self) {
        if let Some(selected) = self.state.selected() {
            if let Some(message) = self.messages.get_mut(selected) {
                // Mark message as read when selected
                message.is_read = true;
                // In the future, this will also trigger loading the message content
            }
        }
    }

    pub fn selected_message(&self) -> Option<&MessageItem> {
        self.state.selected().and_then(|i| self.messages.get(i))
    }

    pub fn mark_selected_as_read(&mut self) {
        if let Some(selected) = self.state.selected() {
            if let Some(message) = self.messages.get_mut(selected) {
                message.is_read = true;
            }
        }
    }

    pub fn toggle_selected_important(&mut self) {
        if let Some(selected) = self.state.selected() {
            if let Some(message) = self.messages.get_mut(selected) {
                message.is_important = !message.is_important;
            }
        }
    }
}

impl Default for MessageList {
    fn default() -> Self {
        Self::new()
    }
}