use crate::email::sync_engine::{SyncPhase, SyncProgress};
use crate::theme::Theme;
use chrono::{Duration as ChronoDuration, Utc};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, Paragraph},
    Frame,
};
use std::collections::HashMap;

/// Sync progress overlay component
pub struct SyncProgressOverlay {
    active_syncs: HashMap<String, SyncProgress>,
    is_visible: bool,
    selected_sync: usize,
}

impl SyncProgressOverlay {
    pub fn new() -> Self {
        Self {
            active_syncs: HashMap::new(),
            is_visible: false,
            selected_sync: 0,
        }
    }

    /// Update sync progress for an account/folder
    pub fn update_progress(&mut self, progress: SyncProgress) {
        let key = format!("{}:{}", progress.account_id, progress.folder_name);

        // If sync is complete or errored, remove it after a delay
        match progress.phase {
            SyncPhase::Complete | SyncPhase::Error(_) => {
                self.active_syncs.insert(key, progress);
                // TODO: Remove after 3 seconds (requires timer)
            }
            _ => {
                self.active_syncs.insert(key, progress);
            }
        }

        // Show overlay if there are active syncs
        self.is_visible = !self.active_syncs.is_empty();
    }

    /// Remove completed syncs older than threshold
    pub fn cleanup_completed(&mut self, threshold: ChronoDuration) {
        let now = Utc::now();
        let keys_to_remove: Vec<String> = self
            .active_syncs
            .iter()
            .filter(|(_, progress)| match progress.phase {
                SyncPhase::Complete | SyncPhase::Error(_) => {
                    now.signed_duration_since(progress.started_at) > threshold
                }
                _ => false,
            })
            .map(|(key, _)| key.clone())
            .collect();

        for key in keys_to_remove {
            self.active_syncs.remove(&key);
        }

        // Hide overlay if no active syncs
        if self.active_syncs.is_empty() {
            self.is_visible = false;
        }
    }

    /// Toggle visibility
    pub fn toggle_visibility(&mut self) {
        self.is_visible = !self.is_visible;
    }

    /// Check if overlay should be visible
    pub fn is_visible(&self) -> bool {
        self.is_visible && !self.active_syncs.is_empty()
    }

    /// Navigate selection
    pub fn next_sync(&mut self) {
        if !self.active_syncs.is_empty() {
            self.selected_sync = (self.selected_sync + 1) % self.active_syncs.len();
        }
    }

    pub fn previous_sync(&mut self) {
        if !self.active_syncs.is_empty() {
            self.selected_sync = if self.selected_sync == 0 {
                self.active_syncs.len() - 1
            } else {
                self.selected_sync - 1
            };
        }
    }

    /// Render the sync progress overlay
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.is_visible() {
            return;
        }

        let overlay_area = centered_rect(80, 70, area);

        // Clear the area
        frame.render_widget(Clear, overlay_area);

        // Main block
        let block = Block::default()
            .title("Sync Progress")
            .borders(Borders::ALL)
            .border_style(theme.get_component_style("border", true));

        let inner_area = block.inner(overlay_area);
        frame.render_widget(block, overlay_area);

        if self.active_syncs.is_empty() {
            // No active syncs
            let no_syncs = Paragraph::new("No active synchronizations")
                .alignment(Alignment::Center)
                .style(Style::default().fg(theme.colors.palette.text_secondary));
            frame.render_widget(no_syncs, inner_area);
            return;
        }

        // Split area for list and details
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(inner_area);

        // Render sync list
        self.render_sync_list(frame, chunks[0], theme);

        // Render selected sync details
        self.render_sync_details(frame, chunks[1], theme);
    }

    fn render_sync_list(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let syncs: Vec<_> = self.active_syncs.values().collect();

        let items: Vec<ListItem> = syncs
            .iter()
            .enumerate()
            .map(|(index, progress)| {
                let status_symbol = match progress.phase {
                    SyncPhase::Initializing => "⏳",
                    SyncPhase::CheckingFolders => "📁",
                    SyncPhase::FetchingHeaders => "📧",
                    SyncPhase::FetchingBodies => "📄",
                    SyncPhase::ProcessingChanges => "⚙️",
                    SyncPhase::Complete => "✅",
                    SyncPhase::Error(_) => "❌",
                };

                let progress_percent = if progress.total_messages > 0 {
                    (progress.messages_processed * 100) / progress.total_messages
                } else {
                    0
                };

                let style = if index == self.selected_sync {
                    Style::default()
                        .bg(theme.colors.palette.accent)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                let line = Line::from(vec![
                    Span::styled(format!("{} ", status_symbol), Style::default()),
                    Span::styled(
                        format!("{} ", progress.account_id),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{} ({}%)", progress.folder_name, progress_percent),
                        Style::default(),
                    ),
                ]);

                ListItem::new(line).style(style)
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .title("Active Syncs")
                .borders(Borders::ALL)
                .border_style(theme.get_component_style("border", false)),
        );

        frame.render_widget(list, area);
    }

    fn render_sync_details(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let syncs: Vec<_> = self.active_syncs.values().collect();

        if let Some(progress) = syncs.get(self.selected_sync) {
            // Split details area
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(6), // Progress gauge
                    Constraint::Length(8), // Stats
                    Constraint::Min(0),    // Phase details
                ])
                .split(area);

            // Render progress gauge
            self.render_progress_gauge(frame, chunks[0], progress, theme);

            // Render sync statistics
            self.render_sync_stats(frame, chunks[1], progress, theme);

            // Render phase details
            self.render_phase_details(frame, chunks[2], progress, theme);
        }
    }

    fn render_progress_gauge(
        &self,
        frame: &mut Frame,
        area: Rect,
        progress: &SyncProgress,
        theme: &Theme,
    ) {
        let progress_percent = if progress.total_messages > 0 {
            ((progress.messages_processed * 100) / progress.total_messages) as u16
        } else {
            0
        };

        let gauge_color = match progress.phase {
            SyncPhase::Complete => Color::Green,
            SyncPhase::Error(_) => Color::Red,
            _ => theme.colors.palette.accent,
        };

        let label = format!(
            "{}/{} messages ({:.1}%)",
            progress.messages_processed, progress.total_messages, progress_percent as f32
        );

        let gauge = Gauge::default()
            .block(
                Block::default()
                    .title("Progress")
                    .borders(Borders::ALL)
                    .border_style(theme.get_component_style("border", false)),
            )
            .gauge_style(Style::default().fg(gauge_color))
            .percent(progress_percent)
            .label(label);

        frame.render_widget(gauge, area);
    }

    fn render_sync_stats(
        &self,
        frame: &mut Frame,
        area: Rect,
        progress: &SyncProgress,
        theme: &Theme,
    ) {
        let elapsed = Utc::now().signed_duration_since(progress.started_at);
        let elapsed_str = format_duration(elapsed);

        let eta_str = if let Some(eta) = progress.estimated_completion {
            let remaining = eta.signed_duration_since(Utc::now());
            if remaining > ChronoDuration::zero() {
                format!("ETA: {}", format_duration(remaining))
            } else {
                "Completing...".to_string()
            }
        } else {
            "Calculating...".to_string()
        };

        let bytes_str = format_bytes(progress.bytes_downloaded);

        let stats_text = vec![
            Line::from(vec![
                Span::styled("Account: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&progress.account_id),
            ]),
            Line::from(vec![
                Span::styled("Folder: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&progress.folder_name),
            ]),
            Line::from(vec![
                Span::styled("Elapsed: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(elapsed_str),
            ]),
            Line::from(vec![
                Span::styled("ETA: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(eta_str),
            ]),
            Line::from(vec![
                Span::styled(
                    "Downloaded: ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(bytes_str),
            ]),
        ];

        let stats = Paragraph::new(stats_text).block(
            Block::default()
                .title("Statistics")
                .borders(Borders::ALL)
                .border_style(theme.get_component_style("border", false)),
        );

        frame.render_widget(stats, area);
    }

    fn render_phase_details(
        &self,
        frame: &mut Frame,
        area: Rect,
        progress: &SyncProgress,
        theme: &Theme,
    ) {
        let phase_text = match &progress.phase {
            SyncPhase::Initializing => vec![
                Line::from("🔄 Initializing synchronization..."),
                Line::from("• Connecting to email server"),
                Line::from("• Authenticating account"),
                Line::from("• Preparing sync process"),
            ],
            SyncPhase::CheckingFolders => vec![
                Line::from("📁 Checking folder structure..."),
                Line::from("• Scanning remote folders"),
                Line::from("• Comparing with local structure"),
                Line::from("• Identifying changes"),
            ],
            SyncPhase::FetchingHeaders => vec![
                Line::from("📧 Fetching message headers..."),
                Line::from("• Downloading message metadata"),
                Line::from("• Processing message flags"),
                Line::from("• Building message index"),
            ],
            SyncPhase::FetchingBodies => vec![
                Line::from("📄 Downloading message bodies..."),
                Line::from("• Fetching message content"),
                Line::from("• Processing attachments"),
                Line::from("• Storing messages locally"),
            ],
            SyncPhase::ProcessingChanges => vec![
                Line::from("⚙️ Processing changes..."),
                Line::from("• Applying local modifications"),
                Line::from("• Resolving conflicts"),
                Line::from("• Updating indices"),
            ],
            SyncPhase::Complete => vec![
                Line::from(Span::styled(
                    "✅ Synchronization completed successfully!",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(format!(
                    "• {} messages processed",
                    progress.messages_processed
                )),
                Line::from(format!(
                    "• {} bytes downloaded",
                    format_bytes(progress.bytes_downloaded)
                )),
                Line::from("• All changes saved"),
            ],
            SyncPhase::Error(error) => vec![
                Line::from(Span::styled(
                    "❌ Synchronization failed",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Error details:",
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Line::from(error.clone()),
            ],
        };

        let phase_details = Paragraph::new(phase_text).block(
            Block::default()
                .title("Current Phase")
                .borders(Borders::ALL)
                .border_style(theme.get_component_style("border", false)),
        );

        frame.render_widget(phase_details, area);
    }
}

impl Default for SyncProgressOverlay {
    fn default() -> Self {
        Self::new()
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

/// Format duration for display
fn format_duration(duration: ChronoDuration) -> String {
    let total_seconds = duration.num_seconds();
    if total_seconds < 60 {
        format!("{}s", total_seconds)
    } else if total_seconds < 3600 {
        format!("{}m {}s", total_seconds / 60, total_seconds % 60)
    } else {
        format!("{}h {}m", total_seconds / 3600, (total_seconds % 3600) / 60)
    }
}

/// Format bytes for display
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(ChronoDuration::seconds(30)), "30s");
        assert_eq!(format_duration(ChronoDuration::seconds(90)), "1m 30s");
        assert_eq!(format_duration(ChronoDuration::seconds(3661)), "1h 1m");
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1048576), "1.0 MB");
        assert_eq!(format_bytes(1073741824), "1.0 GB");
    }

    #[test]
    fn test_sync_progress_overlay() {
        let mut overlay = SyncProgressOverlay::new();
        assert!(!overlay.is_visible());

        let progress = SyncProgress {
            account_id: "test@example.com".to_string(),
            folder_name: "INBOX".to_string(),
            phase: SyncPhase::FetchingHeaders,
            messages_processed: 50,
            total_messages: 100,
            bytes_downloaded: 1024,
            started_at: Utc::now(),
            estimated_completion: None,
        };

        overlay.update_progress(progress);
        assert!(overlay.is_visible());
        assert_eq!(overlay.active_syncs.len(), 1);
    }
}
