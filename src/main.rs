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
    let update_progress = |progress_manager: &StartupProgressManager, terminal: &mut Option<Terminal<CrosstermBackend<std::io::Stdout>>>| -> Result<()> {
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

    // Phase 2: Initialize IMAP account manager with reduced timeout
    if let Err(e) = progress_manager.start_phase("IMAP Manager") {
        tracing::warn!("Failed to start IMAP Manager phase: {}", e);
    }
    update_progress(&progress_manager, &mut terminal)?;
    
    tracing::info!("Initializing IMAP account manager with reduced timeout...");
    match tokio::time::timeout(
        std::time::Duration::from_secs(10), // Reduced from default
        app.initialize_imap_manager(),
    )
    .await
    {
        Ok(Ok(())) => {
            tracing::info!("IMAP account manager initialized successfully");
            if let Err(e) = progress_manager.complete_phase("IMAP Manager") {
                tracing::warn!("Failed to complete IMAP Manager phase: {}", e);
            }
        }
        Ok(Err(e)) => {
            tracing::error!("Failed to initialize IMAP account manager: {}", e);
            if let Err(err) = progress_manager.fail_phase("IMAP Manager", format!("IMAP initialization failed: {}", e)) {
                tracing::warn!("Failed to mark IMAP Manager phase as failed: {}", err);
            }
            // Continue without IMAP manager
        }
        Err(_) => {
            tracing::error!("IMAP account manager initialization timed out after 10 seconds");
            if let Err(e) = progress_manager.timeout_phase("IMAP Manager") {
                tracing::warn!("Failed to mark IMAP Manager phase as timed out: {}", e);
            }
            // Continue without IMAP manager
        }
    }
    update_progress(&progress_manager, &mut terminal)?;

    // Phase 3: Check for existing accounts and run setup wizard if needed
    if let Err(e) = progress_manager.start_phase("Account Setup") {
        tracing::warn!("Failed to start Account Setup phase: {}", e);
    }
    update_progress(&progress_manager, &mut terminal)?;
    
    tracing::info!("Checking accounts and setup with timeout...");
    match tokio::time::timeout(
        std::time::Duration::from_secs(15),
        app.check_accounts_and_setup(),
    )
    .await
    {
        Ok(Ok(())) => {
            tracing::info!("Account check and setup completed");
            if let Err(e) = progress_manager.complete_phase("Account Setup") {
                tracing::warn!("Failed to complete Account Setup phase: {}", e);
            }
        }
        Ok(Err(e)) => {
            tracing::error!("Failed to check accounts and setup: {}", e);
            if let Err(err) = progress_manager.fail_phase("Account Setup", format!("Account setup failed: {}", e)) {
                tracing::warn!("Failed to mark Account Setup phase as failed: {}", err);
            }
            // Continue - UI will show setup wizard if needed
        }
        Err(_) => {
            tracing::error!("Account check and setup timed out after 15 seconds");
            if let Err(e) = progress_manager.timeout_phase("Account Setup") {
                tracing::warn!("Failed to mark Account Setup phase as timed out: {}", e);
            }
            // Continue - UI will show setup wizard if needed
        }
    }
    update_progress(&progress_manager, &mut terminal)?;

    // Phase 4: Initialize other services quickly (with short timeout)
    if let Err(e) = progress_manager.start_phase("Services") {
        tracing::warn!("Failed to start Services phase: {}", e);
    }
    update_progress(&progress_manager, &mut terminal)?;
    
    tracing::info!("Initializing services with timeout...");
    match tokio::time::timeout(std::time::Duration::from_secs(5), app.initialize_services()).await {
        Ok(Ok(())) => {
            tracing::info!("Services initialized successfully");
            if let Err(e) = progress_manager.complete_phase("Services") {
                tracing::warn!("Failed to complete Services phase: {}", e);
            }
        }
        Ok(Err(e)) => {
            tracing::error!("Failed to initialize services: {}", e);
            if let Err(err) = progress_manager.fail_phase("Services", format!("Services initialization failed: {}", e)) {
                tracing::warn!("Failed to mark Services phase as failed: {}", err);
            }
            // Continue without some services
        }
        Err(_) => {
            tracing::error!("Service initialization timed out after 5 seconds");
            if let Err(e) = progress_manager.timeout_phase("Services") {
                tracing::warn!("Failed to mark Services phase as timed out: {}", e);
            }
            // Continue without some services
        }
    }
    update_progress(&progress_manager, &mut terminal)?;

    // Phase 5: Initialize dashboard services (non-critical, short timeout)
    if let Err(e) = progress_manager.start_phase("Dashboard Services") {
        tracing::warn!("Failed to start Dashboard Services phase: {}", e);
    }
    update_progress(&progress_manager, &mut terminal)?;
    
    tracing::info!("Initializing dashboard services with timeout...");
    match tokio::time::timeout(
        std::time::Duration::from_secs(3),
        app.initialize_dashboard_services(),
    )
    .await
    {
        Ok(Ok(())) => {
            tracing::info!("Dashboard services initialized successfully");
            if let Err(e) = progress_manager.complete_phase("Dashboard Services") {
                tracing::warn!("Failed to complete Dashboard Services phase: {}", e);
            }
        }
        Ok(Err(e)) => {
            tracing::error!("Failed to initialize dashboard services: {}", e);
            if let Err(err) = progress_manager.fail_phase("Dashboard Services", format!("Dashboard services failed: {}", e)) {
                tracing::warn!("Failed to mark Dashboard Services phase as failed: {}", err);
            }
            // Continue without dashboard services
        }
        Err(_) => {
            tracing::error!("Dashboard service initialization timed out after 3 seconds");
            if let Err(e) = progress_manager.timeout_phase("Dashboard Services") {
                tracing::warn!("Failed to mark Dashboard Services phase as timed out: {}", e);
            }
            // Continue without dashboard services
        }
    }
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
