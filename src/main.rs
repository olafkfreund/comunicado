use anyhow::Result;
use comunicado::app::App;

#[tokio::main]
async fn main() -> Result<()> {
    // Check for command line arguments
    let args: Vec<String> = std::env::args().collect();
    
    // Handle help flag
    if args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        println!("Comunicado - Modern TUI Email and Calendar Client");
        println!("");
        println!("USAGE:");
        println!("    comunicado [OPTIONS]");
        println!("");
        println!("OPTIONS:");
        println!("    --debug              Enable debug logging (verbose output to comunicado.log)");
        println!("    --clean-content      Clean existing database content (remove HTML/headers)");
        println!("    -h, --help           Show this help message");
        println!("");
        println!("KEYBOARD SHORTCUTS:");
        println!("    Ctrl+R              Refresh account connection");
        println!("    F5 / R              Refresh folder (when folder tree focused)");
        println!("    /                   Search messages");
        println!("    c                   Compose new email");
        println!("    Tab                 Switch between panes");
        println!("");
        println!("Debug logs are written to: comunicado.log");
        return Ok(());
    }
    
    // Check for debug flag
    let debug_mode = args.contains(&"--debug".to_string());
    
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
    println!("🔧 Creating application...");
    let mut app = App::new()?;
    println!("✅ Application created successfully");
    
    // Initialize database connection
    println!("🔧 Initializing database...");
    app.initialize_database().await?;
    println!("✅ Database initialized successfully");
    
    // Check for --clean-content flag to reprocess database content
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
    
    // Initialize IMAP account manager
    println!("🔧 Initializing IMAP account manager...");
    tracing::info!("Initializing IMAP account manager...");
    app.initialize_imap_manager().await?;
    println!("✅ IMAP account manager initialized successfully");
    tracing::info!("IMAP account manager initialized successfully");
    
    // Check for existing accounts and run setup wizard if needed
    println!("🔧 Checking accounts and setup...");
    tracing::info!("Checking accounts and setup...");
    app.check_accounts_and_setup().await?;
    println!("✅ Account check and setup completed");
    tracing::info!("Account check and setup completed");
    
    // Initialize SMTP service and contacts manager
    println!("🔧 Initializing services...");
    tracing::info!("Initializing services...");
    app.initialize_services().await?;
    println!("✅ Services initialized successfully");
    tracing::info!("Services initialized successfully");
    
    // Initialize dashboard services for start page
    println!("🔧 Initializing dashboard services...");
    tracing::info!("Initializing dashboard services...");
    app.initialize_dashboard_services().await?;
    println!("✅ Dashboard services initialized successfully");
    tracing::info!("Dashboard services initialized successfully");
    
    // Run the application
    println!("🚀 Starting application main loop...");
    tracing::info!("Starting application main loop...");
    app.run().await?;

    Ok(())
}