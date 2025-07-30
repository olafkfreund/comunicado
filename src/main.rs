use anyhow::Result;
use clap::Parser;
use comunicado::app::App;
use comunicado::cli::{Cli, CliHandler};
use comunicado::startup::{StartupProgressManager, StartupProgressScreen};
use comunicado::theme::Theme;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    tty::IsTty,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

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
    print!("Initializing Comunicado...\n");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
    
    // Create progress display components
    let mut progress_manager = StartupProgressManager::new();
    let progress_screen = StartupProgressScreen::new();
    let theme = Theme::default();
    
    // Setup terminal for progress display (only if stdout is a TTY)
    let use_progress_ui = std::io::stdout().is_tty();
    let mut terminal = if use_progress_ui {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        Some(Terminal::new(backend)?)
    } else {
        None
    };

    // Progress tracking starts automatically when created

    // Create and initialize the application
    let mut app = App::new()?;

    // Helper function to update progress display
    let mut update_progress = |progress_manager: &StartupProgressManager, terminal: &mut Option<Terminal<CrosstermBackend<std::io::Stdout>>>| -> Result<()> {
        if let Some(ref mut term) = terminal {
            term.draw(|frame| {
                let area = frame.size();
                progress_screen.render(frame, area, progress_manager, &theme);
            })?;
        }
        Ok(())
    };

    // Phase 1: Initialize database connection
    if let Err(e) = progress_manager.start_phase("Database") {
        tracing::warn!("Failed to start Database phase: {}", e);
    }
    update_progress(&progress_manager, &mut terminal)?;
    
    match app.initialize_database().await {
        Ok(()) => {
            if let Err(e) = progress_manager.complete_phase("Database") {
                tracing::warn!("Failed to complete Database phase: {}", e);
            }
        }
        Err(e) => {
            if let Err(err) = progress_manager.fail_phase("Database", format!("Database initialization failed: {}", e)) {
                tracing::warn!("Failed to mark Database phase as failed: {}", err);
            }
            update_progress(&progress_manager, &mut terminal)?;
            
            // Restore terminal before exiting
            if let Some(mut term) = terminal {
                disable_raw_mode()?;
                execute!(term.backend_mut(), LeaveAlternateScreen)?;
            }
            return Err(e);
        }
    }
    update_progress(&progress_manager, &mut terminal)?;

    // Check for --clean-content flag to reprocess database content (raw args check)
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
        return Ok(());
    }

    // Phases 2-5: Initialize services with reduced timeouts for faster startup
    tracing::info!("Starting optimized initialization of optional services...");
    
    // Helper macro to avoid repetitive error handling with real-time progress updates
    macro_rules! init_phase {
        ($phase_name:expr, $timeout_secs:expr, $init_fn:expr) => {
            if let Err(e) = progress_manager.start_phase($phase_name) {
                tracing::warn!("Failed to start {} phase: {}", $phase_name, e);
            }
            update_progress(&progress_manager, &mut terminal)?;
            
            // Add initial progress log
            progress_manager.add_phase_log($phase_name, format!("📡 Connecting to services..."));
            update_progress(&progress_manager, &mut terminal)?;
            
            // Spawn a task to provide progress updates during initialization
            let mut progress_interval = tokio::time::interval(std::time::Duration::from_millis(500));
            let mut progress_value = 10.0;
            
            let init_future = $init_fn;
            tokio::pin!(init_future);
            
            loop {
                tokio::select! {
                    result = &mut init_future => {
                        match result {
                            Ok(()) => {
                                tracing::info!("{} initialized successfully", $phase_name);
                                progress_manager.update_phase_progress($phase_name, 100.0, Some("✅ Initialization complete".to_string())).ok();
                                update_progress(&progress_manager, &mut terminal)?;
                                tokio::time::sleep(std::time::Duration::from_millis(200)).await; // Brief pause to show completion
                                if let Err(e) = progress_manager.complete_phase($phase_name) {
                                    tracing::warn!("Failed to complete {} phase: {}", $phase_name, e);
                                }
                            }
                            Err(e) => {
                                tracing::error!("Failed to initialize {}: {}", $phase_name, e);
                                progress_manager.add_phase_log($phase_name, format!("❌ Error: {}", e));
                                update_progress(&progress_manager, &mut terminal)?;
                                if let Err(err) = progress_manager.fail_phase($phase_name, format!("{} initialization failed: {}", $phase_name, e)) {
                                    tracing::warn!("Failed to mark {} phase as failed: {}", $phase_name, err);
                                }
                            }
                        }
                        break;
                    }
                    _ = progress_interval.tick() => {
                        progress_value = (progress_value + 15.0).min(90.0); // Increment progress but cap at 90%
                        let log_messages = [
                            "🔗 Establishing connections...",
                            "🔍 Verifying configurations...", 
                            "⚙️ Loading components...",
                            "🎯 Finalizing setup..."
                        ];
                        let log_index = ((progress_value / 25.0) as usize).min(log_messages.len() - 1);
                        let _ = progress_manager.update_phase_progress($phase_name, progress_value, Some(log_messages[log_index].to_string()));
                        update_progress(&progress_manager, &mut terminal)?;
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_secs($timeout_secs)) => {
                        tracing::error!("{} initialization timed out after {} seconds", $phase_name, $timeout_secs);
                        progress_manager.add_phase_log($phase_name, format!("⏰ Timeout after {}s", $timeout_secs));
                        update_progress(&progress_manager, &mut terminal)?;
                        if let Err(e) = progress_manager.timeout_phase($phase_name) {
                            tracing::warn!("Failed to mark {} phase as timed out: {}", $phase_name, e);
                        }
                        break;
                    }
                }
            }
        };
    }
    
    // Phase 2: IMAP Manager (reduced timeout from 10s to 5s)
    init_phase!("IMAP Manager", 5, app.initialize_imap_manager());
    
    // Phase 3: Account Setup (reduced timeout from 15s to 8s)
    init_phase!("Account Setup", 8, app.check_accounts_and_setup());
    
    // Phase 4: Services (reduced timeout from 5s to 3s)
    init_phase!("Services", 3, app.initialize_services());
    
    // Phase 5: Dashboard Services (reduced timeout from 3s to 2s)
    init_phase!("Dashboard Services", 2, app.initialize_dashboard_services());
    
    update_progress(&progress_manager, &mut terminal)?;

    // Startup is now complete - the StartupProgressManager automatically handles completion when all phases are done
    update_progress(&progress_manager, &mut terminal)?;
    
    // Brief pause to show completion
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    
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
