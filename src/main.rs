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

    // Configure granular logging to prevent performance issues from noisy dependencies
    use tracing_subscriber::{fmt, EnvFilter, prelude::*};
    
    // Create a filter that allows our app's debug logs but filters out noisy third-party logs
    let env_filter = if debug_mode {
        EnvFilter::new("comunicado=debug,info")
            // Allow our application's debug logs
            .add_directive("comunicado=debug".parse().unwrap())
            // Set noisy HTML parsing crates to ERROR level to filter out WARN spam
            .add_directive("html5ever=error".parse().unwrap())
            .add_directive("scraper=error".parse().unwrap())
            .add_directive("ammonia=error".parse().unwrap())
            .add_directive("selectors=error".parse().unwrap())
            .add_directive("markup5ever=error".parse().unwrap())
            .add_directive("cssparser=error".parse().unwrap())
            // Allow other crates at INFO level
            .add_directive("info".parse().unwrap())
    } else {
        EnvFilter::new("comunicado=info,error")
    };

    tracing_subscriber::registry()
        .with(fmt::layer()
            .with_writer(log_file)
            .with_ansi(false) // Disable ANSI colors in log file
        )
        .with(env_filter)
        .init();

    if debug_mode {
        tracing::info!("🐛 Debug mode enabled - verbose logging active");
    }

    // Create and initialize the application - let it handle all startup progress
    println!("🏗️ Creating application...");
    let mut app = App::new()?;
    println!("✅ Application created");
    
    // Pass the database from CLI to the App
    app.set_database(cli_handler.database());
    println!("📊 Database connected to application");
    
    // Set initial UI mode based on CLI arguments
    app.set_initial_mode(startup_mode);
    println!("🔧 Initial mode set");
    
    // Initialize background services (SMTP, contacts manager, etc.)
    println!("🔄 Initializing background services...");
    app.initialize_services().await?;
    println!("✅ Background services initialized");

    // Check if onboarding is needed before starting main application
    if comunicado::ui::onboarding::should_show_onboarding() {
        println!("👋 First time user detected - starting onboarding...");
        if !comunicado::ui::onboarding::maybe_run_onboarding().await? {
            println!("👋 Onboarding cancelled. Goodbye!");
            return Ok(());
        }
        println!("🎉 Onboarding completed successfully!");
    }

    // Run the application
    tracing::info!("Starting application main loop...");
    app.run().await?;

    Ok(())
}
