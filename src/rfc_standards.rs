// RFC Standards Implementation for vCard and iCalendar parsing
// Using calcard crate for RFC 6350 (vCard) and RFC 5545 (iCalendar) compliance

use crate::calendar::{Event, EventStatus, EventPriority};
use crate::contacts::{Contact, ContactEmail, ContactPhone, ContactSource};
use calcard::{Parser, Entry};
use calcard::vcard::{VCard, VCardProperty, VCardValue, VCardParameter};
use calcard::icalendar::ICalendar;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use tracing::{debug, warn, error};

/// RFC standards parser for vCard and iCalendar data
pub struct RfcStandardsParser;

impl RfcStandardsParser {
    /// Parse vCard data (RFC 6350) into Contact objects
    pub fn parse_vcard_to_contact(
        vcard_data: &str,
        source: ContactSource,
    ) -> Result<Vec<Contact>, RfcStandardsError> {
        debug!("Parsing vCard data with RFC 6350 compliance");
        
        let mut parser = Parser::new(vcard_data);
        let mut contacts = Vec::new();

        loop {
            match parser.entry() {
                Entry::VCard(vcard) => {
                    let contact = Self::convert_vcard_to_contact(vcard, source.clone())?;
                    contacts.push(contact);
                }
                Entry::ICalendar(_) => {
                    // Skip iCalendar entries when parsing vCard
                    continue;
                }
                Entry::InvalidLine(line) => {
                    warn!("vCard parse error: invalid line: {}", line);
                    continue;
                }
                Entry::Eof => break,
                _ => continue,
            }
        }

        if contacts.is_empty() {
            return Err(RfcStandardsError::ParseError(
                "No valid vCard entries found".to_string()
            ));
        }

        debug!("Successfully parsed {} vCard entries", contacts.len());
        Ok(contacts)
    }

    /// Convert a single VCard to Contact
    fn convert_vcard_to_contact(
        vcard: VCard,
        source: ContactSource,
    ) -> Result<Contact, RfcStandardsError> {
        // Helper function to extract property value as string
        let get_property_value = |prop: &VCardProperty| -> Option<String> {
            vcard.entries.iter()
                .find(|entry| entry.name == *prop)
                .and_then(|entry| entry.values.first())
                .map(|value| match value {
                    VCardValue::Text(text) => text.clone(),
                    VCardValue::Integer(i) => i.to_string(),
                    VCardValue::Float(f) => f.to_string(),
                    VCardValue::Boolean(b) => b.to_string(),
                    VCardValue::PartialDateTime(dt) => format!("{:?}", dt),
                    VCardValue::Binary(data) => format!("{:?}", data),
                    VCardValue::Sex(sex) => format!("{:?}", sex),
                    VCardValue::GramGender(gender) => format!("{:?}", gender),
                    VCardValue::Kind(kind) => format!("{:?}", kind),
                })
        };

        // Extract UID or generate one
        let external_id = get_property_value(&VCardProperty::Uid)
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        // Extract formatted name
        let display_name = get_property_value(&VCardProperty::Fn)
            .unwrap_or_else(|| "Unknown".to_string());

        let mut contact = Contact::new(external_id, source, display_name);

        // Extract name components from N property
        if let Some(name_value) = get_property_value(&VCardProperty::N) {
            let name_parts: Vec<&str> = name_value.split(';').collect();
            if name_parts.len() >= 2 {
                if !name_parts[0].is_empty() {
                    contact.last_name = Some(name_parts[0].to_string());
                }
                if !name_parts[1].is_empty() {
                    contact.first_name = Some(name_parts[1].to_string());
                }
            }
        }

        // Extract email addresses
        for entry in &vcard.entries {
            if entry.name == VCardProperty::Email {
                if let Some(VCardValue::Text(address)) = entry.values.first() {
                    // Parse TYPE parameter to determine label
                    let label = entry.params.iter()
                        .find_map(|param| match param {
                            VCardParameter::Type(types) => types.first().map(|t| format!("{:?}", t).to_lowercase()),
                            _ => None,
                        })
                        .unwrap_or_else(|| "other".to_string());

                    // Check for PREF parameter
                    let is_primary = entry.params.iter()
                        .any(|param| matches!(param, VCardParameter::Pref(_)));

                    contact.emails.push(ContactEmail {
                        address: address.clone(),
                        label,
                        is_primary,
                    });
                }
            }
        }

        // Extract phone numbers
        for entry in &vcard.entries {
            if entry.name == VCardProperty::Tel {
                if let Some(VCardValue::Text(number)) = entry.values.first() {
                    // Parse TYPE parameter to determine label
                    let label = entry.params.iter()
                        .find_map(|param| match param {
                            VCardParameter::Type(types) => types.first().map(|t| format!("{:?}", t).to_lowercase()),
                            _ => None,
                        })
                        .unwrap_or_else(|| "other".to_string());

                    // Check for PREF parameter
                    let is_primary = entry.params.iter()
                        .any(|param| matches!(param, VCardParameter::Pref(_)));

                    contact.phones.push(ContactPhone {
                        number: number.clone(),
                        label,
                        is_primary,
                    });
                }
            }
        }

        // Extract organization
        if let Some(org_value) = get_property_value(&VCardProperty::Org) {
            contact.company = Some(org_value);
        }

        // Extract title
        if let Some(title_value) = get_property_value(&VCardProperty::Title) {
            contact.job_title = Some(title_value);
        }

        // Extract photo URL
        for entry in &vcard.entries {
            if entry.name == VCardProperty::Photo {
                // Check if it's a URL or data - default to URI for photos
                if let Some(value) = entry.values.first() {
                    match value {
                        VCardValue::Text(url) => {
                            contact.photo_url = Some(url.clone());
                        }
                        _ => {}
                    }
                }
                break;
            }
        }

        // Extract revision timestamp
        if let Some(rev_value) = get_property_value(&VCardProperty::Rev) {
            if let Ok(dt) = DateTime::parse_from_rfc3339(&rev_value) {
                contact.updated_at = dt.with_timezone(&Utc);
            }
        }

        debug!("Successfully parsed vCard for contact: {}", contact.display_name);
        Ok(contact)
    }

    /// Convert Contact to vCard format (RFC 6350)
    pub fn contact_to_vcard(contact: &Contact) -> Result<String, RfcStandardsError> {
        debug!("Converting contact to RFC 6350 vCard format: {}", contact.display_name);

        // TODO: Implement Contact to vCard conversion using calcard writer
        // For now, return a basic vCard string manually
        let mut vcard_lines = Vec::new();
        
        vcard_lines.push("BEGIN:VCARD".to_string());
        vcard_lines.push("VERSION:4.0".to_string());
        vcard_lines.push(format!("UID:{}", contact.external_id));
        vcard_lines.push(format!("FN:{}", contact.display_name));
        
        // Add name components
        if contact.first_name.is_some() || contact.last_name.is_some() {
            let family_name = contact.last_name.as_deref().unwrap_or("");
            let given_name = contact.first_name.as_deref().unwrap_or("");
            vcard_lines.push(format!("N:{};{};;;", family_name, given_name));
        }
        
        // Add email addresses
        for email in &contact.emails {
            let pref = if email.is_primary { ";PREF=1" } else { "" };
            vcard_lines.push(format!("EMAIL;TYPE={}{}:{}", email.label, pref, email.address));
        }
        
        // Add phone numbers
        for phone in &contact.phones {
            let pref = if phone.is_primary { ";PREF=1" } else { "" };
            vcard_lines.push(format!("TEL;TYPE={}{}:{}", phone.label, pref, phone.number));
        }
        
        // Add organization
        if let Some(company) = &contact.company {
            vcard_lines.push(format!("ORG:{}", company));
        }
        
        // Add title
        if let Some(title) = &contact.job_title {
            vcard_lines.push(format!("TITLE:{}", title));
        }
        
        // Add photo URL
        if let Some(photo_url) = &contact.photo_url {
            vcard_lines.push(format!("PHOTO:{}", photo_url));
        }
        
        // Set revision timestamp
        vcard_lines.push(format!("REV:{}", contact.updated_at.to_rfc3339()));
        vcard_lines.push("END:VCARD".to_string());
        
        let vcard_string = vcard_lines.join("\r\n");
        debug!("Successfully converted contact to vCard format");
        Ok(vcard_string)
    }

    /// Parse iCalendar data (RFC 5545) into Event objects
    pub fn parse_icalendar_to_event(
        icalendar_data: &str,
        _calendar_id: String,
    ) -> Result<Vec<Event>, RfcStandardsError> {
        debug!("Parsing iCalendar data with RFC 5545 compliance");

        let mut parser = Parser::new(icalendar_data);
        let events = Vec::new();

        loop {
            match parser.entry() {
                Entry::ICalendar(_icalendar) => {
                    // TODO: Implement iCalendar to Event conversion
                    // For now, just skip iCalendar entries
                    debug!("Found iCalendar entry, parsing not yet implemented");
                }
                Entry::VCard(_) => {
                    // Skip vCard entries when parsing iCalendar
                    continue;
                }
                Entry::InvalidLine(line) => {
                    warn!("iCalendar parse error: invalid line: {}", line);
                    continue;
                }
                Entry::Eof => break,
                _ => continue,
            }
        }

        debug!("Successfully parsed {} events from iCalendar", events.len());
        Ok(events)
    }

    /// Convert Event to iCalendar format (RFC 5545)
    pub fn event_to_icalendar(events: &[Event]) -> Result<String, RfcStandardsError> {
        debug!("Converting {} events to RFC 5545 iCalendar format", events.len());

        // TODO: Implement Event to iCalendar conversion using calcard writer
        // For now, return a placeholder
        let icalendar_string = format!(
            "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Comunicado//Comunicado Calendar Client//EN\r\n{}END:VCALENDAR\r\n",
            events.iter()
                .map(|e| format!("BEGIN:VEVENT\r\nUID:{}\r\nSUMMARY:{}\r\nEND:VEVENT\r\n", e.uid, e.title))
                .collect::<String>()
        );
        
        debug!("Successfully converted events to iCalendar format");
        Ok(icalendar_string)
    }

    /// Convert calcard Event to our internal Event - TODO: Implement
    #[allow(dead_code)]
    fn convert_cal_event_to_event(
        _icalendar: &ICalendar,
        calendar_id: &str,
    ) -> Result<Event, RfcStandardsError> {
        // TODO: Implement iCalendar to Event conversion using the calcard ICalendar structure
        // For now, return a placeholder event
        let event = Event {
            id: Uuid::new_v4().to_string(),
            uid: Uuid::new_v4().to_string(),
            calendar_id: calendar_id.to_string(),
            title: "Placeholder Event".to_string(),
            description: None,
            location: None,
            start_time: Utc::now(),
            end_time: Utc::now() + chrono::Duration::hours(1),
            all_day: false,
            status: EventStatus::Confirmed,
            priority: EventPriority::Normal,
            organizer: None,
            attendees: Vec::new(),
            recurrence: None,
            reminders: Vec::new(),
            categories: Vec::new(),
            url: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            sequence: 0,
            etag: None,
        };

        Ok(event)
    }

    /// Convert our internal Event to calcard Event - TODO: Implement
    #[allow(dead_code)]
    fn convert_event_to_cal_event(_event: &Event) -> Result<ICalendar, RfcStandardsError> {
        // TODO: Implement Event to ICalendar conversion using calcard writer
        // For now, return an empty ICalendar
        Ok(ICalendar::default())
    }
}

/// Errors that can occur during RFC standards parsing
#[derive(Debug, thiserror::Error)]
pub enum RfcStandardsError {
    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Invalid data format: {0}")]
    InvalidFormat(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Conversion error: {0}")]
    ConversionError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vcard_parsing() {
        let vcard_data = r#"BEGIN:VCARD
VERSION:4.0
UID:12345-67890-abcdef
FN:John Doe
N:Doe;John;;;
EMAIL;TYPE=WORK:john@company.com
EMAIL;TYPE=HOME;PREF=1:john@personal.com
TEL;TYPE=WORK:+1-555-123-4567
TEL;TYPE=CELL;PREF=1:+1-555-987-6543
ORG:Acme Corporation
TITLE:Software Engineer
REV:2025-01-01T12:00:00Z
END:VCARD"#;

        let source = ContactSource::Local;
        let contacts = RfcStandardsParser::parse_vcard_to_contact(vcard_data, source).unwrap();
        assert_eq!(contacts.len(), 1);
        let contact = &contacts[0];

        assert_eq!(contact.display_name, "John Doe");
        assert_eq!(contact.first_name, Some("John".to_string()));
        assert_eq!(contact.last_name, Some("Doe".to_string()));
        assert_eq!(contact.company, Some("Acme Corporation".to_string()));
        assert_eq!(contact.job_title, Some("Software Engineer".to_string()));
        assert_eq!(contact.emails.len(), 2);
        assert_eq!(contact.phones.len(), 2);

        // Check primary email
        let primary_email = contact.emails.iter().find(|e| e.is_primary).unwrap();
        assert_eq!(primary_email.address, "john@personal.com");
    }

    #[test]
    fn test_icalendar_parsing() {
        let icalendar_data = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Test//Test//EN
BEGIN:VEVENT
UID:test-event-12345
SUMMARY:Team Meeting
DESCRIPTION:Weekly team standup meeting
LOCATION:Conference Room A
DTSTART:20250128T100000Z
DTEND:20250128T110000Z
STATUS:CONFIRMED
ORGANIZER;CN=Jane Smith:mailto:jane@company.com
ATTENDEE;CN=John Doe;PARTSTAT=ACCEPTED:mailto:john@company.com
ATTENDEE;CN=Bob Wilson;PARTSTAT=NEEDS-ACTION:mailto:bob@company.com
CREATED:20250120T090000Z
LAST-MODIFIED:20250125T150000Z
SEQUENCE:1
END:VEVENT
END:VCALENDAR"#;

        let events = RfcStandardsParser::parse_icalendar_to_event(
            icalendar_data,
            "test-calendar".to_string(),
        ).unwrap();

        assert_eq!(events.len(), 1);
        
        let event = &events[0];
        assert_eq!(event.title, "Team Meeting");
        assert_eq!(event.description, Some("Weekly team standup meeting".to_string()));
        assert_eq!(event.location, Some("Conference Room A".to_string()));
        assert_eq!(event.status, EventStatus::Confirmed);
        assert_eq!(event.attendees.len(), 2);
        assert!(event.organizer.is_some());

        let organizer = event.organizer.as_ref().unwrap();
        assert_eq!(organizer.email, "jane@company.com");
        assert_eq!(organizer.name, Some("Jane Smith".to_string()));
    }

    #[test]
    fn test_roundtrip_vcard() {
        let original_vcard = r#"BEGIN:VCARD
VERSION:4.0
UID:roundtrip-test
FN:Test User
N:User;Test;;;
EMAIL:test@example.com
TEL:+1-555-TEST
END:VCARD"#;

        let source = ContactSource::Local;
        let contacts = RfcStandardsParser::parse_vcard_to_contact(original_vcard, source).unwrap();
        assert_eq!(contacts.len(), 1);
        let contact = &contacts[0];
        let generated_vcard = RfcStandardsParser::contact_to_vcard(contact).unwrap();

        // Parse the generated vCard back
        let roundtrip_contacts = RfcStandardsParser::parse_vcard_to_contact(
            &generated_vcard,
            ContactSource::Local,
        ).unwrap();
        assert_eq!(roundtrip_contacts.len(), 1);
        let roundtrip_contact = &roundtrip_contacts[0];

        assert_eq!(contact.display_name, roundtrip_contact.display_name);
        assert_eq!(contact.first_name, roundtrip_contact.first_name);
        assert_eq!(contact.last_name, roundtrip_contact.last_name);
        assert_eq!(contact.emails.len(), roundtrip_contact.emails.len());
    }
}