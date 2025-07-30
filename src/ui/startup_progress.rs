//! Startup progress UI component
//! 
//! Shows users real-time startup progress with progress bars and status updates

use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, Paragraph},
    Frame,
};
use std::time::{Duration, Instant};

/// Startup progress display component
pub struct StartupProgressWidget {
    /// Overall startup progress (0-100)
    overall_progress: f64,
    /// Current phase name
    current_phase: String,
    /// Phase progress (0-100)
    phase_progress: f64,
    /// Status message
    status_message: String,
    /// Background tasks status
    background_tasks: Vec<BackgroundTaskStatus>,
    /// Startup start time
    startup_start: Instant,
    /// Show detailed progress
    show_details: bool,
}

/// Background task status for display
#[derive(Clone)]
pub struct BackgroundTaskStatus {
    pub name: String,
    pub progress: f64,
    pub status: String,
    pub completed: bool,
}

impl StartupProgressWidget {
    pub fn new() -> Self {
        Self {
            overall_progress: 0.0,
            current_phase: "Initializing...".to_string(),
            phase_progress: 0.0,
            status_message: "Starting Comunicado".to_string(),
            background_tasks: Vec::new(),
            startup_start: Instant::now(),
            show_details: false,
        }
    }

    /// Update overall startup progress
    pub fn update_progress(&mut self, progress: f64, phase: String, status: String) {
        self.overall_progress = progress.clamp(0.0, 100.0);
        self.current_phase = phase;
        self.status_message = status;
    }

    /// Update current phase progress
    pub fn update_phase_progress(&mut self, progress: f64) {
        self.phase_progress = progress.clamp(0.0, 100.0);
    }

    /// Add or update background task status
    pub fn update_background_task(&mut self, name: String, progress: f64, status: String, completed: bool) {
        let task_status = BackgroundTaskStatus {
            name: name.clone(),
            progress: progress.clamp(0.0, 100.0),
            status,
            completed,
        };

        // Update existing task or add new one
        if let Some(existing) = self.background_tasks.iter_mut().find(|t| t.name == name) {
            *existing = task_status;
        } else {
            self.background_tasks.push(task_status);
        }
    }

    /// Toggle detailed progress view
    pub fn toggle_details(&mut self) {
        self.show_details = !self.show_details;
    }

    /// Check if startup is complete
    pub fn is_startup_complete(&self) -> bool {
        self.overall_progress >= 100.0
    }

    /// Get elapsed startup time
    pub fn elapsed_time(&self) -> Duration {
        self.startup_start.elapsed()
    }

    /// Render the startup progress UI
    pub fn render(&self, f: &mut Frame<'_>, area: Rect) {
        // Create centered popup area
        let popup_area = self.centered_rect(80, if self.show_details { 80 } else { 50 }, area);
        
        // Clear background
        f.render_widget(Clear, popup_area);
        
        // Main container
        let block = Block::default()
            .title(" Comunicado - Starting Up ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        
        f.render_widget(block, popup_area);
        
        // Inner layout
        let inner = popup_area.inner(&ratatui::layout::Margin { vertical: 1, horizontal: 2 });
        
        if self.show_details {
            self.render_detailed_view(f, inner);
        } else {
            self.render_simple_view(f, inner);
        }
        
        // Instructions at bottom
        let instructions = if self.show_details {
            "Press 'd' to hide details, 'q' to quit"
        } else {
            "Press 'd' for details, 'q' to quit"
        };
        
        let instruction_area = Rect {
            x: popup_area.x + 2,
            y: popup_area.y + popup_area.height - 2,
            width: popup_area.width - 4,
            height: 1,
        };
        
        let instruction_paragraph = Paragraph::new(instructions)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        
        f.render_widget(instruction_paragraph, instruction_area);
    }

    /// Render simple startup view
    fn render_simple_view(&self, f: &mut Frame<'_>, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Status
                Constraint::Length(3), // Overall progress
                Constraint::Length(3), // Phase progress
                Constraint::Length(2), // Time
                Constraint::Min(1),    // Background tasks
            ])
            .split(area);

        // Status message
        let status = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::Yellow)),
                Span::styled(&self.status_message, Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("Phase: ", Style::default().fg(Color::Yellow)),
                Span::styled(&self.current_phase, Style::default().fg(Color::Green)),
            ]),
        ]);
        f.render_widget(status, chunks[0]);

        // Overall progress bar
        let overall_gauge = Gauge::default()
            .block(Block::default().title("Overall Progress").borders(Borders::ALL))
            .gauge_style(Style::default().fg(Color::Green).bg(Color::Black))
            .percent(self.overall_progress as u16)
            .label(format!("{:.1}%", self.overall_progress));
        f.render_widget(overall_gauge, chunks[1]);

        // Phase progress bar
        let phase_gauge = Gauge::default()
            .block(Block::default().title("Current Phase").borders(Borders::ALL))
            .gauge_style(Style::default().fg(Color::Cyan).bg(Color::Black))
            .percent(self.phase_progress as u16)
            .label(format!("{:.1}%", self.phase_progress));
        f.render_widget(phase_gauge, chunks[2]);

        // Elapsed time
        let elapsed = self.elapsed_time();
        let time_text = format!("Elapsed: {:.2}s", elapsed.as_secs_f64());
        let time_paragraph = Paragraph::new(time_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        f.render_widget(time_paragraph, chunks[3]);

        // Background tasks summary
        if !self.background_tasks.is_empty() {
            let completed_count = self.background_tasks.iter().filter(|t| t.completed).count();
            let total_count = self.background_tasks.len();
            
            let bg_status = format!("Background Tasks: {}/{} completed", completed_count, total_count);
            let bg_paragraph = Paragraph::new(vec![
                Line::from(Span::styled(bg_status, Style::default().fg(Color::Blue))),
                Line::from(Span::styled("Press 'd' for details", Style::default().fg(Color::Gray))),
            ])
            .alignment(Alignment::Center);
            f.render_widget(bg_paragraph, chunks[4]);
        }
    }

    /// Render detailed startup view with background tasks
    fn render_detailed_view(&self, f: &mut Frame<'_>, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Status
                Constraint::Length(3), // Overall progress
                Constraint::Length(2), // Time
                Constraint::Min(1),    // Background tasks
            ])
            .split(area);

        // Status message
        let status = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::Yellow)),
                Span::styled(&self.status_message, Style::default().fg(Color::White)),
            ]),
        ]);
        f.render_widget(status, chunks[0]);

        // Overall progress bar
        let overall_gauge = Gauge::default()
            .block(Block::default().title("Overall Progress").borders(Borders::ALL))
            .gauge_style(Style::default().fg(Color::Green).bg(Color::Black))
            .percent(self.overall_progress as u16)
            .label(format!("{:.1}%", self.overall_progress));
        f.render_widget(overall_gauge, chunks[1]);

        // Elapsed time
        let elapsed = self.elapsed_time();
        let time_text = format!("Elapsed: {:.2}s", elapsed.as_secs_f64());
        let time_paragraph = Paragraph::new(time_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        f.render_widget(time_paragraph, chunks[2]);

        // Background tasks detailed view
        if !self.background_tasks.is_empty() {
            let task_block = Block::default()
                .title("Background Tasks")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue));
            
            let task_inner = chunks[3].inner(&ratatui::layout::Margin { vertical: 1, horizontal: 1 });
            f.render_widget(task_block, chunks[3]);
            
            // Create layout for each task
            let task_count = self.background_tasks.len();
            let task_height = 2; // Each task takes 2 lines
            
            if task_count > 0 {
                let constraints: Vec<Constraint> = (0..task_count)
                    .map(|_| Constraint::Length(task_height))
                    .collect();
                
                let task_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(constraints)
                    .split(task_inner);
                
                for (i, task) in self.background_tasks.iter().enumerate() {
                    if i < task_chunks.len() {
                        self.render_background_task(f, task_chunks[i], task);
                    }
                }
            }
        }
    }

    /// Render individual background task
    fn render_background_task(&self, f: &mut Frame<'_>, area: Rect, task: &BackgroundTaskStatus) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(area);

        // Task name and status
        let status_color = if task.completed {
            Color::Green
        } else if task.progress > 0.0 {
            Color::Yellow
        } else {
            Color::Gray
        };

        let task_status = vec![
            Span::styled(&task.name, Style::default().fg(status_color)),
            Span::styled(" - ", Style::default().fg(Color::Gray)),
            Span::styled(&task.status, Style::default().fg(Color::White)),
        ];

        let task_paragraph = Paragraph::new(Line::from(task_status));
        f.render_widget(task_paragraph, chunks[0]);

        // Progress bar for active tasks
        if !task.completed && task.progress > 0.0 {
            let task_gauge = Gauge::default()
                .gauge_style(Style::default().fg(status_color).bg(Color::Black))
                .percent(task.progress as u16)
                .label(format!("{:.0}%", task.progress));
            f.render_widget(task_gauge, chunks[1]);
        } else if task.completed {
            let completed_text = Paragraph::new("✅ Completed")
                .style(Style::default().fg(Color::Green));
            f.render_widget(completed_text, chunks[1]);
        }
    }

    /// Helper to create centered rectangle
    fn centered_rect(&self, percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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
}

impl Default for StartupProgressWidget {
    fn default() -> Self {
        Self::new()
    }
}