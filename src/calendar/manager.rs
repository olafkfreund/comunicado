use chrono::{DateTime, Datelike, Duration, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::calendar::database::CalendarDatabase;
use crate::calendar::event::{AttendeeStatus, Event, EventAttendee, EventStatus};
use crate::calendar::{
    CalDAVClient, CalDAVConfig, Calendar, CalendarError, CalendarResult, CalendarSource,
    CalendarStats, GoogleCalendarClient,
};
use crate::oauth2::token::TokenManager;

/// Calendar manager for coordinating all calendar operations
pub struct CalendarManager {
    database: Arc<CalendarDatabase>,
    #[allow(dead_code)]
    token_manager: Arc<TokenManager>,
    caldav_clients: RwLock<HashMap<String, Arc<CalDAVClient>>>,
    google_client: Arc<GoogleCalendarClient>,
    calendars: RwLock<HashMap<String, Calendar>>,
}

impl CalendarManager {
    /// Create a new calendar manager
    pub async fn new(
        database: Arc<CalendarDatabase>,
        token_manager: Arc<TokenManager>,
    ) -> CalendarResult<Self> {
        let google_client = Arc::new(GoogleCalendarClient::new((*token_manager).clone()));

        let manager = Self {
            database,
            token_manager,
            caldav_clients: RwLock::new(HashMap::new()),
            google_client,
            calendars: RwLock::new(HashMap::new()),
        };

        // Load existing calendars from database
        manager.load_calendars().await?;

        Ok(manager)
    }

    /// Load calendars from database
    async fn load_calendars(&self) -> CalendarResult<()> {
        let calendars = self
            .database
            .get_calendars()
            .await
            .map_err(|e| CalendarError::DatabaseError(e.to_string()))?;

        let mut calendars_map = self.calendars.write().await;
        calendars_map.clear();

        for calendar in calendars {
            calendars_map.insert(calendar.id.clone(), calendar);
        }

        Ok(())
    }

    /// Get all calendars
    pub async fn get_calendars(&self) -> Vec<Calendar> {
        let calendars = self.calendars.read().await;
        calendars.values().cloned().collect()
    }

    /// Get a calendar by ID
    pub async fn get_calendar(&self, calendar_id: &str) -> Option<Calendar> {
        let calendars = self.calendars.read().await;
        calendars.get(calendar_id).cloned()
    }

    /// Create a new local calendar
    pub async fn create_local_calendar(
        &self,
        name: String,
        description: Option<String>,
    ) -> CalendarResult<Calendar> {
        let mut calendar = Calendar::new(
            uuid::Uuid::new_v4().to_string(),
            name,
            CalendarSource::Local,
        );
        calendar.description = description;

        // Store in database
        self.database
            .store_calendar(&calendar)
            .await
            .map_err(|e| CalendarError::DatabaseError(e.to_string()))?;

        // Add to local cache
        let mut calendars = self.calendars.write().await;
        calendars.insert(calendar.id.clone(), calendar.clone());

        Ok(calendar)
    }

    /// Add a CalDAV calendar
    pub async fn add_caldav_calendar(&self, config: CalDAVConfig) -> CalendarResult<Vec<Calendar>> {
        // Create CalDAV client
        let client = Arc::new(CalDAVClient::new(
            &config.server_url,
            config.username.clone(),
            config.password.clone(),
        )?);

        // Test connection
        client.test_connection().await?;

        // Discover calendars
        let caldav_calendars = client.discover_calendars().await?;

        let mut created_calendars = Vec::new();

        for caldav_cal in caldav_calendars {
            let source = CalendarSource::CalDAV {
                account_id: config.name.clone(),
                calendar_url: caldav_cal.url.clone(),
            };

            let mut calendar = Calendar::new(
                uuid::Uuid::new_v4().to_string(),
                caldav_cal.display_name,
                source,
            );

            calendar.description = caldav_cal.description;
            calendar.color = caldav_cal.color;
            calendar.timezone = caldav_cal.timezone.unwrap_or_else(|| "UTC".to_string());

            // Store in database
            self.database
                .store_calendar(&calendar)
                .await
                .map_err(|e| CalendarError::DatabaseError(e.to_string()))?;

            // Add to local cache
            let mut calendars = self.calendars.write().await;
            calendars.insert(calendar.id.clone(), calendar.clone());

            created_calendars.push(calendar);
        }

        // Store CalDAV client for future sync operations
        let mut clients = self.caldav_clients.write().await;
        clients.insert(config.name.clone(), client);

        Ok(created_calendars)
    }

    /// Create a new event
    pub async fn create_event(&self, mut event: Event) -> CalendarResult<Event> {
        // Validate that the calendar exists
        if !self.calendars.read().await.contains_key(&event.calendar_id) {
            return Err(CalendarError::InvalidData(format!(
                "Calendar {} not found",
                event.calendar_id
            )));
        }

        // Generate new ID if not set
        if event.id.is_empty() {
            event.id = uuid::Uuid::new_v4().to_string();
        }

        // Set created/updated timestamps
        let now = Utc::now();
        event.created_at = now;
        event.updated_at = now;
        event.sequence = 0;

        // Store in database
        self.database
            .store_event(&event)
            .await
            .map_err(|e| CalendarError::DatabaseError(e.to_string()))?;

        Ok(event)
    }

    /// Update an existing event
    pub async fn update_event(&self, mut event: Event) -> CalendarResult<Event> {
        // Validate that the calendar exists
        if !self.calendars.read().await.contains_key(&event.calendar_id) {
            return Err(CalendarError::InvalidData(format!(
                "Calendar {} not found",
                event.calendar_id
            )));
        }

        // Update timestamp and sequence
        event.updated_at = Utc::now();
        event.sequence += 1;

        // Store in database
        self.database
            .store_event(&event)
            .await
            .map_err(|e| CalendarError::DatabaseError(e.to_string()))?;

        Ok(event)
    }

    /// Delete an event
    pub async fn delete_event(&self, event_id: &str) -> CalendarResult<bool> {
        self.database
            .delete_event(event_id)
            .await
            .map_err(|e| CalendarError::DatabaseError(e.to_string()))
    }

    /// Cancel an event by changing its status to Cancelled
    pub async fn cancel_event(&self, event_id: &str) -> CalendarResult<Event> {
        // Get the event first
        let event = self
            .database
            .get_event(event_id)
            .await
            .map_err(|e| CalendarError::DatabaseError(e.to_string()))?
            .ok_or_else(|| CalendarError::InvalidData(format!("Event {} not found", event_id)))?;

        // Update event status to Cancelled
        let mut cancelled_event = event;
        cancelled_event.status = EventStatus::Cancelled;
        cancelled_event.updated_at = Utc::now();
        cancelled_event.sequence += 1;

        // Store the updated event
        self.database
            .store_event(&cancelled_event)
            .await
            .map_err(|e| CalendarError::DatabaseError(e.to_string()))?;

        Ok(cancelled_event)
    }

    /// Get events from a calendar within a date range
    pub async fn get_events(
        &self,
        calendar_id: &str,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> CalendarResult<Vec<Event>> {
        self.database
            .get_events(calendar_id, start_time, end_time)
            .await
            .map_err(|e| CalendarError::DatabaseError(e.to_string()))
    }

    /// Get all events across all calendars within a date range
    pub async fn get_all_events(
        &self,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> CalendarResult<Vec<Event>> {
        let calendars = self.calendars.read().await;
        let mut all_events = Vec::new();

        for calendar_id in calendars.keys() {
            let events = self
                .database
                .get_events(calendar_id, start_time, end_time)
                .await
                .map_err(|e| CalendarError::DatabaseError(e.to_string()))?;

            all_events.extend(events);
        }

        // Sort by start time
        all_events.sort_by(|a, b| a.start_time.cmp(&b.start_time));

        Ok(all_events)
    }

    /// Get upcoming events (starting within the next N hours)
    pub async fn get_upcoming_events(
        &self,
        hours: u32,
        limit: Option<u32>,
    ) -> CalendarResult<Vec<Event>> {
        self.database
            .get_upcoming_events(hours, limit)
            .await
            .map_err(|e| CalendarError::DatabaseError(e.to_string()))
    }

    /// Search events across all calendars
    pub async fn search_events(
        &self,
        query: &str,
        limit: Option<u32>,
    ) -> CalendarResult<Vec<Event>> {
        self.database
            .search_events(query, limit)
            .await
            .map_err(|e| CalendarError::DatabaseError(e.to_string()))
    }

    /// Get events for today
    pub async fn get_todays_events(&self) -> CalendarResult<Vec<Event>> {
        let now = Utc::now();
        let start_of_day = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
        let end_of_day = start_of_day + Duration::days(1);

        self.get_all_events(Some(start_of_day), Some(end_of_day))
            .await
    }

    /// Get events for this week
    pub async fn get_this_weeks_events(&self) -> CalendarResult<Vec<Event>> {
        let now = Utc::now();
        let days_since_monday = now.weekday().num_days_from_monday();
        let start_of_week = (now - Duration::days(days_since_monday as i64))
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();
        let end_of_week = start_of_week + Duration::weeks(1);

        self.get_all_events(Some(start_of_week), Some(end_of_week))
            .await
    }

    /// RSVP to an event (update attendee status)
    pub async fn rsvp_to_event(
        &self,
        event_id: &str,
        attendee_email: &str,
        status: AttendeeStatus,
    ) -> CalendarResult<Event> {
        // Get the event
        let events = self
            .database
            .get_events("", None, None) // Get all events to find by ID
            .await
            .map_err(|e| CalendarError::DatabaseError(e.to_string()))?;

        let mut event = events
            .into_iter()
            .find(|e| e.id == event_id)
            .ok_or_else(|| CalendarError::InvalidData(format!("Event {} not found", event_id)))?;

        // Update attendee status
        if !event.update_attendee_status(attendee_email, status) {
            return Err(CalendarError::InvalidData(format!(
                "Attendee {} not found in event",
                attendee_email
            )));
        }

        // Save updated event
        self.update_event(event).await
    }

    /// Add attendee to an event
    pub async fn add_attendee_to_event(
        &self,
        event_id: &str,
        attendee: EventAttendee,
    ) -> CalendarResult<Event> {
        // Get the event
        let events = self
            .database
            .get_events("", None, None) // Get all events to find by ID
            .await
            .map_err(|e| CalendarError::DatabaseError(e.to_string()))?;

        let mut event = events
            .into_iter()
            .find(|e| e.id == event_id)
            .ok_or_else(|| CalendarError::InvalidData(format!("Event {} not found", event_id)))?;

        // Add attendee
        event.add_attendee(attendee);

        // Save updated event
        self.update_event(event).await
    }

    /// Get calendar statistics
    pub async fn get_stats(&self) -> CalendarResult<CalendarStats> {
        let calendars = self.calendars.read().await;

        let mut stats = CalendarStats::default();
        stats.total_calendars = calendars.len();

        // Count calendars by source
        for calendar in calendars.values() {
            match &calendar.source {
                CalendarSource::Google { .. } => stats.google_calendars += 1,
                CalendarSource::Outlook { .. } => stats.outlook_calendars += 1,
                CalendarSource::CalDAV { .. } => stats.caldav_calendars += 1,
                CalendarSource::Local => stats.local_calendars += 1,
            }

            // Update last sync time
            if let Some(last_synced) = calendar.last_synced {
                if stats.last_sync.is_none() || stats.last_sync.unwrap() < last_synced {
                    stats.last_sync = Some(last_synced);
                }
            }
        }

        // Get event counts - simplified query for all events
        // In practice, you'd want more efficient counting queries
        let all_events = self.get_all_events(None, None).await?;
        stats.total_events = all_events.len();

        let now = Utc::now();
        stats.upcoming_events = all_events.iter().filter(|e| e.start_time > now).count();

        stats.overdue_events = all_events
            .iter()
            .filter(|e| e.end_time < now && e.status != EventStatus::Cancelled)
            .count();

        Ok(stats)
    }

    /// Sync calendars with remote sources
    pub async fn sync_calendars(&self) -> CalendarResult<()> {
        let calendars = self.calendars.read().await.clone();

        for calendar in calendars.values() {
            match &calendar.source {
                CalendarSource::CalDAV {
                    account_id,
                    calendar_url,
                } => {
                    self.sync_caldav_calendar(account_id, calendar_url, &calendar.id)
                        .await?;
                }
                CalendarSource::Google {
                    account_id,
                    calendar_id,
                } => {
                    self.sync_google_calendar(account_id, calendar_id, &calendar.id)
                        .await?;
                }
                CalendarSource::Outlook {
                    account_id: _,
                    calendar_id,
                } => {
                    // TODO: Implement Outlook Calendar sync
                    tracing::info!(
                        "Outlook Calendar sync not yet implemented for {}",
                        calendar_id
                    );
                }
                CalendarSource::Local => {
                    // Local calendars don't need syncing
                }
            }
        }

        Ok(())
    }

    /// Sync a specific CalDAV calendar with bidirectional synchronization
    async fn sync_caldav_calendar(
        &self,
        account_id: &str,
        calendar_url: &str,
        local_calendar_id: &str,
    ) -> CalendarResult<()> {
        let clients = self.caldav_clients.read().await;

        if let Some(client) = clients.get(account_id) {
            tracing::debug!(
                "Starting CalDAV sync for account {} calendar {}",
                account_id,
                calendar_url
            );

            // Step 1: Get remote event list with ETags
            let remote_events = match client.get_event_list(calendar_url).await {
                Ok(events) => events,
                Err(e) => {
                    tracing::error!("Failed to get CalDAV event list: {}", e);
                    return Err(CalendarError::SyncError(format!(
                        "Failed to get remote events: {}",
                        e
                    )));
                }
            };

            // Step 2: Get local events for this calendar
            let local_events = self
                .database
                .get_events(local_calendar_id, None, None)
                .await
                .map_err(|e| CalendarError::DatabaseError(e.to_string()))?;

            let mut events_synced = 0;
            let mut events_updated = 0;
            let mut events_created = 0;

            // Step 3: Download new or modified events from server
            for (event_url, remote_etag) in &remote_events {
                // Check if we have this event locally and if it needs updating
                let needs_update = if let Some(local_event) = local_events
                    .iter()
                    .find(|e| e.url.as_ref().map(|url| url == event_url).unwrap_or(false))
                {
                    // Compare ETags to see if event was modified
                    local_event
                        .etag
                        .as_ref()
                        .map(|etag| etag != remote_etag)
                        .unwrap_or(true)
                } else {
                    // Event doesn't exist locally
                    true
                };

                if needs_update {
                    match client.get_event(event_url).await {
                        Ok(caldav_event) => {
                            // Parse iCalendar data into Event structure
                            match client.parse_icalendar_to_event(
                                &caldav_event.icalendar_data,
                                local_calendar_id.to_string(),
                            ) {
                                Ok(mut event) => {
                                    // Set CalDAV-specific metadata
                                    event.url = Some(event_url.clone());
                                    event.etag = Some(caldav_event.etag);

                                    // Check if this is an update or new event
                                    let is_update = local_events.iter().any(|e| e.uid == event.uid);

                                    // Store in database
                                    match self.database.store_event(&event).await {
                                        Ok(_) => {
                                            if is_update {
                                                events_updated += 1;
                                                tracing::debug!(
                                                    "Updated CalDAV event: {}",
                                                    event.title
                                                );
                                            } else {
                                                events_created += 1;
                                                tracing::debug!(
                                                    "Created CalDAV event: {}",
                                                    event.title
                                                );
                                            }
                                            events_synced += 1;
                                        }
                                        Err(e) => {
                                            tracing::error!(
                                                "Failed to store CalDAV event {}: {}",
                                                event.title,
                                                e
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::error!(
                                        "Failed to parse iCalendar data from {}: {}",
                                        event_url,
                                        e
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to get CalDAV event from {}: {}", event_url, e);
                        }
                    }
                }
            }

            // Step 4: Upload local changes to server (simplified for now)
            // In a full implementation, we would:
            // - Track local modifications since last sync
            // - Upload new local events to server
            // - Update modified local events on server
            // - Handle conflict resolution

            for local_event in &local_events {
                // Check if this is a local-only event that needs to be uploaded
                if local_event.url.is_none() && !local_event.uid.is_empty() {
                    // This is a local event that should be uploaded to the server
                    let event_url = format!(
                        "{}/{}.ics",
                        calendar_url.trim_end_matches('/'),
                        local_event.uid
                    );
                    let icalendar_data = local_event.to_icalendar();

                    match client.put_event(&event_url, &icalendar_data, None).await {
                        Ok(new_etag) => {
                            // Update local event with server URL and ETag
                            let mut updated_event = local_event.clone();
                            updated_event.url = Some(event_url);
                            updated_event.etag = Some(new_etag);

                            match self.database.store_event(&updated_event).await {
                                Ok(_) => {
                                    tracing::debug!(
                                        "Uploaded local event to CalDAV: {}",
                                        local_event.title
                                    );
                                }
                                Err(e) => {
                                    tracing::error!(
                                        "Failed to update local event after upload: {}",
                                        e
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to upload local event {}: {}",
                                local_event.title,
                                e
                            );
                        }
                    }
                }
            }

            // Step 5: Handle deletions (events that exist locally but not on server)
            let mut events_deleted = 0;
            for local_event in &local_events {
                if let Some(url) = &local_event.url {
                    if !remote_events.contains_key(url) {
                        // Event was deleted on server, remove locally
                        match self.database.delete_event(&local_event.id).await {
                            Ok(deleted) => {
                                if deleted {
                                    events_deleted += 1;
                                    tracing::debug!(
                                        "Deleted local event removed from server: {}",
                                        local_event.title
                                    );
                                }
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Failed to delete local event {}: {}",
                                    local_event.title,
                                    e
                                );
                            }
                        }
                    }
                }
            }

            tracing::info!(
                "CalDAV sync completed for {}: {} events synced ({} created, {} updated, {} deleted)", 
                calendar_url, events_synced, events_created, events_updated, events_deleted
            );
        } else {
            return Err(CalendarError::SyncError(format!(
                "CalDAV client not found for account: {}",
                account_id
            )));
        }

        Ok(())
    }

    /// Process calendar invite from email
    pub async fn process_email_invite(
        &self,
        email_content: &str,
        sender_email: &str,
    ) -> CalendarResult<Option<Event>> {
        // Simple iCalendar detection and parsing
        if !email_content.contains("BEGIN:VCALENDAR") || !email_content.contains("BEGIN:VEVENT") {
            return Ok(None);
        }

        // Extract basic event information (simplified parser)
        let mut title = "Meeting Invitation".to_string();
        let mut start_time = Utc::now() + Duration::hours(1);
        let mut end_time = start_time + Duration::hours(1);
        let mut description = None;
        let mut location = None;

        // Parse iCalendar content (very basic implementation)
        for line in email_content.lines() {
            if line.starts_with("SUMMARY:") {
                title = line.strip_prefix("SUMMARY:").unwrap_or(&title).to_string();
            } else if line.starts_with("DTSTART:") {
                if let Some(datetime_str) = line.strip_prefix("DTSTART:") {
                    if let Ok(dt) = DateTime::parse_from_str(datetime_str, "%Y%m%dT%H%M%SZ") {
                        start_time = dt.with_timezone(&Utc);
                    }
                }
            } else if line.starts_with("DTEND:") {
                if let Some(datetime_str) = line.strip_prefix("DTEND:") {
                    if let Ok(dt) = DateTime::parse_from_str(datetime_str, "%Y%m%dT%H%M%SZ") {
                        end_time = dt.with_timezone(&Utc);
                    }
                }
            } else if line.starts_with("DESCRIPTION:") {
                description = Some(line.strip_prefix("DESCRIPTION:").unwrap_or("").to_string());
            } else if line.starts_with("LOCATION:") {
                location = Some(line.strip_prefix("LOCATION:").unwrap_or("").to_string());
            }
        }

        // Create event with default calendar (first available)
        let calendars = self.calendars.read().await;
        if let Some(calendar) = calendars.values().next() {
            let mut event = Event::new(calendar.id.clone(), title, start_time, end_time);

            event.description = description;
            event.location = location;
            event.status = EventStatus::Tentative; // Meeting invites are tentative by default

            // Add organizer
            let organizer = EventAttendee::organizer(sender_email.to_string(), None);
            event.organizer = Some(organizer);

            Ok(Some(event))
        } else {
            Err(CalendarError::InvalidData(
                "No calendars available".to_string(),
            ))
        }
    }

    /// Sync a specific Google Calendar
    async fn sync_google_calendar(
        &self,
        account_id: &str,
        calendar_id: &str,
        local_calendar_id: &str,
    ) -> CalendarResult<()> {
        tracing::debug!(
            "Starting Google Calendar sync for account {} calendar {}",
            account_id,
            calendar_id
        );

        // Get current time for syncing recent events
        let now = Utc::now();
        let time_min = Some(now - Duration::days(30)); // Sync events from 30 days ago
        let time_max = Some(now + Duration::days(365)); // Sync events up to 1 year ahead

        // Fetch events from Google Calendar API
        let google_events_result = self
            .google_client
            .list_events(account_id, calendar_id, time_min, time_max, None)
            .await?;

        let events_count = google_events_result.items.len();
        tracing::info!(
            "Fetched {} events from Google Calendar {}",
            events_count,
            calendar_id
        );

        // Convert Google events to our internal Event structure and store them
        for google_event in google_events_result.items {
            // Convert Google event to our internal format
            let mut event: Event = google_event.into();
            event.calendar_id = local_calendar_id.to_string();

            // Check if event already exists in database by ID
            match self.database.get_event(&event.id).await {
                Ok(Some(existing_event)) => {
                    // Update existing event if it has been modified
                    if existing_event.updated_at < event.updated_at {
                        match self.database.store_event(&event).await {
                            Ok(_) => {
                                tracing::debug!(
                                    "Updated existing Google Calendar event: {}",
                                    event.id
                                );
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Failed to update Google Calendar event {}: {}",
                                    event.id,
                                    e
                                );
                            }
                        }
                    }
                }
                Ok(None) => {
                    // Create new event
                    match self.database.store_event(&event).await {
                        Ok(_) => {
                            tracing::debug!("Created new Google Calendar event: {}", event.id);
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to create Google Calendar event {}: {}",
                                event.id,
                                e
                            );
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to query existing Google Calendar event {}: {}",
                        event.id,
                        e
                    );
                }
            }
        }

        tracing::info!(
            "Google Calendar sync completed for {} - processed {} events",
            calendar_id,
            events_count
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::database::CalendarDatabase;
    use crate::oauth2::token::{TokenManager, TokenStats};

    #[tokio::test]
    async fn test_calendar_manager_creation() {
        let db = Arc::new(CalendarDatabase::new_in_memory().await.unwrap());
        let token_manager = Arc::new(TokenManager::new());

        let manager = CalendarManager::new(db, token_manager).await.unwrap();

        let calendars = manager.get_calendars().await;
        assert_eq!(calendars.len(), 0); // No calendars initially
    }

    #[tokio::test]
    async fn test_local_calendar_creation() {
        let db = Arc::new(CalendarDatabase::new_in_memory().await.unwrap());
        let token_manager = Arc::new(TokenManager::new());

        let manager = CalendarManager::new(db, token_manager).await.unwrap();

        let calendar = manager
            .create_local_calendar(
                "My Calendar".to_string(),
                Some("Personal calendar".to_string()),
            )
            .await
            .unwrap();

        assert_eq!(calendar.name, "My Calendar");
        assert_eq!(calendar.description, Some("Personal calendar".to_string()));
        assert!(matches!(calendar.source, CalendarSource::Local));

        let calendars = manager.get_calendars().await;
        assert_eq!(calendars.len(), 1);
    }

    #[tokio::test]
    async fn test_event_creation_and_retrieval() {
        let db = Arc::new(CalendarDatabase::new_in_memory().await.unwrap());
        let token_manager = Arc::new(TokenManager::new());

        let manager = CalendarManager::new(db, token_manager).await.unwrap();

        // Create a calendar first
        let calendar = manager
            .create_local_calendar("Test Calendar".to_string(), None)
            .await
            .unwrap();

        // Create an event
        let start = Utc::now() + Duration::hours(1);
        let end = start + Duration::hours(1);

        let event = Event::new(calendar.id.clone(), "Test Meeting".to_string(), start, end);

        let created_event = manager.create_event(event).await.unwrap();
        assert_eq!(created_event.title, "Test Meeting");
        assert_eq!(created_event.calendar_id, calendar.id);

        // Retrieve events
        let events = manager.get_events(&calendar.id, None, None).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].title, "Test Meeting");
    }

    #[tokio::test]
    async fn test_email_invite_processing() {
        let db = Arc::new(CalendarDatabase::new_in_memory().await.unwrap());
        let token_manager = Arc::new(TokenManager::new());

        let manager = CalendarManager::new(db, token_manager).await.unwrap();

        // Create a calendar first
        let _calendar = manager
            .create_local_calendar("Test Calendar".to_string(), None)
            .await
            .unwrap();

        let ical_content = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Test//Test//EN
BEGIN:VEVENT
UID:test-event@example.com
DTSTART:20250128T100000Z
DTEND:20250128T110000Z
SUMMARY:Team Meeting
DESCRIPTION:Weekly team standup
LOCATION:Conference Room A
END:VEVENT
END:VCALENDAR"#;

        let event = manager
            .process_email_invite(ical_content, "organizer@example.com")
            .await
            .unwrap();

        assert!(event.is_some());
        let event = event.unwrap();
        assert_eq!(event.title, "Team Meeting");
        assert_eq!(event.description, Some("Weekly team standup".to_string()));
        assert_eq!(event.location, Some("Conference Room A".to_string()));
        assert_eq!(event.status, EventStatus::Tentative);
        assert!(event.organizer.is_some());
        assert_eq!(event.organizer.unwrap().email, "organizer@example.com");
    }
}
