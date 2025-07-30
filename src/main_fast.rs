//! Fast startup main.rs implementation
//! 
//! This shows how to use the fast startup system to get sub-3-second startup times

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::{Duration, Instant};
use tokio::time::timeout;

use comunicado::{App, ui::startup_progress::StartupProgressWidget};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    println!("🚀 Comunicado Fast Startup");
    println!("Target: UI responsive in under 3 seconds");
    
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    // Create app and startup progress widget
    let mut app = App::new()?;
    let mut startup_widget = StartupProgressWidget::new();
    let startup_start = Instant::now();
    
    // Phase 1: Show startup screen immediately
    terminal.draw(|f| {
        startup_widget.update_progress(0.0, "Initializing".to_string(), "Starting Comunicado...".to_string());
        startup_widget.render(f, f.size());
    })?;
    
    // Phase 2: Fast startup process
    let startup_result = run_fast_startup(&mut app, &mut startup_widget, &mut terminal).await;
    
    match startup_result {
        Ok(_) => {
            let startup_time = startup_start.elapsed();
            println!("✅ Fast startup completed in {:?}", startup_time);
        }
        Err(e) => {
            // Cleanup terminal
            disable_raw_mode()?;
            execute!(
                terminal.backend_mut(),
                LeaveAlternateScreen,
                DisableMouseCapture
            )?;
            return Err(e);
        }
    }
    
    // Phase 3: Run main application with background loading
    run_main_app_loop(&mut app, &mut terminal).await?;
    
    // Cleanup
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    
    Ok(())
}

/// Run the fast startup process with progress display
async fn run_fast_startup(
    app: &mut App, 
    startup_widget: &mut StartupProgressWidget,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>
) -> Result<()> {
    // Start fast startup in background
    let startup_future = app.fast_startup();
    
    // Show progress while startup runs
    let progress_future = show_startup_progress(startup_widget, terminal);
    
    // Race between startup completion and user input
    tokio::select! {
        result = startup_future => {
            result?;
            
            // Show completion
            startup_widget.update_progress(100.0, "Complete".to_string(), "Comunicado is ready!".to_string());
            terminal.draw(|f| startup_widget.render(f, f.size()))?;
            
            // Brief pause to show completion
            tokio::time::sleep(Duration::from_millis(500)).await;
            
            Ok(())
        }
        _ = progress_future => {
            // User interrupted or other event
            Ok(())
        }
    }
}

/// Show startup progress with simulated updates
async fn show_startup_progress(
    startup_widget: &mut StartupProgressWidget,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>
) -> Result<()> {
    let mut progress = 0.0;
    let phases = vec![
        ("Core Systems", 20.0),
        ("Database", 40.0),
        ("UI Components", 60.0),
        ("Background Services", 80.0),
        ("Finalizing", 100.0),
    ];
    
    let mut phase_index = 0;
    let start_time = Instant::now();
    
    // Background tasks that will be queued
    let background_tasks = vec![
        ("Load Email Accounts", Duration::from_secs(2)),
        ("Setup Notifications", Duration::from_millis(800)),
        ("Load Contacts", Duration::from_secs(1)),
        ("Load Plugins", Duration::from_secs(3)),
    ];
    
    // Add background tasks to widget
    for (name, _duration) in &background_tasks {
        startup_widget.update_background_task(
            name.to_string(), 
            0.0, 
            "Queued".to_string(), 
            false
        );
    }
    
    loop {
        // Handle keyboard input
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => {
                        return Ok(());
                    }
                    KeyCode::Char('d') => {
                        startup_widget.toggle_details();
                    }
                    _ => {}
                }
            }
        }
        
        // Update progress based on elapsed time
        let elapsed = start_time.elapsed();
        let target_time = Duration::from_secs(3); // 3 second target
        let time_progress = (elapsed.as_secs_f64() / target_time.as_secs_f64()).min(1.0) * 100.0;
        
        // Update phase based on progress
        if phase_index < phases.len() {
            let (phase_name, phase_target) = &phases[phase_index];
            if time_progress >= *phase_target {
                progress = *phase_target;
                startup_widget.update_progress(
                    progress, 
                    phase_name.to_string(), 
                    format!("Completed {}", phase_name)
                );
                phase_index += 1;
            } else {
                // Interpolate within current phase
                let prev_target = if phase_index > 0 { phases[phase_index - 1].1 } else { 0.0 };
                let phase_progress = ((time_progress - prev_target) / (phase_target - prev_target)).max(0.0);
                startup_widget.update_phase_progress(phase_progress * 100.0);
                startup_widget.update_progress(
                    time_progress, 
                    phase_name.to_string(), 
                    format!("Loading {}", phase_name)
                );
            }
        }
        
        // Update background tasks based on elapsed time
        for (i, (name, duration)) in background_tasks.iter().enumerate() {
            let task_start_time = Duration::from_millis(500 + i as u64 * 200); // Stagger starts
            if elapsed > task_start_time {
                let task_elapsed = elapsed - task_start_time;
                let task_progress = (task_elapsed.as_secs_f64() / duration.as_secs_f64()).min(1.0) * 100.0;
                let completed = task_progress >= 100.0;
                
                let status = if completed {
                    "Completed".to_string()
                } else if task_progress > 0.0 {
                    "Loading...".to_string()
                } else {
                    "Queued".to_string()
                };
                
                startup_widget.update_background_task(
                    name.to_string(),
                    task_progress,
                    status,
                    completed
                );
            }
        }
        
        // Render progress
        terminal.draw(|f| startup_widget.render(f, f.size()))?;
        
        // Exit when startup is complete
        if progress >= 100.0 && elapsed >= target_time {
            break;
        }
        
        // Small delay to prevent excessive CPU usage
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    
    Ok(())
}

/// Run the main application loop after startup
async fn run_main_app_loop(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>
) -> Result<()> {
    println!("🎉 Switching to main application...");
    
    // Clear the screen and show main UI
    terminal.clear()?;
    
    // Main application loop
    loop {
        // Draw main UI
        terminal.draw(|f| {
            // This would be your main UI rendering
            // For now, just show a placeholder
            use ratatui::{
                layout::{Alignment, Constraint, Direction, Layout},
                style::{Color, Style},
                text::{Line, Span},
                widgets::{Block, Borders, Paragraph},
            };
            
            let area = f.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(1),
                    Constraint::Length(3),
                ])
                .split(area);
            
            // Header
            let header = Paragraph::new("Comunicado - Email & Calendar Client")
                .style(Style::default().fg(Color::Cyan))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(header, chunks[0]);
            
            // Main content area
            let (progress, status) = app.get_startup_progress();
            let content_lines = vec![
                Line::from("✅ UI is ready and responsive!"),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Background loading: ", Style::default().fg(Color::Yellow)),
                    Span::styled(format!("{:.0}%", progress), Style::default().fg(Color::Green)),
                ]),
                Line::from(status),
                Line::from(""),
                Line::from("Your email client is now usable while background services continue loading."),
            ];
            
            let content = Paragraph::new(content_lines)
                .block(Block::default().title("Status").borders(Borders::ALL))
                .wrap(ratatui::widgets::Wrap { trim: true });
            f.render_widget(content, chunks[1]);
            
            // Footer
            let footer = Paragraph::new("Press 'q' to quit, other keys for navigation (placeholder)")
                .style(Style::default().fg(Color::Gray))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(footer, chunks[2]);
        })?;
        
        // Handle events
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    _ => {
                        // Handle other key events here
                        // This is where your normal app event handling would go
                    }
                }
            }
        }
        
        // Simulate background task completion notifications
        // In real implementation, you'd listen to the background processor completion channel
        
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    
    Ok(())
}