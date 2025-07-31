use anyhow::Result;
use clap::Parser;
use comunicado::app::App;
use comunicado::cli::{Cli, CliHandler};
use comunicado::startup::StartupProgressScreen;
use comunicado::theme::Theme;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    tty::IsTty,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

/// Setup the terminal for the TUI application.
fn setup_terminal() -> Result<Option<Terminal<CrosstermBackend<io::Stdout>>>> {
    if !io::stdout().is_tty() {
        println!("No TTY detected, using text progress...");
        return Ok(None);
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(Some(terminal))
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let cli_handler = CliHandler::new(cli.config_dir.clone()).await?;

    // Handle CLI commands that exit immediately
    if cli.clean_content {
        return cli_handler.handle_clean_content().await;
    }
    if let Some(command) = cli.command {
        return cli_handler.handle_command(command, cli.dry_run).await;
    }

    // Continue with normal TUI application
    let debug_mode = cli.debug;
    let startup_mode = cli.get_startup_mode();

    // Initialize tracing for logging - write to file to avoid interfering with TUI
    let log_file = match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("comunicado.log")
    {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Failed to open log file: {}", e);
            return Err(e.into());
        }
    };

    // Set log level based on debug flag
    let log_level = if debug_mode {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    tracing_subscriber::fmt()
        .with_writer(log_file)
        .with_ansi(false) // Disable ANSI colors in log file
        .with_max_level(log_level)
        .init();

    if debug_mode {
        tracing::info!("🐛 Debug mode enabled - verbose logging active");
    }

    // Setup terminal for startup progress display
    print!("Initializing Comunicado...\n");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
    
    // Create progress display components  
    let progress_screen = StartupProgressScreen::new();
    let theme = Theme::default();
    
    // Setup terminal for progress display
    let mut terminal = setup_terminal()?;

    // Progress tracking starts automatically when created

    // Create and initialize the application
    let mut app = App::new()?;
    
    // Set initial UI mode based on CLI arguments
    app.set_initial_mode(startup_mode);

    // Helper function to update progress display
    let update_progress = |app: &App, terminal: &mut Option<Terminal<CrosstermBackend<std::io::Stdout>>>| -> Result<()> {
        let progress_manager = app.startup_progress_manager();
        if let Some(ref mut term) = terminal {
            if let Err(e) = term.draw(|frame| {
                let area = frame.size();
                progress_screen.render(frame, area, progress_manager, &theme);
            }) {
                tracing::warn!("Failed to update progress display: {}", e);
                // Continue without visual progress
            }
        } else {
            // No terminal available, log progress instead
            if progress_manager.is_visible() {
                let progress = progress_manager.overall_progress_percentage();
                if let Some(current_phase) = progress_manager.current_phase() {
                    println!("Progress: {:.1}% - {}", progress, current_phase.name());
                }
            }
        }
        Ok(())
    };

    // Perform initialization
    tracing::info!("Starting application initialization...");
    
    // Show initial progress
    update_progress(&app, &mut terminal)?;
    
    // Perform the initialization - this now handles progress tracking internally
    if let Err(e) = app.perform_deferred_initialization().await {
        tracing::error!("Application initialization failed: {}", e);
        
        // Restore terminal before exiting
        if let Some(mut term) = terminal {
            disable_raw_mode()?;
            execute!(term.backend_mut(), LeaveAlternateScreen)?;
        }
        return Err(e);
    }
    
    // Show final progress state
    update_progress(&app, &mut terminal)?;
    
    // Brief pause to show completion
    tokio::time::sleep(std::time::Duration::from_millis(800)).await;
    
    // Restore terminal before starting main app
    if let Some(mut term) = terminal {
        disable_raw_mode()?;
        execute!(term.backend_mut(), LeaveAlternateScreen)?;
    }

    // Run the application
    tracing::info!("Starting application main loop...");
    app.run().await?;

    Ok(())
}
