use chrono::{DateTime, FixedOffset, Local, Offset, TimeZone, Utc};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// Errors that can occur during timestamp operations
#[derive(Error, Debug)]
pub enum TimestampError {
    #[error("Invalid timestamp: {0}")]
    InvalidTimestamp(String),
    
    #[error("Timezone conversion error: {0}")]
    TimezoneError(String),
    
    #[error("File system error: {0}")]
    FileSystemError(#[from] std::io::Error),
    
    #[error("Parse error: {0}")]
    ParseError(String),
}

pub type TimestampResult<T> = Result<T, TimestampError>;

/// Utility functions for handling timestamps in Maildir operations
pub struct TimestampUtils;

impl TimestampUtils {
    /// Preserve the original timestamp of a file when copying/moving
    pub fn preserve_file_timestamp<P: AsRef<Path>, Q: AsRef<Path>>(
        source: P,
        destination: Q,
    ) -> TimestampResult<()> {
        let source_metadata = fs::metadata(&source)?;
        let modified_time = source_metadata.modified()?;
        let accessed_time = source_metadata.accessed().unwrap_or(modified_time);

        // Set the timestamps on the destination file
        let destination = destination.as_ref();
        if destination.exists() {
            filetime::set_file_times(
                destination,
                filetime::FileTime::from_system_time(accessed_time),
                filetime::FileTime::from_system_time(modified_time),
            )
            .map_err(|e| TimestampError::FileSystemError(e.into()))?;
        }

        Ok(())
    }

    /// Convert a DateTime to Unix timestamp
    pub fn datetime_to_timestamp(datetime: &DateTime<Utc>) -> i64 {
        datetime.timestamp()
    }

    /// Convert Unix timestamp to DateTime<Utc>
    pub fn timestamp_to_datetime(timestamp: i64) -> TimestampResult<DateTime<Utc>> {
        DateTime::from_timestamp(timestamp, 0)
            .ok_or_else(|| TimestampError::InvalidTimestamp(timestamp.to_string()))
    }

    /// Convert DateTime to local timezone
    pub fn utc_to_local(utc_datetime: &DateTime<Utc>) -> DateTime<Local> {
        utc_datetime.with_timezone(&Local)
    }

    /// Convert local DateTime to UTC
    pub fn local_to_utc(local_datetime: &DateTime<Local>) -> DateTime<Utc> {
        local_datetime.with_timezone(&Utc)
    }

    /// Parse RFC2822 date string to DateTime<Utc>
    pub fn parse_rfc2822(date_str: &str) -> TimestampResult<DateTime<Utc>> {
        DateTime::parse_from_rfc2822(date_str)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| TimestampError::ParseError(format!("RFC2822 parse error: {}", e)))
    }

    /// Parse RFC3339 date string to DateTime<Utc>
    pub fn parse_rfc3339(date_str: &str) -> TimestampResult<DateTime<Utc>> {
        DateTime::parse_from_rfc3339(date_str)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| TimestampError::ParseError(format!("RFC3339 parse error: {}", e)))
    }

    /// Format DateTime<Utc> as RFC2822 string
    pub fn format_rfc2822(datetime: &DateTime<Utc>) -> String {
        datetime.format("%a, %d %b %Y %H:%M:%S %z").to_string()
    }

    /// Format DateTime<Utc> as RFC3339 string
    pub fn format_rfc3339(datetime: &DateTime<Utc>) -> String {
        datetime.to_rfc3339()
    }

    /// Get current system time as DateTime<Utc>
    pub fn now_utc() -> DateTime<Utc> {
        Utc::now()
    }

    /// Get current system time as DateTime<Local>
    pub fn now_local() -> DateTime<Local> {
        Local::now()
    }

    /// Convert timezone offset to FixedOffset
    pub fn offset_from_seconds(offset_seconds: i32) -> TimestampResult<FixedOffset> {
        FixedOffset::east_opt(offset_seconds)
            .ok_or_else(|| TimestampError::TimezoneError(format!("Invalid offset: {}", offset_seconds)))
    }

    /// Get timezone offset from DateTime
    pub fn get_timezone_offset<Tz: TimeZone>(datetime: &DateTime<Tz>) -> i32 {
        datetime.offset().fix().local_minus_utc()
    }

    /// Normalize DateTime to UTC and ensure it's valid
    pub fn normalize_datetime<Tz: TimeZone>(datetime: &DateTime<Tz>) -> DateTime<Utc> {
        datetime.with_timezone(&Utc)
    }

    /// Calculate age of a timestamp in seconds
    pub fn age_in_seconds(datetime: &DateTime<Utc>) -> i64 {
        let now = Utc::now();
        (now - *datetime).num_seconds()
    }

    /// Calculate age of a timestamp in days
    pub fn age_in_days(datetime: &DateTime<Utc>) -> i64 {
        let now = Utc::now();
        (now - *datetime).num_days()
    }

    /// Check if a DateTime falls within a specific time range
    pub fn is_within_range(
        datetime: &DateTime<Utc>,
        start: &DateTime<Utc>, 
        end: &DateTime<Utc>
    ) -> bool {
        datetime >= start && datetime <= end
    }

    /// Get file creation time as DateTime<Utc>
    pub fn get_file_creation_time<P: AsRef<Path>>(path: P) -> TimestampResult<DateTime<Utc>> {
        let metadata = fs::metadata(path)?;
        let created = metadata.created().or_else(|_| metadata.modified())?;
        
        let timestamp = created
            .duration_since(UNIX_EPOCH)
            .map_err(|e| TimestampError::InvalidTimestamp(e.to_string()))?
            .as_secs() as i64;
            
        Self::timestamp_to_datetime(timestamp)
    }

    /// Get file modification time as DateTime<Utc>
    pub fn get_file_modification_time<P: AsRef<Path>>(path: P) -> TimestampResult<DateTime<Utc>> {
        let metadata = fs::metadata(path)?;
        let modified = metadata.modified()?;
        
        let timestamp = modified
            .duration_since(UNIX_EPOCH)
            .map_err(|e| TimestampError::InvalidTimestamp(e.to_string()))?
            .as_secs() as i64;
            
        Self::timestamp_to_datetime(timestamp)
    }

    /// Set file modification time from DateTime<Utc>
    pub fn set_file_modification_time<P: AsRef<Path>>(
        path: P,
        datetime: &DateTime<Utc>
    ) -> TimestampResult<()> {
        let timestamp = datetime.timestamp();
        let system_time = UNIX_EPOCH + std::time::Duration::from_secs(timestamp as u64);
        let file_time = filetime::FileTime::from_system_time(system_time);
        
        filetime::set_file_mtime(path, file_time)
            .map_err(|e| TimestampError::FileSystemError(e.into()))
    }

    /// Generate a Maildir-compatible timestamp string for filenames
    pub fn generate_maildir_timestamp() -> String {
        let now = Utc::now();
        format!("{}", now.timestamp())
    }

    /// Parse various common email date formats
    pub fn parse_email_date(date_str: &str) -> TimestampResult<DateTime<Utc>> {
        // Try RFC2822 first (most common in emails)
        if let Ok(dt) = Self::parse_rfc2822(date_str) {
            return Ok(dt);
        }

        // Try RFC3339
        if let Ok(dt) = Self::parse_rfc3339(date_str) {
            return Ok(dt);
        }

        // Try parsing with common email date formats that include timezone
        let tz_formats = [
            "%a, %d %b %Y %H:%M:%S %z",      // RFC2822
            "%d %b %Y %H:%M:%S %z",          // Without day name
            "%Y-%m-%d %H:%M:%S %z",          // ISO-ish with timezone
            "%Y-%m-%dT%H:%M:%S%z",           // RFC3339-ish
        ];

        for format in &tz_formats {
            if let Ok(dt) = DateTime::parse_from_str(date_str, format) {
                return Ok(dt.with_timezone(&Utc));
            }
        }

        // Try parsing with formats without timezone (assume UTC)
        let naive_formats = [
            "%a, %d %b %Y %H:%M:%S",         // RFC2822 without timezone
            "%d %b %Y %H:%M:%S",             // Without day name and timezone
            "%Y-%m-%d %H:%M:%S",             // ISO without timezone
        ];

        for format in &naive_formats {
            if let Ok(naive_dt) = chrono::NaiveDateTime::parse_from_str(date_str, format) {
                return Ok(Utc.from_utc_datetime(&naive_dt));
            }
        }

        Err(TimestampError::ParseError(format!(
            "Unable to parse date string: {}",
            date_str
        )))
    }

    /// Validate that a timestamp is reasonable (not too far in past/future)
    pub fn validate_timestamp(datetime: &DateTime<Utc>) -> TimestampResult<()> {
        let now = Utc::now();
        let min_date = Utc.with_ymd_and_hms(1970, 1, 1, 0, 0, 0).unwrap();
        let max_date = now + chrono::Duration::days(365); // One year in future

        if *datetime < min_date {
            return Err(TimestampError::InvalidTimestamp(format!(
                "Timestamp too far in the past: {}",
                datetime
            )));
        }

        if *datetime > max_date {
            return Err(TimestampError::InvalidTimestamp(format!(
                "Timestamp too far in the future: {}",
                datetime
            )));
        }

        Ok(())
    }
}

/// Helper struct for managing timestamp preservation during file operations
pub struct TimestampPreserver<P: AsRef<Path>> {
    path: P,
    original_modified: Option<SystemTime>,
    original_accessed: Option<SystemTime>,
}

impl<P: AsRef<Path>> TimestampPreserver<P> {
    /// Create a new timestamp preserver and capture current timestamps
    pub fn new(path: P) -> TimestampResult<Self> {
        let metadata = fs::metadata(&path)?;
        let original_modified = metadata.modified().ok();
        let original_accessed = metadata.accessed().ok();

        Ok(Self {
            path,
            original_modified,
            original_accessed,
        })
    }

    /// Restore the original timestamps
    pub fn restore(self) -> TimestampResult<()> {
        if let (Some(modified), Some(accessed)) = (self.original_modified, self.original_accessed) {
            filetime::set_file_times(
                &self.path,
                filetime::FileTime::from_system_time(accessed),
                filetime::FileTime::from_system_time(modified),
            )
            .map_err(|e| TimestampError::FileSystemError(e.into()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use tempfile::NamedTempFile;

    #[test]
    fn test_datetime_timestamp_conversion() {
        let datetime = Utc.with_ymd_and_hms(2021, 1, 1, 12, 0, 0).unwrap();
        let timestamp = TimestampUtils::datetime_to_timestamp(&datetime);
        let converted_back = TimestampUtils::timestamp_to_datetime(timestamp).unwrap();
        
        assert_eq!(datetime, converted_back);
    }

    #[test]
    fn test_timezone_conversions() {
        let utc_time = Utc.with_ymd_and_hms(2021, 1, 1, 12, 0, 0).unwrap();
        let local_time = TimestampUtils::utc_to_local(&utc_time);
        let back_to_utc = TimestampUtils::local_to_utc(&local_time);
        
        assert_eq!(utc_time, back_to_utc);
    }

    #[test]
    fn test_rfc2822_parsing() {
        let date_str = "Wed, 01 Jan 2020 12:00:00 +0000";
        let parsed = TimestampUtils::parse_rfc2822(date_str).unwrap();
        let expected = Utc.with_ymd_and_hms(2020, 1, 1, 12, 0, 0).unwrap();
        
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_rfc3339_parsing() {
        let date_str = "2020-01-01T12:00:00Z";
        let parsed = TimestampUtils::parse_rfc3339(date_str).unwrap();
        let expected = Utc.with_ymd_and_hms(2020, 1, 1, 12, 0, 0).unwrap();
        
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_format_rfc2822() {
        let datetime = Utc.with_ymd_and_hms(2020, 1, 1, 12, 0, 0).unwrap();
        let formatted = TimestampUtils::format_rfc2822(&datetime);
        
        assert!(formatted.contains("01 Jan 2020"));
        assert!(formatted.contains("12:00:00"));
    }

    #[test]
    fn test_format_rfc3339() {
        let datetime = Utc.with_ymd_and_hms(2020, 1, 1, 12, 0, 0).unwrap();
        let formatted = TimestampUtils::format_rfc3339(&datetime);
        
        assert_eq!(formatted, "2020-01-01T12:00:00+00:00");
    }

    #[test]
    fn test_age_calculations() {
        let past_time = Utc::now() - chrono::Duration::days(5);
        
        let age_seconds = TimestampUtils::age_in_seconds(&past_time);
        let age_days = TimestampUtils::age_in_days(&past_time);
        
        assert!(age_seconds > 0);
        assert_eq!(age_days, 5);
    }

    #[test]
    fn test_time_range_check() {
        let start = Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2020, 12, 31, 23, 59, 59).unwrap();
        let middle = Utc.with_ymd_and_hms(2020, 6, 15, 12, 0, 0).unwrap();
        let outside = Utc.with_ymd_and_hms(2021, 1, 1, 0, 0, 0).unwrap();
        
        assert!(TimestampUtils::is_within_range(&middle, &start, &end));
        assert!(!TimestampUtils::is_within_range(&outside, &start, &end));
    }

    #[test]
    fn test_timezone_offset() {
        let utc_time = Utc.with_ymd_and_hms(2020, 1, 1, 12, 0, 0).unwrap();
        let offset = TimestampUtils::get_timezone_offset(&utc_time);
        
        assert_eq!(offset, 0); // UTC has no offset
    }

    #[test]
    fn test_offset_from_seconds() {
        let offset = TimestampUtils::offset_from_seconds(3600).unwrap(); // 1 hour
        assert_eq!(offset.local_minus_utc(), 3600);
        
        let invalid_offset = TimestampUtils::offset_from_seconds(100000); // Invalid
        assert!(invalid_offset.is_err());
    }

    #[test]
    fn test_normalize_datetime() {
        let local_time = Local::now();
        let normalized = TimestampUtils::normalize_datetime(&local_time);
        
        // Should be UTC
        assert_eq!(normalized.timezone(), Utc);
    }

    #[test]
    fn test_parse_email_date_formats() {
        let formats = [
            "Wed, 01 Jan 2020 12:00:00 +0000",
            "01 Jan 2020 12:00:00 +0000",
            "2020-01-01T12:00:00+00:00",
            "Wed, 01 Jan 2020 12:00:00",
        ];
        
        for format in &formats {
            let result = TimestampUtils::parse_email_date(format);
            assert!(result.is_ok(), "Failed to parse: {}", format);
        }
    }

    #[test]
    fn test_validate_timestamp() {
        let valid_time = Utc.with_ymd_and_hms(2020, 1, 1, 12, 0, 0).unwrap();
        assert!(TimestampUtils::validate_timestamp(&valid_time).is_ok());
        
        let too_old = Utc.with_ymd_and_hms(1960, 1, 1, 12, 0, 0).unwrap();
        assert!(TimestampUtils::validate_timestamp(&too_old).is_err());
        
        let too_new = Utc::now() + chrono::Duration::days(400);
        assert!(TimestampUtils::validate_timestamp(&too_new).is_err());
    }

    #[test]
    fn test_file_operations() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        
        // Write some content
        std::fs::write(path, "test content").unwrap();
        
        // Test getting file times
        let creation_time = TimestampUtils::get_file_creation_time(path);
        let modification_time = TimestampUtils::get_file_modification_time(path);
        
        assert!(creation_time.is_ok());
        assert!(modification_time.is_ok());
        
        // Test setting modification time
        let new_time = Utc.with_ymd_and_hms(2020, 1, 1, 12, 0, 0).unwrap();
        let result = TimestampUtils::set_file_modification_time(path, &new_time);
        assert!(result.is_ok());
        
        // Verify the time was set (within reasonable tolerance)
        let updated_time = TimestampUtils::get_file_modification_time(path).unwrap();
        let diff = (updated_time - new_time).num_seconds().abs();
        assert!(diff <= 1); // Allow 1 second tolerance for filesystem precision
    }

    #[test]
    fn test_timestamp_preserver() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        
        // Write initial content
        std::fs::write(path, "initial content").unwrap();
        
        // Create preserver
        let preserver = TimestampPreserver::new(path).unwrap();
        
        // Modify the file
        std::fs::write(path, "modified content").unwrap();
        
        // Restore timestamps
        let result = preserver.restore();
        assert!(result.is_ok());
    }

    #[test]
    fn test_generate_maildir_timestamp() {
        let timestamp = TimestampUtils::generate_maildir_timestamp();
        
        // Should be a valid integer string
        let parsed: i64 = timestamp.parse().unwrap();
        assert!(parsed > 0);
        
        // Should be reasonably recent (within last hour)
        let now = Utc::now().timestamp();
        assert!((now - parsed).abs() < 3600);
    }
}