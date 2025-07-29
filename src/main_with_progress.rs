use anyhow::Result;
use clap::Parser;
use comunicado::app::App;
use comunicado::cli::{Cli, CliHandler};
use comunicado::startup::{StartupProgressManager, StartupProgressScreen};
use comunicado::theme::Theme;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use tokio::time::{timeout, Duration, Instant};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Handle CLI commands
    if let Some(command) = cli.command {
        let cli_handler = CliHandler::new(cli.config_dir).await?;
        return cli_handler.handle_command(command, cli.dry_run).await;
    }

    // Continue with normal TUI application
    let debug_mode = cli.debug;

    // Initialize tracing for logging - write to file to avoid interfering with TUI
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("comunicado.log")
        .expect("Failed to create log file");

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
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Initialize startup progress system
    let mut progress_manager = StartupProgressManager::new();
    let progress_screen = StartupProgressScreen::new();
    let theme = Theme::default();

    // Create and initialize the application with progress tracking
    let result = run_startup_with_progress(
        &mut terminal, 
        &mut progress_manager, 
        &progress_screen, 
        &theme,
        debug_mode
    ).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    match result {
        Ok(app) => {
            // Switch to normal app mode
            tracing::info!("Startup complete - switching to main application");
            
            // Re-setup terminal for main app
            enable_raw_mode()?;
            let mut stdout = io::stdout();
            execute!(stdout, EnterAlternateScreen)?;
            let backend = CrosstermBackend::new(stdout);
            let mut terminal = Terminal::new(backend)?;
            
            // Run the main application
            let result = app.run().await;
            
            // Restore terminal
            disable_raw_mode()?;
            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
            terminal.show_cursor()?;
            
            result
        }
        Err(e) => {
            eprintln!("❌ Startup failed: {}", e);
            Err(e)
        }
    }
}

async fn run_startup_with_progress(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    progress_manager: &mut StartupProgressManager,
    progress_screen: &StartupProgressScreen,
    theme: &Theme,
    debug_mode: bool,
) -> Result<App> {
    // Start the startup process
    progress_manager.start_startup();
    
    // Create the application instance
    let mut app = App::new()?;
    
    // Run startup phases with progress display
    let startup_result = run_startup_phases(
        terminal,
        progress_manager,
        progress_screen,
        theme,
        &mut app,
        debug_mode,
    ).await;
    
    // Mark startup as complete
    progress_manager.complete_startup();
    
    // Final progress update
    update_progress_display(terminal, progress_manager, progress_screen, theme)?;
    
    // Brief pause to show completion
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    startup_result.map(|_| app)
}

async fn run_startup_phases(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    progress_manager: &mut StartupProgressManager,
    progress_screen: &StartupProgressScreen,
    theme: &Theme,
    app: &mut App,
    debug_mode: bool,
) -> Result<()> {
    // Phase 1: Database initialization
    run_phase_with_progress(
        terminal, progress_manager, progress_screen, theme,
        "Database",
        Duration::from_secs(10),
        || async {
            app.initialize_database().await
        }
    ).await?;

    // Check for --clean-content flag (quick check, no progress needed)
    let args: Vec<String> = std::env::args().collect();
    if args.contains(&"--clean-content".to_string()) {
        println!("🧹 Starting database content cleaning...");
        
        if let Some(db) = app.get_database() {
            match db.reprocess_message_content().await {
                Ok(count) => {
                    println!("✅ Successfully cleaned {} messages", count);
                    println!("   - Raw HTML/CSS content converted to plain text");
                    println!("   - Email headers and technical metadata removed");
                    println!("   - Content should now display cleanly in the email viewer");
                    println!("\nRestart the application to see the changes.");
                }
                Err(e) => {
                    eprintln!("❌ Error cleaning content: {}", e);
                    std::process::exit(1);
                }
            }
        } else {
            eprintln!("❌ Database not available");
            std::process::exit(1);
        }
        std::process::exit(0);
    }

    // Phase 2: IMAP account manager
    run_phase_with_progress(
        terminal, progress_manager, progress_screen, theme,
        "IMAP Manager",
        Duration::from_secs(10),
        || async {
            tracing::info!("Initializing IMAP account manager with reduced timeout...");
            app.initialize_imap_manager().await
        }
    ).await.unwrap_or_else(|e| {
        tracing::error!("Failed to initialize IMAP account manager: {}", e);
        // Continue without IMAP manager
    });

    // Phase 3: Account check and setup
    run_phase_with_progress(
        terminal, progress_manager, progress_screen, theme,
        "Account Setup",
        Duration::from_secs(15),
        || async {
            tracing::info!("Checking accounts and setup with timeout...");
            app.check_accounts_and_setup().await
        }
    ).await.unwrap_or_else(|e| {
        tracing::error!("Failed to check accounts and setup: {}", e);
        // Continue - UI will show setup wizard if needed
    });

    // Phase 4: Services initialization
    run_phase_with_progress(
        terminal, progress_manager, progress_screen, theme,
        "Services",
        Duration::from_secs(5),
        || async {
            tracing::info!("Initializing services with timeout...");
            app.initialize_services().await
        }
    ).await.unwrap_or_else(|e| {
        tracing::error!("Failed to initialize services: {}", e);
        // Continue without some services
    });

    // Phase 5: Dashboard services
    run_phase_with_progress(
        terminal, progress_manager, progress_screen, theme,
        "Dashboard Services",
        Duration::from_secs(3),
        || async {
            tracing::info!("Initializing dashboard services with timeout...");
            app.initialize_dashboard_services().await
        }
    ).await.unwrap_or_else(|e| {
        tracing::error!("Failed to initialize dashboard services: {}", e);
        // Continue without dashboard services
    });

    Ok(())
}

async fn run_phase_with_progress<F, Fut>(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    progress_manager: &mut StartupProgressManager,
    progress_screen: &StartupProgressScreen,
    theme: &Theme,
    phase_name: &str,
    phase_timeout: Duration,
    operation: F,
) -> Result<()>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
{
    // Start the phase
    progress_manager.start_phase_by_name(phase_name)?;
    
    // Create a background task for progress updates
    let progress_update_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(100));
        
        loop {
            interval.tick().await;
            // Progress updates would happen here in a real implementation
            // For now, we'll rely on the timeout and completion events
        }
    });
    
    // Run the operation with timeout and progress display
    let operation_result = timeout(phase_timeout, async {
        // Update display before starting operation
        update_progress_display(terminal, progress_manager, progress_screen, theme)?;
        
        // Run the actual operation
        let result = operation().await;
        
        // Update display after operation
        update_progress_display(terminal, progress_manager, progress_screen, theme)?;
        
        result
    }).await;
    
    // Cancel the progress update task
    progress_update_task.abort();
    
    // Handle the result
    match operation_result {
        Ok(Ok(())) => {
            // Success
            progress_manager.complete_current_phase()?;
            tracing::info!("{} initialization completed successfully", phase_name);
        }
        Ok(Err(e)) => {
            // Operation failed
            progress_manager.fail_current_phase(&format!("Failed: {}", e))?;
            tracing::error!("Failed to initialize {}: {}", phase_name, e);
            return Err(e);
        }
        Err(_) => {
            // Timeout
            progress_manager.timeout_current_phase(&format!("{} initialization timed out", phase_name))?;
            tracing::error!("{} initialization timed out after {:?}", phase_name, phase_timeout);
            // Don't return error for timeout - continue with degraded functionality
        }
    }
    
    // Final display update for this phase
    update_progress_display(terminal, progress_manager, progress_screen, theme)?;
    
    Ok(())
}

fn update_progress_display(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    progress_manager: &StartupProgressManager,
    progress_screen: &StartupProgressScreen,
    theme: &Theme,
) -> Result<()> {
    terminal.draw(|frame| {
        let area = frame.size();
        progress_screen.render(frame, area, progress_manager, theme);
    })?;
    
    // Handle any input events (like ESC to quit)
    if crossterm::event::poll(Duration::from_millis(0))? {
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    return Err(anyhow::anyhow!("Startup cancelled by user"));
                }
                _ => {}
            }
        }
    }
    
    Ok(())
}