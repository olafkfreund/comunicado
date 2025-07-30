use crate::startup::manager::StartupProgressManager;
use crate::theme::Theme;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Wrap},
    Frame,
};
use std::time::Duration;

/// Full-screen startup progress component
pub struct StartupProgressScreen {
    show_details: bool,
}

impl StartupProgressScreen {
    pub fn new() -> Self {
        Self {
            show_details: true,
        }
    }

    /// Toggle detailed view
    pub fn toggle_details(&mut self) {
        self.show_details = !self.show_details;
    }

    /// Render the startup progress screen
    pub fn render(&self, frame: &mut Frame, area: Rect, manager: &StartupProgressManager, theme: &Theme) {
        if !manager.is_visible() {
            return;
        }

        // Clear the entire screen
        frame.render_widget(Clear, area);

        // Main layout with margins
        let main_area = area.inner(&Margin {
            vertical: 2,
            horizontal: 4,
        });

        // Create the main container
        let main_block = Block::default()
            .title("Comunicado - Initializing...")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(theme.get_component_style("border", true));

        let content_area = main_block.inner(main_area);
        frame.render_widget(main_block, main_area);

        // Split the content area
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6),  // Overall progress
                Constraint::Length(8),  // Current phase details
                Constraint::Min(10),    // Phase list
                Constraint::Length(4),  // Error summary (if any)
            ])
            .split(content_area);

        // Render overall progress
        self.render_overall_progress(frame, chunks[0], manager, theme);

        // Render current phase details
        self.render_current_phase_details(frame, chunks[1], manager, theme);

        // Render phase list
        self.render_phase_list(frame, chunks[2], manager, theme);

        // Render error summary if there are errors
        if !manager.error_states().is_empty() {
            self.render_error_summary(frame, chunks[3], manager, theme);
        }
    }

    fn render_overall_progress(&self, frame: &mut Frame, area: Rect, manager: &StartupProgressManager, theme: &Theme) {
        let progress_percentage = manager.overall_progress_percentage();
        let total_duration = manager.total_duration();
        
        let progress_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Progress bar
                Constraint::Length(2), // Stats
            ])
            .split(area.inner(&Margin { vertical: 1, horizontal: 2 }));

        // Progress bar
        let progress_bar = Gauge::default()
            .block(Block::default().title("Overall Progress").borders(Borders::ALL))
            .gauge_style(
                Style::default()
                    .fg(if progress_percentage < 100.0 {
                        theme.colors.palette.accent
                    } else {
                        theme.colors.palette.success
                    })
                    .add_modifier(Modifier::BOLD)
            )
            .percent(progress_percentage as u16)
            .label(format!("{:.1}%", progress_percentage));

        frame.render_widget(progress_bar, progress_chunks[0]);

        // Stats line
        let mut stats_line = vec![
            Span::styled(
                format!("Duration: {:.1}s", total_duration.as_secs_f64()),
                Style::default().fg(theme.colors.palette.text_secondary),
            ),
        ];

        if let Some(eta) = manager.estimated_time_remaining() {
            stats_line.push(Span::raw("  |  "));
            stats_line.push(Span::styled(
                format!("ETA: {:.1}s", eta.as_secs_f64()),
                Style::default().fg(theme.colors.palette.text_secondary),
            ));
        }

        let stats = Paragraph::new(Line::from(stats_line))
            .alignment(Alignment::Center);

        frame.render_widget(stats, progress_chunks[1]);
    }

    fn render_current_phase_details(&self, frame: &mut Frame, area: Rect, manager: &StartupProgressManager, theme: &Theme) {
        let block = Block::default()
            .title("Current Phase")
            .borders(Borders::ALL)
            .border_style(theme.get_component_style("border", false));

        let inner_area = block.inner(area);
        frame.render_widget(block, area);

        if let Some(current_phase) = manager.current_phase() {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // Phase name
                    Constraint::Length(1), // Description
                    Constraint::Length(2), // Progress bar for current phase
                    Constraint::Min(1),    // Phase logs
                ])
                .split(inner_area.inner(&Margin { vertical: 1, horizontal: 1 }));

            // Phase name with icon
            let phase_line = Line::from(vec![
                Span::styled(
                    current_phase.status_icon(),
                    Style::default().fg(self.get_status_color(current_phase.status(), theme)),
                ),
                Span::raw(" "),
                Span::styled(
                    current_phase.name(),
                    Style::default()
                        .fg(theme.colors.palette.text_primary)
                        .add_modifier(Modifier::BOLD),
                ),
            ]);
            frame.render_widget(Paragraph::new(phase_line), chunks[0]);

            // Description
            let description = Paragraph::new(current_phase.description())
                .style(Style::default().fg(theme.colors.palette.text_secondary));
            frame.render_widget(description, chunks[1]);

            // Current phase progress bar
            let phase_progress = manager.current_phase_progress();
            if current_phase.status().is_in_progress() && phase_progress > 0.0 {
                let phase_gauge = Gauge::default()
                    .gauge_style(Style::default().fg(theme.colors.palette.accent))
                    .percent(phase_progress as u16)
                    .label(format!("{:.1}%", phase_progress));
                frame.render_widget(phase_gauge, chunks[2]);
            } else {
                // Show status text instead
                let status_text = self.format_phase_status(current_phase.status(), &current_phase.timeout());
                let status = Paragraph::new(status_text)
                    .style(Style::default().fg(theme.colors.palette.text_secondary));
                frame.render_widget(status, chunks[2]);
            }

            // Show phase logs
            let logs = manager.get_phase_logs(current_phase.name());
            if !logs.is_empty() {
                let log_lines: Vec<Line> = logs.iter()
                    .rev() // Show most recent first
                    .take(3) // Show last 3 logs
                    .map(|log| Line::from(Span::styled(log, Style::default().fg(theme.colors.palette.text_muted))))
                    .collect();
                
                let logs_paragraph = Paragraph::new(log_lines)
                    .wrap(Wrap { trim: true });
                frame.render_widget(logs_paragraph, chunks[3]);
            }
        } else {
            let no_phase = Paragraph::new("Initialization complete")
                .alignment(Alignment::Center)
                .style(Style::default().fg(theme.colors.palette.success));
            frame.render_widget(no_phase, inner_area);
        }
    }

    fn render_phase_list(&self, frame: &mut Frame, area: Rect, manager: &StartupProgressManager, theme: &Theme) {
        let block = Block::default()
            .title("Initialization Phases")
            .borders(Borders::ALL)
            .border_style(theme.get_component_style("border", false));

        let inner_area = block.inner(area);
        frame.render_widget(block, area);

        let items: Vec<ListItem> = manager
            .phases()
            .iter()
            .map(|phase| {
                let mut spans = vec![
                    Span::styled(
                        phase.status_icon(),
                        Style::default().fg(self.get_status_color(phase.status(), theme)),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        phase.name(),
                        Style::default().fg(theme.colors.palette.text_primary),
                    ),
                ];

                // Add duration if available
                if let Some(duration) = phase.status().duration() {
                    spans.push(Span::raw(format!(" ({:.1}s)", duration.as_secs_f64())));
                }

                // Add error message if failed
                if let Some(error) = phase.status().error_message() {
                    spans.push(Span::raw(" - "));
                    spans.push(Span::styled(
                        error,
                        Style::default().fg(theme.colors.palette.error),
                    ));
                }

                ListItem::new(Line::from(spans))
            })
            .collect();

        let list = List::new(items)
            .style(Style::default().fg(theme.colors.palette.text_secondary));

        frame.render_widget(list, inner_area);
    }

    fn render_error_summary(&self, frame: &mut Frame, area: Rect, manager: &StartupProgressManager, theme: &Theme) {
        let error_count = manager.error_states().len();
        let critical_errors = manager.error_states().values().filter(|e| e.is_critical).count();

        let title = if critical_errors > 0 {
            format!("Critical Errors ({})", critical_errors)
        } else {
            format!("Non-Critical Issues ({})", error_count)
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(
                Style::default().fg(if critical_errors > 0 {
                    theme.colors.palette.error
                } else {
                    theme.colors.palette.warning
                })
            );

        let inner_area = block.inner(area);
        frame.render_widget(block, area);

        let error_text = if critical_errors > 0 {
            "Critical errors detected. Some functionality may be unavailable."
        } else {
            "Some services failed to initialize but core functionality remains available."
        };

        let paragraph = Paragraph::new(error_text)
            .style(Style::default().fg(
                if critical_errors > 0 {
                    theme.colors.palette.error
                } else {
                    theme.colors.palette.warning
                }
            ))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, inner_area);
    }

    fn get_status_color(&self, status: &crate::startup::progress::PhaseStatus, theme: &Theme) -> Color {
        use crate::startup::progress::PhaseStatus;
        match status {
            PhaseStatus::Pending => theme.colors.palette.text_secondary,
            PhaseStatus::InProgress { .. } => theme.colors.palette.accent,
            PhaseStatus::Completed { .. } => theme.colors.palette.success,
            PhaseStatus::Failed { .. } => theme.colors.palette.error,
            PhaseStatus::TimedOut { .. } => theme.colors.palette.warning,
        }
    }

    fn format_phase_status(&self, status: &crate::startup::progress::PhaseStatus, timeout: &Duration) -> String {
        use crate::startup::progress::PhaseStatus;
        match status {
            PhaseStatus::Pending => "Waiting to start...".to_string(),
            PhaseStatus::InProgress { started_at } => {
                let elapsed = started_at.elapsed();
                format!("Running for {:.1}s (timeout: {:.0}s)", elapsed.as_secs_f64(), timeout.as_secs_f64())
            }
            PhaseStatus::Completed { duration } => {
                format!("Completed in {:.1}s", duration.as_secs_f64())
            }
            PhaseStatus::Failed { error } => {
                format!("Failed: {}", error)
            }
            PhaseStatus::TimedOut { duration } => {
                format!("Timed out after {:.1}s", duration.as_secs_f64())
            }
        }
    }
}

impl Default for StartupProgressScreen {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::startup::manager::StartupProgressManager;
    use crate::theme::{Theme, ThemeManager};
    use ratatui::{backend::TestBackend, Terminal};
    use std::time::{Duration, Instant};

    fn create_test_theme() -> Theme {
        ThemeManager::new().current_theme().clone()
    }

    #[test]
    fn test_startup_progress_screen_creation() {
        let screen = StartupProgressScreen::new();
        assert!(screen.show_details);
    }

    #[test]
    fn test_toggle_details() {
        let mut screen = StartupProgressScreen::new();
        assert!(screen.show_details);
        
        screen.toggle_details();
        assert!(!screen.show_details);
        
        screen.toggle_details();
        assert!(screen.show_details);
    }

    #[test]
    fn test_status_color_mapping() {
        let screen = StartupProgressScreen::new();
        let theme = create_test_theme();
        
        use crate::startup::progress::PhaseStatus;
        
        // Test all status types return valid colors
        let pending = PhaseStatus::Pending;
        let in_progress = PhaseStatus::InProgress { started_at: Instant::now() };
        let completed = PhaseStatus::Completed { duration: Duration::from_secs(1) };
        let failed = PhaseStatus::Failed { error: "Test error".to_string() };
        let timed_out = PhaseStatus::TimedOut { duration: Duration::from_secs(10) };
        
        assert_eq!(screen.get_status_color(&pending, &theme), theme.colors.palette.text_secondary);
        assert_eq!(screen.get_status_color(&in_progress, &theme), theme.colors.palette.accent);
        assert_eq!(screen.get_status_color(&completed, &theme), theme.colors.palette.success);
        assert_eq!(screen.get_status_color(&failed, &theme), theme.colors.palette.error);
        assert_eq!(screen.get_status_color(&timed_out, &theme), theme.colors.palette.warning);
    }

    #[test]
    fn test_format_phase_status() {
        let screen = StartupProgressScreen::new();
        let timeout = Duration::from_secs(30);
        
        use crate::startup::progress::PhaseStatus;
        
        let pending = PhaseStatus::Pending;
        assert_eq!(screen.format_phase_status(&pending, &timeout), "Waiting to start...");
        
        let completed = PhaseStatus::Completed { duration: Duration::from_secs(5) };
        assert_eq!(screen.format_phase_status(&completed, &timeout), "Completed in 5.0s");
        
        let failed = PhaseStatus::Failed { error: "Connection failed".to_string() };
        assert_eq!(screen.format_phase_status(&failed, &timeout), "Failed: Connection failed");
        
        let timed_out = PhaseStatus::TimedOut { duration: Duration::from_secs(30) };
        assert_eq!(screen.format_phase_status(&timed_out, &timeout), "Timed out after 30.0s");
    }

    #[test]
    fn test_render_with_invisible_manager() {
        let mut backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let screen = StartupProgressScreen::new();
        let mut manager = StartupProgressManager::new();
        let theme = create_test_theme();
        
        // Hide the manager
        manager.hide();
        
        terminal.draw(|frame| {
            let area = frame.size();
            screen.render(frame, area, &manager, &theme);
        }).unwrap();
        
        // Should not render anything when manager is not visible
        let buffer = terminal.backend().buffer();
        assert!(buffer.content.iter().all(|cell| cell.symbol() == " " || cell.symbol() == ""));
    }

    #[test]
    fn test_render_with_visible_manager() {
        let mut backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let screen = StartupProgressScreen::new();
        let mut manager = StartupProgressManager::new();
        let theme = create_test_theme();
        
        // Start a phase to make it interesting
        manager.start_phase("Database").unwrap();
        
        terminal.draw(|frame| {
            let area = frame.size();
            screen.render(frame, area, &manager, &theme);
        }).unwrap();
        
        // Should render content when manager is visible
        let buffer = terminal.backend().buffer();
        let content: String = buffer.content.iter().map(|cell| cell.symbol()).collect();
        
        // Check for key elements
        assert!(content.contains("Comunicado"));
        assert!(content.contains("Initializing"));
        assert!(content.contains("Overall Progress"));
        assert!(content.contains("Current Phase"));
        assert!(content.contains("Database"));
    }

    #[test]
    fn test_render_with_errors() {
        let mut backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let screen = StartupProgressScreen::new();
        let mut manager = StartupProgressManager::new();
        let theme = create_test_theme();
        
        // Create an error scenario
        manager.start_phase("IMAP Manager").unwrap();
        manager.fail_phase("IMAP Manager", "Connection timeout".to_string()).unwrap();
        
        terminal.draw(|frame| {
            let area = frame.size();
            screen.render(frame, area, &manager, &theme);
        }).unwrap();
        
        let buffer = terminal.backend().buffer();
        let content: String = buffer.content.iter().map(|cell| cell.symbol()).collect();
        
        // Should show error information
        assert!(content.contains("Non-Critical Issues"));
        assert!(content.contains("❌")); // Error icon should be present
    }

    #[test]
    fn test_render_with_completion() {
        let mut backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let screen = StartupProgressScreen::new();
        let mut manager = StartupProgressManager::new();
        let theme = create_test_theme();
        
        // Complete all phases
        for phase_name in &["Database", "IMAP Manager", "Account Setup", "Services"] {
            manager.start_phase(phase_name).unwrap();
            manager.complete_phase(phase_name).unwrap();
        }
        
        // Check completion status
        assert_eq!(manager.overall_progress_percentage(), 100.0);
        assert!(manager.is_complete());
        
        // When complete, manager should not be visible
        assert!(!manager.is_visible());
        
        terminal.draw(|frame| {
            let area = frame.size();
            screen.render(frame, area, &manager, &theme);
        }).unwrap();
        
        let buffer = terminal.backend().buffer();
        let content: String = buffer.content.iter().map(|cell| cell.symbol()).collect();
        
        // Should not render anything when complete (invisible)
        assert!(buffer.content.iter().all(|cell| cell.symbol() == " " || cell.symbol() == ""));
    }
    
    #[test]
    fn test_render_with_near_completion() {
        let mut backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let screen = StartupProgressScreen::new();
        let mut manager = StartupProgressManager::new();
        let theme = create_test_theme();
        
        // Complete most phases but keep one in progress
        for phase_name in &["Database", "IMAP Manager", "Account Setup"] {
            manager.start_phase(phase_name).unwrap();
            manager.complete_phase(phase_name).unwrap();
        }
        
        // Start the last phase but don't complete it
        manager.start_phase("Services").unwrap();
        
        terminal.draw(|frame| {
            let area = frame.size();
            screen.render(frame, area, &manager, &theme);
        }).unwrap();
        
        let buffer = terminal.backend().buffer();
        let content: String = buffer.content.iter().map(|cell| cell.symbol()).collect();
        
        // Should show high progress percentage
        assert!(manager.overall_progress_percentage() > 80.0);
        assert!(content.contains("✅")); // Success icons should be present for completed phases
        assert!(content.contains("🔄")); // In-progress icon for current phase
        assert!(content.contains("Services"));
    }
}