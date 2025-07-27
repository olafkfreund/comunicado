use std::path::Path;
use std::fs;
use std::io::Write;
use uuid::Uuid;
use crate::email::StoredAttachment;

/// Supported attachment types and their handling
#[derive(Debug, Clone, PartialEq)]
pub enum AttachmentType {
    // Documents
    Pdf,
    Word,
    Excel,
    PowerPoint,
    Text,
    
    // Images
    Jpeg,
    Png,
    Gif,
    Webp,
    Svg,
    
    // Archives
    Zip,
    Rar,
    SevenZip,
    Tar,
    
    // Other
    Unknown,
}

impl AttachmentType {
    /// Determine attachment type from content type
    pub fn from_content_type(content_type: &str) -> Self {
        let content_type = content_type.to_lowercase();
        
        match content_type.as_str() {
            // Documents
            "application/pdf" => Self::Pdf,
            "application/msword" => Self::Word,
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => Self::Word,
            "application/vnd.ms-excel" => Self::Excel,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => Self::Excel,
            "application/vnd.ms-powerpoint" => Self::PowerPoint,
            "application/vnd.openxmlformats-officedocument.presentationml.presentation" => Self::PowerPoint,
            "text/plain" => Self::Text,
            "text/csv" => Self::Text,
            "text/html" => Self::Text,
            "text/xml" => Self::Text,
            "application/xml" => Self::Text,
            "application/json" => Self::Text,
            
            // Images
            "image/jpeg" => Self::Jpeg,
            "image/jpg" => Self::Jpeg,
            "image/png" => Self::Png,
            "image/gif" => Self::Gif,
            "image/webp" => Self::Webp,
            "image/svg+xml" => Self::Svg,
            
            // Archives
            "application/zip" => Self::Zip,
            "application/x-zip-compressed" => Self::Zip,
            "application/x-rar-compressed" => Self::Rar,
            "application/x-7z-compressed" => Self::SevenZip,
            "application/x-tar" => Self::Tar,
            "application/gzip" => Self::Tar,
            
            _ => {
                // Try to determine from filename if content type is generic
                if content_type.starts_with("application/octet-stream") {
                    Self::Unknown
                } else {
                    Self::Unknown
                }
            }
        }
    }
    
    /// Determine attachment type from filename extension
    pub fn from_filename(filename: &str) -> Self {
        let extension = Path::new(filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase())
            .unwrap_or_default();
            
        match extension.as_str() {
            // Documents
            "pdf" => Self::Pdf,
            "doc" | "docx" => Self::Word,
            "xls" | "xlsx" => Self::Excel,
            "ppt" | "pptx" => Self::PowerPoint,
            "txt" | "csv" | "html" | "htm" | "xml" | "json" => Self::Text,
            
            // Images
            "jpg" | "jpeg" => Self::Jpeg,
            "png" => Self::Png,
            "gif" => Self::Gif,
            "webp" => Self::Webp,
            "svg" => Self::Svg,
            
            // Archives
            "zip" => Self::Zip,
            "rar" => Self::Rar,
            "7z" => Self::SevenZip,
            "tar" | "gz" | "tgz" => Self::Tar,
            
            _ => Self::Unknown,
        }
    }
    
    /// Get a human-readable description of the attachment type
    pub fn description(&self) -> &'static str {
        match self {
            Self::Pdf => "PDF Document",
            Self::Word => "Word Document",
            Self::Excel => "Excel Spreadsheet",
            Self::PowerPoint => "PowerPoint Presentation",
            Self::Text => "Text Document",
            Self::Jpeg => "JPEG Image",
            Self::Png => "PNG Image",
            Self::Gif => "GIF Image",
            Self::Webp => "WebP Image",
            Self::Svg => "SVG Image",
            Self::Zip => "ZIP Archive",
            Self::Rar => "RAR Archive",
            Self::SevenZip => "7-Zip Archive",
            Self::Tar => "TAR Archive",
            Self::Unknown => "Unknown File",
        }
    }
    
    /// Check if this attachment type can be safely previewed
    pub fn is_previewable(&self) -> bool {
        matches!(self, 
            Self::Text | 
            Self::Jpeg | Self::Png | Self::Gif | Self::Webp | Self::Svg
        )
    }
    
    /// Check if this attachment type is an image
    pub fn is_image(&self) -> bool {
        matches!(self, 
            Self::Jpeg | Self::Png | Self::Gif | Self::Webp | Self::Svg
        )
    }
    
    /// Check if this attachment type is a document
    pub fn is_document(&self) -> bool {
        matches!(self, 
            Self::Pdf | Self::Word | Self::Excel | Self::PowerPoint | Self::Text
        )
    }
    
    /// Check if this attachment type is an archive
    pub fn is_archive(&self) -> bool {
        matches!(self, 
            Self::Zip | Self::Rar | Self::SevenZip | Self::Tar
        )
    }
    
    /// Get appropriate icon/emoji for this attachment type
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Pdf => "📄",
            Self::Word => "📝",
            Self::Excel => "📊",
            Self::PowerPoint => "📋",
            Self::Text => "📃",
            Self::Jpeg | Self::Png | Self::Gif | Self::Webp | Self::Svg => "🖼️",
            Self::Zip | Self::Rar | Self::SevenZip | Self::Tar => "🗜️",
            Self::Unknown => "📎",
        }
    }
}

/// Enhanced attachment information with type detection
#[derive(Debug, Clone)]
pub struct AttachmentInfo {
    pub stored: StoredAttachment,
    pub attachment_type: AttachmentType,
    pub is_safe: bool,
    pub display_name: String,
}

impl AttachmentInfo {
    /// Create AttachmentInfo from StoredAttachment
    pub fn from_stored(stored: StoredAttachment) -> Self {
        // Determine type from both content type and filename
        let type_from_content = AttachmentType::from_content_type(&stored.content_type);
        let type_from_filename = AttachmentType::from_filename(&stored.filename);
        
        // Prefer the more specific type (not Unknown)
        let attachment_type = match (type_from_content, type_from_filename) {
            (AttachmentType::Unknown, other) => other,
            (other, AttachmentType::Unknown) => other,
            (content_type, _) => content_type, // Prefer content type when both are known
        };
        
        // Basic safety check (can be enhanced)
        let is_safe = Self::is_safe_type(&attachment_type, &stored.filename);
        
        // Create display name
        let display_name = if stored.filename.is_empty() {
            format!("Attachment.{}", Self::default_extension(&attachment_type))
        } else {
            stored.filename.clone()
        };
        
        Self {
            stored,
            attachment_type,
            is_safe,
            display_name,
        }
    }
    
    /// Check if an attachment is considered safe to handle
    fn is_safe_type(attachment_type: &AttachmentType, filename: &str) -> bool {
        // Basic safety check - avoid executable types
        let dangerous_extensions = [
            "exe", "bat", "cmd", "com", "scr", "vbs", "js", "jar", "app", "deb", "rpm"
        ];
        
        let extension = Path::new(filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase())
            .unwrap_or_default();
            
        if dangerous_extensions.contains(&extension.as_str()) {
            return false;
        }
        
        // Generally consider document, image, and archive types safe
        matches!(attachment_type, 
            AttachmentType::Pdf | AttachmentType::Word | AttachmentType::Excel | 
            AttachmentType::PowerPoint | AttachmentType::Text |
            AttachmentType::Jpeg | AttachmentType::Png | AttachmentType::Gif | 
            AttachmentType::Webp | AttachmentType::Svg |
            AttachmentType::Zip | AttachmentType::Rar | AttachmentType::SevenZip | 
            AttachmentType::Tar
        )
    }
    
    /// Get default file extension for attachment type
    fn default_extension(attachment_type: &AttachmentType) -> &'static str {
        match attachment_type {
            AttachmentType::Pdf => "pdf",
            AttachmentType::Word => "docx",
            AttachmentType::Excel => "xlsx", 
            AttachmentType::PowerPoint => "pptx",
            AttachmentType::Text => "txt",
            AttachmentType::Jpeg => "jpg",
            AttachmentType::Png => "png",
            AttachmentType::Gif => "gif",
            AttachmentType::Webp => "webp",
            AttachmentType::Svg => "svg",
            AttachmentType::Zip => "zip",
            AttachmentType::Rar => "rar",
            AttachmentType::SevenZip => "7z",
            AttachmentType::Tar => "tar",
            AttachmentType::Unknown => "bin",
        }
    }
    
    /// Format file size for display
    pub fn format_size(&self) -> String {
        let size = self.stored.size as f64;
        
        if size < 1024.0 {
            format!("{} B", size as u64)
        } else if size < 1024.0 * 1024.0 {
            format!("{:.1} KB", size / 1024.0)
        } else if size < 1024.0 * 1024.0 * 1024.0 {
            format!("{:.1} MB", size / (1024.0 * 1024.0))
        } else {
            format!("{:.1} GB", size / (1024.0 * 1024.0 * 1024.0))
        }
    }
    
    /// Get summary description for UI display
    pub fn summary(&self) -> String {
        format!("{} ({}) - {}", 
                self.display_name, 
                self.attachment_type.description(),
                self.format_size())
    }
}

/// Attachment manager for handling file operations
pub struct AttachmentManager {
    attachments_dir: std::path::PathBuf,
}

impl AttachmentManager {
    /// Create new attachment manager with specified directory
    pub fn new(attachments_dir: std::path::PathBuf) -> std::io::Result<Self> {
        // Create attachments directory if it doesn't exist
        if !attachments_dir.exists() {
            fs::create_dir_all(&attachments_dir)?;
        }
        
        Ok(Self {
            attachments_dir,
        })
    }
    
    /// Save attachment data to disk and return file path
    pub fn save_attachment(&self, attachment: &AttachmentInfo, data: &[u8]) -> std::io::Result<String> {
        // Generate unique filename to avoid conflicts
        let unique_id = Uuid::new_v4().to_string();
        let extension = Path::new(&attachment.display_name)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or(AttachmentInfo::default_extension(&attachment.attachment_type));
            
        let filename = format!("{}_{}.{}", 
                              attachment.stored.id, 
                              unique_id, 
                              extension);
        let file_path = self.attachments_dir.join(&filename);
        
        // Write data to file
        let mut file = fs::File::create(&file_path)?;
        file.write_all(data)?;
        
        Ok(file_path.to_string_lossy().to_string())
    }
    
    /// Load attachment data from disk
    pub fn load_attachment(&self, file_path: &str) -> std::io::Result<Vec<u8>> {
        fs::read(file_path)
    }
    
    /// Delete attachment file from disk
    pub fn delete_attachment(&self, file_path: &str) -> std::io::Result<()> {
        fs::remove_file(file_path)
    }
    
    /// Get attachment directory path
    pub fn attachments_dir(&self) -> &Path {
        &self.attachments_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attachment_type_from_content_type() {
        assert_eq!(AttachmentType::from_content_type("application/pdf"), AttachmentType::Pdf);
        assert_eq!(AttachmentType::from_content_type("image/jpeg"), AttachmentType::Jpeg);
        assert_eq!(AttachmentType::from_content_type("application/zip"), AttachmentType::Zip);
        assert_eq!(AttachmentType::from_content_type("unknown/type"), AttachmentType::Unknown);
    }

    #[test]
    fn test_attachment_type_from_filename() {
        assert_eq!(AttachmentType::from_filename("document.pdf"), AttachmentType::Pdf);
        assert_eq!(AttachmentType::from_filename("photo.jpg"), AttachmentType::Jpeg);
        assert_eq!(AttachmentType::from_filename("archive.zip"), AttachmentType::Zip);
        assert_eq!(AttachmentType::from_filename("unknown.xyz"), AttachmentType::Unknown);
    }

    #[test]
    fn test_attachment_type_properties() {
        assert!(AttachmentType::Jpeg.is_image());
        assert!(AttachmentType::Pdf.is_document());
        assert!(AttachmentType::Zip.is_archive());
        assert!(AttachmentType::Text.is_previewable());
    }

    #[test]
    fn test_attachment_info_creation() {
        let stored = StoredAttachment {
            id: "test_id".to_string(),
            filename: "test.pdf".to_string(),
            content_type: "application/pdf".to_string(),
            size: 1024,
            content_id: None,
            is_inline: false,
            data: None,
            file_path: None,
        };
        
        let info = AttachmentInfo::from_stored(stored);
        assert_eq!(info.attachment_type, AttachmentType::Pdf);
        assert!(info.is_safe);
        assert_eq!(info.display_name, "test.pdf");
    }
}