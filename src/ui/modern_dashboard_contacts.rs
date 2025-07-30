//! Modern Dashboard Contacts and Startup Widgets

use super::modern_dashboard::*;
use chrono::{DateTime, Local};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Paragraph, Wrap, List, ListItem, LineGauge,
    },
    Frame,
};

use crate::theme::Theme;

impl ModernDashboard {
    /// Render modern contacts widget
    pub fn render_contacts_widget(&self, f: &mut Frame<'_>, area: Rect, theme: &Theme) {
        let block = Block::default()
            .title("👥 Contacts")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.colors.palette.border));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let contacts_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Header with view mode
                Constraint::Min(0),    // Contacts list
            ])
            .split(inner);

        // Header with view mode selector
        self.render_contacts_header(f, contacts_chunks[0], theme);
        
        // Contacts list based on view mode
        match self.contacts_widget.view_mode {
            ContactViewMode::Recent => self.render_recent_contacts(f, contacts_chunks[1], theme),
            ContactViewMode::Favorites => self.render_favorite_contacts(f, contacts_chunks[1], theme),
            ContactViewMode::All => self.render_all_contacts(f, contacts_chunks[1], theme),
        }
    }

    /// Render contacts header with view mode
    fn render_contacts_header(&self, f: &mut Frame<'_>, area: Rect, theme: &Theme) {
        let view_mode_text = match self.contacts_widget.view_mode {
            ContactViewMode::Recent => "📕 Recent",
            ContactViewMode::Favorites => "⭐ Favorites", 
            ContactViewMode::All => "📖 All Contacts",
        };

        let header_line = Line::from(vec![
            Span::styled(
                view_mode_text,
                Style::default()
                    .fg(theme.colors.palette.accent)
                    .add_modifier(Modifier::BOLD)
            ),
            Span::styled(
                format!(" ({})", self.contacts_widget.contact_count),
                Style::default().fg(theme.colors.palette.text_muted)
            )
        ]);

        let header_paragraph = Paragraph::new(header_line)
            .alignment(Alignment::Left);

        f.render_widget(header_paragraph, area);
    }

    /// Render recent contacts
    fn render_recent_contacts(&self, f: &mut Frame<'_>, area: Rect, theme: &Theme) {
        if self.contacts_widget.recent_contacts.is_empty() {
            self.render_empty_contacts(f, area, theme, "No recent contacts");
        } else {
            let contact_items: Vec<ListItem> = self.contacts_widget.recent_contacts
                .iter()
                .take((area.height as usize).saturating_sub(1))
                .map(|contact| self.create_contact_item(contact, theme))
                .collect();

            let contacts_list = List::new(contact_items)
                .style(Style::default().fg(theme.colors.palette.text_primary));

            f.render_widget(contacts_list, area);
        }
    }

    /// Render favorite contacts
    fn render_favorite_contacts(&self, f: &mut Frame<'_>, area: Rect, theme: &Theme) {
        if self.contacts_widget.favorite_contacts.is_empty() {
            self.render_empty_contacts(f, area, theme, "No favorite contacts");
        } else {
            let contact_items: Vec<ListItem> = self.contacts_widget.favorite_contacts
                .iter()
                .take((area.height as usize).saturating_sub(1))
                .map(|contact| self.create_contact_item(contact, theme))
                .collect();

            let contacts_list = List::new(contact_items)
                .style(Style::default().fg(theme.colors.palette.text_primary));

            f.render_widget(contacts_list, area);
        }
    }

    /// Render all contacts view
    fn render_all_contacts(&self, f: &mut Frame<'_>, area: Rect, theme: &Theme) {
        let all_contacts: Vec<&Contact> = self.contacts_widget.recent_contacts
            .iter()
            .chain(self.contacts_widget.favorite_contacts.iter())
            .collect();

        if all_contacts.is_empty() {
            self.render_empty_contacts(f, area, theme, "No contacts found");
        } else {
            let contact_items: Vec<ListItem> = all_contacts
                .iter()
                .take((area.height as usize).saturating_sub(1))
                .map(|contact| self.create_contact_item(contact, theme))
                .collect();

            let contacts_list = List::new(contact_items)
                .style(Style::default().fg(theme.colors.palette.text_primary));

            f.render_widget(contacts_list, area);
        }
    }

    /// Create contact list item with modern card layout
    fn create_contact_item(&self, contact: &Contact, theme: &Theme) -> ListItem {
        let status_icon = self.get_contact_status_icon(&contact.status);
        let status_color = self.get_contact_status_color(&contact.status, theme);
        
        let favorite_icon = if contact.is_favorite { "⭐" } else { "" };
        
        let mut lines = vec![
            Line::from(vec![
                Span::styled(
                    format!("{} {} {}", 
                        self.get_contact_avatar(contact), 
                        contact.name,
                        favorite_icon
                    ),
                    Style::default()
                        .fg(theme.colors.palette.text_primary)
                        .add_modifier(Modifier::BOLD)
                ),
                Span::styled(
                    format!(" {}", status_icon),
                    Style::default().fg(status_color)
                )
            ])
        ];

        // Add email if available
        if !contact.email.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  📧 {}", contact.email),
                    Style::default().fg(theme.colors.palette.text_muted)
                )
            ]));
        }

        // Add last contact time if available
        if let Some(last_contact) = &contact.last_contact {
            let time_ago = self.format_time_ago(last_contact);
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  🕒 {}", time_ago),
                    Style::default().fg(theme.colors.palette.text_muted)
                )
            ]));
        }

        ListItem::new(lines)
    }

    /// Render empty contacts message
    fn render_empty_contacts(&self, f: &mut Frame<'_>, area: Rect, theme: &Theme, message: &str) {
        let empty_text = vec![
            Line::from(vec![
                Span::styled(
                    message,
                    Style::default().fg(theme.colors.palette.text_muted)
                )
            ])
        ];

        let empty_paragraph = Paragraph::new(empty_text)
            .alignment(Alignment::Center);

        f.render_widget(empty_paragraph, area);
    }

    /// Get contact avatar (placeholder for now)
    fn get_contact_avatar(&self, contact: &Contact) -> &str {
        if self.contacts_widget.show_avatars {
            if contact.avatar.is_some() {
                "👤" // Would show actual avatar in real implementation
            } else {
                "👤"
            }
        } else {
            ""
        }
    }

    /// Get contact status icon
    fn get_contact_status_icon(&self, status: &ContactStatus) -> &str {
        match status {
            ContactStatus::Online => "🟢",
            ContactStatus::Away => "🟡",
            ContactStatus::Busy => "🔴",
            ContactStatus::Offline => "⚫",
            ContactStatus::Unknown => "⚪",
        }
    }

    /// Get contact status color
    fn get_contact_status_color(&self, status: &ContactStatus, theme: &Theme) -> Color {
        match status {
            ContactStatus::Online => Color::Green,
            ContactStatus::Away => Color::Yellow,
            ContactStatus::Busy => Color::Red,
            ContactStatus::Offline => theme.colors.palette.text_muted,
            ContactStatus::Unknown => theme.colors.palette.text_muted,
        }
    }

    /// Format time ago string
    fn format_time_ago(&self, time: &DateTime<Local>) -> String {
        let now = Local::now();
        let duration = now.signed_duration_since(*time);
        
        if duration.num_days() > 0 {
            format!("{} days ago", duration.num_days())
        } else if duration.num_hours() > 0 {
            format!("{} hours ago", duration.num_hours())
        } else if duration.num_minutes() > 0 {
            format!("{} minutes ago", duration.num_minutes())
        } else {
            "Just now".to_string()
        }
    }

    /// Render startup progress widget with real-time updates
    pub fn render_startup_widget(&self, f: &mut Frame<'_>, area: Rect, theme: &Theme) {
        let pulse_intensity = (self.animation_state.pulse_phase.sin() + 1.0) / 2.0;
        let border_color = if self.startup_widget.overall_progress < 100.0 {
            Color::Rgb(
                (match theme.colors.palette.accent { Color::Rgb(r, _, _) => r, _ => 88 } as f32 * (0.7 + pulse_intensity * 0.3)) as u8,
                (match theme.colors.palette.accent { Color::Rgb(_, g, _) => g, _ => 166 } as f32 * (0.7 + pulse_intensity * 0.3)) as u8,
                (match theme.colors.palette.accent { Color::Rgb(_, _, b) => b, _ => 255 } as f32 * (0.7 + pulse_intensity * 0.3)) as u8,
            )
        } else {
            Color::Green
        };

        let block = Block::default()
            .title("🚀 Startup Progress")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        let inner = block.inner(area);
        f.render_widget(block, area);

        if self.startup_widget.phases.is_empty() {
            // Show default startup state
            self.render_default_startup_state(f, inner, theme);
        } else {
            // Show detailed startup progress
            self.render_detailed_startup_progress(f, inner, theme);
        }
    }

    /// Render default startup state
    fn render_default_startup_state(&self, f: &mut Frame<'_>, area: Rect, theme: &Theme) {
        let startup_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Overall progress
                Constraint::Length(1), // Progress bar
                Constraint::Min(0),    // Status text
            ])
            .split(area);

        // Overall progress percentage
        let progress_text = Paragraph::new(format!(
            "Overall Progress: {:.1}%",
            self.startup_widget.overall_progress
        ))
        .style(Style::default().fg(theme.colors.palette.text_primary))
        .alignment(Alignment::Center);

        f.render_widget(progress_text, startup_chunks[0]);

        // Animated progress bar
        let progress_ratio = self.startup_widget.overall_progress / 100.0;
        let progress_color = if self.startup_widget.overall_progress < 100.0 {
            Color::Yellow
        } else {
            Color::Green
        };

        let progress_bar = LineGauge::default()
            .gauge_style(Style::default().fg(progress_color))
            .ratio(progress_ratio)
            .label(format!("{:.0}%", self.startup_widget.overall_progress));

        f.render_widget(progress_bar, startup_chunks[1]);

        // Status messages
        let status_lines = if self.startup_widget.overall_progress < 100.0 {
            vec![
                Line::from(vec![
                    Span::styled(
                        "🔄 Initializing systems...",
                        Style::default().fg(theme.colors.palette.accent)
                    )
                ]),
                Line::from(vec![
                    Span::styled(
                        "⚡ Flash Fast integration active",
                        Style::default().fg(theme.colors.palette.accent)
                    )
                ]),
                Line::from(vec![
                    Span::styled(
                        format!("📊 {} updates/sec", 
                            (1000.0 / self.update_interval.as_millis() as f64) as u32
                        ),
                        Style::default().fg(theme.colors.palette.text_muted)
                    )
                ]),
            ]
        } else {
            vec![
                Line::from(vec![
                    Span::styled(
                        "✅ Startup Complete!",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD)
                    )
                ]),
                Line::from(vec![
                    Span::styled(
                        "🚀 All systems operational",
                        Style::default().fg(theme.colors.palette.accent)
                    )
                ]),
                Line::from(vec![
                    Span::styled(
                        "⚡ Flash Fast mode enabled",
                        Style::default().fg(theme.colors.palette.accent)
                    )
                ]),
            ]
        };

        let status_paragraph = Paragraph::new(status_lines)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        f.render_widget(status_paragraph, startup_chunks[2]);
    }

    /// Render detailed startup progress with phases
    fn render_detailed_startup_progress(&self, f: &mut Frame<'_>, area: Rect, theme: &Theme) {
        let progress_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Overall progress
                Constraint::Min(0),    // Phase details
            ])
            .split(area);

        // Overall progress
        let overall_text = vec![
            Line::from(vec![
                Span::styled(
                    format!("Overall: {:.1}%", self.startup_widget.overall_progress),
                    Style::default()
                        .fg(theme.colors.palette.accent)
                        .add_modifier(Modifier::BOLD)
                )
            ])
        ];

        let overall_paragraph = Paragraph::new(overall_text)
            .alignment(Alignment::Center);

        f.render_widget(overall_paragraph, progress_chunks[0]);

        // Phase details
        let phase_items: Vec<ListItem> = self.startup_widget.phases
            .iter()
            .enumerate()
            .map(|(i, phase)| {
                let status_icon = match phase.status {
                    PhaseStatus::Pending => "⏳",
                    PhaseStatus::InProgress => "🔄",
                    PhaseStatus::Completed => "✅",
                    PhaseStatus::Error => "❌",
                    PhaseStatus::Skipped => "⏭️",
                };

                let status_color = match phase.status {
                    PhaseStatus::Pending => theme.colors.palette.text_muted,
                    PhaseStatus::InProgress => Color::Yellow,
                    PhaseStatus::Completed => Color::Green,
                    PhaseStatus::Error => Color::Red,
                    PhaseStatus::Skipped => theme.colors.palette.text_muted,
                };

                let is_current = i == self.startup_widget.current_phase;
                let name_style = if is_current {
                    Style::default()
                        .fg(theme.colors.palette.accent)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.colors.palette.text_primary)
                };

                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(
                            format!("{} {}", status_icon, phase.name),
                            name_style
                        )
                    ]),
                    Line::from(vec![
                        Span::styled(
                            format!("  {:.1}%", phase.progress),
                            Style::default().fg(status_color)
                        )
                    ])
                ])
            })
            .collect();

        let phases_list = List::new(phase_items)
            .style(Style::default().fg(theme.colors.palette.text_primary));

        f.render_widget(phases_list, progress_chunks[1]);
    }
}