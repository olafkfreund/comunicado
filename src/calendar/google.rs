use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::calendar::{CalendarError, CalendarResult, Event, EventPriority, EventStatus};
use crate::oauth2::TokenManager;

/// Google Calendar API client
pub struct GoogleCalendarClient {
    client: Client,
    token_manager: TokenManager,
}

/// Google Calendar API response structures
#[derive(Debug, Deserialize)]
pub struct GoogleCalendarList {
    pub items: Vec<GoogleCalendar>,
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GoogleCalendar {
    pub id: String,
    pub summary: String,
    pub description: Option<String>,
    #[serde(rename = "backgroundColor")]
    pub background_color: Option<String>,
    #[serde(rename = "accessRole")]
    pub access_role: String,
    pub selected: Option<bool>,
    pub primary: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct GoogleEventList {
    pub items: Vec<GoogleEvent>,
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
    #[serde(rename = "nextSyncToken")]
    pub next_sync_token: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GoogleEvent {
    pub id: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub start: GoogleEventDateTime,
    pub end: GoogleEventDateTime,
    pub status: Option<String>,
    pub location: Option<String>,
    pub creator: Option<GoogleEventCreator>,
    pub organizer: Option<GoogleEventOrganizer>,
    pub attendees: Option<Vec<GoogleEventAttendee>>,
    pub recurrence: Option<Vec<String>>,
    #[serde(rename = "htmlLink")]
    pub html_link: Option<String>,
    pub created: Option<DateTime<Utc>>,
    pub updated: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GoogleEventDateTime {
    #[serde(rename = "dateTime")]
    pub date_time: Option<DateTime<Utc>>,
    pub date: Option<String>, // For all-day events
    #[serde(rename = "timeZone")]
    pub time_zone: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GoogleEventCreator {
    pub email: String,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GoogleEventOrganizer {
    pub email: String,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GoogleEventAttendee {
    pub email: String,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    #[serde(rename = "responseStatus")]
    pub response_status: Option<String>, // "accepted", "declined", "tentative", "needsAction"
    pub organizer: Option<bool>,
}

impl GoogleCalendarClient {
    /// Create a new Google Calendar client
    pub fn new(token_manager: TokenManager) -> Self {
        Self {
            client: Client::new(),
            token_manager,
        }
    }

    /// Get access token for Google Calendar API
    async fn get_access_token(&self, account_id: &str) -> CalendarResult<String> {
        // TEMPORARY: Load token directly from file to bypass TokenManager cache issue
        let config_dir = dirs::config_dir()
            .ok_or_else(|| CalendarError::AuthError("Cannot find config directory".to_string()))?
            .join("comunicado");
        let token_file = config_dir.join(format!("{}.access.token", account_id));
        
        if token_file.exists() {
            let encoded_token = std::fs::read_to_string(&token_file)
                .map_err(|e| CalendarError::AuthError(format!("Failed to read token file: {}", e)))?;
            let encoded_token = encoded_token.trim();
            
            use base64::{Engine as _, engine::general_purpose};
            let decoded_token = general_purpose::STANDARD.decode(encoded_token)
                .map_err(|e| CalendarError::AuthError(format!("Failed to decode token: {}", e)))?;
            let token_str = String::from_utf8(decoded_token)
                .map_err(|e| CalendarError::AuthError(format!("Invalid token encoding: {}", e)))?;
                
            println!("🔍 DEBUG: Using file token (first 50 chars): {}", &token_str[..50.min(token_str.len())]);
            return Ok(token_str);
        }
        
        // Fallback to TokenManager (original code)
        let token = self
            .token_manager
            .get_valid_access_token(account_id)
            .await
            .map_err(|e| CalendarError::AuthError(format!("Failed to get access token: {}", e)))?;

        match token {
            Some(access_token) => {
                println!("🔍 DEBUG: Using TokenManager token (first 50 chars): {}", &access_token.token[..50.min(access_token.token.len())]);
                Ok(access_token.token.to_string())
            },
            None => Err(CalendarError::AuthError(
                "No access token available".to_string(),
            )),
        }
    }

    /// List all calendars for the user
    pub async fn list_calendars(&self, account_id: &str) -> CalendarResult<Vec<GoogleCalendar>> {
        let token = self.get_access_token(account_id).await?;
        
        // Debug: Print token info
        println!("🔍 DEBUG: Using token (first 50 chars): {}", &token[..50.min(token.len())]);

        let url = "https://www.googleapis.com/calendar/v3/users/me/calendarList";

        let response = self.client.get(url).bearer_auth(&token).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(CalendarError::AuthError(format!(
                "Google Calendar list calendars error {}: {}",
                status, error_text
            )));
        }

        let calendar_list: GoogleCalendarList = response.json().await?;
        Ok(calendar_list.items)
    }

    /// Get events from a specific calendar
    pub async fn list_events(
        &self,
        account_id: &str,
        calendar_id: &str,
        time_min: Option<DateTime<Utc>>,
        time_max: Option<DateTime<Utc>>,
        sync_token: Option<&str>,
    ) -> CalendarResult<GoogleEventList> {
        let token = self.get_access_token(account_id).await?;

        let mut url = format!(
            "https://www.googleapis.com/calendar/v3/calendars/{}/events",
            percent_encoding::utf8_percent_encode(calendar_id, percent_encoding::NON_ALPHANUMERIC)
        );

        let mut params = Vec::new();
        params.push(("singleEvents", "true".to_string()));
        params.push(("orderBy", "startTime".to_string()));

        if let Some(time_min) = time_min {
            params.push(("timeMin", time_min.to_rfc3339()));
        }

        if let Some(time_max) = time_max {
            params.push(("timeMax", time_max.to_rfc3339()));
        }

        if let Some(sync_token) = sync_token {
            params.push(("syncToken", sync_token.to_string()));
        }

        if !params.is_empty() {
            url.push('?');
            url.push_str(
                &params
                    .iter()
                    .map(|(k, v)| {
                        format!(
                            "{}={}",
                            k,
                            percent_encoding::utf8_percent_encode(
                                v,
                                percent_encoding::NON_ALPHANUMERIC
                            )
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("&"),
            );
        }

        tracing::debug!("Fetching Google Calendar events from: {}", url);

        let response = self.client.get(&url).bearer_auth(&token).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            tracing::error!("Google Calendar API error {}: {}", status, error_text);
            return Err(CalendarError::AuthError(format!(
                "Google Calendar list events error {}: {}",
                status, error_text
            )));
        }

        let event_list: GoogleEventList = response.json().await?;
        tracing::debug!(
            "Fetched {} events from Google Calendar",
            event_list.items.len()
        );

        Ok(event_list)
    }

    /// Create a new event in Google Calendar
    pub async fn create_event(
        &self,
        account_id: &str,
        calendar_id: &str,
        event: &GoogleEvent,
    ) -> CalendarResult<GoogleEvent> {
        let token = self.get_access_token(account_id).await?;

        let url = format!(
            "https://www.googleapis.com/calendar/v3/calendars/{}/events",
            percent_encoding::utf8_percent_encode(calendar_id, percent_encoding::NON_ALPHANUMERIC)
        );

        let response = self
            .client
            .post(&url)
            .bearer_auth(&token)
            .json(event)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            tracing::error!(
                "Google Calendar create event error {}: {}",
                status,
                error_text
            );
            return Err(CalendarError::AuthError(format!(
                "Google Calendar create event error {}: {}",
                status, error_text
            )));
        }

        let created_event: GoogleEvent = response.json().await?;
        tracing::debug!("Created Google Calendar event: {}", created_event.id);

        Ok(created_event)
    }

    /// Update an existing event in Google Calendar
    pub async fn update_event(
        &self,
        account_id: &str,
        calendar_id: &str,
        event_id: &str,
        event: &GoogleEvent,
    ) -> CalendarResult<GoogleEvent> {
        let token = self.get_access_token(account_id).await?;

        let url = format!(
            "https://www.googleapis.com/calendar/v3/calendars/{}/events/{}",
            percent_encoding::utf8_percent_encode(calendar_id, percent_encoding::NON_ALPHANUMERIC),
            percent_encoding::utf8_percent_encode(event_id, percent_encoding::NON_ALPHANUMERIC)
        );

        let response = self
            .client
            .put(&url)
            .bearer_auth(&token)
            .json(event)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            tracing::error!(
                "Google Calendar update event error {}: {}",
                status,
                error_text
            );
            return Err(CalendarError::AuthError(format!(
                "Google Calendar update event error {}: {}",
                status, error_text
            )));
        }

        let updated_event: GoogleEvent = response.json().await?;
        tracing::debug!("Updated Google Calendar event: {}", updated_event.id);

        Ok(updated_event)
    }

    /// Delete an event from Google Calendar
    pub async fn delete_event(
        &self,
        account_id: &str,
        calendar_id: &str,
        event_id: &str,
    ) -> CalendarResult<()> {
        let token = self.get_access_token(account_id).await?;

        let url = format!(
            "https://www.googleapis.com/calendar/v3/calendars/{}/events/{}",
            percent_encoding::utf8_percent_encode(calendar_id, percent_encoding::NON_ALPHANUMERIC),
            percent_encoding::utf8_percent_encode(event_id, percent_encoding::NON_ALPHANUMERIC)
        );

        let response = self.client.delete(&url).bearer_auth(&token).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            tracing::error!(
                "Google Calendar delete event error {}: {}",
                status,
                error_text
            );
            return Err(CalendarError::AuthError(format!(
                "Google Calendar delete event error {}: {}",
                status, error_text
            )));
        }

        tracing::debug!("Deleted Google Calendar event: {}", event_id);

        Ok(())
    }
}

/// Convert Google Event to our internal Event structure
impl From<GoogleEvent> for Event {
    fn from(google_event: GoogleEvent) -> Self {
        let start_time = google_event.start.date_time.unwrap_or_else(|| {
            // Handle all-day events
            if let Some(date_str) = &google_event.start.date {
                date_str
                    .parse::<DateTime<Utc>>()
                    .unwrap_or_else(|_| Utc::now())
            } else {
                Utc::now()
            }
        });

        let end_time = google_event.end.date_time.unwrap_or_else(|| {
            // Handle all-day events
            if let Some(date_str) = &google_event.end.date {
                date_str
                    .parse::<DateTime<Utc>>()
                    .unwrap_or_else(|_| start_time)
            } else {
                start_time
            }
        });

        let status = match google_event.status.as_deref() {
            Some("confirmed") => EventStatus::Confirmed,
            Some("tentative") => EventStatus::Tentative,
            Some("cancelled") => EventStatus::Cancelled,
            _ => EventStatus::Confirmed,
        };

        Event {
            id: google_event.id.clone(),
            uid: google_event.id,        // Use Google event ID as UID
            calendar_id: "".to_string(), // Will be set by caller
            title: google_event
                .summary
                .unwrap_or_else(|| "Untitled Event".to_string()),
            description: google_event.description,
            location: google_event.location,
            start_time,
            end_time,
            all_day: google_event.start.date.is_some(),
            status,
            priority: EventPriority::Normal, // Google Calendar doesn't have priority
            organizer: google_event.organizer.map(|organizer| {
                crate::calendar::event::EventAttendee::organizer(
                    organizer.email,
                    organizer.display_name,
                )
            }),
            attendees: google_event
                .attendees
                .unwrap_or_default()
                .into_iter()
                .map(|attendee| {
                    let mut event_attendee = crate::calendar::event::EventAttendee::new(
                        attendee.email,
                        attendee.display_name,
                    );
                    event_attendee.status = match attendee.response_status.as_deref() {
                        Some("accepted") => crate::calendar::event::AttendeeStatus::Accepted,
                        Some("declined") => crate::calendar::event::AttendeeStatus::Declined,
                        Some("tentative") => crate::calendar::event::AttendeeStatus::Tentative,
                        _ => crate::calendar::event::AttendeeStatus::NeedsAction,
                    };
                    if attendee.organizer.unwrap_or(false) {
                        event_attendee.role = crate::calendar::event::AttendeeRole::Chair;
                    }
                    event_attendee
                })
                .collect(),
            recurrence: None,       // TODO: Parse Google Calendar recurrence rules
            reminders: Vec::new(),  // TODO: Parse Google Calendar reminders
            categories: Vec::new(), // Google Calendar doesn't have categories
            url: google_event.html_link,
            created_at: google_event.created.unwrap_or_else(|| Utc::now()),
            updated_at: google_event.updated.unwrap_or_else(|| Utc::now()),
            sequence: 0, // Google Calendar doesn't expose sequence
            etag: None,  // Google Calendar uses different sync mechanism
        }
    }
}

/// Convert our internal Event to Google Event structure
impl From<&Event> for GoogleEvent {
    fn from(event: &Event) -> Self {
        let start = if event.all_day {
            GoogleEventDateTime {
                date_time: None,
                date: Some(event.start_time.format("%Y-%m-%d").to_string()),
                time_zone: None,
            }
        } else {
            GoogleEventDateTime {
                date_time: Some(event.start_time),
                date: None,
                time_zone: Some("UTC".to_string()),
            }
        };

        let end = if event.all_day {
            GoogleEventDateTime {
                date_time: None,
                date: Some(event.end_time.format("%Y-%m-%d").to_string()),
                time_zone: None,
            }
        } else {
            GoogleEventDateTime {
                date_time: Some(event.end_time),
                date: None,
                time_zone: Some("UTC".to_string()),
            }
        };

        let status = match event.status {
            EventStatus::Confirmed => Some("confirmed".to_string()),
            EventStatus::Tentative => Some("tentative".to_string()),
            EventStatus::Cancelled => Some("cancelled".to_string()),
        };

        let attendees = if event.attendees.is_empty() {
            None
        } else {
            Some(
                event
                    .attendees
                    .iter()
                    .map(|attendee| GoogleEventAttendee {
                        email: attendee.email.clone(),
                        display_name: attendee.name.clone(),
                        response_status: Some(match attendee.status {
                            crate::calendar::event::AttendeeStatus::Accepted => {
                                "accepted".to_string()
                            }
                            crate::calendar::event::AttendeeStatus::Declined => {
                                "declined".to_string()
                            }
                            crate::calendar::event::AttendeeStatus::Tentative => {
                                "tentative".to_string()
                            }
                            crate::calendar::event::AttendeeStatus::NeedsAction => {
                                "needsAction".to_string()
                            }
                            crate::calendar::event::AttendeeStatus::Delegated => {
                                "delegated".to_string()
                            }
                        }),
                        organizer: Some(
                            attendee.role == crate::calendar::event::AttendeeRole::Chair,
                        ),
                    })
                    .collect(),
            )
        };

        let organizer = event.organizer.as_ref().map(|org| GoogleEventOrganizer {
            email: org.email.clone(),
            display_name: org.name.clone(),
        });

        GoogleEvent {
            id: event.id.clone(),
            summary: Some(event.title.clone()),
            description: event.description.clone(),
            start,
            end,
            status,
            location: event.location.clone(),
            creator: None, // Will be set by Google
            organizer,
            attendees,
            recurrence: None, // TODO: Convert recurrence rules
            html_link: event.url.clone(),
            created: Some(event.created_at),
            updated: Some(event.updated_at),
        }
    }
}
