//! Services module for Comunicado
//!
//! This module contains various services that provide data and functionality
//! for the start page and other application features.

// Services temporarily disabled after start page removal
// TODO: Refactor or remove these services since start page is removed
// pub mod system_stats;
// pub mod tasks;
// pub mod weather;

// pub use system_stats::SystemStatsService;
// pub use tasks::TaskService;
// pub use weather::WeatherService;

/// Service manager that coordinates all services
/// Temporarily empty after start page removal
pub struct ServiceManager {
    // pub weather: WeatherService,
    // pub system_stats: SystemStatsService,
    // pub tasks: TaskService,
}

impl ServiceManager {
    /// Create a new service manager with all services initialized
    /// Temporarily disabled after start page removal
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            // weather: WeatherService::new(),
            // system_stats: SystemStatsService::new(),
            // tasks: TaskService::new()?,
        })
    }

    /// Create service manager with weather API key
    /// Temporarily disabled after start page removal
    pub fn with_weather_api_key(_api_key: String) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            // weather: WeatherService::with_api_key(api_key),
            // system_stats: SystemStatsService::new(),
            // tasks: TaskService::new()?,
        })
    }
}

impl Default for ServiceManager {
    fn default() -> Self {
        Self::new().unwrap_or_else(|e| {
            tracing::error!("Failed to initialize service manager: {}", e);
            Self {
                // weather: WeatherService::new(),
                // system_stats: SystemStatsService::new(),
                // tasks: TaskService::default(),
            }
        })
    }
}
