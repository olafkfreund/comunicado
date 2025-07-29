use crate::email::{EmailDatabase, MaildirHandler, MaildirError, MaildirStats};
use std::path::Path;
use std::sync::Arc;

/// UI wrapper for maildir operations
pub struct MaildirUI {
    database: Arc<EmailDatabase>,
}

impl MaildirUI {
    /// Create a new maildir UI
    pub fn new(database: Arc<EmailDatabase>) -> Self {
        Self { database }
    }
    
    /// Export account to maildir with progress reporting
    pub async fn export_account_with_progress<F>(&self,
                                                 account_id: &str,
                                                 export_path: &Path,
                                                 progress_callback: F) -> Result<usize, MaildirError>
    where
        F: Fn(usize, usize) + Send + Sync,
    {
        let handler = MaildirHandler::new(export_path);
        
        // Get export stats first
        let stats = handler.get_export_stats(&self.database, account_id).await?;
        
        tracing::info!("Starting export of {} messages from {} folders", 
                      stats.total_messages, stats.total_folders);
        
        let mut exported = 0;
        
        // Export each folder with progress reporting
        for folder_stats in &stats.folders {
            let folder_exported = handler.export_folder(&self.database, account_id, &folder_stats.name).await?;
            exported += folder_exported;
            progress_callback(exported, stats.total_messages);
        }
        
        Ok(exported)
    }
    
    /// Import account from maildir with progress reporting
    pub async fn import_account_with_progress<F>(&self,
                                                 account_id: &str,
                                                 import_path: &Path,
                                                 progress_callback: F) -> Result<usize, MaildirError>
    where
        F: Fn(usize) + Send + Sync,
    {
        let handler = MaildirHandler::new(import_path);
        
        tracing::info!("Starting import from maildir path: {:?}", import_path);
        
        let imported = handler.import_account(&self.database, account_id).await?;
        progress_callback(imported);
        
        Ok(imported)
    }
    
    /// Get preview of what would be exported
    pub async fn get_export_preview(&self, account_id: &str, export_path: &Path) -> Result<MaildirExportPreview, MaildirError> {
        let handler = MaildirHandler::new(export_path);
        let stats = handler.get_export_stats(&self.database, account_id).await?;
        
        Ok(MaildirExportPreview {
            account_id: account_id.to_string(),
            export_path: export_path.to_path_buf(),
            total_folders: stats.total_folders,
            total_messages: stats.total_messages,
            estimated_size_mb: self.estimate_export_size(&stats).await,
            folders: stats.folders,
        })
    }
    
    /// Validate export path
    pub fn validate_export_path(&self, path: &Path) -> Result<(), MaildirError> {
        if !path.exists() {
            std::fs::create_dir_all(path)
                .map_err(|e| MaildirError::Io(e))?;
        }
        
        if !path.is_dir() {
            return Err(MaildirError::InvalidPath("Export path is not a directory".to_string()));
        }
        
        // Check if path is writable
        let test_file = path.join(".comunicado_test");
        match std::fs::write(&test_file, "test") {
            Ok(_) => {
                let _ = std::fs::remove_file(&test_file);
                Ok(())
            }
            Err(e) => Err(MaildirError::Io(e)),
        }
    }
    
    /// Validate import path
    pub fn validate_import_path(&self, path: &Path) -> Result<MaildirImportPreview, MaildirError> {
        if !path.exists() {
            return Err(MaildirError::InvalidPath("Import path does not exist".to_string()));
        }
        
        if !path.is_dir() {
            return Err(MaildirError::InvalidPath("Import path is not a directory".to_string()));
        }
        
        // Scan for maildir folders
        let mut folders = Vec::new();
        let mut total_messages = 0;
        
        for entry in std::fs::read_dir(path).map_err(MaildirError::Io)? {
            let entry = entry.map_err(MaildirError::Io)?;
            let folder_path = entry.path();
            
            if folder_path.is_dir() {
                if let Ok(message_count) = self.count_messages_in_maildir(&folder_path) {
                    if message_count > 0 {
                        let folder_name = folder_path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        
                        folders.push(MaildirImportFolder {
                            name: folder_name,
                            message_count,
                            path: folder_path,
                        });
                        
                        total_messages += message_count;
                    }
                }
            }
        }
        
        Ok(MaildirImportPreview {
            import_path: path.to_path_buf(),
            total_folders: folders.len(),
            total_messages,
            folders,
        })
    }
    
    /// Count messages in a maildir folder
    fn count_messages_in_maildir(&self, folder_path: &Path) -> Result<usize, MaildirError> {
        let new_dir = folder_path.join("new");
        let cur_dir = folder_path.join("cur");
        
        let mut count = 0;
        
        if new_dir.exists() {
            count += std::fs::read_dir(&new_dir)
                .map_err(MaildirError::Io)?
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.path().is_file())
                .count();
        }
        
        if cur_dir.exists() {
            count += std::fs::read_dir(&cur_dir)
                .map_err(MaildirError::Io)?
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.path().is_file())
                .count();
        }
        
        Ok(count)
    }
    
    /// Estimate export size in MB
    async fn estimate_export_size(&self, stats: &MaildirStats) -> f64 {
        // Rough estimate: average email size of 15KB including headers
        const AVERAGE_EMAIL_SIZE_KB: f64 = 15.0;
        (stats.total_messages as f64 * AVERAGE_EMAIL_SIZE_KB) / 1024.0
    }
}

/// Preview information for maildir export
#[derive(Debug, Clone)]
pub struct MaildirExportPreview {
    pub account_id: String,
    pub export_path: std::path::PathBuf,
    pub total_folders: usize,
    pub total_messages: usize,
    pub estimated_size_mb: f64,
    pub folders: Vec<crate::email::MaildirFolderStats>,
}

/// Preview information for maildir import
#[derive(Debug, Clone)]
pub struct MaildirImportPreview {
    pub import_path: std::path::PathBuf,
    pub total_folders: usize,
    pub total_messages: usize,
    pub folders: Vec<MaildirImportFolder>,
}

/// Information about a folder to be imported
#[derive(Debug, Clone)]
pub struct MaildirImportFolder {
    pub name: String,
    pub message_count: usize,
    pub path: std::path::PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_maildir_ui_creation() {
        let database = Arc::new(crate::email::EmailDatabase::new_in_memory().await.unwrap());
        let ui = MaildirUI::new(database);
        
        // Test path validation
        let temp_dir = TempDir::new().unwrap();
        assert!(ui.validate_export_path(temp_dir.path()).is_ok());
    }
    
    #[test]
    fn test_validate_export_path() {
        let database = Arc::new(tokio_test::block_on(crate::email::EmailDatabase::new_in_memory()).unwrap());
        let ui = MaildirUI::new(database);
        
        let temp_dir = TempDir::new().unwrap();
        assert!(ui.validate_export_path(temp_dir.path()).is_ok());
        
        // Test non-existent path (should create it)
        let new_path = temp_dir.path().join("new_folder");
        assert!(ui.validate_export_path(&new_path).is_ok());
        assert!(new_path.exists());
    }
}