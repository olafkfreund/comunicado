use crate::smtp::{SmtpError, SmtpResult};
use crate::ui::EmailComposeData;
use lettre::{
    message::{header::ContentType, Mailbox, MultiPart, SinglePart},
    Address, Message,
};
use std::str::FromStr;

/// Email message builder with MIME support
pub struct MessageBuilder {
    from: Option<Mailbox>,
    to: Vec<Mailbox>,
    cc: Vec<Mailbox>,
    bcc: Vec<Mailbox>,
    subject: String,
    body_text: String,
    body_html: Option<String>,
    message_id: Option<String>,
    in_reply_to: Option<String>,
    references: Option<String>,
    user_agent: String,
}

impl MessageBuilder {
    /// Create a new message builder
    pub fn new() -> Self {
        Self {
            from: None,
            to: Vec::new(),
            cc: Vec::new(),
            bcc: Vec::new(),
            subject: String::new(),
            body_text: String::new(),
            body_html: None,
            message_id: None,
            in_reply_to: None,
            references: None,
            user_agent: "Comunicado/0.1.0".to_string(),
        }
    }

    /// Set the sender
    pub fn from(mut self, from: Mailbox) -> Self {
        self.from = Some(from);
        self
    }

    /// Set the sender from string
    pub fn from_str(mut self, from: &str) -> SmtpResult<Self> {
        let mailbox = parse_mailbox(from)?;
        self.from = Some(mailbox);
        Ok(self)
    }

    /// Add a To recipient
    pub fn to(mut self, to: Mailbox) -> Self {
        self.to.push(to);
        self
    }

    /// Add To recipients from string
    pub fn to_str(mut self, to: &str) -> SmtpResult<Self> {
        let addresses = parse_address_list(to)?;
        self.to.extend(addresses);
        Ok(self)
    }

    /// Add a Cc recipient
    pub fn cc(mut self, cc: Mailbox) -> Self {
        self.cc.push(cc);
        self
    }

    /// Add Cc recipients from string
    pub fn cc_str(mut self, cc: &str) -> SmtpResult<Self> {
        let addresses = parse_address_list(cc)?;
        self.cc.extend(addresses);
        Ok(self)
    }

    /// Add a Bcc recipient
    pub fn bcc(mut self, bcc: Mailbox) -> Self {
        self.bcc.push(bcc);
        self
    }

    /// Add Bcc recipients from string
    pub fn bcc_str(mut self, bcc: &str) -> SmtpResult<Self> {
        let addresses = parse_address_list(bcc)?;
        self.bcc.extend(addresses);
        Ok(self)
    }

    /// Set the subject
    pub fn subject<S: Into<String>>(mut self, subject: S) -> Self {
        self.subject = subject.into();
        self
    }

    /// Set the plain text body
    pub fn body_text<S: Into<String>>(mut self, body: S) -> Self {
        self.body_text = body.into();
        self
    }

    /// Set the HTML body
    pub fn body_html<S: Into<String>>(mut self, body: S) -> Self {
        self.body_html = Some(body.into());
        self
    }

    /// Set message ID (will generate one if not provided)
    pub fn message_id<S: Into<String>>(mut self, message_id: S) -> Self {
        self.message_id = Some(message_id.into());
        self
    }

    /// Set In-Reply-To header (for replies)
    pub fn in_reply_to<S: Into<String>>(mut self, in_reply_to: S) -> Self {
        self.in_reply_to = Some(in_reply_to.into());
        self
    }

    /// Set References header (for threading)
    pub fn references<S: Into<String>>(mut self, references: S) -> Self {
        self.references = Some(references.into());
        self
    }

    /// Set user agent
    pub fn user_agent<S: Into<String>>(mut self, user_agent: S) -> Self {
        self.user_agent = user_agent.into();
        self
    }

    /// Build the message
    pub fn build(self) -> SmtpResult<Message> {
        // Validate required fields
        let from = self
            .from
            .ok_or_else(|| SmtpError::MessageFormatError("From address is required".to_string()))?;

        if self.to.is_empty() && self.cc.is_empty() && self.bcc.is_empty() {
            return Err(SmtpError::MessageFormatError(
                "At least one recipient is required".to_string(),
            ));
        }

        if self.subject.is_empty() {
            return Err(SmtpError::MessageFormatError(
                "Subject is required".to_string(),
            ));
        }

        // Start building the message
        let mut message_builder = Message::builder().from(from).subject(self.subject);

        // Add recipients
        for to in self.to {
            message_builder = message_builder.to(to);
        }

        for cc in self.cc {
            message_builder = message_builder.cc(cc);
        }

        for bcc in self.bcc {
            message_builder = message_builder.bcc(bcc);
        }

        // Add headers
        if let Some(message_id) = self.message_id {
            message_builder = message_builder.message_id(Some(message_id));
        }

        if let Some(in_reply_to) = self.in_reply_to {
            message_builder = message_builder.in_reply_to(in_reply_to);
        }

        if let Some(references) = self.references {
            message_builder = message_builder.references(references);
        }

        // Add User-Agent header (skip for now as lettre requires specific header types)

        // Add Date header
        message_builder = message_builder.date_now();

        // Create message body
        let message = if let Some(html_body) = self.body_html {
            // Multipart message with both text and HTML
            let text_part = SinglePart::builder()
                .header(ContentType::TEXT_PLAIN)
                .body(self.body_text);

            let html_part = SinglePart::builder()
                .header(ContentType::TEXT_HTML)
                .body(html_body);

            let multipart = MultiPart::alternative()
                .singlepart(text_part)
                .singlepart(html_part);

            message_builder.multipart(multipart)
        } else {
            // Plain text only
            message_builder.body(self.body_text)
        };

        message.map_err(|e| SmtpError::MessageBuildError(e))
    }
}

impl Default for MessageBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Wrapper for email messages with metadata
#[derive(Debug, Clone)]
pub struct EmailMessage {
    pub from: String,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub subject: String,
    pub body_text: String,
    pub body_html: Option<String>,
    pub message_id: Option<String>,
    pub in_reply_to: Option<String>,
    pub references: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl EmailMessage {
    /// Create a new email message
    pub fn new(from: String, to: Vec<String>, subject: String, body: String) -> Self {
        Self {
            from,
            to,
            cc: Vec::new(),
            bcc: Vec::new(),
            subject,
            body_text: body,
            body_html: None,
            message_id: None,
            in_reply_to: None,
            references: None,
            created_at: chrono::Utc::now(),
        }
    }

    /// Create from compose UI data
    pub fn from_compose_data(data: &EmailComposeData, from_address: String) -> SmtpResult<Self> {
        let to = EmailComposeData::parse_addresses(&data.to);
        let cc = EmailComposeData::parse_addresses(&data.cc);
        let bcc = EmailComposeData::parse_addresses(&data.bcc);

        // Validate at least one recipient
        if to.is_empty() && cc.is_empty() && bcc.is_empty() {
            return Err(SmtpError::MessageFormatError(
                "At least one recipient is required".to_string(),
            ));
        }

        Ok(Self {
            from: from_address,
            to,
            cc,
            bcc,
            subject: data.subject.clone(),
            body_text: data.body.clone(),
            body_html: None,
            message_id: None,
            in_reply_to: None,
            references: None,
            created_at: chrono::Utc::now(),
        })
    }

    /// Create a reply message
    pub fn create_reply(
        original: &EmailMessage,
        from_address: String,
        body: String,
        reply_all: bool,
    ) -> Self {
        let mut reply = Self {
            from: from_address.clone(),
            to: vec![original.from.clone()],
            cc: if reply_all {
                original.cc.clone()
            } else {
                Vec::new()
            },
            bcc: Vec::new(),
            subject: if original.subject.starts_with("Re: ") {
                original.subject.clone()
            } else {
                format!("Re: {}", original.subject)
            },
            body_text: body,
            body_html: None,
            message_id: None,
            in_reply_to: original.message_id.clone(),
            references: None,
            created_at: chrono::Utc::now(),
        };

        // Build references chain
        if let Some(original_refs) = &original.references {
            if let Some(original_id) = &original.message_id {
                reply.references = Some(format!("{} {}", original_refs, original_id));
            } else {
                reply.references = Some(original_refs.clone());
            }
        } else if let Some(original_id) = &original.message_id {
            reply.references = Some(original_id.clone());
        }

        // Filter out our own address from recipients if reply_all
        if reply_all {
            reply.to.retain(|addr| addr != &from_address);
            reply.cc.retain(|addr| addr != &from_address);
        }

        reply
    }

    /// Create a forward message
    pub fn create_forward(original: &EmailMessage, from_address: String, body: String) -> Self {
        Self {
            from: from_address,
            to: Vec::new(),
            cc: Vec::new(),
            bcc: Vec::new(),
            subject: if original.subject.starts_with("Fwd: ") {
                original.subject.clone()
            } else {
                format!("Fwd: {}", original.subject)
            },
            body_text: body,
            body_html: None,
            message_id: None,
            in_reply_to: None,
            references: None,
            created_at: chrono::Utc::now(),
        }
    }

    /// Convert to lettre Message
    pub fn to_lettre_message(&self) -> SmtpResult<Message> {
        let mut builder = MessageBuilder::new()
            .from_str(&self.from)?
            .subject(&self.subject)
            .body_text(&self.body_text);

        // Add recipients
        if !self.to.is_empty() {
            builder = builder.to_str(&self.to.join(", "))?;
        }

        if !self.cc.is_empty() {
            builder = builder.cc_str(&self.cc.join(", "))?;
        }

        if !self.bcc.is_empty() {
            builder = builder.bcc_str(&self.bcc.join(", "))?;
        }

        // Add optional headers
        if let Some(ref html) = self.body_html {
            builder = builder.body_html(html);
        }

        if let Some(ref msg_id) = self.message_id {
            builder = builder.message_id(msg_id);
        }

        if let Some(ref reply_to) = self.in_reply_to {
            builder = builder.in_reply_to(reply_to);
        }

        if let Some(ref refs) = self.references {
            builder = builder.references(refs);
        }

        builder.build()
    }

    /// Generate a unique message ID
    pub fn generate_message_id(&mut self, domain: &str) {
        let uuid = uuid::Uuid::new_v4();
        self.message_id = Some(format!("<{}@{}>", uuid, domain));
    }

    /// Get all recipients (to + cc + bcc)
    pub fn all_recipients(&self) -> Vec<String> {
        let mut recipients = Vec::new();
        recipients.extend(self.to.clone());
        recipients.extend(self.cc.clone());
        recipients.extend(self.bcc.clone());
        recipients
    }

    /// Create RSVP response email for calendar invitations
    pub fn create_rsvp_response(
        from_address: String,
        organizer_email: String,
        meeting_title: &str,
        meeting_uid: &str,
        response: &str, // "ACCEPTED", "DECLINED", "TENTATIVE"
        comment: Option<String>,
        original_request_ical: &str,
    ) -> SmtpResult<Self> {
        // Validate response type
        let response_upper = response.to_uppercase();
        if !["ACCEPTED", "DECLINED", "TENTATIVE"].contains(&response_upper.as_str()) {
            return Err(SmtpError::MessageFormatError(format!(
                "Invalid RSVP response: {}",
                response
            )));
        }

        // Create subject line
        let subject = match response_upper.as_str() {
            "ACCEPTED" => format!("Accepted: {}", meeting_title),
            "DECLINED" => format!("Declined: {}", meeting_title),
            "TENTATIVE" => format!("Tentative: {}", meeting_title),
            _ => format!("Response: {}", meeting_title),
        };

        // Create body text
        let body_text = format!(
            "This is an automated response to your meeting invitation.\n\n\
            Meeting: {}\n\
            Response: {}\n\
            {}\n\n\
            This message was sent automatically by Comunicado.",
            meeting_title,
            response_upper,
            comment.as_deref().unwrap_or("")
        );

        // Create iCalendar REPLY content
        let reply_ical = Self::create_ical_reply(
            meeting_uid,
            &from_address,
            &response_upper,
            original_request_ical,
        )?;

        // Create the message with iCalendar attachment
        let mut message = Self {
            from: from_address,
            to: vec![organizer_email],
            cc: Vec::new(),
            bcc: Vec::new(),
            subject,
            body_text,
            body_html: None,
            message_id: None,
            in_reply_to: None,
            references: None,
            created_at: chrono::Utc::now(),
        };

        // Generate a unique message ID
        message.generate_message_id("comunicado.local");

        // For now, include the iCalendar in the body - later we can add proper attachment support
        message.body_text = format!(
            "{}\n\n--- iCalendar Reply ---\n{}",
            message.body_text, reply_ical
        );

        Ok(message)
    }

    /// Create iCalendar REPLY content for RSVP
    fn create_ical_reply(
        meeting_uid: &str,
        attendee_email: &str,
        response: &str, // "ACCEPTED", "DECLINED", "TENTATIVE"
        _original_request_ical: &str,
    ) -> SmtpResult<String> {
        let now = chrono::Utc::now();
        let timestamp = now.format("%Y%m%dT%H%M%SZ").to_string();

        // Map response to iCalendar PARTSTAT
        let partstat = match response {
            "ACCEPTED" => "ACCEPTED",
            "DECLINED" => "DECLINED",
            "TENTATIVE" => "TENTATIVE",
            _ => "NEEDS-ACTION",
        };

        // Create basic iCalendar REPLY
        let ical_reply = format!(
            "BEGIN:VCALENDAR\r\n\
            VERSION:2.0\r\n\
            PRODID:-//Comunicado//Calendar//EN\r\n\
            METHOD:REPLY\r\n\
            BEGIN:VEVENT\r\n\
            UID:{}\r\n\
            DTSTAMP:{}\r\n\
            ATTENDEE;PARTSTAT={}:mailto:{}\r\n\
            END:VEVENT\r\n\
            END:VCALENDAR\r\n",
            meeting_uid, timestamp, partstat, attendee_email
        );

        Ok(ical_reply)
    }

    /// Validate the message
    pub fn validate(&self) -> SmtpResult<()> {
        if self.from.is_empty() {
            return Err(SmtpError::MessageFormatError(
                "From address is required".to_string(),
            ));
        }

        if self.all_recipients().is_empty() {
            return Err(SmtpError::MessageFormatError(
                "At least one recipient is required".to_string(),
            ));
        }

        if self.subject.is_empty() {
            return Err(SmtpError::MessageFormatError(
                "Subject is required".to_string(),
            ));
        }

        // Validate email addresses
        for addr in std::iter::once(&self.from).chain(self.all_recipients().iter()) {
            if !is_valid_email(addr) {
                return Err(SmtpError::InvalidAddress(addr.clone()));
            }
        }

        Ok(())
    }
}

/// Parse a single email address or mailbox
fn parse_mailbox(address: &str) -> SmtpResult<Mailbox> {
    let trimmed = address.trim();

    if trimmed.is_empty() {
        return Err(SmtpError::InvalidAddress("Empty address".to_string()));
    }

    // Handle "Name <email@domain.com>" format
    if let Some(start) = trimmed.find('<') {
        if let Some(end) = trimmed.find('>') {
            let name = trimmed[..start].trim().trim_matches('"');
            let email = &trimmed[start + 1..end];

            let addr = Address::from_str(email)?;
            if name.is_empty() {
                return Ok(Mailbox::new(None, addr));
            } else {
                return Ok(Mailbox::new(Some(name.to_string()), addr));
            }
        }
    }

    // Handle plain email address
    let addr = Address::from_str(trimmed)?;
    Ok(Mailbox::new(None, addr))
}

/// Parse a comma-separated list of email addresses
fn parse_address_list(addresses: &str) -> SmtpResult<Vec<Mailbox>> {
    if addresses.trim().is_empty() {
        return Ok(Vec::new());
    }

    let mut result = Vec::new();
    for addr in addresses.split(',') {
        let mailbox = parse_mailbox(addr)?;
        result.push(mailbox);
    }

    Ok(result)
}

/// Basic email validation
fn is_valid_email(email: &str) -> bool {
    email.contains('@') && email.len() > 3 && !email.starts_with('@') && !email.ends_with('@')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mailbox() {
        // Test plain email
        let mailbox = parse_mailbox("test@example.com").unwrap();
        assert_eq!(mailbox.email.to_string(), "test@example.com");
        assert!(mailbox.name.is_none());

        // Test named email
        let mailbox = parse_mailbox("John Doe <john@example.com>").unwrap();
        assert_eq!(mailbox.email.to_string(), "john@example.com");
        assert_eq!(mailbox.name.as_ref().unwrap(), "John Doe");

        // Test quoted name
        let mailbox = parse_mailbox("\"Jane Smith\" <jane@example.com>").unwrap();
        assert_eq!(mailbox.email.to_string(), "jane@example.com");
        assert_eq!(mailbox.name.as_ref().unwrap(), "Jane Smith");
    }

    #[test]
    fn test_parse_address_list() {
        let addresses =
            parse_address_list("test1@example.com, John <test2@example.com>, test3@example.com")
                .unwrap();
        assert_eq!(addresses.len(), 3);
        assert_eq!(addresses[0].email.to_string(), "test1@example.com");
        assert_eq!(addresses[1].email.to_string(), "test2@example.com");
        assert_eq!(addresses[1].name.as_ref().unwrap(), "John");
        assert_eq!(addresses[2].email.to_string(), "test3@example.com");
    }

    #[test]
    fn test_email_validation() {
        assert!(is_valid_email("test@example.com"));
        assert!(is_valid_email("user.name+tag@domain.co.uk"));
        assert!(!is_valid_email("invalid"));
        assert!(!is_valid_email("@example.com"));
        assert!(!is_valid_email("test@"));
        assert!(!is_valid_email(""));
    }

    #[test]
    fn test_message_builder() {
        let message = MessageBuilder::new()
            .from_str("sender@example.com")
            .unwrap()
            .to_str("recipient@example.com")
            .unwrap()
            .subject("Test Subject")
            .body_text("Test body")
            .build()
            .unwrap();

        // Verify message was built successfully
        assert!(message.headers().get_raw("Subject").is_some());
        assert!(message.headers().get_raw("From").is_some());
        assert!(message.headers().get_raw("To").is_some());
    }

    #[test]
    fn test_email_message_validation() {
        let mut msg = EmailMessage::new(
            "sender@example.com".to_string(),
            vec!["recipient@example.com".to_string()],
            "Test".to_string(),
            "Body".to_string(),
        );

        assert!(msg.validate().is_ok());

        // Test empty subject
        msg.subject = String::new();
        assert!(msg.validate().is_err());

        // Test empty recipients
        msg.subject = "Test".to_string();
        msg.to.clear();
        assert!(msg.validate().is_err());
    }
}
