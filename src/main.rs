use anyhow::Result;
use comunicado::app::App;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

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
    
    // Run the application
    app.run().await?;

    Ok(())
}