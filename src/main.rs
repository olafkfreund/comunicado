use anyhow::Result;
use clap::Parser;
use comunicado::cli::{Cli, CliHandler};

// Feature flag to switch between implementations
#[cfg(feature = "modular-ui")]
use comunicado::ModularApp as App;
#[cfg(not(feature = "modular-ui"))]
use comunicado::App;


#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Comunicado starting...");
    let cli = Cli::parse();
    println!("📋 CLI parsed");
    let cli_handler = CliHandler::new(cli.config_dir.clone()).await?;
    println!("🔧 CLI handler created");

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

    // Create and initialize the application - let it handle all startup progress
    println!("🏗️ Creating application...");
    let mut app = App::new()?;
    println!("✅ Application created");
    
    // Set initial UI mode based on CLI arguments
    app.set_initial_mode(startup_mode);
    println!("🔧 Initial mode set");

    // Run the application
    tracing::info!("Starting application main loop...");
    app.run().await?;

    Ok(())
}
