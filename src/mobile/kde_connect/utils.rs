use regex::Regex;
use once_cell::sync::Lazy;
use chrono::{DateTime, Utc};

/// Format a phone number for display
pub fn format_phone_number(number: &str) -> String {
    static PHONE_REGEX: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^\+?1?([0-9]{3})([0-9]{3})([0-9]{4})$").unwrap()
    });

    // Remove all non-digits except leading +
    let mut cleaned = String::new();
    let mut chars = number.chars();
    
    if number.starts_with('+') {
        cleaned.push('+');
        chars.next(); // Skip the +
    }
    
    for c in chars {
        if c.is_ascii_digit() {
            cleaned.push(c);
        }
    }

    // Try to format as US number
    if let Some(captures) = PHONE_REGEX.captures(&cleaned) {
        let area = captures.get(1).map_or("", |m| m.as_str());
        let exchange = captures.get(2).map_or("", |m| m.as_str());
        let number = captures.get(3).map_or("", |m| m.as_str());
        
        if cleaned.starts_with('+') {
            format!("+1 ({}) {}-{}", area, exchange, number)
        } else {
            format!("({}) {}-{}", area, exchange, number)
        }
    } else if cleaned.starts_with("+1") && cleaned.len() == 12 {
        // Handle +1XXXXXXXXXX format
        let area = &cleaned[2..5];
        let exchange = &cleaned[5..8];
        let number = &cleaned[8..12];
        format!("+1 ({}) {}-{}", area, exchange, number)
    } else {
        // Return original if we can't format
        number.to_string()
    }
}

/// Parse timestamp from KDE Connect message (milliseconds to DateTime)
pub fn parse_message_timestamp(timestamp_ms: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(timestamp_ms / 1000, ((timestamp_ms % 1000) * 1_000_000) as u32)
        .unwrap_or_else(|| Utc::now())
}

/// Sanitize message content for safe display
pub fn sanitize_message_content(content: &str) -> String {
    static URL_REGEX: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"https?://[^\s]+").unwrap()
    });

    let mut sanitized = content.to_string();
    
    // Remove potentially harmful characters
    sanitized = sanitized
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect();
    
    // Limit length to prevent UI issues
    if sanitized.len() > 5000 {
        sanitized.truncate(4997);
        sanitized.push_str("...");
    }
    
    // Highlight URLs (simple detection)
    sanitized = URL_REGEX.replace_all(&sanitized, "[URL: $0]").to_string();
    
    sanitized
}

/// Extract contact name from address if it contains both
pub fn extract_contact_info(address: &str) -> (String, Option<String>) {
    static NAME_PHONE_REGEX: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^(.+?)\s*<(.+?)>$").unwrap()
    });
    
    if let Some(captures) = NAME_PHONE_REGEX.captures(address) {
        let name = captures.get(1).map_or("", |m| m.as_str()).trim();
        let phone = captures.get(2).map_or("", |m| m.as_str()).trim();
        
        (phone.to_string(), Some(name.to_string()))
    } else {
        (address.to_string(), None)
    }
}

/// Validate phone number format
pub fn is_valid_phone_number(number: &str) -> bool {
    static PHONE_VALIDATION_REGEX: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^\+?[1-9]\d{1,14}$").unwrap()
    });
    
    let cleaned: String = number.chars()
        .filter(|c| c.is_ascii_digit() || *c == '+')
        .collect();
    
    PHONE_VALIDATION_REGEX.is_match(&cleaned)
}

/// Generate display name for conversation from addresses
pub fn generate_conversation_display_name(addresses: &[crate::mobile::kde_connect::types::ContactInfo]) -> String {
    if addresses.is_empty() {
        return "Unknown".to_string();
    }
    
    if addresses.len() == 1 {
        return addresses[0].display_text().to_string();
    }
    
    // For group conversations, show first few names
    let names: Vec<&str> = addresses.iter()
        .take(3)
        .map(|contact| contact.display_text())
        .collect();
    
    if addresses.len() > 3 {
        format!("{} and {} others", names.join(", "), addresses.len() - 3)
    } else {
        names.join(", ")
    }
}

/// Estimate SMS segment count (standard SMS is 160 characters)
pub fn calculate_sms_segments(message: &str) -> usize {
    let char_count = message.chars().count();
    
    // Check if message contains non-GSM characters
    let has_unicode = message.chars().any(|c| {
        !matches!(c, 
            'A'..='Z' | 'a'..='z' | '0'..='9' | 
            ' ' | '!' | '"' | '#' | '$' | '%' | '&' | '\'' | 
            '(' | ')' | '*' | '+' | ',' | '-' | '.' | '/' |
            ':' | ';' | '<' | '=' | '>' | '?' | '@' |
            '[' | '\\' | ']' | '^' | '_' | '`' | '{' | '|' | '}' | '~'
        )
    });
    
    let segment_size = if has_unicode { 67 } else { 160 };
    
    if char_count <= segment_size {
        1
    } else {
        // For multi-part messages, each segment is slightly smaller
        let multi_segment_size = if has_unicode { 67 } else { 153 };
        ((char_count - 1) / multi_segment_size) + 1
    }
}

/// Truncate message for preview display
pub fn truncate_for_preview(message: &str, max_length: usize, preserve_words: bool) -> String {
    if message.len() <= max_length {
        return message.to_string();
    }
    
    if preserve_words {
        // Find the last space before the limit
        let truncate_pos = message[..max_length].rfind(' ').unwrap_or(max_length);
        format!("{}...", &message[..truncate_pos])
    } else {
        format!("{}...", &message[..max_length])
    }
}

// D-Bus utility functions - for future real KDE Connect implementation
// Currently not used by the simple client implementation

// D-Bus utility functions for future real KDE Connect D-Bus integration
// These would be used when implementing proper D-Bus message monitoring

/// Convert D-Bus variant to string safely
#[cfg(feature = "kde-connect")]
pub fn dbus_variant_to_string(_variant: &str) -> Option<String> {
    // Would be implemented with proper D-Bus variant parsing
    None
}

/// Convert D-Bus variant to i64 safely  
#[cfg(feature = "kde-connect")]
pub fn dbus_variant_to_i64(_variant: &str) -> Option<i64> {
    // Would be implemented with proper D-Bus variant parsing
    None
}

/// Convert D-Bus variant to boolean safely
#[cfg(feature = "kde-connect")]
pub fn dbus_variant_to_bool(_variant: &str) -> Option<bool> {
    // Would be implemented with proper D-Bus variant parsing
    None
}

/// Escape string for D-Bus message
pub fn escape_dbus_string(input: &str) -> String {
    input.replace('\\', "\\\\")
         .replace('"', "\\\"")
         .replace('\n', "\\n")
         .replace('\r', "\\r")
         .replace('\t', "\\t")
}

/// Format relative time (e.g., "2 minutes ago", "1 hour ago")
pub fn format_relative_time(timestamp: i64) -> String {
    let now = Utc::now().timestamp() * 1000; // Convert to milliseconds
    let diff_ms = now - timestamp;
    let diff_seconds = diff_ms / 1000;
    
    if diff_seconds < 60 {
        "Just now".to_string()
    } else if diff_seconds < 3600 {
        let minutes = diff_seconds / 60;
        if minutes == 1 {
            "1 minute ago".to_string()
        } else {
            format!("{} minutes ago", minutes)
        }
    } else if diff_seconds < 86400 {
        let hours = diff_seconds / 3600;
        if hours == 1 {
            "1 hour ago".to_string()
        } else {
            format!("{} hours ago", hours)
        }
    } else {
        let days = diff_seconds / 86400;
        if days == 1 {
            "1 day ago".to_string()
        } else if days < 7 {
            format!("{} days ago", days)
        } else {
            // For older messages, show the actual date
            let datetime = parse_message_timestamp(timestamp);
            datetime.format("%Y-%m-%d").to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mobile::kde_connect::types::ContactInfo;

    #[test]
    fn test_format_phone_number() {
        assert_eq!(format_phone_number("+11234567890"), "+1 (123) 456-7890");
        assert_eq!(format_phone_number("1234567890"), "(123) 456-7890");
        assert_eq!(format_phone_number("123-456-7890"), "(123) 456-7890");
        assert_eq!(format_phone_number("(123) 456-7890"), "(123) 456-7890");
        assert_eq!(format_phone_number("invalid"), "invalid");
    }

    #[test]
    fn test_parse_message_timestamp() {
        let timestamp_ms = 1640995200000; // 2022-01-01 00:00:00 UTC
        let datetime = parse_message_timestamp(timestamp_ms);
        assert_eq!(datetime.format("%Y-%m-%d").to_string(), "2022-01-01");
    }

    #[test]
    fn test_sanitize_message_content() {
        let content = "Hello\x00World\nThis is a test\thttps://example.com";
        let sanitized = sanitize_message_content(content);
        
        assert!(!sanitized.contains('\x00'));
        assert!(sanitized.contains('\n'));
        assert!(sanitized.contains('\t'));
        assert!(sanitized.contains("[URL: https://example.com]"));
    }

    #[test]
    fn test_extract_contact_info() {
        let (phone, name) = extract_contact_info("John Doe <+1234567890>");
        assert_eq!(phone, "+1234567890");
        assert_eq!(name, Some("John Doe".to_string()));

        let (phone, name) = extract_contact_info("+1234567890");
        assert_eq!(phone, "+1234567890");
        assert_eq!(name, None);
    }

    #[test]
    fn test_is_valid_phone_number() {
        assert!(is_valid_phone_number("+1234567890"));
        assert!(is_valid_phone_number("1234567890"));
        assert!(!is_valid_phone_number("123"));
        assert!(!is_valid_phone_number("abc"));
        assert!(!is_valid_phone_number(""));
    }

    #[test]
    fn test_generate_conversation_display_name() {
        let contacts = vec![
            ContactInfo::new("+1111111111".to_string(), Some("Alice".to_string())),
            ContactInfo::new("+2222222222".to_string(), Some("Bob".to_string())),
        ];
        
        assert_eq!(generate_conversation_display_name(&contacts), "Alice, Bob");
        
        let single_contact = vec![
            ContactInfo::new("+1111111111".to_string(), Some("Alice".to_string())),
        ];
        
        assert_eq!(generate_conversation_display_name(&single_contact), "Alice");
        
        assert_eq!(generate_conversation_display_name(&[]), "Unknown");
    }

    #[test]
    fn test_calculate_sms_segments() {
        // Standard SMS (160 chars)
        let short_message = "Hello";
        assert_eq!(calculate_sms_segments(short_message), 1);
        
        // Long ASCII message
        let long_message = "a".repeat(200);
        assert_eq!(calculate_sms_segments(&long_message), 2);
        
        // Unicode message (67 chars per segment)
        let unicode_message = "Hello 🌍";
        assert_eq!(calculate_sms_segments(unicode_message), 1);
        
        let long_unicode = "🌍".repeat(70);
        assert_eq!(calculate_sms_segments(&long_unicode), 2);
    }

    #[test]
    fn test_truncate_for_preview() {
        let message = "This is a long message that should be truncated";
        
        let truncated = truncate_for_preview(message, 20, false);
        assert_eq!(truncated, "This is a long messa...");
        
        let word_preserved = truncate_for_preview(message, 20, true);
        assert_eq!(word_preserved, "This is a long...");
        
        let short = truncate_for_preview("Short", 20, true);
        assert_eq!(short, "Short");
    }

    #[test]
    fn test_escape_dbus_string() {
        let input = "Hello \"World\"\nNew line\tTab\\Backslash";
        let escaped = escape_dbus_string(input);
        
        assert!(escaped.contains("\\\""));
        assert!(escaped.contains("\\n"));
        assert!(escaped.contains("\\t"));
        assert!(escaped.contains("\\\\"));
    }

    #[test]
    fn test_format_relative_time() {
        let now = Utc::now().timestamp() * 1000;
        
        // Recent message
        let recent = now - 30 * 1000; // 30 seconds ago
        assert_eq!(format_relative_time(recent), "Just now");
        
        // 5 minutes ago
        let minutes_ago = now - 5 * 60 * 1000;
        assert_eq!(format_relative_time(minutes_ago), "5 minutes ago");
        
        // 2 hours ago
        let hours_ago = now - 2 * 60 * 60 * 1000;
        assert_eq!(format_relative_time(hours_ago), "2 hours ago");
    }
}