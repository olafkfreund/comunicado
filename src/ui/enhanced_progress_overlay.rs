/// Enhanced progress overlay with cancellation support
/// 
/// This provides a modern progress overlay that shows background tasks and allows users to cancel them.
/// It enhances the existing sync progress overlay with better UX and cancellation capabilities.

use crate::email::sync_engine::{SyncPhase, SyncProgress};
use crate::performance::background_processor::{BackgroundProcessor, TaskResult, TaskStatus};
use crate::theme::Theme;
use chrono::Utc;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, Paragraph},
    Frame,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{Duration, Instant};
use uuid::Uuid;

/// Progress item that can be displayed and potentially cancelled
#[derive(Debug, Clone)]
pub struct ProgressItem {
    pub id: Uuid,
    pub title: String,
    pub status: ProgressStatus,
    pub progress: f64, // 0.0 to 1.0
    pub details: String,
    pub can_cancel: bool,
    pub started_at: Instant,
    pub estimated_completion: Option<Instant>,
    pub task_type: ProgressTaskType,
}

#[derive(Debug, Clone)]
pub enum ProgressStatus {
    Running,
    Completed,
    Failed(String),
    Cancelled,
    Paused,
}

#[derive(Debug, Clone)]
pub enum ProgressTaskType {
    EmailSync { account_id: String, folder_name: String },
    CalendarSync { calendar_id: String },
    Search { query: String },
    Indexing,
    Export,
    Import,
    Other(String),
}

/// Enhanced progress overlay with cancellation support
pub struct EnhancedProgressOverlay {
    /// Active progress items
    progress_items: HashMap<Uuid, ProgressItem>,
    /// Currently selected item
    selected_index: usize,
    /// Whether overlay is visible
    is_visible: bool,
    /// Background processor for cancellation
    background_processor: Option<Arc<BackgroundProcessor>>,
    /// Show confirmation dialog for cancellation
    show_cancel_dialog: bool,
    /// Last update time for animations
    last_update: Instant,
}

impl EnhancedProgressOverlay {
    pub fn new() -> Self {
        Self {
            progress_items: HashMap::new(),
            selected_index: 0,
            is_visible: false,
            background_processor: None,
            show_cancel_dialog: false,
            last_update: Instant::now(),
        }
    }

    /// Set the background processor for task cancellation
    pub fn set_background_processor(&mut self, processor: Arc<BackgroundProcessor>) {
        self.background_processor = Some(processor);
    }

    /// Add or update a progress item from sync progress
    pub fn update_sync_progress(&mut self, progress: SyncProgress) {
        // Find existing item by account and folder, or create new one
        let existing_id = self.progress_items.iter()
            .find(|(_, item)| {
                matches!(&item.task_type, ProgressTaskType::EmailSync { account_id, folder_name } 
                    if account_id == &progress.account_id && folder_name == &progress.folder_name)
            })
            .map(|(id, _)| *id);
        
        let task_id = existing_id.unwrap_or_else(|| Uuid::new_v4());
        
        let item = ProgressItem {
            id: task_id,
            title: format!("Syncing {} • {}", progress.account_id, progress.folder_name),
            status: match progress.phase {
                SyncPhase::Complete => ProgressStatus::Completed,
                SyncPhase::Error(ref err) => ProgressStatus::Failed(err.clone()),
                _ => ProgressStatus::Running,
            },
            progress: if progress.total_messages > 0 {
                progress.messages_processed as f64 / progress.total_messages as f64
            } else {
                0.0
            },
            details: format!("{}/{} messages • {}", 
                progress.messages_processed, 
                progress.total_messages,
                Self::format_phase(&progress.phase)
            ),
            can_cancel: matches!(progress.phase, SyncPhase::Initializing | SyncPhase::CheckingFolders | SyncPhase::FetchingHeaders | SyncPhase::FetchingBodies | SyncPhase::ProcessingChanges),
            started_at: Instant::now() - std::time::Duration::from_secs(
                (Utc::now().timestamp() - progress.started_at.timestamp()) as u64
            ),
            estimated_completion: progress.estimated_completion.map(|eta| {
                Instant::now() + std::time::Duration::from_secs(
                    (eta.timestamp() - Utc::now().timestamp()).max(0) as u64
                )
            }),
            task_type: ProgressTaskType::EmailSync {
                account_id: progress.account_id.clone(),
                folder_name: progress.folder_name.clone(),
            },
        };
        
        self.progress_items.insert(task_id, item);
        self.update_visibility();
    }

    /// Add a background task progress item
    pub fn add_task_progress(&mut self, task_id: Uuid, title: String, task_type: ProgressTaskType) {
        let item = ProgressItem {
            id: task_id,
            title,
            status: ProgressStatus::Running,
            progress: 0.0,
            details: "Starting...".to_string(),
            can_cancel: true,
            started_at: Instant::now(),
            estimated_completion: None,
            task_type,
        };

        self.progress_items.insert(task_id, item);
        self.update_visibility();
    }

    /// Update task progress
    pub fn update_task_progress(&mut self, task_id: Uuid, progress: f64, details: String) {
        if let Some(item) = self.progress_items.get_mut(&task_id) {
            item.progress = progress.clamp(0.0, 1.0);
            item.details = details;
            
            // Update estimated completion based on progress
            if progress > 0.0 && progress < 1.0 {
                let elapsed = item.started_at.elapsed();
                let total_estimated = elapsed.as_secs_f64() / progress;
                let remaining = Duration::from_secs_f64((total_estimated - elapsed.as_secs_f64()).max(0.0));
                item.estimated_completion = Some(Instant::now() + remaining);
            }
        }
    }

    /// Handle task completion
    pub fn handle_task_completion(&mut self, result: TaskResult) {
        if let Some(item) = self.progress_items.get_mut(&result.task_id) {
            item.status = match result.status {
                TaskStatus::Completed => ProgressStatus::Completed,
                TaskStatus::Failed(error) => ProgressStatus::Failed(error),
                TaskStatus::Cancelled => ProgressStatus::Cancelled,
                _ => item.status.clone(),
            };
            item.progress = 1.0;
            
            // Remove completed/failed tasks after delay
            if matches!(item.status, ProgressStatus::Completed | ProgressStatus::Failed(_) | ProgressStatus::Cancelled) {
                // TODO: Schedule for removal after 3 seconds
            }
        }
    }

    /// Remove old completed tasks
    pub fn cleanup_completed(&mut self, threshold: Duration) {
        let now = Instant::now();
        let ids_to_remove: Vec<Uuid> = self
            .progress_items
            .iter()
            .filter(|(_, item)| {
                matches!(item.status, ProgressStatus::Completed | ProgressStatus::Failed(_) | ProgressStatus::Cancelled) &&
                now.duration_since(item.started_at) > threshold
            })
            .map(|(id, _)| *id)
            .collect();

        for id in ids_to_remove {
            self.progress_items.remove(&id);
        }

        self.update_visibility();
    }

    /// Update visibility based on active items
    fn update_visibility(&mut self) {
        let has_active = self.progress_items.iter().any(|(_, item)| {
            matches!(item.status, ProgressStatus::Running | ProgressStatus::Paused)
        });
        
        if !has_active && self.progress_items.len() <= 3 {
            // Keep overlay visible briefly after completion
            if self.last_update.elapsed() > Duration::from_secs(2) {
                self.is_visible = false;
            }
        } else {
            self.is_visible = true;
        }
    }

    /// Show the overlay
    pub fn show(&mut self) {
        self.is_visible = true;
    }

    /// Hide the overlay
    pub fn hide(&mut self) {
        self.is_visible = false;
        self.show_cancel_dialog = false;
    }

    /// Toggle visibility
    pub fn toggle_visibility(&mut self) {
        self.is_visible = !self.is_visible;
        if !self.is_visible {
            self.show_cancel_dialog = false;
        }
    }

    /// Check if overlay is visible
    pub fn is_visible(&self) -> bool {
        self.is_visible
    }

    /// Check if cancel dialog is showing
    pub fn is_cancel_dialog_showing(&self) -> bool {
        self.show_cancel_dialog
    }

    /// Check if currently selected task can be cancelled
    pub fn has_cancellable_task(&self) -> bool {
        if let Some(selected_item) = self.get_selected_item() {
            selected_item.can_cancel && matches!(selected_item.status, ProgressStatus::Running)
        } else {
            false
        }
    }

    /// Navigate selection
    pub fn select_next(&mut self) {
        if !self.progress_items.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.progress_items.len();
        }
    }

    pub fn select_previous(&mut self) {
        if !self.progress_items.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.progress_items.len() - 1
            } else {
                self.selected_index - 1
            };
        }
    }

    /// Show cancel confirmation dialog
    pub fn show_cancel_dialog(&mut self) {
        if let Some(selected_item) = self.get_selected_item() {
            if selected_item.can_cancel && matches!(selected_item.status, ProgressStatus::Running) {
                self.show_cancel_dialog = true;
            }
        }
    }

    /// Hide cancel confirmation dialog
    pub fn hide_cancel_dialog(&mut self) {
        self.show_cancel_dialog = false;
    }

    /// Cancel the selected task
    pub async fn cancel_selected_task(&mut self) {
        if let Some(selected_item) = self.get_selected_item() {
            if selected_item.can_cancel && matches!(selected_item.status, ProgressStatus::Running) {
                if let Some(ref processor) = self.background_processor {
                    let task_id = selected_item.id;
                    if processor.cancel_task(task_id).await {
                        // Update item status immediately
                        if let Some(item) = self.progress_items.get_mut(&task_id) {
                            item.status = ProgressStatus::Cancelled;
                            item.details = "Cancelling...".to_string();
                        }
                    }
                }
            }
        }
        self.hide_cancel_dialog();
    }

    /// Get currently selected item
    fn get_selected_item(&self) -> Option<&ProgressItem> {
        let items: Vec<_> = self.progress_items.values().collect();
        items.get(self.selected_index).copied()
    }

    /// Format sync phase for display
    fn format_phase(phase: &SyncPhase) -> String {
        match phase {
            SyncPhase::Initializing => "Initializing".to_string(),
            SyncPhase::CheckingFolders => "Checking folders".to_string(),
            SyncPhase::FetchingHeaders => "Fetching headers".to_string(),
            SyncPhase::FetchingBodies => "Fetching messages".to_string(),
            SyncPhase::ProcessingChanges => "Processing changes".to_string(),
            SyncPhase::Complete => "Complete".to_string(),
            SyncPhase::Error(err) => format!("Error: {}", err),
        }
    }

    /// Render the enhanced progress overlay
    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.is_visible() {
            return;
        }

        self.last_update = Instant::now();

        let overlay_area = Self::centered_rect(50, 40, area); // Much smaller: 50% width, 40% height

        // Clear the area
        frame.render_widget(Clear, overlay_area);

        // Main block
        let title = if self.progress_items.len() == 1 {
            "Background Task"
        } else {
            "Background Tasks"
        };

        let block = Block::default()
            .title(format!("{} ({})", title, self.progress_items.len()))
            .borders(Borders::ALL)
            .border_style(theme.get_component_style("border", true));

        let inner_area = block.inner(overlay_area);
        frame.render_widget(block, overlay_area);

        if self.progress_items.is_empty() {
            let no_tasks = Paragraph::new("No active background tasks")
                .alignment(Alignment::Center)
                .style(Style::default().fg(theme.colors.palette.text_secondary));
            frame.render_widget(no_tasks, inner_area);
            return;
        }

        // Split area for list and details
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(inner_area);

        // Render task list
        self.render_task_list(frame, chunks[0], theme);

        // Render selected task details
        self.render_task_details(frame, chunks[1], theme);

        // Render cancel confirmation dialog if shown
        if self.show_cancel_dialog {
            self.render_cancel_dialog(frame, overlay_area, theme);
        }
    }

    fn render_task_list(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let items: Vec<_> = self.progress_items.values().collect();

        let list_items: Vec<ListItem> = items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let status_symbol = match &item.status {
                    ProgressStatus::Running => "⏳",
                    ProgressStatus::Completed => "✅",
                    ProgressStatus::Failed(_) => "❌",
                    ProgressStatus::Cancelled => "⏹️",
                    ProgressStatus::Paused => "⏸️",
                };

                let progress_percent = (item.progress * 100.0) as u16;
                let cancel_indicator = if item.can_cancel && matches!(item.status, ProgressStatus::Running) {
                    " (ESC to cancel)"
                } else {
                    ""
                };

                let style = if index == self.selected_index {
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
                        &item.title,
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!(" ({}%){}", progress_percent, cancel_indicator),
                        Style::default().fg(theme.colors.palette.text_secondary),
                    ),
                ]);

                ListItem::new(line).style(style)
            })
            .collect();

        let list = List::new(list_items).block(
            Block::default()
                .title("Tasks")
                .borders(Borders::ALL)
                .border_style(theme.get_component_style("border", false)),
        );

        frame.render_widget(list, area);
    }

    fn render_task_details(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let items: Vec<_> = self.progress_items.values().collect();

        if let Some(item) = items.get(self.selected_index) {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(6), // Progress gauge
                    Constraint::Length(10), // Details
                    Constraint::Min(0),    // Controls
                ])
                .split(area);

            // Render progress gauge
            self.render_progress_gauge(frame, chunks[0], item, theme);

            // Render task details
            self.render_task_info(frame, chunks[1], item, theme);

            // Render controls
            self.render_controls(frame, chunks[2], item, theme);
        }
    }

    fn render_progress_gauge(&self, frame: &mut Frame, area: Rect, item: &ProgressItem, theme: &Theme) {
        let progress_percent = (item.progress * 100.0) as u16;

        let gauge_color = match &item.status {
            ProgressStatus::Completed => Color::Green,
            ProgressStatus::Failed(_) => Color::Red,
            ProgressStatus::Cancelled => Color::Yellow,
            ProgressStatus::Paused => Color::Gray,
            ProgressStatus::Running => theme.colors.palette.accent,
        };

        let label = format!("{:.1}%", item.progress * 100.0);

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

    fn render_task_info(&self, frame: &mut Frame, area: Rect, item: &ProgressItem, theme: &Theme) {
        let elapsed = item.started_at.elapsed();
        let elapsed_str = Self::format_duration(elapsed);

        let eta_str = if let Some(eta) = item.estimated_completion {
            let remaining = eta.saturating_duration_since(Instant::now());
            if remaining > Duration::from_secs(0) {
                format!("ETA: {}", Self::format_duration(remaining))
            } else {
                "Completing...".to_string()
            }
        } else {
            "Calculating...".to_string()
        };

        let status_str = match &item.status {
            ProgressStatus::Running => "Running",
            ProgressStatus::Completed => "Completed",
            ProgressStatus::Failed(err) => &format!("Failed: {}", err),
            ProgressStatus::Cancelled => "Cancelled",
            ProgressStatus::Paused => "Paused",
        };

        let info_text = vec![
            Line::from(vec![
                Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(status_str),
            ]),
            Line::from(vec![
                Span::styled("Details: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&item.details),
            ]),
            Line::from(vec![
                Span::styled("Elapsed: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(elapsed_str),
            ]),
            Line::from(vec![
                Span::styled("ETA: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(eta_str),
            ]),
        ];

        let info_paragraph = Paragraph::new(info_text)
            .block(
                Block::default()
                    .title("Details")
                    .borders(Borders::ALL)
                    .border_style(theme.get_component_style("border", false)),
            )
            .wrap(ratatui::widgets::Wrap { trim: true });

        frame.render_widget(info_paragraph, area);
    }

    fn render_controls(&self, frame: &mut Frame, area: Rect, item: &ProgressItem, theme: &Theme) {
        let can_cancel = item.can_cancel && matches!(item.status, ProgressStatus::Running);
        
        let controls_text = if can_cancel {
            "ESC: Cancel task • Q: Close overlay • ↑↓: Navigate"
        } else {
            "Q: Close overlay • ↑↓: Navigate"
        };

        let controls = Paragraph::new(controls_text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.colors.palette.text_secondary))
            .block(
                Block::default()
                    .title("Controls")
                    .borders(Borders::ALL)
                    .border_style(theme.get_component_style("border", false)),
            );

        frame.render_widget(controls, area);
    }

    fn render_cancel_dialog(&self, frame: &mut Frame, parent_area: Rect, theme: &Theme) {
        let dialog_area = Self::centered_rect(50, 30, parent_area);

        // Clear the area
        frame.render_widget(Clear, dialog_area);

        let block = Block::default()
            .title("Cancel Task")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        let inner_area = block.inner(dialog_area);
        frame.render_widget(block, dialog_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(inner_area);

        let message = Paragraph::new("Are you sure you want to cancel this task?\n\nThis action cannot be undone.")
            .alignment(Alignment::Center)
            .wrap(ratatui::widgets::Wrap { trim: true });

        let controls = Paragraph::new("ENTER: Yes, cancel • ESC: No, keep running")
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.colors.palette.text_secondary));

        frame.render_widget(message, chunks[0]);
        frame.render_widget(controls, chunks[1]);
    }

    /// Helper function to create centered rectangle
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
    fn format_duration(duration: Duration) -> String {
        let secs = duration.as_secs();
        if secs < 60 {
            format!("{}s", secs)
        } else if secs < 3600 {
            format!("{}m {}s", secs / 60, secs % 60)
        } else {
            format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
        }
    }
}