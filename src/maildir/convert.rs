use crate::email::database::StoredMessage;
use crate::maildir::types::{MaildirError, MaildirFlag, MaildirResult};
use crate::ui::content_preview::ContentType;
/// Conversion utilities between Maildir and StoredMessage formats
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Represents a simplified message structure for Maildir operations
#[derive(Debug, Clone)]
pub struct MaildirMessage {
    pub id: String,
    pub message_id: Option<String>,
    pub account_id: String,
    pub folder_name: String,
    pub from_addr: String,
    pub from_name: Option<String>,
    pub to_addrs: Vec<String>,
    pub cc_addrs: Vec<String>,
    pub bcc_addrs: Vec<String>,
    pub subject: String,
    pub body_text: Option<String>,
    pub body_html: Option<String>,
    pub content_type: ContentType,
    pub date: DateTime<Utc>,
    pub is_read: bool,
    pub is_flagged: bool,
    pub has_attachments: bool,
    pub in_reply_to: Option<String>,
    pub references: Vec<String>,
}

impl MaildirMessage {
    /// Convert from database StoredMessage to MaildirMessage
    pub fn from_stored_message(stored: &StoredMessage) -> Self {
        // Determine content type based on available body types
        let content_type = if stored.body_html.is_some() {
            ContentType::Html
        } else {
            ContentType::PlainText
        };

        // Check flags for read/flagged status
        let is_read = stored.flags.contains(&"\\Seen".to_string());
        let is_flagged = stored.flags.contains(&"\\Flagged".to_string());
        let has_attachments = !stored.attachments.is_empty();

        Self {
            id: stored.id.to_string(),
            message_id: stored.message_id.clone(),
            account_id: stored.account_id.clone(),
            folder_name: stored.folder_name.clone(),
            from_addr: stored.from_addr.clone(),
            from_name: stored.from_name.clone(),
            to_addrs: stored.to_addrs.clone(),
            cc_addrs: stored.cc_addrs.clone(),
            bcc_addrs: stored.bcc_addrs.clone(),
            subject: stored.subject.clone(),
            body_text: stored.body_text.clone(),
            body_html: stored.body_html.clone(),
            content_type,
            date: stored.date,
            is_read,
            is_flagged,
            has_attachments,
            in_reply_to: stored.in_reply_to.clone(),
            references: stored.references.clone(),
        }
    }

    /// Convert to database StoredMessage
    pub fn to_stored_message(&self) -> MaildirResult<StoredMessage> {
        let id = Uuid::parse_str(&self.id)
            .map_err(|_| MaildirError::MessageParsing(format!("Invalid UUID: {}", self.id)))?;

        // Convert flags
        let mut flags = Vec::new();
        if self.is_read {
            flags.push("\\Seen".to_string());
        }
        if self.is_flagged {
            flags.push("\\Flagged".to_string());
        }

        Ok(StoredMessage {
            id,
            account_id: self.account_id.clone(),
            folder_name: self.folder_name.clone(),
            imap_uid: 0, // Will be set by IMAP sync
            message_id: self.message_id.clone(),
            thread_id: None, // Will be calculated
            in_reply_to: self.in_reply_to.clone(),
            references: self.references.clone(),
            subject: self.subject.clone(),
            from_addr: self.from_addr.clone(),
            from_name: self.from_name.clone(),
            to_addrs: self.to_addrs.clone(),
            cc_addrs: self.cc_addrs.clone(),
            bcc_addrs: self.bcc_addrs.clone(),
            reply_to: None,
            date: self.date,
            body_text: self.body_text.clone(),
            body_html: self.body_html.clone(),
            attachments: Vec::new(), // Attachments would be parsed separately
            flags,
            labels: Vec::new(),
            size: None,
            priority: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_synced: Utc::now(),
            sync_version: 1,
            is_draft: false,
            is_deleted: false,
        })
    }

    /// Get the email body based on content type preference
    pub fn get_body(&self) -> String {
        match self.content_type {
            ContentType::Html => self
                .body_html
                .clone()
                .unwrap_or_else(|| self.body_text.clone().unwrap_or_default()),
            _ => self
                .body_text
                .clone()
                .unwrap_or_else(|| self.body_html.clone().unwrap_or_default()),
        }
    }

    /// Convert Maildir flags to our internal flags
    pub fn from_maildir_flags(flags: &[MaildirFlag]) -> (bool, bool) {
        let is_read = flags.contains(&MaildirFlag::Seen);
        let is_flagged = flags.contains(&MaildirFlag::Flagged);
        (is_read, is_flagged)
    }

    /// Convert internal flags to Maildir flags
    pub fn to_maildir_flags(&self) -> Vec<MaildirFlag> {
        let mut flags = Vec::new();

        if self.is_read {
            flags.push(MaildirFlag::Seen);
        }

        if self.is_flagged {
            flags.push(MaildirFlag::Flagged);
        }

        flags
    }

    /// Generate email content in RFC 5322 format for Maildir export
    pub fn to_rfc5322(&self) -> String {
        let mut content = String::new();

        // Write headers
        if let Some(ref message_id) = self.message_id {
            content.push_str(&format!("Message-ID: {}\n", message_id));
        }

        content.push_str(&format!("From: {}", self.from_addr));
        if let Some(ref from_name) = self.from_name {
            content = content.replace(
                &format!("From: {}", self.from_addr),
                &format!("From: \"{}\" <{}>", from_name, self.from_addr),
            );
        }
        content.push('\n');

        if !self.to_addrs.is_empty() {
            content.push_str(&format!("To: {}\n", self.to_addrs.join(", ")));
        }

        if !self.cc_addrs.is_empty() {
            content.push_str(&format!("Cc: {}\n", self.cc_addrs.join(", ")));
        }

        if !self.bcc_addrs.is_empty() {
            content.push_str(&format!("Bcc: {}\n", self.bcc_addrs.join(", ")));
        }

        content.push_str(&format!("Subject: {}\n", self.subject));
        content.push_str(&format!(
            "Date: {}\n",
            self.date.format("%a, %d %b %Y %H:%M:%S %z")
        ));

        // Add content type
        let content_type_header = match self.content_type {
            ContentType::Html => "text/html; charset=utf-8",
            ContentType::PlainText => "text/plain; charset=utf-8",
            ContentType::Markdown => "text/plain; charset=utf-8",
            ContentType::Code(_) => "text/plain; charset=utf-8",
        };
        content.push_str(&format!("Content-Type: {}\n", content_type_header));

        // Add transfer encoding
        content.push_str("Content-Transfer-Encoding: 8bit\n");

        // Add MIME version
        content.push_str("MIME-Version: 1.0\n");

        // Add In-Reply-To if present
        if let Some(ref in_reply_to) = self.in_reply_to {
            content.push_str(&format!("In-Reply-To: {}\n", in_reply_to));
        }

        // Add References if present
        if !self.references.is_empty() {
            content.push_str(&format!("References: {}\n", self.references.join(" ")));
        }

        // Add User-Agent
        content.push_str("User-Agent: Comunicado Email Client\n");

        // Add separator between headers and body
        content.push_str("\n");

        // Add body
        content.push_str(&self.get_body());

        content
    }
}

/// Convert from raw email text to MaildirMessage
pub fn parse_raw_email(
    raw_content: &str,
    account_id: &str,
    folder_name: &str,
) -> MaildirResult<MaildirMessage> {
    // Split headers and body
    let parts: Vec<&str> = raw_content.splitn(2, "\n\n").collect();
    if parts.len() < 2 {
        return Err(MaildirError::MessageParsing(
            "Invalid email format: no body separator found".to_string(),
        ));
    }

    let headers_text = parts[0];
    let body = parts[1];

    // Parse headers
    let headers = parse_headers(headers_text)?;

    // Determine content type
    let content_type = determine_content_type(&headers);

    // Generate a unique ID if none exists
    let id = Uuid::new_v4().to_string();

    // Parse addresses
    let to_addrs = parse_address_list(headers.get("To").map_or("", |v| v));
    let cc_addrs = parse_address_list(headers.get("Cc").map_or("", |v| v));
    let bcc_addrs = parse_address_list(headers.get("Bcc").map_or("", |v| v));

    // Parse from address
    let (from_addr, from_name) = parse_from_address(headers.get("From").map_or("Unknown", |v| v));

    // Parse date
    let date = parse_date(headers.get("Date").map_or("", |v| v)).unwrap_or_else(Utc::now);

    // Parse references
    let references = parse_references(headers.get("References").map_or("", |v| v));

    let message = MaildirMessage {
        id,
        message_id: headers.get("Message-ID").cloned(),
        account_id: account_id.to_string(),
        folder_name: folder_name.to_string(),
        from_addr,
        from_name,
        to_addrs,
        cc_addrs,
        bcc_addrs,
        subject: headers
            .get("Subject")
            .map_or("(No Subject)", |v| v)
            .to_string(),
        body_text: if content_type == ContentType::PlainText {
            Some(body.to_string())
        } else {
            None
        },
        body_html: if content_type == ContentType::Html {
            Some(body.to_string())
        } else {
            None
        },
        content_type,
        date,
        is_read: false,         // Will be set based on Maildir flags
        is_flagged: false,      // Will be set based on Maildir flags
        has_attachments: false, // Will be determined during parsing
        in_reply_to: headers.get("In-Reply-To").cloned(),
        references,
    };

    Ok(message)
}

// Helper functions

fn parse_headers(headers_text: &str) -> MaildirResult<std::collections::HashMap<String, String>> {
    let mut headers = std::collections::HashMap::new();
    let mut current_header = String::new();
    let mut current_value = String::new();

    for line in headers_text.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            // Continuation of previous header
            current_value.push('\n');
            current_value.push_str(line.trim());
        } else if let Some(colon_pos) = line.find(':') {
            // New header
            if !current_header.is_empty() {
                headers.insert(current_header.clone(), current_value.trim().to_string());
            }

            current_header = line[..colon_pos].trim().to_string();
            current_value = line[colon_pos + 1..].trim().to_string();
        }
    }

    // Don't forget the last header
    if !current_header.is_empty() {
        headers.insert(current_header, current_value.trim().to_string());
    }

    Ok(headers)
}

fn determine_content_type(headers: &std::collections::HashMap<String, String>) -> ContentType {
    if let Some(content_type) = headers.get("Content-Type") {
        let content_type_lower = content_type.to_lowercase();
        if content_type_lower.contains("text/html") {
            ContentType::Html
        } else if content_type_lower.contains("text/plain") {
            ContentType::PlainText
        } else {
            ContentType::PlainText
        }
    } else {
        ContentType::PlainText
    }
}

fn parse_address_list(addresses: &str) -> Vec<String> {
    if addresses.trim().is_empty() {
        return Vec::new();
    }

    addresses
        .split(',')
        .map(|addr| addr.trim().to_string())
        .filter(|addr| !addr.is_empty())
        .collect()
}

fn parse_from_address(from: &str) -> (String, Option<String>) {
    // Simple parsing of "Name <email>" format
    if let Some(start) = from.find('<') {
        if let Some(end) = from.find('>') {
            let name = from[..start].trim().trim_matches('"');
            let email = from[start + 1..end].trim();
            if name.is_empty() {
                (email.to_string(), None)
            } else {
                (email.to_string(), Some(name.to_string()))
            }
        } else {
            (from.to_string(), None)
        }
    } else {
        (from.to_string(), None)
    }
}

fn parse_date(date_str: &str) -> Option<DateTime<Utc>> {
    if date_str.is_empty() {
        return None;
    }

    // Try parsing RFC 2822 format first
    if let Ok(parsed) = DateTime::parse_from_rfc2822(date_str) {
        return Some(parsed.with_timezone(&Utc));
    }

    // Try parsing RFC 3339 format
    if let Ok(parsed) = DateTime::parse_from_rfc3339(date_str) {
        return Some(parsed.with_timezone(&Utc));
    }

    None
}

fn parse_references(references: &str) -> Vec<String> {
    if references.trim().is_empty() {
        return Vec::new();
    }

    references
        .split_whitespace()
        .map(|r| r.trim().to_string())
        .filter(|r| !r.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_from_address() {
        let (addr, name) = parse_from_address("John Doe <john@example.com>");
        assert_eq!(addr, "john@example.com");
        assert_eq!(name, Some("John Doe".to_string()));

        let (addr, name) = parse_from_address("john@example.com");
        assert_eq!(addr, "john@example.com");
        assert_eq!(name, None);
    }

    #[test]
    fn test_parse_address_list() {
        let addrs = parse_address_list("john@example.com, jane@example.com");
        assert_eq!(
            addrs,
            vec![
                "john@example.com".to_string(),
                "jane@example.com".to_string()
            ]
        );

        let addrs = parse_address_list("");
        assert!(addrs.is_empty());
    }

    #[test]
    fn test_determine_content_type() {
        let mut headers = std::collections::HashMap::new();

        headers.insert("Content-Type".to_string(), "text/plain".to_string());
        assert_eq!(determine_content_type(&headers), ContentType::PlainText);

        headers.insert("Content-Type".to_string(), "text/html".to_string());
        assert_eq!(determine_content_type(&headers), ContentType::Html);
    }
}
