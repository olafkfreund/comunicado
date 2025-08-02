//! AI-powered meeting scheduling from email content
//! 
//! This module provides intelligent parsing of meeting requests from emails
//! and automatic calendar event creation with user confirmation.

use crate::ai::{AIResult, EnhancedAIService, EnhancedAIRequest, AIOperationType};
use crate::calendar::manager::CalendarManager;
use crate::calendar::event::{Event, EventAttendee, EventStatus, EventPriority, EventRecurrence, RecurrenceFrequency as CalendarRecurrenceFrequency, RecurrenceDay, AttendeeStatus as CalendarAttendeeStatus, AttendeeRole as CalendarAttendeeRole};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

/// Configuration for AI meeting scheduling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingSchedulerConfig {
    /// Enable AI meeting scheduling
    pub enabled: bool,
    /// Auto-create meetings without confirmation (for trusted senders)
    pub auto_create_enabled: bool,
    /// List of trusted email domains for auto-creation
    pub trusted_domains: Vec<String>,
    /// Default meeting duration in minutes
    pub default_duration_minutes: u32,
    /// Buffer time before meetings in minutes
    pub buffer_time_minutes: u32,
    /// Maximum look-ahead days for scheduling
    pub max_lookahead_days: u32,
    /// Enable smart location detection
    pub enable_location_detection: bool,
    /// Enable attendee extraction from email
    pub enable_attendee_extraction: bool,
    /// Meeting confirmation timeout (seconds)
    pub confirmation_timeout_seconds: u64,
}

impl Default for MeetingSchedulerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_create_enabled: false,
            trusted_domains: vec!["company.com".to_string()],
            default_duration_minutes: 60,
            buffer_time_minutes: 15,
            max_lookahead_days: 30,
            enable_location_detection: true,
            enable_attendee_extraction: true,
            confirmation_timeout_seconds: 300, // 5 minutes
        }
    }
}

/// Meeting request extracted from email content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingRequest {
    /// Unique request ID
    pub id: Uuid,
    /// Source email ID
    pub email_id: String,
    /// Meeting title/subject
    pub title: String,
    /// Meeting description
    pub description: Option<String>,
    /// Proposed date and time
    pub proposed_datetime: Option<chrono::DateTime<chrono::Utc>>,
    /// Alternative time options
    pub alternative_times: Vec<chrono::DateTime<chrono::Utc>>,
    /// Duration in minutes
    pub duration_minutes: Option<u32>,
    /// Meeting location
    pub location: Option<MeetingLocation>,
    /// Meeting attendees
    pub attendees: Vec<MeetingAttendee>,
    /// Meeting type
    pub meeting_type: MeetingType,
    /// Priority level
    pub priority: MeetingPriority,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Extracted agenda items
    pub agenda_items: Vec<String>,
    /// Meeting organizer
    pub organizer: MeetingAttendee,
    /// Recurrence pattern if any
    pub recurrence: Option<RecurrencePattern>,
    /// Meeting link (for virtual meetings)
    pub meeting_link: Option<String>,
    /// Timezone information
    pub timezone: Option<String>,
}

/// Meeting location information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingLocation {
    /// Location type
    pub location_type: LocationType,
    /// Location name or address
    pub name: String,
    /// Additional location details
    pub details: Option<String>,
    /// Coordinates if available
    pub coordinates: Option<(f64, f64)>,
}

/// Types of meeting locations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LocationType {
    /// Physical office location
    Office,
    /// Conference room
    ConferenceRoom,
    /// Virtual meeting (online)
    Virtual,
    /// External location
    External,
    /// Phone call
    Phone,
    /// To be determined
    TBD,
}

/// Meeting attendee information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingAttendee {
    /// Email address
    pub email: String,
    /// Display name
    pub name: Option<String>,
    /// Attendance status
    pub status: AttendeeStatus,
    /// Whether attendance is required
    pub required: bool,
    /// Role in the meeting
    pub role: AttendeeRole,
}

/// Attendee status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AttendeeStatus {
    /// Needs response
    NeedsAction,
    /// Accepted invitation
    Accepted,
    /// Declined invitation
    Declined,
    /// Tentatively accepted
    Tentative,
}

/// Attendee role
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AttendeeRole {
    /// Meeting organizer
    Organizer,
    /// Required attendee
    Required,
    /// Optional attendee
    Optional,
    /// Resource (room, equipment)
    Resource,
}

/// Meeting type classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MeetingType {
    /// One-on-one meeting
    OneOnOne,
    /// Team meeting
    Team,
    /// Project meeting
    Project,
    /// Client meeting
    Client,
    /// Interview
    Interview,
    /// Training or workshop
    Training,
    /// Presentation or demo
    Presentation,
    /// Social or informal
    Social,
    /// Other/unclassified
    Other,
}

/// Meeting priority
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum MeetingPriority {
    /// Low priority
    Low = 1,
    /// Normal priority
    Normal = 2,
    /// High priority
    High = 3,
    /// Urgent priority
    Urgent = 4,
}

/// Recurrence pattern for recurring meetings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecurrencePattern {
    /// Recurrence frequency
    pub frequency: RecurrenceFrequency,
    /// Interval between recurrences
    pub interval: u32,
    /// Days of week (for weekly recurrence)
    pub days_of_week: Vec<chrono::Weekday>,
    /// End date for recurrence
    pub end_date: Option<chrono::DateTime<chrono::Utc>>,
    /// Number of occurrences
    pub count: Option<u32>,
}

/// Recurrence frequency
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecurrenceFrequency {
    /// Daily recurrence
    Daily,
    /// Weekly recurrence
    Weekly,
    /// Monthly recurrence
    Monthly,
    /// Yearly recurrence
    Yearly,
}

/// Meeting creation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingCreationResult {
    /// Whether meeting was created successfully
    pub success: bool,
    /// Created calendar event ID
    pub event_id: Option<String>,
    /// Error message if creation failed
    pub error: Option<String>,
    /// Conflicts detected
    pub conflicts: Vec<ConflictInfo>,
    /// Alternative time suggestions
    pub alternative_suggestions: Vec<chrono::DateTime<chrono::Utc>>,
}

/// Conflict information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictInfo {
    /// Conflicting event ID
    pub event_id: String,
    /// Conflicting event title
    pub title: String,
    /// Conflict start time
    pub start_time: chrono::DateTime<chrono::Utc>,
    /// Conflict end time
    pub end_time: chrono::DateTime<chrono::Utc>,
    /// Severity of conflict
    pub severity: ConflictSeverity,
}

/// Conflict severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConflictSeverity {
    /// Minor overlap (can be adjusted)
    Minor,
    /// Moderate conflict (requires attention)
    Moderate,
    /// Major conflict (needs resolution)
    Major,
    /// Critical conflict (blocking)
    Critical,
}

/// Meeting scheduler statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MeetingSchedulerStats {
    /// Total meeting requests processed
    pub total_requests: usize,
    /// Successfully parsed requests
    pub successful_parsings: usize,
    /// Meetings created automatically
    pub auto_created_meetings: usize,
    /// Meetings created with confirmation
    pub confirmed_meetings: usize,
    /// Failed meeting creations
    pub failed_creations: usize,
    /// Average parsing confidence
    pub avg_confidence: f32,
    /// Requests by meeting type
    pub requests_by_type: HashMap<String, usize>,
    /// Parsing accuracy rate
    pub parsing_accuracy_rate: f32,
    /// Conflict resolution rate
    pub conflict_resolution_rate: f32,
}

/// AI-powered meeting scheduler service
pub struct MeetingSchedulerService {
    /// Enhanced AI service for parsing
    ai_service: Arc<EnhancedAIService>,
    /// Calendar manager for event creation
    calendar_manager: Arc<CalendarManager>,
    /// Configuration
    config: Arc<RwLock<MeetingSchedulerConfig>>,
    /// Pending confirmations
    pending_confirmations: Arc<RwLock<HashMap<Uuid, MeetingRequest>>>,
    /// Service statistics
    stats: Arc<RwLock<MeetingSchedulerStats>>,
}

impl MeetingSchedulerService {
    /// Create a new meeting scheduler service
    pub fn new(
        ai_service: Arc<EnhancedAIService>,
        calendar_manager: Arc<CalendarManager>,
        config: MeetingSchedulerConfig,
    ) -> Self {
        Self {
            ai_service,
            calendar_manager,
            config: Arc::new(RwLock::new(config)),
            pending_confirmations: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(MeetingSchedulerStats::default())),
        }
    }

    /// Parse meeting request from email content
    pub async fn parse_meeting_request(
        &self,
        email_id: String,
        email_content: &str,
        sender_email: &str,
        email_subject: &str,
    ) -> AIResult<Option<MeetingRequest>> {
        let config = self.config.read().await;
        if !config.enabled {
            return Ok(None);
        }

        let parsing_prompt = self.build_parsing_prompt(
            email_content,
            sender_email,
            email_subject,
        );

        let ai_request = EnhancedAIRequest::high_priority(AIOperationType::Custom {
            operation_name: "meeting_parsing".to_string(),
            prompt: parsing_prompt,
            context: Some(email_content.to_string()),
        });

        drop(config);

        let response = self.ai_service.process_request(ai_request).await?;
        
        // Parse the AI response into a meeting request
        let meeting_request = self.parse_ai_response(
            &response.content,
            email_id,
            sender_email,
        ).await?;

        // Update statistics
        self.update_parsing_stats(&meeting_request).await;

        if let Some(ref request) = meeting_request {
            info!("Parsed meeting request '{}' with confidence {:.2}", 
                  request.title, request.confidence);
        }

        Ok(meeting_request)
    }

    /// Create calendar event from meeting request
    pub async fn create_meeting(
        &self,
        meeting_request: &MeetingRequest,
        auto_confirm: bool,
    ) -> AIResult<MeetingCreationResult> {
        // Check for conflicts first
        let conflicts = self.check_conflicts(meeting_request).await?;
        
        if !conflicts.is_empty() && !auto_confirm {
            return Ok(MeetingCreationResult {
                success: false,
                event_id: None,
                error: Some("Conflicts detected".to_string()),
                conflicts,
                alternative_suggestions: self.suggest_alternative_times(meeting_request).await?,
            });
        }

        // Create calendar event
        let calendar_event = self.convert_to_calendar_event(meeting_request).await?;
        
        match self.calendar_manager.create_event(calendar_event).await {
            Ok(event) => {
                let mut stats = self.stats.write().await;
                if auto_confirm {
                    stats.auto_created_meetings += 1;
                } else {
                    stats.confirmed_meetings += 1;
                }

                info!("Created meeting '{}' with ID {}", meeting_request.title, event.id);

                Ok(MeetingCreationResult {
                    success: true,
                    event_id: Some(event.id),
                    error: None,
                    conflicts,
                    alternative_suggestions: vec![],
                })
            }
            Err(error) => {
                let mut stats = self.stats.write().await;
                stats.failed_creations += 1;

                warn!("Failed to create meeting '{}': {}", meeting_request.title, error);

                Ok(MeetingCreationResult {
                    success: false,
                    event_id: None,
                    error: Some(error.to_string()),
                    conflicts,
                    alternative_suggestions: self.suggest_alternative_times(meeting_request).await?,
                })
            }
        }
    }

    /// Process email for meeting scheduling
    pub async fn process_email_for_meetings(
        &self,
        email_id: String,
        email_content: &str,
        sender_email: &str,
        email_subject: &str,
    ) -> AIResult<Option<MeetingCreationResult>> {
        // Parse meeting request
        let meeting_request = match self.parse_meeting_request(
            email_id,
            email_content,
            sender_email,
            email_subject,
        ).await? {
            Some(request) => request,
            None => return Ok(None),
        };

        // Check if sender is trusted for auto-creation
        let config = self.config.read().await;
        let auto_create = config.auto_create_enabled && 
            self.is_trusted_sender(sender_email, &config);
        drop(config);

        if auto_create && meeting_request.confidence > 0.8 {
            // Auto-create meeting for trusted senders with high confidence
            Ok(Some(self.create_meeting(&meeting_request, true).await?))
        } else {
            // Store for user confirmation
            let request_id = meeting_request.id;
            let mut pending = self.pending_confirmations.write().await;
            pending.insert(request_id, meeting_request);
            
            // Return result indicating confirmation needed
            Ok(Some(MeetingCreationResult {
                success: false,
                event_id: None,
                error: Some("User confirmation required".to_string()),
                conflicts: vec![],
                alternative_suggestions: vec![],
            }))
        }
    }

    /// Confirm pending meeting creation
    pub async fn confirm_meeting(&self, request_id: Uuid) -> AIResult<MeetingCreationResult> {
        let meeting_request = {
            let mut pending = self.pending_confirmations.write().await;
            pending.remove(&request_id)
        };

        match meeting_request {
            Some(request) => self.create_meeting(&request, false).await,
            None => Ok(MeetingCreationResult {
                success: false,
                event_id: None,
                error: Some("Meeting request not found or expired".to_string()),
                conflicts: vec![],
                alternative_suggestions: vec![],
            }),
        }
    }

    /// Get pending meeting confirmations
    pub async fn get_pending_confirmations(&self) -> Vec<MeetingRequest> {
        let pending = self.pending_confirmations.read().await;
        pending.values().cloned().collect()
    }

    /// Cancel pending meeting request
    pub async fn cancel_pending_meeting(&self, request_id: Uuid) -> bool {
        let mut pending = self.pending_confirmations.write().await;
        pending.remove(&request_id).is_some()
    }

    /// Build AI parsing prompt
    fn build_parsing_prompt(
        &self,
        email_content: &str,
        sender_email: &str,
        email_subject: &str,
    ) -> String {
        format!(
            r#"Analyze this email for meeting scheduling information. Extract meeting details if present.

Email Subject: {}
Sender: {}
Email Content:
{}

Please extract the following information if available:
1. Meeting title/subject
2. Proposed date and time (be specific about timezone if mentioned)
3. Duration or end time
4. Location (virtual, physical address, room name)
5. Attendees mentioned
6. Meeting agenda or purpose
7. Alternative time suggestions
8. Recurrence pattern (daily, weekly, monthly)
9. Priority level indicators
10. Meeting type (1:1, team, client, etc.)

Respond in JSON format with the following structure:
{{
    "has_meeting_request": boolean,
    "confidence": float (0.0-1.0),
    "title": "string",
    "description": "string or null",
    "proposed_datetime": "ISO datetime or null",
    "alternative_times": ["ISO datetime array"],
    "duration_minutes": number or null,
    "location": {{
        "type": "office|conference_room|virtual|external|phone|tbd",
        "name": "string",
        "details": "string or null"
    }} or null,
    "attendees": [{{
        "email": "string",
        "name": "string or null",
        "required": boolean
    }}],
    "meeting_type": "one_on_one|team|project|client|interview|training|presentation|social|other",
    "priority": "low|normal|high|urgent",
    "agenda_items": ["string array"],
    "recurrence": {{
        "frequency": "daily|weekly|monthly|yearly",
        "interval": number,
        "days_of_week": ["monday", "tuesday", etc] or null,
        "end_date": "ISO datetime or null"
    }} or null,
    "meeting_link": "string or null",
    "timezone": "string or null"
}}

If no meeting request is detected, set has_meeting_request to false and confidence to 0.0."#,
            email_subject, sender_email, email_content
        )
    }

    /// Parse AI response into meeting request
    async fn parse_ai_response(
        &self,
        ai_response: &str,
        email_id: String,
        sender_email: &str,
    ) -> AIResult<Option<MeetingRequest>> {
        // Parse JSON response from AI
        let parsed: serde_json::Value = serde_json::from_str(ai_response)
            .map_err(|e| crate::ai::AIError::internal_error(format!("Failed to parse AI response: {}", e)))?;

        let has_meeting = parsed["has_meeting_request"].as_bool().unwrap_or(false);
        if !has_meeting {
            return Ok(None);
        }

        let confidence = parsed["confidence"].as_f64().unwrap_or(0.0) as f32;
        if confidence < 0.3 {
            // Too low confidence to consider as meeting request
            return Ok(None);
        }

        // Extract meeting details
        let title = parsed["title"].as_str().unwrap_or("Meeting").to_string();
        let description = parsed["description"].as_str().map(|s| s.to_string());

        // Parse datetime
        let proposed_datetime = parsed["proposed_datetime"].as_str()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc));

        // Parse alternative times
        let alternative_times = parsed["alternative_times"].as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .filter_map(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .collect()
            })
            .unwrap_or_default();

        let duration_minutes = parsed["duration_minutes"].as_u64().map(|d| d as u32);

        // Parse location
        let location = parsed["location"].as_object().map(|loc| {
            let location_type = match loc["type"].as_str().unwrap_or("tbd") {
                "office" => LocationType::Office,
                "conference_room" => LocationType::ConferenceRoom,
                "virtual" => LocationType::Virtual,
                "external" => LocationType::External,
                "phone" => LocationType::Phone,
                _ => LocationType::TBD,
            };

            MeetingLocation {
                location_type,
                name: loc["name"].as_str().unwrap_or("").to_string(),
                details: loc["details"].as_str().map(|s| s.to_string()),
                coordinates: None,
            }
        });

        // Parse attendees
        let mut attendees: Vec<MeetingAttendee> = parsed["attendees"].as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_object())
                    .map(|att| MeetingAttendee {
                        email: att["email"].as_str().unwrap_or("").to_string(),
                        name: att["name"].as_str().map(|s| s.to_string()),
                        status: AttendeeStatus::NeedsAction,
                        required: att["required"].as_bool().unwrap_or(true),
                        role: AttendeeRole::Required,
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Add organizer
        let organizer = MeetingAttendee {
            email: sender_email.to_string(),
            name: None,
            status: AttendeeStatus::Accepted,
            required: true,
            role: AttendeeRole::Organizer,
        };

        attendees.push(organizer.clone());

        // Parse meeting type
        let meeting_type = match parsed["meeting_type"].as_str().unwrap_or("other") {
            "one_on_one" => MeetingType::OneOnOne,
            "team" => MeetingType::Team,
            "project" => MeetingType::Project,
            "client" => MeetingType::Client,
            "interview" => MeetingType::Interview,
            "training" => MeetingType::Training,
            "presentation" => MeetingType::Presentation,
            "social" => MeetingType::Social,
            _ => MeetingType::Other,
        };

        // Parse priority
        let priority = match parsed["priority"].as_str().unwrap_or("normal") {
            "low" => MeetingPriority::Low,
            "high" => MeetingPriority::High,
            "urgent" => MeetingPriority::Urgent,
            _ => MeetingPriority::Normal,
        };

        // Parse agenda items
        let agenda_items = parsed["agenda_items"].as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        // Parse recurrence
        let recurrence = parsed["recurrence"].as_object().map(|rec| {
            let frequency = match rec["frequency"].as_str().unwrap_or("weekly") {
                "daily" => RecurrenceFrequency::Daily,
                "monthly" => RecurrenceFrequency::Monthly,
                "yearly" => RecurrenceFrequency::Yearly,
                _ => RecurrenceFrequency::Weekly,
            };

            RecurrencePattern {
                frequency,
                interval: rec["interval"].as_u64().unwrap_or(1) as u32,
                days_of_week: vec![], // Would need more complex parsing
                end_date: rec["end_date"].as_str()
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc)),
                count: None,
            }
        });

        let meeting_link = parsed["meeting_link"].as_str().map(|s| s.to_string());
        let timezone = parsed["timezone"].as_str().map(|s| s.to_string());

        Ok(Some(MeetingRequest {
            id: Uuid::new_v4(),
            email_id,
            title,
            description,
            proposed_datetime,
            alternative_times,
            duration_minutes,
            location,
            attendees,
            meeting_type,
            priority,
            confidence,
            agenda_items,
            organizer,
            recurrence,
            meeting_link,
            timezone,
        }))
    }

    /// Check for scheduling conflicts
    async fn check_conflicts(&self, _meeting_request: &MeetingRequest) -> AIResult<Vec<ConflictInfo>> {
        // This would check against existing calendar events
        // For now, return empty conflicts
        Ok(vec![])
    }

    /// Suggest alternative meeting times
    async fn suggest_alternative_times(
        &self,
        _meeting_request: &MeetingRequest,
    ) -> AIResult<Vec<chrono::DateTime<chrono::Utc>>> {
        // This would analyze calendar availability and suggest alternatives
        // For now, return empty suggestions
        Ok(vec![])
    }

    /// Convert meeting request to calendar event
    async fn convert_to_calendar_event(
        &self,
        meeting_request: &MeetingRequest,
    ) -> AIResult<Event> {
        let config = self.config.read().await;
        
        let start_time = meeting_request.proposed_datetime
            .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::hours(1));

        let duration_minutes = meeting_request.duration_minutes
            .unwrap_or(config.default_duration_minutes);

        let end_time = start_time + chrono::Duration::minutes(duration_minutes as i64);

        let mut description = meeting_request.description.clone().unwrap_or_default();
        
        // Add agenda items to description
        if !meeting_request.agenda_items.is_empty() {
            description.push_str("\n\nAgenda:\n");
            for (i, item) in meeting_request.agenda_items.iter().enumerate() {
                description.push_str(&format!("{}. {}\n", i + 1, item));
            }
        }

        // Add meeting link if available
        if let Some(ref link) = meeting_request.meeting_link {
            description.push_str(&format!("\nMeeting Link: {}\n", link));
        }

        let location = meeting_request.location.as_ref()
            .map(|loc| loc.name.clone());

        // Convert recurrence pattern if present (simplified for now)
        let recurrence = meeting_request.recurrence.as_ref().map(|_rec| {
            // For now, create a simple weekly recurrence
            // This would need proper implementation based on the actual EventRecurrence structure
            EventRecurrence {
                frequency: CalendarRecurrenceFrequency::Weekly,
                interval: 1,
                count: None,
                until: None,
                by_day: vec![],
                by_month_day: vec![],
                by_month: vec![],
                by_week_no: vec![],
                by_year_day: vec![],
                week_start: RecurrenceDay::Monday,
            }
        });

        // Create the event using the proper constructor
        let mut event = Event::new(
            "default".to_string(), // calendar_id
            meeting_request.title.clone(),
            start_time,
            end_time,
        );

        // Set additional properties
        event.description = Some(description);
        event.location = location;
        event.recurrence = recurrence;
        event.status = EventStatus::Confirmed;
        event.priority = match meeting_request.priority {
            MeetingPriority::Low => EventPriority::Low,
            MeetingPriority::Normal => EventPriority::Normal, 
            MeetingPriority::High => EventPriority::High,
            MeetingPriority::Urgent => EventPriority::High, // Map urgent to high
        };

        // Convert attendees
        event.attendees = meeting_request.attendees.iter()
            .map(|att| {
                let mut attendee = EventAttendee::new(att.email.clone(), att.name.clone());
                attendee.status = CalendarAttendeeStatus::NeedsAction;
                attendee.role = CalendarAttendeeRole::RequiredParticipant;
                attendee.rsvp = att.required;
                attendee
            })
            .collect();

        Ok(event)
    }

    /// Check if sender is trusted for auto-creation
    fn is_trusted_sender(&self, sender_email: &str, config: &MeetingSchedulerConfig) -> bool {
        config.trusted_domains.iter().any(|domain| {
            sender_email.ends_with(&format!("@{}", domain))
        })
    }

    /// Update parsing statistics
    async fn update_parsing_stats(&self, meeting_request: &Option<MeetingRequest>) {
        let mut stats = self.stats.write().await;
        stats.total_requests += 1;

        if let Some(request) = meeting_request {
            stats.successful_parsings += 1;
            
            // Update average confidence
            let total_successful = stats.successful_parsings as f32;
            stats.avg_confidence = ((stats.avg_confidence * (total_successful - 1.0)) + request.confidence) / total_successful;

            // Update meeting type statistics
            let type_name = format!("{:?}", request.meeting_type);
            *stats.requests_by_type.entry(type_name).or_insert(0) += 1;
        }

        // Update parsing accuracy rate
        stats.parsing_accuracy_rate = (stats.successful_parsings as f32 / stats.total_requests as f32) * 100.0;
    }

    /// Get service statistics
    pub async fn get_stats(&self) -> MeetingSchedulerStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Get configuration
    pub async fn get_config(&self) -> MeetingSchedulerConfig {
        let config = self.config.read().await;
        config.clone()
    }

    /// Update configuration
    pub async fn update_config(&self, new_config: MeetingSchedulerConfig) {
        let mut config = self.config.write().await;
        *config = new_config;
        info!("Meeting scheduler configuration updated");
    }

    /// Cleanup expired pending confirmations
    pub async fn cleanup_expired_confirmations(&self) -> usize {
        let config = self.config.read().await;
        let _timeout = Duration::from_secs(config.confirmation_timeout_seconds);
        drop(config);

        let mut pending = self.pending_confirmations.write().await;
        let initial_count = pending.len();

        // Remove expired confirmations (simplified - would need actual timestamp tracking)
        pending.clear(); // For now, clear all - would need proper expiration logic

        let removed_count = initial_count - pending.len();
        if removed_count > 0 {
            info!("Cleaned up {} expired meeting confirmations", removed_count);
        }

        removed_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> MeetingSchedulerConfig {
        MeetingSchedulerConfig {
            enabled: true,
            auto_create_enabled: false,
            trusted_domains: vec!["test.com".to_string()],
            default_duration_minutes: 30,
            buffer_time_minutes: 10,
            max_lookahead_days: 14,
            enable_location_detection: true,
            enable_attendee_extraction: true,
            confirmation_timeout_seconds: 600,
        }
    }

    #[test]
    fn test_meeting_scheduler_config_defaults() {
        let config = MeetingSchedulerConfig::default();
        assert!(config.enabled);
        assert!(!config.auto_create_enabled);
        assert_eq!(config.default_duration_minutes, 60);
        assert_eq!(config.buffer_time_minutes, 15);
    }

    #[test]
    fn test_meeting_request_creation() {
        let request = MeetingRequest {
            id: Uuid::new_v4(),
            email_id: "test123".to_string(),
            title: "Team Standup".to_string(),
            description: Some("Daily team sync".to_string()),
            proposed_datetime: Some(chrono::Utc::now()),
            alternative_times: vec![],
            duration_minutes: Some(30),
            location: Some(MeetingLocation {
                location_type: LocationType::Virtual,
                name: "Zoom Room".to_string(),
                details: None,
                coordinates: None,
            }),
            attendees: vec![],
            meeting_type: MeetingType::Team,
            priority: MeetingPriority::Normal,
            confidence: 0.9,
            agenda_items: vec!["Updates".to_string(), "Blockers".to_string()],
            organizer: MeetingAttendee {
                email: "manager@test.com".to_string(),
                name: Some("Manager".to_string()),
                status: AttendeeStatus::Accepted,
                required: true,
                role: AttendeeRole::Organizer,
            },
            recurrence: None,
            meeting_link: Some("https://zoom.us/123".to_string()),
            timezone: Some("UTC".to_string()),
        };

        assert_eq!(request.title, "Team Standup");
        assert_eq!(request.meeting_type, MeetingType::Team);
        assert_eq!(request.confidence, 0.9);
        assert_eq!(request.agenda_items.len(), 2);
    }

    #[test]
    fn test_meeting_priority_ordering() {
        assert!(MeetingPriority::Low < MeetingPriority::Normal);
        assert!(MeetingPriority::Normal < MeetingPriority::High);
        assert!(MeetingPriority::High < MeetingPriority::Urgent);
    }

    #[test]
    fn test_location_type_classification() {
        let virtual_location = MeetingLocation {
            location_type: LocationType::Virtual,
            name: "Teams Room".to_string(),
            details: None,
            coordinates: None,
        };

        assert_eq!(virtual_location.location_type, LocationType::Virtual);
        assert_eq!(virtual_location.name, "Teams Room");
    }

    #[test]
    fn test_attendee_roles_and_status() {
        let organizer = MeetingAttendee {
            email: "organizer@test.com".to_string(),
            name: Some("Organizer".to_string()),
            status: AttendeeStatus::Accepted,
            required: true,
            role: AttendeeRole::Organizer,
        };

        let optional_attendee = MeetingAttendee {
            email: "optional@test.com".to_string(),
            name: None,
            status: AttendeeStatus::NeedsAction,
            required: false,
            role: AttendeeRole::Optional,
        };

        assert_eq!(organizer.role, AttendeeRole::Organizer);
        assert_eq!(organizer.status, AttendeeStatus::Accepted);
        assert!(organizer.required);

        assert_eq!(optional_attendee.role, AttendeeRole::Optional);
        assert_eq!(optional_attendee.status, AttendeeStatus::NeedsAction);
        assert!(!optional_attendee.required);
    }

    #[test]
    fn test_recurrence_pattern() {
        let weekly_pattern = RecurrencePattern {
            frequency: RecurrenceFrequency::Weekly,
            interval: 1,
            days_of_week: vec![chrono::Weekday::Mon, chrono::Weekday::Wed, chrono::Weekday::Fri],
            end_date: None,
            count: Some(10),
        };

        assert_eq!(weekly_pattern.frequency, RecurrenceFrequency::Weekly);
        assert_eq!(weekly_pattern.interval, 1);
        assert_eq!(weekly_pattern.days_of_week.len(), 3);
        assert_eq!(weekly_pattern.count, Some(10));
    }

    #[test]
    fn test_conflict_severity_levels() {
        let minor_conflict = ConflictInfo {
            event_id: "test1".to_string(),
            title: "Brief overlap".to_string(),
            start_time: chrono::Utc::now(),
            end_time: chrono::Utc::now() + chrono::Duration::hours(1),
            severity: ConflictSeverity::Minor,
        };

        let critical_conflict = ConflictInfo {
            event_id: "test2".to_string(),
            title: "Board meeting".to_string(),
            start_time: chrono::Utc::now(),
            end_time: chrono::Utc::now() + chrono::Duration::hours(2),
            severity: ConflictSeverity::Critical,
        };

        assert_eq!(minor_conflict.severity, ConflictSeverity::Minor);
        assert_eq!(critical_conflict.severity, ConflictSeverity::Critical);
    }
}