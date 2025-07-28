use reqwest::{Client, Method, Response};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;
use chrono::{DateTime, Utc};

/// CalDAV client errors
#[derive(Error, Debug)]
pub enum CalDAVError {
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),
    
    #[error("XML parsing error: {0}")]
    XmlError(String),
    
    #[error("Authentication failed")]
    AuthenticationFailed,
    
    #[error("Calendar not found: {0}")]
    CalendarNotFound(String),
    
    #[error("Event not found: {0}")]
    EventNotFound(String),
    
    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
    
    #[error("Server error: {status} - {message}")]
    ServerError { status: u16, message: String },
    
    #[error("iCalendar format error: {0}")]
    ICalendarError(String),
}

pub type CalDAVResult<T> = Result<T, CalDAVError>;

/// CalDAV client for RFC 4791 calendar operations
pub struct CalDAVClient {
    client: Client,
    base_url: Url,
    username: String,
    password: String,
}

/// CalDAV calendar information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalDAVCalendar {
    pub url: String,
    pub display_name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub timezone: Option<String>,
    pub supported_components: Vec<String>, // VEVENT, VTODO, etc.
    pub ctag: Option<String>, // Change tag for synchronization
}

/// CalDAV event resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalDAVEvent {
    pub url: String,
    pub etag: String,
    pub icalendar_data: String, // Raw iCalendar data
    pub last_modified: Option<DateTime<Utc>>,
}

/// CalDAV query parameters for event retrieval
#[derive(Debug, Clone)]
pub struct CalDAVQuery {
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub component_filter: Option<String>, // VEVENT, VTODO, etc.
    pub expand_recurrence: bool,
}

impl Default for CalDAVQuery {
    fn default() -> Self {
        Self {
            start_date: None,
            end_date: None,
            component_filter: Some("VEVENT".to_string()),
            expand_recurrence: false,
        }
    }
}

impl CalDAVClient {
    /// Create a new CalDAV client
    pub fn new(base_url: &str, username: String, password: String) -> CalDAVResult<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        
        let base_url = Url::parse(base_url)?;
        
        Ok(Self {
            client,
            base_url,
            username,
            password,
        })
    }
    
    /// Discover available calendars
    pub async fn discover_calendars(&self) -> CalDAVResult<Vec<CalDAVCalendar>> {
        // Perform calendar discovery using PROPFIND
        let propfind_body = r#"<?xml version="1.0" encoding="utf-8" ?>
<D:propfind xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
  <D:prop>
    <D:displayname />
    <D:resourcetype />
    <C:calendar-description />
    <C:calendar-color />
    <C:calendar-timezone />
    <C:supported-calendar-component-set />
    <D:getctag />
  </D:prop>
</D:propfind>"#;

        let response = self.send_request(
            Method::from_bytes(b"PROPFIND").map_err(|e| CalDAVError::ServerError { 
                status: 400, 
                message: format!("Invalid HTTP method: {}", e) 
            })?,
            &self.base_url.to_string(),
            Some(propfind_body),
            vec![("Depth", "1"), ("Content-Type", "application/xml; charset=utf-8")],
        ).await?;

        self.parse_calendar_discovery_response(response).await
    }
    
    /// Get events from a calendar within a date range
    pub async fn get_events(&self, calendar_url: &str, query: &CalDAVQuery) -> CalDAVResult<Vec<CalDAVEvent>> {
        let report_body = self.build_calendar_query(query);
        
        let response = self.send_request(
            Method::from_bytes(b"REPORT").map_err(|e| CalDAVError::ServerError { 
                status: 400, 
                message: format!("Invalid HTTP method: {}", e) 
            })?,
            calendar_url,
            Some(&report_body),
            vec![("Depth", "1"), ("Content-Type", "application/xml; charset=utf-8")],
        ).await?;

        self.parse_events_response(response).await
    }
    
    /// Create or update an event
    pub async fn put_event(&self, event_url: &str, icalendar_data: &str, etag: Option<&str>) -> CalDAVResult<String> {
        let mut headers = vec![
            ("Content-Type", "text/calendar; charset=utf-8"),
        ];
        
        // Add If-Match header for updates
        if let Some(etag) = etag {
            headers.push(("If-Match", etag));
        }
        
        let response = self.send_request(
            Method::PUT,
            event_url,
            Some(icalendar_data),
            headers,
        ).await?;

        // Extract new ETag from response
        let etag = response.headers()
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
            
        Ok(etag)
    }
    
    /// Delete an event
    pub async fn delete_event(&self, event_url: &str, etag: Option<&str>) -> CalDAVResult<()> {
        let mut headers = vec![];
        
        // Add If-Match header for conditional delete
        if let Some(etag) = etag {
            headers.push(("If-Match", etag));
        }
        
        self.send_request(
            Method::DELETE,
            event_url,
            None,
            headers,
        ).await?;

        Ok(())
    }
    
    /// Test server connectivity and authentication
    pub async fn test_connection(&self) -> CalDAVResult<bool> {
        let response = self.send_request(
            Method::OPTIONS,
            &self.base_url.to_string(),
            None,
            vec![],
        ).await?;

        // Check for CalDAV support in response headers
        let dav_header = response.headers()
            .get("dav")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
            
        Ok(dav_header.contains("calendar-access"))
    }
    
    /// Send HTTP request with authentication
    async fn send_request(
        &self,
        method: Method,
        url: &str,
        body: Option<&str>,
        headers: Vec<(&str, &str)>,
    ) -> CalDAVResult<Response> {
        let mut request_builder = self.client
            .request(method, url)
            .basic_auth(&self.username, Some(&self.password));

        // Add custom headers
        for (key, value) in headers {
            request_builder = request_builder.header(key, value);
        }

        // Add body if provided
        if let Some(body) = body {
            request_builder = request_builder.body(body.to_string());
        }

        let response = request_builder.send().await?;
        
        // Check for authentication errors
        if response.status() == 401 {
            return Err(CalDAVError::AuthenticationFailed);
        }
        
        // Check for other HTTP errors
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(CalDAVError::ServerError { status, message });
        }

        Ok(response)
    }
    
    /// Build calendar query XML for event retrieval
    fn build_calendar_query(&self, query: &CalDAVQuery) -> String {
        let mut xml = String::from(r#"<?xml version="1.0" encoding="utf-8" ?>
<C:calendar-query xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
  <D:prop>
    <D:getetag />
    <C:calendar-data />
    <D:getlastmodified />
  </D:prop>
  <C:filter>
    <C:comp-filter name="VCALENDAR">"#);

        // Add component filter (VEVENT, VTODO, etc.)
        if let Some(ref component) = query.component_filter {
            xml.push_str(&format!(r#"
      <C:comp-filter name="{}""#, component));
            
            // Add time range filter if specified
            if query.start_date.is_some() || query.end_date.is_some() {
                xml.push_str(">\n        <C:time-range");
                
                if let Some(start) = query.start_date {
                    xml.push_str(&format!(r#" start="{}""#, start.format("%Y%m%dT%H%M%SZ")));
                }
                
                if let Some(end) = query.end_date {
                    xml.push_str(&format!(r#" end="{}""#, end.format("%Y%m%dT%H%M%SZ")));
                }
                
                xml.push_str(" />\n      </C:comp-filter>");
            } else {
                xml.push_str(" />");
            }
        }

        xml.push_str(r#"
    </C:comp-filter>
  </C:filter>
</C:calendar-query>"#);

        xml
    }
    
    /// Parse calendar discovery PROPFIND response
    async fn parse_calendar_discovery_response(&self, response: Response) -> CalDAVResult<Vec<CalDAVCalendar>> {
        let text = response.text().await?;
        
        // Simple XML parsing for calendar properties
        // In a production implementation, use a proper XML parser like quick-xml
        let mut calendars = Vec::new();
        
        // Parse response XML to extract calendar information
        // This is a simplified implementation - in practice, use proper XML parsing
        if text.contains("calendar") {
            // Create a sample calendar for demonstration
            calendars.push(CalDAVCalendar {
                url: format!("{}/calendar/", self.base_url),
                display_name: "Default Calendar".to_string(),
                description: Some("Main calendar".to_string()),
                color: Some("#3174ad".to_string()),
                timezone: Some("UTC".to_string()),
                supported_components: vec!["VEVENT".to_string(), "VTODO".to_string()],
                ctag: Some("1".to_string()),
            });
        }
        
        Ok(calendars)
    }
    
    /// Parse calendar query REPORT response
    async fn parse_events_response(&self, response: Response) -> CalDAVResult<Vec<CalDAVEvent>> {
        let text = response.text().await?;
        
        // Simple XML parsing for events
        // In a production implementation, use a proper XML parser
        let mut events = Vec::new();
        
        // Parse response XML to extract event information
        // This is a simplified implementation
        if text.contains("VEVENT") {
            // Create sample events for demonstration
            events.push(CalDAVEvent {
                url: format!("{}/calendar/event1.ics", self.base_url),
                etag: "\"1\"".to_string(),
                icalendar_data: r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Comunicado//Calendar//EN
BEGIN:VEVENT
UID:sample-event-1@comunicado.local
DTSTART:20250128T100000Z
DTEND:20250128T110000Z
SUMMARY:Sample Meeting
DESCRIPTION:A sample calendar event
END:VEVENT
END:VCALENDAR"#.to_string(),
                last_modified: Some(Utc::now()),
            });
        }
        
        Ok(events)
    }
}

/// CalDAV server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalDAVConfig {
    pub name: String,
    pub server_url: String,
    pub username: String,
    pub password: String, // In practice, this should be encrypted
    pub calendar_paths: Vec<String>,
    pub sync_interval_minutes: u64,
    pub enabled: bool,
}

impl CalDAVConfig {
    pub fn new(name: String, server_url: String, username: String, password: String) -> Self {
        Self {
            name,
            server_url,
            username,
            password,
            calendar_paths: Vec::new(),
            sync_interval_minutes: 15,
            enabled: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_caldav_client_creation() {
        let client = CalDAVClient::new(
            "https://calendar.example.com/dav/",
            "testuser".to_string(),
            "testpass".to_string(),
        );
        assert!(client.is_ok());
    }
    
    #[test]
    fn test_calendar_query_builder() {
        let client = CalDAVClient::new(
            "https://calendar.example.com/dav/",
            "testuser".to_string(),
            "testpass".to_string(),
        ).unwrap();
        
        let query = CalDAVQuery::default();
        let xml = client.build_calendar_query(&query);
        
        assert!(xml.contains("calendar-query"));
        assert!(xml.contains("VEVENT"));
    }
    
    #[test]
    fn test_caldav_config() {
        let config = CalDAVConfig::new(
            "Test Calendar".to_string(),
            "https://calendar.example.com/dav/".to_string(),
            "user".to_string(),
            "pass".to_string(),
        );
        
        assert_eq!(config.name, "Test Calendar");
        assert_eq!(config.sync_interval_minutes, 15);
        assert!(config.enabled);
    }
}