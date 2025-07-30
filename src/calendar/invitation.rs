use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

use crate::calendar::event::{AttendeeRole, AttendeeStatus, Event, EventAttendee, EventStatus};
use crate::calendar::CalendarError;
use crate::email::{StoredAttachment, StoredMessage};

/// Meeting invitation processing errors
#[derive(Error, Debug)]
pub enum InvitationError {
    #[error("Invalid iCalendar format: {0}")]
    InvalidFormat(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Date parsing error: {0}")]
    DateParse(#[from] chrono::ParseError),

    #[error("Email processing error: {0}")]
    EmailError(String),

    #[error("Calendar error: {0}")]
    CalendarError(#[from] CalendarError),
}

pub type InvitationResult<T> = Result<T, InvitationError>;

/// Meeting invitation extracted from email
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingInvitation {
    /// Unique invitation ID (from iCal UID)
    pub uid: String,

    /// Event title/summary
    pub title: String,

    /// Event description
    pub description: Option<String>,

    /// Event location
    pub location: Option<String>,

    /// Start time
    pub start_time: DateTime<Utc>,

    /// End time
    pub end_time: DateTime<Utc>,

    /// All-day event flag
    pub all_day: bool,

    /// Event organizer
    pub organizer: Option<EventAttendee>,

    /// List of attendees
    pub attendees: Vec<EventAttendee>,

    /// Invitation method (REQUEST, REPLY, CANCEL, etc.)
    pub method: InvitationMethod,

    /// Event status
    pub status: EventStatus,

    /// Sequence number for updates
    pub sequence: u32,

    /// Original email message ID
    pub email_message_id: Option<String>,

    /// iCalendar source data
    pub icalendar_data: String,

    /// Time when invitation was processed
    pub processed_at: DateTime<Utc>,
}

/// iCalendar invitation methods
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvitationMethod {
    /// Initial meeting request
    Request,
    /// Response to a request
    Reply,
    /// Meeting cancellation
    Cancel,
    /// Meeting update
    Refresh,
    /// Request for updated information
    Counter,
    /// Decline counter proposal
    DeclineCounter,
}

impl InvitationMethod {
    pub fn from_ical(value: &str) -> Self {
        match value.to_uppercase().as_str() {
            "REQUEST" => InvitationMethod::Request,
            "REPLY" => InvitationMethod::Reply,
            "CANCEL" => InvitationMethod::Cancel,
            "REFRESH" => InvitationMethod::Refresh,
            "COUNTER" => InvitationMethod::Counter,
            "DECLINECOUNTER" => InvitationMethod::DeclineCounter,
            _ => InvitationMethod::Request, // Default fallback
        }
    }

    pub fn to_ical(&self) -> &str {
        match self {
            InvitationMethod::Request => "REQUEST",
            InvitationMethod::Reply => "REPLY",
            InvitationMethod::Cancel => "CANCEL",
            InvitationMethod::Refresh => "REFRESH",
            InvitationMethod::Counter => "COUNTER",
            InvitationMethod::DeclineCounter => "DECLINECOUNTER",
        }
    }
}

/// RSVP response options
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RSVPResponse {
    Accept,
    Decline,
    Tentative,
    NeedsAction,
}

impl RSVPResponse {
    pub fn to_attendee_status(&self) -> AttendeeStatus {
        match self {
            RSVPResponse::Accept => AttendeeStatus::Accepted,
            RSVPResponse::Decline => AttendeeStatus::Declined,
            RSVPResponse::Tentative => AttendeeStatus::Tentative,
            RSVPResponse::NeedsAction => AttendeeStatus::NeedsAction,
        }
    }
}

/// Meeting invitation processor
pub struct InvitationProcessor {
    /// User's email addresses for attendee matching
    user_emails: Vec<String>,
}

impl InvitationProcessor {
    /// Create a new invitation processor
    pub fn new(user_emails: Vec<String>) -> Self {
        Self { user_emails }
    }

    /// Check if an email contains a meeting invitation
    pub fn has_invitation(&self, message: &StoredMessage) -> bool {
        // Check for iCalendar attachments
        for attachment in &message.attachments {
            if self.is_calendar_attachment(&attachment) {
                return true;
            }
        }

        // Check for inline calendar data in email body
        if let Some(ref body) = message.body_text {
            if body.contains("BEGIN:VCALENDAR") && body.contains("METHOD:") {
                return true;
            }
        }

        // Note: Headers are not directly accessible in StoredMessage
        // Calendar content is primarily detected through attachments and body content

        false
    }

    /// Extract meeting invitation from email
    pub async fn extract_invitation(
        &self,
        message: &StoredMessage,
    ) -> InvitationResult<Option<MeetingInvitation>> {
        // First, try to find iCalendar data in attachments
        for attachment in &message.attachments {
            if self.is_calendar_attachment(attachment) {
                if let Some(ref data) = attachment.data {
                    if let Ok(ical_data) = std::str::from_utf8(data) {
                        if let Ok(invitation) =
                            self.parse_icalendar(ical_data, message.message_id.clone())
                        {
                            return Ok(Some(invitation));
                        }
                    }
                }
            }
        }

        // Try to find inline calendar data in email body
        if let Some(ref body) = message.body_text {
            if body.contains("BEGIN:VCALENDAR") {
                if let Ok(invitation) = self.parse_icalendar(body, message.message_id.clone()) {
                    return Ok(Some(invitation));
                }
            }
        }

        Ok(None)
    }

    /// Check if attachment is a calendar file
    fn is_calendar_attachment(&self, attachment: &StoredAttachment) -> bool {
        // Check MIME type
        if attachment.content_type.contains("text/calendar")
            || attachment.content_type.contains("application/ics")
        {
            return true;
        }

        // Check file extension
        let filename = attachment.filename.to_lowercase();
        filename.ends_with(".ics")
            || filename.ends_with(".vcs")
            || filename.ends_with(".ifb")
            || filename.ends_with(".ical")
    }

    /// Parse iCalendar data into a meeting invitation
    fn parse_icalendar(
        &self,
        ical_data: &str,
        email_message_id: Option<String>,
    ) -> InvitationResult<MeetingInvitation> {
        let lines = ical_data
            .lines()
            .map(|line| line.trim())
            .collect::<Vec<_>>();
        let mut properties = HashMap::new();
        let mut attendees = Vec::new();
        let mut organizer = None;

        // Basic iCalendar parsing - find VEVENT section
        let mut in_event = false;
        let mut in_calendar = false;
        let mut method = InvitationMethod::Request;

        for line in lines {
            if line == "BEGIN:VCALENDAR" {
                in_calendar = true;
                continue;
            }

            if line == "END:VCALENDAR" {
                let _ = in_calendar; // Acknowledge the variable is intentionally unused here
                break;
            }

            if !in_calendar {
                continue;
            }

            if line == "BEGIN:VEVENT" {
                in_event = true;
                continue;
            }

            if line == "END:VEVENT" {
                in_event = false;
                continue;
            }

            // Parse calendar-level properties
            if !in_event && line.starts_with("METHOD:") {
                let method_str = line.strip_prefix("METHOD:").unwrap_or("REQUEST");
                method = InvitationMethod::from_ical(method_str);
                continue;
            }

            if !in_event {
                continue;
            }

            // Parse event properties
            if let Some(colon_pos) = line.find(':') {
                let key = &line[..colon_pos];
                let value = &line[colon_pos + 1..];

                // Handle special properties
                if key.starts_with("ATTENDEE") {
                    if let Some(attendee) = self.parse_attendee(line) {
                        attendees.push(attendee);
                    }
                } else if key.starts_with("ORGANIZER") {
                    organizer = self.parse_organizer(line);
                } else {
                    // Store other properties
                    let clean_key = key.split(';').next().unwrap_or(key);
                    properties.insert(clean_key.to_string(), value.to_string());
                }
            }
        }

        // Extract required fields
        let uid = properties
            .get("UID")
            .ok_or_else(|| InvitationError::MissingField("UID".to_string()))?
            .clone();

        let title = properties
            .get("SUMMARY")
            .ok_or_else(|| InvitationError::MissingField("SUMMARY".to_string()))?
            .clone();

        let start_time = self.parse_datetime(
            properties
                .get("DTSTART")
                .ok_or_else(|| InvitationError::MissingField("DTSTART".to_string()))?,
        )?;

        let end_time = self.parse_datetime(
            properties
                .get("DTEND")
                .ok_or_else(|| InvitationError::MissingField("DTEND".to_string()))?,
        )?;

        // Parse optional fields
        let description = properties.get("DESCRIPTION").cloned();
        let location = properties.get("LOCATION").cloned();
        let sequence = properties
            .get("SEQUENCE")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let status = properties
            .get("STATUS")
            .map(|s| EventStatus::from_icalendar(s))
            .unwrap_or(EventStatus::Confirmed);

        // Check for all-day event
        let all_day = properties
            .get("DTSTART")
            .map(|dt| !dt.contains("T"))
            .unwrap_or(false);

        Ok(MeetingInvitation {
            uid,
            title,
            description,
            location,
            start_time,
            end_time,
            all_day,
            organizer,
            attendees,
            method,
            status,
            sequence,
            email_message_id,
            icalendar_data: ical_data.to_string(),
            processed_at: Utc::now(),
        })
    }

    /// Parse datetime from iCalendar format
    fn parse_datetime(&self, dt_str: &str) -> InvitationResult<DateTime<Utc>> {
        // Handle different datetime formats
        if dt_str.ends_with('Z') {
            // UTC format: 20250128T100000Z
            DateTime::parse_from_str(dt_str, "%Y%m%dT%H%M%SZ")
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(InvitationError::DateParse)
        } else if dt_str.contains('T') {
            // Local format: 20250128T100000
            let naive = chrono::NaiveDateTime::parse_from_str(dt_str, "%Y%m%dT%H%M%S")?;
            Ok(naive.and_utc())
        } else {
            // Date only: 20250128
            let naive_date = chrono::NaiveDate::parse_from_str(dt_str, "%Y%m%d")?;
            Ok(naive_date.and_hms_opt(0, 0, 0).unwrap().and_utc())
        }
    }

    /// Parse attendee from iCalendar ATTENDEE line
    fn parse_attendee(&self, line: &str) -> Option<EventAttendee> {
        // Example: ATTENDEE;CN=John Doe;PARTSTAT=NEEDS-ACTION:mailto:john@example.com
        let mut email = String::new();
        let mut name = None;
        let mut status = AttendeeStatus::NeedsAction;
        let mut role = AttendeeRole::RequiredParticipant;
        let mut rsvp = false;

        // Extract email from mailto: URI
        if let Some(mailto_pos) = line.find("mailto:") {
            email = line[mailto_pos + 7..].to_string();
        }

        // Parse parameters
        let parts: Vec<&str> = line.split(';').collect();
        for part in parts {
            if let Some(eq_pos) = part.find('=') {
                let key = &part[..eq_pos];
                let value = &part[eq_pos + 1..];

                match key {
                    "CN" => name = Some(value.to_string()),
                    "PARTSTAT" => status = AttendeeStatus::from_icalendar(value),
                    "ROLE" => {
                        role = match value {
                            "CHAIR" => AttendeeRole::Chair,
                            "REQ-PARTICIPANT" => AttendeeRole::RequiredParticipant,
                            "OPT-PARTICIPANT" => AttendeeRole::OptionalParticipant,
                            "NON-PARTICIPANT" => AttendeeRole::NonParticipant,
                            _ => AttendeeRole::RequiredParticipant,
                        };
                    }
                    "RSVP" => rsvp = value.to_uppercase() == "TRUE",
                    _ => {}
                }
            }
        }

        if !email.is_empty() {
            let now = Utc::now();
            Some(EventAttendee {
                email,
                name,
                status,
                role,
                rsvp,
                created_at: now,
                updated_at: now,
            })
        } else {
            None
        }
    }

    /// Parse organizer from iCalendar ORGANIZER line  
    fn parse_organizer(&self, line: &str) -> Option<EventAttendee> {
        // Similar to attendee parsing but for organizer
        if let Some(attendee) = self.parse_attendee(line) {
            let mut organizer = attendee;
            organizer.role = AttendeeRole::Chair;
            organizer.status = AttendeeStatus::Accepted;
            organizer.rsvp = false;
            Some(organizer)
        } else {
            None
        }
    }

    /// Check if user is invited to this meeting
    pub fn is_user_invited(&self, invitation: &MeetingInvitation) -> bool {
        invitation.attendees.iter().any(|attendee| {
            self.user_emails
                .iter()
                .any(|user_email| user_email.eq_ignore_ascii_case(&attendee.email))
        })
    }

    /// Get user's current RSVP status for the invitation
    pub fn get_user_rsvp_status(&self, invitation: &MeetingInvitation) -> Option<AttendeeStatus> {
        invitation
            .attendees
            .iter()
            .find(|attendee| {
                self.user_emails
                    .iter()
                    .any(|user_email| user_email.eq_ignore_ascii_case(&attendee.email))
            })
            .map(|attendee| attendee.status.clone())
    }

    /// Convert invitation to Event for calendar storage
    pub fn invitation_to_event(
        &self,
        invitation: &MeetingInvitation,
        calendar_id: String,
    ) -> Event {
        let now = Utc::now();

        Event {
            id: invitation.uid.clone(),
            uid: invitation.uid.clone(),
            calendar_id,
            title: invitation.title.clone(),
            description: invitation.description.clone(),
            location: invitation.location.clone(),
            start_time: invitation.start_time,
            end_time: invitation.end_time,
            all_day: invitation.all_day,
            status: invitation.status,
            priority: crate::calendar::event::EventPriority::Normal,
            organizer: invitation.organizer.clone(),
            attendees: invitation.attendees.clone(),
            recurrence: None,      // TODO: Parse recurrence rules
            reminders: Vec::new(), // TODO: Parse alarms
            categories: Vec::new(),
            url: None,
            created_at: now,
            updated_at: now,
            sequence: invitation.sequence,
            etag: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invitation_method_parsing() {
        assert_eq!(
            InvitationMethod::from_ical("REQUEST"),
            InvitationMethod::Request
        );
        assert_eq!(
            InvitationMethod::from_ical("REPLY"),
            InvitationMethod::Reply
        );
        assert_eq!(
            InvitationMethod::from_ical("CANCEL"),
            InvitationMethod::Cancel
        );
        assert_eq!(
            InvitationMethod::from_ical("invalid"),
            InvitationMethod::Request
        );
    }

    #[test]
    fn test_rsvp_response_conversion() {
        assert_eq!(
            RSVPResponse::Accept.to_attendee_status(),
            AttendeeStatus::Accepted
        );
        assert_eq!(
            RSVPResponse::Decline.to_attendee_status(),
            AttendeeStatus::Declined
        );
        assert_eq!(
            RSVPResponse::Tentative.to_attendee_status(),
            AttendeeStatus::Tentative
        );
        assert_eq!(
            RSVPResponse::NeedsAction.to_attendee_status(),
            AttendeeStatus::NeedsAction
        );
    }

    #[test]
    fn test_datetime_parsing() {
        let processor = InvitationProcessor::new(vec!["test@example.com".to_string()]);

        // Test UTC format
        let utc_dt = processor.parse_datetime("20250128T100000Z").unwrap();
        assert_eq!(
            utc_dt.format("%Y%m%dT%H%M%SZ").to_string(),
            "20250128T100000Z"
        );

        // Test local format
        let local_dt = processor.parse_datetime("20250128T100000").unwrap();
        assert_eq!(
            local_dt.format("%Y%m%dT%H%M%S").to_string(),
            "20250128T100000"
        );

        // Test date only
        let date_only = processor.parse_datetime("20250128").unwrap();
        assert_eq!(date_only.format("%Y%m%d").to_string(), "20250128");
    }
}
