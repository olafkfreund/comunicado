use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur during folder hierarchy operations
#[derive(Error, Debug)]
pub enum FolderHierarchyError {
    #[error("Invalid folder path: {0}")]
    InvalidPath(String),
    
    #[error("Path traversal attempt detected: {0}")]
    PathTraversal(String),
    
    #[error("Reserved folder name: {0}")]
    ReservedName(String),
    
    #[error("Folder name too long: {0} (max: {1})")]
    NameTooLong(String, usize),
}

pub type FolderHierarchyResult<T> = Result<T, FolderHierarchyError>;

/// Maximum length for sanitized folder names
const MAX_FOLDER_NAME_LENGTH: usize = 255;

/// Reserved folder names that should not be used
const RESERVED_NAMES: &[&str] = &[".", "..", "con", "prn", "aux", "nul", "tmp"];

/// Folder hierarchy mapper for converting between IMAP-style paths and filesystem structure
pub struct FolderHierarchyMapper {
    /// Character used to separate folders in IMAP (typically '/' or '.')
    imap_separator: char,
    /// String used in filesystem to represent nested folders
    filesystem_separator: String,
    /// Enable strict path validation
    strict_validation: bool,
}

impl Default for FolderHierarchyMapper {
    fn default() -> Self {
        Self::new()
    }
}

impl FolderHierarchyMapper {
    /// Create a new folder hierarchy mapper with default settings
    pub fn new() -> Self {
        Self {
            imap_separator: '/',
            filesystem_separator: "__".to_string(),
            strict_validation: true,
        }
    }

    /// Create a mapper with custom settings
    pub fn with_settings(
        imap_separator: char,
        filesystem_separator: String,
        strict_validation: bool,
    ) -> Self {
        Self {
            imap_separator,
            filesystem_separator,
            strict_validation,
        }
    }

    /// Convert IMAP folder path to filesystem-safe path
    pub fn imap_to_filesystem(&self, imap_path: &str) -> FolderHierarchyResult<String> {
        if imap_path.is_empty() {
            return Err(FolderHierarchyError::InvalidPath("Empty path".to_string()));
        }

        // Split by IMAP separator and process each component
        let components: Result<Vec<String>, FolderHierarchyError> = imap_path
            .split(self.imap_separator)
            .map(|component| self.sanitize_folder_component(component))
            .collect();

        let components = components?;

        // Join with filesystem separator
        Ok(components.join(&self.filesystem_separator))
    }

    /// Convert filesystem path back to IMAP folder path
    pub fn filesystem_to_imap(&self, filesystem_path: &str) -> FolderHierarchyResult<String> {
        if filesystem_path.is_empty() {
            return Err(FolderHierarchyError::InvalidPath("Empty path".to_string()));
        }

        // Split by filesystem separator and reconstruct IMAP path
        let components: Vec<&str> = filesystem_path.split(&self.filesystem_separator).collect();

        // Validate each component if strict validation is enabled
        if self.strict_validation {
            for component in &components {
                self.validate_component(component)?;
            }
        }

        // Reverse the sanitization process (limited - some information may be lost)
        let imap_components: Vec<String> = components
            .iter()
            .map(|component| self.reverse_sanitize_component(component))
            .collect();

        Ok(imap_components.join(&self.imap_separator.to_string()))
    }

    /// Create a filesystem path for a Maildir folder structure
    pub fn create_maildir_path<P: AsRef<Path>>(
        &self,
        base_path: P,
        account_id: &str,
        imap_path: &str,
    ) -> FolderHierarchyResult<PathBuf> {
        let filesystem_path = self.imap_to_filesystem(imap_path)?;
        let sanitized_account = self.sanitize_folder_component(account_id)?;

        Ok(base_path
            .as_ref()
            .join(sanitized_account)
            .join(filesystem_path))
    }

    /// Extract folder hierarchy from a filesystem path
    pub fn extract_hierarchy_from_path<P: AsRef<Path>>(
        &self,
        base_path: P,
        full_path: P,
    ) -> FolderHierarchyResult<FolderHierarchy> {
        let base = base_path.as_ref();
        let full = full_path.as_ref();

        // Get the relative path from base to full
        let relative_path = full
            .strip_prefix(base)
            .map_err(|_| FolderHierarchyError::InvalidPath(format!(
                "Path {:?} is not within base {:?}",
                full, base
            )))?;

        let components: Vec<&str> = relative_path
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .collect();

        if components.is_empty() {
            return Err(FolderHierarchyError::InvalidPath(
                "No path components found".to_string(),
            ));
        }

        // First component is account ID, rest are folder hierarchy
        let account_id = components[0].to_string();
        let folder_path = if components.len() > 1 {
            self.filesystem_to_imap(&components[1..].join(&self.filesystem_separator))?
        } else {
            "INBOX".to_string() // Default root folder
        };

        Ok(FolderHierarchy {
            account_id,
            imap_path: folder_path,
            filesystem_path: relative_path.to_string_lossy().to_string(),
            depth: components.len() - 1, // Subtract 1 for account ID
        })
    }

    /// Sanitize a single folder component for filesystem use
    fn sanitize_folder_component(&self, component: &str) -> FolderHierarchyResult<String> {
        if component.is_empty() {
            return Err(FolderHierarchyError::InvalidPath(
                "Empty folder component".to_string(),
            ));
        }

        // Check for path traversal attempts first
        if self.strict_validation && (component.contains("..") || component.starts_with('.')) {
            return Err(FolderHierarchyError::PathTraversal(component.to_string()));
        }

        // Check for reserved names
        if RESERVED_NAMES.contains(&component.to_lowercase().as_str()) {
            return Err(FolderHierarchyError::ReservedName(component.to_string()));
        }

        let mut sanitized = component
            .chars()
            .map(|c| match c {
                // Replace filesystem-unsafe characters
                '/' | '\\' => '_',
                ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                // Replace email characters (but not dots for account IDs)
                '@' | '#' | '%' | '&' | '+' | '=' => '_',
                // Replace control characters
                c if c.is_control() => '_',
                // Keep safe characters (including dots)
                c => c,
            })
            .collect::<String>();

        // Remove leading/trailing whitespace and dots only
        sanitized = sanitized.trim().trim_matches('.').to_string();

        // Ensure it's not empty after sanitization
        if sanitized.is_empty() {
            sanitized = "folder".to_string();
        }

        // Check length
        if sanitized.len() > MAX_FOLDER_NAME_LENGTH {
            return Err(FolderHierarchyError::NameTooLong(
                sanitized,
                MAX_FOLDER_NAME_LENGTH,
            ));
        }

        Ok(sanitized)
    }

    /// Reverse sanitization (best effort - some information may be lost)
    fn reverse_sanitize_component(&self, component: &str) -> String {
        // This is a best-effort reverse operation
        // Some sanitization is irreversible (e.g., multiple different chars became '_')
        component.to_string()
    }

    /// Validate a filesystem component
    fn validate_component(&self, component: &str) -> FolderHierarchyResult<()> {
        if component.is_empty() {
            return Err(FolderHierarchyError::InvalidPath(
                "Empty component".to_string(),
            ));
        }

        if component.len() > MAX_FOLDER_NAME_LENGTH {
            return Err(FolderHierarchyError::NameTooLong(
                component.to_string(),
                MAX_FOLDER_NAME_LENGTH,
            ));
        }

        if RESERVED_NAMES.contains(&component.to_lowercase().as_str()) {
            return Err(FolderHierarchyError::ReservedName(component.to_string()));
        }

        if component.contains("..") || component.starts_with('.') {
            return Err(FolderHierarchyError::PathTraversal(component.to_string()));
        }

        Ok(())
    }

    /// Get the IMAP separator character
    pub fn imap_separator(&self) -> char {
        self.imap_separator
    }

    /// Get the filesystem separator string
    pub fn filesystem_separator(&self) -> &str {
        &self.filesystem_separator
    }

    /// Check if strict validation is enabled
    pub fn is_strict_validation(&self) -> bool {
        self.strict_validation
    }

    /// Generate a list of parent folders for a given IMAP path
    pub fn get_parent_folders(&self, imap_path: &str) -> Vec<String> {
        let mut parents = Vec::new();
        let components: Vec<&str> = imap_path.split(self.imap_separator).collect();

        for i in 1..components.len() {
            let parent_path = components[0..i].join(&self.imap_separator.to_string());
            parents.push(parent_path);
        }

        parents
    }

    /// Check if one folder is a parent of another
    pub fn is_parent_folder(&self, potential_parent: &str, child: &str) -> bool {
        if potential_parent.is_empty() {
            return false;
        }

        child.starts_with(potential_parent)
            && child.len() > potential_parent.len()
            && child.chars().nth(potential_parent.len()) == Some(self.imap_separator)
    }

    /// Get the immediate parent folder of a given path
    pub fn get_parent_folder(&self, imap_path: &str) -> Option<String> {
        if let Some(last_separator) = imap_path.rfind(self.imap_separator) {
            Some(imap_path[..last_separator].to_string())
        } else {
            None
        }
    }

    /// Get the folder name (last component) from a path
    pub fn get_folder_name(&self, imap_path: &str) -> String {
        if let Some(last_separator) = imap_path.rfind(self.imap_separator) {
            imap_path[last_separator + 1..].to_string()
        } else {
            imap_path.to_string()
        }
    }
}

/// Represents the hierarchy information of a folder
#[derive(Debug, Clone, PartialEq)]
pub struct FolderHierarchy {
    /// Account ID this folder belongs to
    pub account_id: String,
    /// IMAP-style folder path
    pub imap_path: String,
    /// Filesystem path relative to base
    pub filesystem_path: String,
    /// Depth in the folder hierarchy (0 = root level)
    pub depth: usize,
}

impl FolderHierarchy {
    /// Create a new folder hierarchy
    pub fn new(
        account_id: String,
        imap_path: String,
        filesystem_path: String,
        depth: usize,
    ) -> Self {
        Self {
            account_id,
            imap_path,
            filesystem_path,
            depth,
        }
    }

    /// Check if this is a root-level folder
    pub fn is_root_level(&self) -> bool {
        self.depth == 0
    }

    /// Get the display name for this folder
    pub fn display_name(&self) -> &str {
        if let Some(last_slash) = self.imap_path.rfind('/') {
            &self.imap_path[last_slash + 1..]
        } else {
            &self.imap_path
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_mapper_creation() {
        let mapper = FolderHierarchyMapper::new();
        assert_eq!(mapper.imap_separator(), '/');
        assert_eq!(mapper.filesystem_separator(), "__");
        assert!(mapper.is_strict_validation());
    }

    #[test]
    fn test_mapper_with_settings() {
        let mapper = FolderHierarchyMapper::with_settings('.', "++".to_string(), false);
        assert_eq!(mapper.imap_separator(), '.');
        assert_eq!(mapper.filesystem_separator(), "++");
        assert!(!mapper.is_strict_validation());
    }

    #[test]
    fn test_imap_to_filesystem_simple() {
        let mapper = FolderHierarchyMapper::new();
        let result = mapper.imap_to_filesystem("INBOX").unwrap();
        assert_eq!(result, "INBOX");
    }

    #[test]
    fn test_imap_to_filesystem_nested() {
        let mapper = FolderHierarchyMapper::new();
        let result = mapper.imap_to_filesystem("INBOX/Work/Project").unwrap();
        assert_eq!(result, "INBOX__Work__Project");
    }

    #[test]
    fn test_imap_to_filesystem_special_chars() {
        let mapper = FolderHierarchyMapper::new();
        let result = mapper.imap_to_filesystem("INBOX/Folder:With*Special?Chars").unwrap();
        assert_eq!(result, "INBOX__Folder_With_Special_Chars");
    }

    #[test]
    fn test_filesystem_to_imap() {
        let mapper = FolderHierarchyMapper::new();
        let result = mapper.filesystem_to_imap("INBOX__Work__Project").unwrap();
        assert_eq!(result, "INBOX/Work/Project");
    }

    #[test]
    fn test_create_maildir_path() {
        let mapper = FolderHierarchyMapper::new();
        let temp_dir = TempDir::new().unwrap();
        
        let result = mapper
            .create_maildir_path(temp_dir.path(), "user@example.com", "INBOX/Work")
            .unwrap();
        
        let expected = temp_dir.path().join("user_example.com").join("INBOX__Work");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_sanitize_folder_component() {
        let mapper = FolderHierarchyMapper::new();
        
        // Normal case
        let result = mapper.sanitize_folder_component("NormalFolder").unwrap();
        assert_eq!(result, "NormalFolder");
        
        // Special characters
        let result = mapper.sanitize_folder_component("Folder/With\\Special:Chars*").unwrap();
        assert_eq!(result, "Folder_With_Special_Chars_");
        
        // Leading/trailing dots and spaces
        let result = mapper.sanitize_folder_component("  .folder.  ").unwrap();
        assert_eq!(result, "folder");
    }

    #[test]
    fn test_sanitize_reserved_name() {
        let mapper = FolderHierarchyMapper::new();
        let result = mapper.sanitize_folder_component("con");
        assert!(result.is_err());
        
        if let Err(FolderHierarchyError::ReservedName(name)) = result {
            assert_eq!(name, "con");
        } else {
            panic!("Expected ReservedName error");
        }
    }

    #[test]
    fn test_sanitize_empty_component() {
        let mapper = FolderHierarchyMapper::new();
        let result = mapper.sanitize_folder_component("");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_parent_folders() {
        let mapper = FolderHierarchyMapper::new();
        let parents = mapper.get_parent_folders("INBOX/Work/Project/Subfolder");
        
        assert_eq!(parents, vec![
            "INBOX".to_string(),
            "INBOX/Work".to_string(),
            "INBOX/Work/Project".to_string(),
        ]);
    }

    #[test]
    fn test_is_parent_folder() {
        let mapper = FolderHierarchyMapper::new();
        
        assert!(mapper.is_parent_folder("INBOX", "INBOX/Work"));
        assert!(mapper.is_parent_folder("INBOX/Work", "INBOX/Work/Project"));
        assert!(!mapper.is_parent_folder("INBOX/Work", "INBOX/Personal"));
        assert!(!mapper.is_parent_folder("INBOX/Work", "INBOX"));
    }

    #[test]
    fn test_get_parent_folder() {
        let mapper = FolderHierarchyMapper::new();
        
        assert_eq!(mapper.get_parent_folder("INBOX/Work/Project"), Some("INBOX/Work".to_string()));
        assert_eq!(mapper.get_parent_folder("INBOX/Work"), Some("INBOX".to_string()));
        assert_eq!(mapper.get_parent_folder("INBOX"), None);
    }

    #[test]
    fn test_get_folder_name() {
        let mapper = FolderHierarchyMapper::new();
        
        assert_eq!(mapper.get_folder_name("INBOX/Work/Project"), "Project");
        assert_eq!(mapper.get_folder_name("INBOX/Work"), "Work");
        assert_eq!(mapper.get_folder_name("INBOX"), "INBOX");
    }

    #[test]
    fn test_folder_hierarchy_creation() {
        let hierarchy = FolderHierarchy::new(
            "user@example.com".to_string(),
            "INBOX/Work".to_string(),
            "user_example_com/INBOX__Work".to_string(),
            1,
        );
        
        assert_eq!(hierarchy.account_id, "user@example.com");
        assert_eq!(hierarchy.imap_path, "INBOX/Work");
        assert!(!hierarchy.is_root_level());
        assert_eq!(hierarchy.display_name(), "Work");
    }

    #[test]
    fn test_extract_hierarchy_from_path() {
        let mapper = FolderHierarchyMapper::new();
        let temp_dir = TempDir::new().unwrap();
        
        let base_path = temp_dir.path();
        let full_path = base_path.join("user_example_com").join("INBOX__Work");
        
        let hierarchy = mapper.extract_hierarchy_from_path(base_path, &full_path).unwrap();
        
        assert_eq!(hierarchy.account_id, "user_example_com");
        assert_eq!(hierarchy.imap_path, "INBOX/Work");
        assert_eq!(hierarchy.depth, 1);
    }

    #[test]
    fn test_path_traversal_detection() {
        let mapper = FolderHierarchyMapper::new();
        
        let result = mapper.sanitize_folder_component("../../../etc/passwd");
        assert!(result.is_err());
        
        if let Err(FolderHierarchyError::PathTraversal(_)) = result {
            // Expected
        } else {
            panic!("Expected PathTraversal error");
        }
    }

    #[test]
    fn test_name_too_long() {
        let mapper = FolderHierarchyMapper::new();
        let long_name = "a".repeat(300);
        
        let result = mapper.sanitize_folder_component(&long_name);
        assert!(result.is_err());
        
        if let Err(FolderHierarchyError::NameTooLong(_, max)) = result {
            assert_eq!(max, MAX_FOLDER_NAME_LENGTH);
        } else {
            panic!("Expected NameTooLong error");
        }
    }
}