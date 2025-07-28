use std::time::Duration;
use chrono::{DateTime, Local, Timelike};
use serde::Deserialize;

use crate::ui::start_page::{WeatherInfo, WeatherForecast};

/// Weather service for fetching current weather and forecasts
pub struct WeatherService {
    api_key: Option<String>,
    last_update: Option<DateTime<Local>>,
    cached_weather: Option<WeatherInfo>,
    update_interval: Duration,
}

/// OpenWeatherMap API response structures
#[derive(Debug, Deserialize)]
struct OpenWeatherResponse {
    name: String,
    main: OpenWeatherMain,
    weather: Vec<OpenWeatherCondition>,
    wind: OpenWeatherWind,
}

#[derive(Debug, Deserialize)]
struct OpenWeatherMain {
    temp: f32,
    humidity: u32,
}

#[derive(Debug, Deserialize)]
struct OpenWeatherCondition {
    main: String,
    #[allow(dead_code)]
    description: String,
}

#[derive(Debug, Deserialize)]
struct OpenWeatherWind {
    speed: f32,
}

impl WeatherService {
    /// Create a new weather service
    pub fn new() -> Self {
        Self {
            api_key: None,
            last_update: None,
            cached_weather: None,
            update_interval: Duration::from_secs(30 * 60), // 30 minutes
        }
    }

    /// Create weather service with API key
    pub fn with_api_key(api_key: String) -> Self {
        Self {
            api_key: Some(api_key),
            last_update: None,
            cached_weather: None,
            update_interval: Duration::from_secs(30 * 60), // 30 minutes
        }
    }

    /// Set the API key for weather service
    pub fn set_api_key(&mut self, api_key: String) {
        self.api_key = Some(api_key);
    }

    /// Get current weather information
    pub async fn get_weather(&mut self, location: Option<&str>) -> Result<WeatherInfo, Box<dyn std::error::Error>> {
        // Check if we have cached data that's still fresh
        if let Some(ref cached) = self.cached_weather {
            if let Some(last_update) = self.last_update {
                if Local::now().signed_duration_since(last_update) < chrono::Duration::from_std(self.update_interval)? {
                    return Ok(cached.clone());
                }
            }
        }

        // Try to fetch from API, fall back to mock data
        match self.fetch_from_api(location).await {
            Ok(weather) => {
                self.cached_weather = Some(weather.clone());
                self.last_update = Some(Local::now());
                Ok(weather)
            }
            Err(e) => {
                tracing::warn!("Failed to fetch weather from API: {}, using mock data", e);
                Ok(self.get_mock_weather(location))
            }
        }
    }

    /// Fetch weather from OpenWeatherMap API
    async fn fetch_from_api(&self, location: Option<&str>) -> Result<WeatherInfo, Box<dyn std::error::Error>> {
        let api_key = self.api_key.as_ref()
            .ok_or("No API key configured for weather service")?;

        let location = location.unwrap_or("London");
        let url = format!(
            "https://api.openweathermap.org/data/2.5/weather?q={}&appid={}&units=metric",
            location, api_key
        );

        // Use a timeout for the request
        let client = reqwest::Client::new();
        let response = tokio::time::timeout(
            Duration::from_secs(10),
            client.get(&url).send()
        ).await??;

        if !response.status().is_success() {
            return Err(format!("Weather API returned status: {}", response.status()).into());
        }

        let weather_data: OpenWeatherResponse = response.json().await?;

        let forecast = self.get_mock_forecast(); // TODO: Implement real forecast API

        Ok(WeatherInfo {
            location: weather_data.name,
            temperature: weather_data.main.temp as i32,
            condition: weather_data.weather.first()
                .map(|w| w.main.clone())
                .unwrap_or_else(|| "Unknown".to_string()),
            humidity: weather_data.main.humidity,
            wind_speed: weather_data.wind.speed * 3.6, // Convert m/s to km/h
            forecast,
        })
    }

    /// Get mock weather data for testing/fallback
    fn get_mock_weather(&self, location: Option<&str>) -> WeatherInfo {
        let location = location.unwrap_or("Local Area").to_string();
        
        // Simulate realistic weather based on time of day
        let now = Local::now();
        let hour = now.hour();
        
        let (temp, condition) = match hour {
            6..=11 => (18, "Partly Cloudy"),
            12..=17 => (24, "Sunny"),
            18..=21 => (20, "Clear"),
            _ => (15, "Clear Night"),
        };

        WeatherInfo {
            location,
            temperature: temp,
            condition: condition.to_string(),
            humidity: 65,
            wind_speed: 12.5,
            forecast: self.get_mock_forecast(),
        }
    }

    /// Generate mock forecast data
    fn get_mock_forecast(&self) -> Vec<WeatherForecast> {
        vec![
            WeatherForecast {
                day: "Today".to_string(),
                high: 24,
                low: 18,
                condition: "Sunny".to_string(),
            },
            WeatherForecast {
                day: "Tomorrow".to_string(),
                high: 22,
                low: 16,
                condition: "Cloudy".to_string(),
            },
            WeatherForecast {
                day: "Tuesday".to_string(),
                high: 19,
                low: 13,
                condition: "Rainy".to_string(),
            },
            WeatherForecast {
                day: "Wednesday".to_string(),
                high: 21,
                low: 15,
                condition: "Partly Cloudy".to_string(),
            },
            WeatherForecast {
                day: "Thursday".to_string(),
                high: 25,
                low: 19,
                condition: "Sunny".to_string(),
            },
        ]
    }

    /// Force refresh weather data
    pub async fn refresh(&mut self, location: Option<&str>) -> Result<WeatherInfo, Box<dyn std::error::Error>> {
        self.last_update = None; // Force refresh
        self.get_weather(location).await
    }

    /// Get location from IP geolocation (fallback method)
    pub async fn detect_location(&self) -> Result<String, Box<dyn std::error::Error>> {
        // Try to get location from IP geolocation service
        let client = reqwest::Client::new();
        
        match tokio::time::timeout(
            Duration::from_secs(5),
            client.get("http://ip-api.com/json/").send()
        ).await {
            Ok(Ok(response)) => {
                if response.status().is_success() {
                    let data: serde_json::Value = response.json().await?;
                    if let Some(city) = data.get("city").and_then(|c| c.as_str()) {
                        return Ok(city.to_string());
                    }
                }
            }
            _ => {}
        }

        // Fallback to system timezone or default
        Ok("Local Area".to_string())
    }
}

impl Default for WeatherService {
    fn default() -> Self {
        Self::new()
    }
}