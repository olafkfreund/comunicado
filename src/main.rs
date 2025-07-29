use anyhow::Result;
use clap::Parser;
use comunicado::app::App;
use comunicado::cli::{Cli, CliHandler};

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
        .with_ansi(false)  // Disable ANSI colors in log file
        .with_max_level(log_level)
        .init();
    
    if debug_mode {
        tracing::info!("🐛 Debug mode enabled - verbose logging active");
    }

    // Create and initialize the application
    let mut app = App::new()?;
    
    // Initialize database connection
    app.initialize_database().await?;
    
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
    
    // Initialize IMAP account manager with reduced timeout
    tracing::info!("Initializing IMAP account manager with reduced timeout...");
    match tokio::time::timeout(
        std::time::Duration::from_secs(10), // Reduced from default
        app.initialize_imap_manager()
    ).await {
        Ok(Ok(())) => {
            tracing::info!("IMAP account manager initialized successfully");
        }
        Ok(Err(e)) => {
            tracing::error!("Failed to initialize IMAP account manager: {}", e);
            // Continue without IMAP manager
        }
        Err(_) => {
            tracing::error!("IMAP account manager initialization timed out after 10 seconds");
            // Continue without IMAP manager
        }
    }
    
    // Check for existing accounts and run setup wizard if needed (with timeout)
    tracing::info!("Checking accounts and setup with timeout...");
    match tokio::time::timeout(
        std::time::Duration::from_secs(15),
        app.check_accounts_and_setup()
    ).await {
        Ok(Ok(())) => {
            tracing::info!("Account check and setup completed");
        }
        Ok(Err(e)) => {
            tracing::error!("Failed to check accounts and setup: {}", e);
            // Continue - UI will show setup wizard if needed
        }
        Err(_) => {
            tracing::error!("Account check and setup timed out after 15 seconds");
            // Continue - UI will show setup wizard if needed
        }
    }
    
    // Initialize other services quickly (with short timeout)
    tracing::info!("Initializing services with timeout...");
    match tokio::time::timeout(
        std::time::Duration::from_secs(5),
        app.initialize_services()
    ).await {
        Ok(Ok(())) => {
            tracing::info!("Services initialized successfully");
        }
        Ok(Err(e)) => {
            tracing::error!("Failed to initialize services: {}", e);
            // Continue without some services
        }
        Err(_) => {
            tracing::error!("Service initialization timed out after 5 seconds");
            // Continue without some services
        }
    }
    
    // Initialize dashboard services (non-critical, short timeout)
    tracing::info!("Initializing dashboard services with timeout...");
    match tokio::time::timeout(
        std::time::Duration::from_secs(3),
        app.initialize_dashboard_services()
    ).await {
        Ok(Ok(())) => {
            tracing::info!("Dashboard services initialized successfully");
        }
        Ok(Err(e)) => {
            tracing::error!("Failed to initialize dashboard services: {}", e);
            // Continue without dashboard services
        }
        Err(_) => {
            tracing::error!("Dashboard service initialization timed out after 3 seconds");
            // Continue without dashboard services
        }
    }
    
    // Run the application
    tracing::info!("Starting application main loop...");
    app.run().await?;

    Ok(())
}