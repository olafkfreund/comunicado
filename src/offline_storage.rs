// Offline-first storage implementation for calendars and contacts
// Follows khal/khard architecture with .ics/.vcf file storage
// Integrates with RFC standards parser for proper format compliance

use crate::calendar::{Calendar, CalendarSource, Event};
use crate::contacts::{Contact, ContactSource};
use crate::rfc_standards::RfcStandardsParser;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs as async_fs;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Offline storage manager for calendars and contacts
pub struct OfflineStorageManager {
    /// Base directory for all offline storage
    #[allow(dead_code)]
    base_dir: PathBuf,
    /// Calendar storage directory (.ics files)
    calendar_dir: PathBuf,
    /// Contacts storage directory (.vcf files)
    contacts_dir: PathBuf,
    /// Cache of loaded calendars
    calendar_cache: HashMap<String, Calendar>,
    /// Cache of loaded contacts
    contact_cache: HashMap<String, Contact>,
    /// Last sync timestamps for conflict resolution
    last_sync: HashMap<String, DateTime<Utc>>,
}

impl OfflineStorageManager {
    /// Create a new offline storage manager
    pub async fn new(base_dir: PathBuf) -> Result<Self, OfflineStorageError> {
        let calendar_dir = base_dir.join("calendars");
        let contacts_dir = base_dir.join("contacts");

        // Create directories if they don't exist
        async_fs::create_dir_all(&calendar_dir).await
            .map_err(|e| OfflineStorageError::IoError(format!("Failed to create calendar directory: {}", e)))?;
        
        async_fs::create_dir_all(&contacts_dir).await
            .map_err(|e| OfflineStorageError::IoError(format!("Failed to create contacts directory: {}", e)))?;

        info!("Initialized offline storage at: {}", base_dir.display());
        debug!("Calendar storage: {}", calendar_dir.display());
        debug!("Contacts storage: {}", contacts_dir.display());

        Ok(Self {
            base_dir,
            calendar_dir,
            contacts_dir,
            calendar_cache: HashMap::new(),
            contact_cache: HashMap::new(),
            last_sync: HashMap::new(),
        })
    }

    /// Get the default storage directory (XDG compliant)
    pub fn default_storage_dir() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")))
            .join("comunicado")
            .join("offline_storage")
    }

    /// Load all calendars from storage
    pub async fn load_calendars(&mut self) -> Result<Vec<Calendar>, OfflineStorageError> {
        debug!("Loading calendars from offline storage");
        let mut calendars = Vec::new();

        let mut entries = async_fs::read_dir(&self.calendar_dir).await
            .map_err(|e| OfflineStorageError::IoError(format!("Failed to read calendar directory: {}", e)))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| OfflineStorageError::IoError(format!("Failed to read directory entry: {}", e)))? {
            
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("ics") {
                match self.load_calendar_from_file(&path).await {
                    Ok(calendar) => {
                        debug!("Loaded calendar: {} from {}", calendar.name, path.display());
                        self.calendar_cache.insert(calendar.id.clone(), calendar.clone());
                        calendars.push(calendar);
                    }
                    Err(e) => {
                        warn!("Failed to load calendar from {}: {}", path.display(), e);
                    }
                }
            }
        }

        info!("Loaded {} calendars from offline storage", calendars.len());
        Ok(calendars)
    }

    /// Load all contacts from storage
    pub async fn load_contacts(&mut self) -> Result<Vec<Contact>, OfflineStorageError> {
        debug!("Loading contacts from offline storage");
        let mut contacts = Vec::new();

        let mut entries = async_fs::read_dir(&self.contacts_dir).await
            .map_err(|e| OfflineStorageError::IoError(format!("Failed to read contacts directory: {}", e)))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| OfflineStorageError::IoError(format!("Failed to read directory entry: {}", e)))? {
            
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("vcf") {
                match self.load_contacts_from_file(&path).await {
                    Ok(mut file_contacts) => {
                        debug!("Loaded {} contacts from {}", file_contacts.len(), path.display());
                        for contact in &file_contacts {
                            self.contact_cache.insert(contact.external_id.clone(), contact.clone());
                        }
                        contacts.append(&mut file_contacts);
                    }
                    Err(e) => {
                        warn!("Failed to load contacts from {}: {}", path.display(), e);
                    }
                }
            }
        }

        info!("Loaded {} contacts from offline storage", contacts.len());
        Ok(contacts)
    }

    /// Save a calendar to offline storage
    pub async fn save_calendar(&mut self, calendar: &Calendar) -> Result<(), OfflineStorageError> {
        debug!("Saving calendar: {} to offline storage", calendar.name);

        let filename = format!("{}.ics", sanitize_filename(&calendar.name));
        let file_path = self.calendar_dir.join(filename);

        // Load events for this calendar
        let events = self.load_calendar_events(&calendar.id).await?;

        // Convert to iCalendar format using RFC standards parser
        let icalendar_content = RfcStandardsParser::event_to_icalendar(&events)
            .map_err(|e| OfflineStorageError::ParseError(format!("Failed to generate iCalendar: {}", e)))?;

        // Write to file
        async_fs::write(&file_path, icalendar_content).await
            .map_err(|e| OfflineStorageError::IoError(format!("Failed to write calendar file: {}", e)))?;

        // Update cache and sync timestamp
        self.calendar_cache.insert(calendar.id.clone(), calendar.clone());
        self.last_sync.insert(calendar.id.clone(), Utc::now());

        info!("Saved calendar {} to {}", calendar.name, file_path.display());
        Ok(())
    }

    /// Save a contact to offline storage
    pub async fn save_contact(&mut self, contact: &Contact) -> Result<(), OfflineStorageError> {
        debug!("Saving contact: {} to offline storage", contact.display_name);

        let filename = format!("{}.vcf", sanitize_filename(&contact.display_name));
        let file_path = self.contacts_dir.join(filename);

        // Convert to vCard format using RFC standards parser
        let vcard_content = RfcStandardsParser::contact_to_vcard(contact)
            .map_err(|e| OfflineStorageError::ParseError(format!("Failed to generate vCard: {}", e)))?;

        // Write to file
        async_fs::write(&file_path, vcard_content).await
            .map_err(|e| OfflineStorageError::IoError(format!("Failed to write contact file: {}", e)))?;

        // Update cache and sync timestamp
        self.contact_cache.insert(contact.external_id.clone(), contact.clone());
        self.last_sync.insert(contact.external_id.clone(), Utc::now());

        info!("Saved contact {} to {}", contact.display_name, file_path.display());
        Ok(())
    }

    /// Delete a calendar from offline storage
    pub async fn delete_calendar(&mut self, calendar_id: &str) -> Result<(), OfflineStorageError> {
        debug!("Deleting calendar: {} from offline storage", calendar_id);

        // Find and remove the calendar file
        if let Some(calendar) = self.calendar_cache.get(calendar_id) {
            let filename = format!("{}.ics", sanitize_filename(&calendar.name));
            let file_path = self.calendar_dir.join(filename);

            if file_path.exists() {
                async_fs::remove_file(&file_path).await
                    .map_err(|e| OfflineStorageError::IoError(format!("Failed to delete calendar file: {}", e)))?;
                
                info!("Deleted calendar file: {}", file_path.display());
            }
        }

        // Remove from cache
        self.calendar_cache.remove(calendar_id);
        self.last_sync.remove(calendar_id);

        Ok(())
    }

    /// Delete a contact from offline storage
    pub async fn delete_contact(&mut self, contact_id: &str) -> Result<(), OfflineStorageError> {
        debug!("Deleting contact: {} from offline storage", contact_id);

        // Find and remove the contact file
        if let Some(contact) = self.contact_cache.get(contact_id) {
            let filename = format!("{}.vcf", sanitize_filename(&contact.display_name));
            let file_path = self.contacts_dir.join(filename);

            if file_path.exists() {
                async_fs::remove_file(&file_path).await
                    .map_err(|e| OfflineStorageError::IoError(format!("Failed to delete contact file: {}", e)))?;
                
                info!("Deleted contact file: {}", file_path.display());
            }
        }

        // Remove from cache
        self.contact_cache.remove(contact_id);
        self.last_sync.remove(contact_id);

        Ok(())
    }

    /// Export all calendars to a directory
    pub async fn export_calendars(&self, export_dir: &Path) -> Result<usize, OfflineStorageError> {
        debug!("Exporting calendars to: {}", export_dir.display());

        async_fs::create_dir_all(export_dir).await
            .map_err(|e| OfflineStorageError::IoError(format!("Failed to create export directory: {}", e)))?;

        let mut exported_count = 0;

        // Copy all .ics files to export directory
        let mut entries = async_fs::read_dir(&self.calendar_dir).await
            .map_err(|e| OfflineStorageError::IoError(format!("Failed to read calendar directory: {}", e)))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| OfflineStorageError::IoError(format!("Failed to read directory entry: {}", e)))? {
            
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("ics") {
                let filename = path.file_name().unwrap();
                let dest_path = export_dir.join(filename);

                async_fs::copy(&path, &dest_path).await
                    .map_err(|e| OfflineStorageError::IoError(format!("Failed to copy calendar file: {}", e)))?;

                exported_count += 1;
                debug!("Exported calendar: {}", dest_path.display());
            }
        }

        info!("Exported {} calendars to {}", exported_count, export_dir.display());
        Ok(exported_count)
    }

    /// Export all contacts to a directory
    pub async fn export_contacts(&self, export_dir: &Path) -> Result<usize, OfflineStorageError> {
        debug!("Exporting contacts to: {}", export_dir.display());

        async_fs::create_dir_all(export_dir).await
            .map_err(|e| OfflineStorageError::IoError(format!("Failed to create export directory: {}", e)))?;

        let mut exported_count = 0;

        // Copy all .vcf files to export directory
        let mut entries = async_fs::read_dir(&self.contacts_dir).await
            .map_err(|e| OfflineStorageError::IoError(format!("Failed to read contacts directory: {}", e)))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| OfflineStorageError::IoError(format!("Failed to read directory entry: {}", e)))? {
            
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("vcf") {
                let filename = path.file_name().unwrap();
                let dest_path = export_dir.join(filename);

                async_fs::copy(&path, &dest_path).await
                    .map_err(|e| OfflineStorageError::IoError(format!("Failed to copy contact file: {}", e)))?;

                exported_count += 1;
                debug!("Exported contact: {}", dest_path.display());
            }
        }

        info!("Exported {} contacts to {}", exported_count, export_dir.display());
        Ok(exported_count)
    }

    /// Import calendars from a directory
    pub async fn import_calendars(&mut self, import_dir: &Path) -> Result<usize, OfflineStorageError> {
        debug!("Importing calendars from: {}", import_dir.display());

        let mut imported_count = 0;

        let mut entries = async_fs::read_dir(import_dir).await
            .map_err(|e| OfflineStorageError::IoError(format!("Failed to read import directory: {}", e)))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| OfflineStorageError::IoError(format!("Failed to read directory entry: {}", e)))? {
            
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("ics") {
                match self.import_calendar_file(&path).await {
                    Ok(_) => {
                        imported_count += 1;
                        debug!("Imported calendar: {}", path.display());
                    }
                    Err(e) => {
                        warn!("Failed to import calendar from {}: {}", path.display(), e);
                    }
                }
            }
        }

        info!("Imported {} calendars from {}", imported_count, import_dir.display());
        Ok(imported_count)
    }

    /// Import contacts from a directory
    pub async fn import_contacts(&mut self, import_dir: &Path) -> Result<usize, OfflineStorageError> {
        debug!("Importing contacts from: {}", import_dir.display());

        let mut imported_count = 0;

        let mut entries = async_fs::read_dir(import_dir).await
            .map_err(|e| OfflineStorageError::IoError(format!("Failed to read import directory: {}", e)))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| OfflineStorageError::IoError(format!("Failed to read directory entry: {}", e)))? {
            
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("vcf") {
                match self.import_contacts_file(&path).await {
                    Ok(contact_count) => {
                        imported_count += contact_count;
                        debug!("Imported {} contacts from {}", contact_count, path.display());
                    }
                    Err(e) => {
                        warn!("Failed to import contacts from {}: {}", path.display(), e);
                    }
                }
            }
        }

        info!("Imported {} contacts from {}", imported_count, import_dir.display());
        Ok(imported_count)
    }

    /// Get storage statistics
    pub async fn get_storage_stats(&self) -> Result<StorageStats, OfflineStorageError> {
        let calendar_count = self.count_files_with_extension(&self.calendar_dir, "ics").await?;
        let contact_count = self.count_files_with_extension(&self.contacts_dir, "vcf").await?;

        let calendar_size = self.calculate_directory_size(&self.calendar_dir).await?;
        let contact_size = self.calculate_directory_size(&self.contacts_dir).await?;

        Ok(StorageStats {
            calendar_count,
            contact_count,
            calendar_size_bytes: calendar_size,
            contact_size_bytes: contact_size,
            total_size_bytes: calendar_size + contact_size,
            last_updated: Utc::now(),
        })
    }

    // Private helper methods

    /// Load a calendar from an .ics file
    async fn load_calendar_from_file(&self, file_path: &Path) -> Result<Calendar, OfflineStorageError> {
        let _content = async_fs::read_to_string(file_path).await
            .map_err(|e| OfflineStorageError::IoError(format!("Failed to read calendar file: {}", e)))?;

        // TODO: Parse iCalendar content and extract calendar metadata
        // For now, create a basic calendar from filename
        let name = file_path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown Calendar")
            .to_string();

        Ok(Calendar {
            id: Uuid::new_v4().to_string(),
            name,
            description: Some("Loaded from offline storage".to_string()),
            color: Some("#3174ad".to_string()), // Default blue color
            source: CalendarSource::Local,
            read_only: false,
            timezone: "UTC".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_synced: Some(Utc::now()),
        })
    }

    /// Load contacts from a .vcf file
    async fn load_contacts_from_file(&self, file_path: &Path) -> Result<Vec<Contact>, OfflineStorageError> {
        let content = async_fs::read_to_string(file_path).await
            .map_err(|e| OfflineStorageError::IoError(format!("Failed to read contact file: {}", e)))?;

        RfcStandardsParser::parse_vcard_to_contact(&content, ContactSource::Local)
            .map_err(|e| OfflineStorageError::ParseError(format!("Failed to parse vCard: {}", e)))
    }

    /// Load events for a calendar (placeholder - would integrate with calendar manager)
    async fn load_calendar_events(&self, _calendar_id: &str) -> Result<Vec<Event>, OfflineStorageError> {
        // TODO: Integrate with calendar manager to get events for calendar
        Ok(Vec::new())
    }

    /// Import a single calendar file
    async fn import_calendar_file(&mut self, file_path: &Path) -> Result<(), OfflineStorageError> {
        let filename = file_path.file_name().unwrap();
        let dest_path = self.calendar_dir.join(filename);

        async_fs::copy(file_path, &dest_path).await
            .map_err(|e| OfflineStorageError::IoError(format!("Failed to copy calendar file: {}", e)))?;

        Ok(())
    }

    /// Import a single contacts file
    async fn import_contacts_file(&mut self, file_path: &Path) -> Result<usize, OfflineStorageError> {
        let contacts = self.load_contacts_from_file(file_path).await?;
        let contact_count = contacts.len();

        for contact in contacts {
            self.save_contact(&contact).await?;
        }

        Ok(contact_count)
    }

    /// Count files with a specific extension in a directory
    async fn count_files_with_extension(&self, dir: &Path, extension: &str) -> Result<usize, OfflineStorageError> {
        let mut count = 0;
        let mut entries = async_fs::read_dir(dir).await
            .map_err(|e| OfflineStorageError::IoError(format!("Failed to read directory: {}", e)))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| OfflineStorageError::IoError(format!("Failed to read directory entry: {}", e)))? {
            
            if entry.path().extension().and_then(|s| s.to_str()) == Some(extension) {
                count += 1;
            }
        }

        Ok(count)
    }

    /// Calculate total size of files in a directory
    async fn calculate_directory_size(&self, dir: &Path) -> Result<u64, OfflineStorageError> {
        let mut total_size = 0;
        let mut entries = async_fs::read_dir(dir).await
            .map_err(|e| OfflineStorageError::IoError(format!("Failed to read directory: {}", e)))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| OfflineStorageError::IoError(format!("Failed to read directory entry: {}", e)))? {
            
            let metadata = entry.metadata().await
                .map_err(|e| OfflineStorageError::IoError(format!("Failed to read file metadata: {}", e)))?;
            
            if metadata.is_file() {
                total_size += metadata.len();
            }
        }

        Ok(total_size)
    }
}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    pub calendar_count: usize,
    pub contact_count: usize,
    pub calendar_size_bytes: u64,
    pub contact_size_bytes: u64,
    pub total_size_bytes: u64,
    pub last_updated: DateTime<Utc>,
}

impl StorageStats {
    /// Get human-readable total size
    pub fn total_size_human(&self) -> String {
        format_bytes(self.total_size_bytes)
    }

    /// Get human-readable calendar size
    pub fn calendar_size_human(&self) -> String {
        format_bytes(self.calendar_size_bytes)
    }

    /// Get human-readable contact size
    pub fn contact_size_human(&self) -> String {
        format_bytes(self.contact_size_bytes)
    }
}

/// Offline storage errors
#[derive(Debug, thiserror::Error)]
pub enum OfflineStorageError {
    #[error("IO error: {0}")]
    IoError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid format: {0}")]
    InvalidFormat(String),
}

/// Sanitize a filename for cross-platform compatibility
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

/// Format bytes in human-readable format
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    const THRESHOLD: u64 = 1024;

    if bytes == 0 {
        return "0 B".to_string();
    }

    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= THRESHOLD as f64 && unit_index < UNITS.len() - 1 {
        size /= THRESHOLD as f64;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_offline_storage_creation() {
        let temp_dir = TempDir::new().unwrap();
        let storage = OfflineStorageManager::new(temp_dir.path().to_path_buf()).await.unwrap();

        assert!(storage.calendar_dir.exists());
        assert!(storage.contacts_dir.exists());
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("normal_name"), "normal_name");
        assert_eq!(sanitize_filename("name/with\\bad:chars"), "name_with_bad_chars");
        assert_eq!(sanitize_filename("file<>with|bad*chars?"), "file__with_bad_chars_");
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1048576), "1.0 MB");
        assert_eq!(format_bytes(1073741824), "1.0 GB");
    }
}