use anyhow::Result;
use comunicado::app::App;

#[tokio::main]
async fn main() -> Result<()> {
    // Check for command line arguments
    let args: Vec<String> = std::env::args().collect();
    
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
    
    // Check for --clean-content flag to reprocess database content
    if args.len() > 1 && args[1] == "--clean-content" {
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