use anyhow::Result;
use comunicado::app::App;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    // Create and initialize the application
    let mut app = App::new();
    
    // Initialize database connection
    app.initialize_database().await?;
    
    // Load sample data if available
    app.load_sample_data().await?;
    
    // Run the application
    app.run().await?;

    Ok(())
}