use anyhow::Result;
use comunicado::app::App;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing for logging - write to file to avoid interfering with TUI
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("comunicado.log")
        .expect("Failed to create log file");
    
    tracing_subscriber::fmt()
        .with_writer(log_file)
        .with_ansi(false)  // Disable ANSI colors in log file
        .init();

    // Create and initialize the application
    let mut app = App::new()?;
    
    // Initialize database connection
    app.initialize_database().await?;
    
    // Initialize IMAP account manager
    app.initialize_imap_manager().await?;
    
    // Check for existing accounts and run setup wizard if needed
    app.check_accounts_and_setup().await?;
    
    // Initialize SMTP service and contacts manager
    app.initialize_services().await?;
    
    // Initialize dashboard services for start page
    app.initialize_dashboard_services().await?;
    
    // Run the application
    app.run().await?;

    Ok(())
}