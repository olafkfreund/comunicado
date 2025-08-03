//! Markdown parser and content processor
//! 
//! Handles parsing of markdown files with YAML frontmatter, wiki-link extraction,
//! and content processing including title extraction, word counting, and tag parsing.

use super::types::{Note, NoteFrontmatter, WikiLink, LinkType};
use super::manager::NoteResult;

use std::path::PathBuf;
use regex::Regex;
use serde_yaml;
use pulldown_cmark::{Parser, Event, Tag, HeadingLevel};
use chrono::Utc;
use std::collections::HashMap;

/// Markdown parser for note content
#[derive(Debug, Clone)]
pub struct MarkdownParser {
    /// Regex for extracting wiki links
    wiki_link_regex: Regex,
    /// Regex for extracting tags from content
    tag_regex: Regex,
}

impl Default for MarkdownParser {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownParser {
    /// Create a new markdown parser
    pub fn new() -> Self {
        // Updated regex to capture embed links with ! prefix
        let wiki_link_regex = Regex::new(r"!?\[\[([^\]]+)\]\]").expect("Invalid wiki link regex");
        let tag_regex = Regex::new(r"(?:^|\s)#([a-zA-Z0-9_-]+)").expect("Invalid tag regex");
        
        Self {
            wiki_link_regex,
            tag_regex,
        }
    }

    /// Parse a markdown file and extract all note information
    pub fn parse_note(&self, 
                     id: String, 
                     file_path: PathBuf, 
                     raw_content: &str) -> NoteResult<Note> {
        // Split frontmatter and content
        let (frontmatter, content) = self.extract_frontmatter(raw_content)?;
        
        // Extract title (from frontmatter or first heading)
        let title = self.extract_title(&frontmatter, &content, &file_path);
        
        // Extract wiki links
        let links = self.extract_wiki_links(&content);
        
        // Extract tags (from frontmatter and content)
        let tags = self.extract_tags(&frontmatter, &content);
        
        // Calculate word count
        let word_count = self.calculate_word_count(&content);
        
        // Calculate content hash
        let content_hash = self.calculate_content_hash(&content);
        
        // Get file size
        let file_size = raw_content.len() as u64;
        
        let now = Utc::now();
        
        Ok(Note {
            id,
            title,
            content,
            path: file_path,
            frontmatter,
            created_at: now,
            modified_at: now,
            word_count,
            tags,
            links,
            file_size,
            content_hash,
            is_deleted: false,
        })
    }

    /// Extract YAML frontmatter from markdown content
    pub fn extract_frontmatter(&self, content: &str) -> NoteResult<(Option<NoteFrontmatter>, String)> {
        if !content.starts_with("---\n") {
            return Ok((None, content.to_string()));
        }

        // Find the end of frontmatter
        let content_lines: Vec<&str> = content.lines().collect();
        if content_lines.len() < 3 {
            return Ok((None, content.to_string()));
        }

        let mut end_line = None;
        for (i, line) in content_lines.iter().enumerate().skip(1) {
            if line.trim() == "---" {
                end_line = Some(i);
                break;
            }
        }

        let end_line = match end_line {
            Some(line) => line,
            None => return Ok((None, content.to_string())),
        };

        // Extract frontmatter YAML
        let frontmatter_lines: Vec<&str> = content_lines[1..end_line].to_vec();
        let frontmatter_yaml = frontmatter_lines.join("\n");
        
        // Parse YAML frontmatter
        let frontmatter: NoteFrontmatter = match serde_yaml::from_str(&frontmatter_yaml) {
            Ok(fm) => fm,
            Err(_) => return Ok((None, content.to_string())),
        };

        // Extract remaining content
        let remaining_content = if end_line + 1 < content_lines.len() {
            content_lines[end_line + 1..].join("\n")
        } else {
            String::new()
        };

        Ok((Some(frontmatter), remaining_content))
    }

    /// Extract title from frontmatter or first heading
    pub fn extract_title(&self, frontmatter: &Option<NoteFrontmatter>, content: &str, file_path: &std::path::Path) -> String {
        // Check frontmatter first
        if let Some(ref fm) = frontmatter {
            if let Some(ref title) = fm.title {
                return title.clone();
            }
        }

        // Extract from first heading - extract raw markdown
        let lines: Vec<&str> = content.lines().collect();
        for line in lines {
            let trimmed = line.trim();
            if trimmed.starts_with("# ") && trimmed.len() > 2 {
                return trimmed[2..].to_string();
            }
        }

        // Fallback: use filename without extension
        self.extract_filename_title(file_path)
    }

    /// Extract filename as title fallback
    fn extract_filename_title(&self, path: &std::path::Path) -> String {
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled")
            .to_string()
    }

    /// Extract wiki links from content with enhanced parsing
    pub fn extract_wiki_links(&self, content: &str) -> Vec<WikiLink> {
        self.extract_wiki_links_with_positions(content).into_iter().map(|(link, _)| link).collect()
    }

    /// Extract wiki links with line numbers and positions
    pub fn extract_wiki_links_with_positions(&self, content: &str) -> Vec<(WikiLink, usize)> {
        let mut links = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        
        for (line_number, line) in lines.iter().enumerate() {
            for cap in self.wiki_link_regex.captures_iter(line) {
                if let Some(link_text) = cap.get(1) {
                    let text = link_text.as_str();
                    let full_match = cap.get(0).unwrap().as_str();
                    
                    // Check if this is an embed link by looking at the full match
                    let is_embed = full_match.starts_with("![[");
                    
                    // Parse different types of wiki links
                    let (link_target, display_text, mut link_type) = self.parse_wiki_link_content(text);
                    
                    // Override link type if this is an embed
                    if is_embed {
                        link_type = LinkType::Embed;
                    }

                    // Create a wiki link with actual line number
                    let mut link = WikiLink::new(
                        "placeholder".to_string(), // source_note_id will be set when storing
                        link_target,
                        line_number + 1, // Convert to 1-based line numbers
                        link_type,
                    );
                    
                    if let Some(display) = display_text {
                        link.display_text = Some(display);
                    }

                    links.push((link, line_number + 1));
                }
            }
        }
        
        links
    }

    /// Parse the content inside wiki link brackets to determine type and extract information
    fn parse_wiki_link_content(&self, content: &str) -> (String, Option<String>, LinkType) {
        let trimmed = content.trim();
        
        // Check for embed links ![[target]]
        if trimmed.starts_with('!') {
            let embed_content = &trimmed[1..];
            if let Some((target, display)) = self.parse_link_with_display(embed_content) {
                return (target, display, LinkType::Embed);
            }
        }
        
        // Check for block references [[target#^block]]
        if trimmed.contains("#^") {
            if let Some((target, display)) = self.parse_link_with_display(trimmed) {
                return (target, display, LinkType::Block);
            }
        }
        
        // Check for tag links [[#tag]]
        if trimmed.starts_with('#') {
            let tag_content = &trimmed[1..];
            if let Some((target, display)) = self.parse_link_with_display(tag_content) {
                return (target, display, LinkType::Tag);
            }
        }
        
        // Standard wiki link
        if let Some((target, display)) = self.parse_link_with_display(trimmed) {
            (target, display, LinkType::Wiki)
        } else {
            (trimmed.to_string(), None, LinkType::Wiki)
        }
    }

    /// Parse link content with optional display text
    fn parse_link_with_display(&self, content: &str) -> Option<(String, Option<String>)> {
        if content.contains('|') {
            let parts: Vec<&str> = content.splitn(2, '|').collect();
            if parts.len() == 2 {
                let target = parts[0].trim().to_string();
                let display = parts[1].trim().to_string();
                return Some((target, if display.is_empty() { None } else { Some(display) }));
            }
        }
        
        Some((content.trim().to_string(), None))
    }

    /// Extract all types of links from content (wiki links, markdown links, URLs)
    pub fn extract_all_links(&self, content: &str) -> Vec<WikiLink> {
        let mut links = Vec::new();
        
        // Extract wiki links
        links.extend(self.extract_wiki_links(content));
        
        // Extract markdown links [text](url)
        let markdown_link_regex = Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").expect("Invalid markdown link regex");
        for cap in markdown_link_regex.captures_iter(content) {
            if let (Some(text), Some(url)) = (cap.get(1), cap.get(2)) {
                let mut link = WikiLink::new(
                    "placeholder".to_string(),
                    url.as_str().to_string(),
                    0,
                    LinkType::Wiki, // Treat as wiki link for now
                );
                link.display_text = Some(text.as_str().to_string());
                links.push(link);
            }
        }
        
        links
    }

    /// Validate wiki link target format
    pub fn validate_wiki_link_target(&self, target: &str) -> NoteResult<()> {
        if target.trim().is_empty() {
            return Err(super::manager::NoteError::Parse("Empty wiki link target".into()));
        }
        
        if target.len() > 200 {
            return Err(super::manager::NoteError::Parse("Wiki link target too long (max 200 characters)".into()));
        }
        
        // Check for invalid characters
        let invalid_chars = ['<', '>', ':', '"', '|', '?', '*'];
        for ch in invalid_chars {
            if target.contains(ch) {
                return Err(super::manager::NoteError::Parse(format!("Invalid character '{}' in wiki link target", ch)));
            }
        }
        
        Ok(())
    }

    /// Resolve wiki link target to potential file paths
    pub fn resolve_wiki_link_paths(&self, target: &str, base_dir: &std::path::Path) -> Vec<std::path::PathBuf> {
        let mut paths = Vec::new();
        
        // Direct match
        paths.push(base_dir.join(format!("{}.md", target)));
        
        // Case-insensitive match
        let lowercase_target = target.to_lowercase();
        paths.push(base_dir.join(format!("{}.md", lowercase_target)));
        
        // With spaces replaced by hyphens
        let hyphenated = target.replace(' ', "-");
        paths.push(base_dir.join(format!("{}.md", hyphenated)));
        
        // With spaces replaced by underscores
        let underscored = target.replace(' ', "_");
        paths.push(base_dir.join(format!("{}.md", underscored)));
        
        // Slugified version
        let slugified = target
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
            .collect::<String>();
        paths.push(base_dir.join(format!("{}.md", slugified)));
        
        paths
    }

    /// Update wiki links in content with new target names
    pub fn update_wiki_links_in_content(&self, content: &str, old_target: &str, new_target: &str) -> String {
        let old_pattern = format!("[[{}]]", old_target);
        let new_pattern = format!("[[{}]]", new_target);
        let result = content.replace(&old_pattern, &new_pattern);
        
        // Also handle links with display text
        let old_pattern_with_display = format!("[[{}|", old_target);
        let new_pattern_with_display = format!("[[{}|", new_target);
        result.replace(&old_pattern_with_display, &new_pattern_with_display)
    }

    /// Extract tags from frontmatter and content
    pub fn extract_tags(&self, frontmatter: &Option<NoteFrontmatter>, content: &str) -> Vec<String> {
        let mut tags = std::collections::HashSet::new();
        
        // Extract from frontmatter
        if let Some(ref fm) = frontmatter {
            for tag in &fm.tags {
                tags.insert(tag.clone());
            }
        }
        
        // Extract hashtags from content
        for cap in self.tag_regex.captures_iter(content) {
            if let Some(tag_match) = cap.get(1) {
                tags.insert(tag_match.as_str().to_string());
            }
        }
        
        let mut tag_vec: Vec<String> = tags.into_iter().collect();
        tag_vec.sort();
        tag_vec
    }

    /// Calculate word count for content
    pub fn calculate_word_count(&self, content: &str) -> usize {
        // Parse markdown and extract text content excluding headings
        let parser = Parser::new(content);
        let mut plain_text = String::new();
        let mut in_heading = false;
        
        for event in parser {
            match event {
                Event::Start(Tag::Heading(_, _, _)) => {
                    in_heading = true;
                }
                Event::End(Tag::Heading(_, _, _)) => {
                    in_heading = false;
                }
                Event::Text(text) => {
                    // Only exclude text in headings
                    if !in_heading {
                        plain_text.push_str(&text);
                        plain_text.push(' ');
                    }
                }
                _ => {}
            }
        }
        
        // Count words, excluding empty strings and pure whitespace
        plain_text
            .split_whitespace()
            .filter(|word| !word.is_empty() && word.chars().any(|c| c.is_alphanumeric()))
            .count()
    }

    /// Calculate content hash for change detection
    pub fn calculate_content_hash(&self, content: &str) -> String {
        use sha2::{Sha256, Digest};
        
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Validate frontmatter structure with comprehensive checks
    pub fn validate_frontmatter(&self, frontmatter: &NoteFrontmatter) -> NoteResult<()> {
        // Title validation
        if let Some(ref title) = frontmatter.title {
            if title.trim().is_empty() {
                return Err(super::manager::NoteError::Parse("Empty title in frontmatter".into()));
            }
            if title.len() > 200 {
                return Err(super::manager::NoteError::Parse("Title too long (max 200 characters)".into()));
            }
            // Check for invalid characters in title
            if title.contains('\n') || title.contains('\r') {
                return Err(super::manager::NoteError::Parse("Title cannot contain line breaks".into()));
            }
        }

        // Tags validation
        if frontmatter.tags.len() > 50 {
            return Err(super::manager::NoteError::Parse("Too many tags (max 50)".into()));
        }
        for tag in &frontmatter.tags {
            if tag.trim().is_empty() {
                return Err(super::manager::NoteError::Parse("Empty tag found".into()));
            }
            if tag.len() > 50 {
                return Err(super::manager::NoteError::Parse("Tag too long (max 50 characters)".into()));
            }
            // Tags should only contain alphanumeric, hyphens, and underscores
            if !tag.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
                return Err(super::manager::NoteError::Parse(format!("Invalid tag format: '{}'", tag)));
            }
        }

        // Author validation
        if let Some(ref author) = frontmatter.author {
            if author.trim().is_empty() {
                return Err(super::manager::NoteError::Parse("Empty author field".into()));
            }
            if author.len() > 100 {
                return Err(super::manager::NoteError::Parse("Author name too long (max 100 characters)".into()));
            }
        }

        // Template validation
        if let Some(ref template) = frontmatter.template {
            if template.trim().is_empty() {
                return Err(super::manager::NoteError::Parse("Empty template field".into()));
            }
            if template.len() > 50 {
                return Err(super::manager::NoteError::Parse("Template name too long (max 50 characters)".into()));
            }
        }

        // Categories validation
        if frontmatter.categories.len() > 20 {
            return Err(super::manager::NoteError::Parse("Too many categories (max 20)".into()));
        }
        for category in &frontmatter.categories {
            if category.trim().is_empty() {
                return Err(super::manager::NoteError::Parse("Empty category found".into()));
            }
            if category.len() > 50 {
                return Err(super::manager::NoteError::Parse("Category too long (max 50 characters)".into()));
            }
        }

        // Aliases validation
        if frontmatter.aliases.len() > 10 {
            return Err(super::manager::NoteError::Parse("Too many aliases (max 10)".into()));
        }
        for alias in &frontmatter.aliases {
            if alias.trim().is_empty() {
                return Err(super::manager::NoteError::Parse("Empty alias found".into()));
            }
            if alias.len() > 100 {
                return Err(super::manager::NoteError::Parse("Alias too long (max 100 characters)".into()));
            }
        }

        // Date validation
        if let Some(date) = frontmatter.date {
            let now = Utc::now();
            let one_year_future = now + chrono::Duration::days(365);
            let far_past = now - chrono::Duration::days(365 * 50); // 50 years ago
            
            if date > one_year_future {
                return Err(super::manager::NoteError::Parse("Date cannot be more than one year in the future".into()));
            }
            if date < far_past {
                return Err(super::manager::NoteError::Parse("Date cannot be more than 50 years in the past".into()));
            }
        }

        // Custom metadata validation
        if frontmatter.metadata.len() > 50 {
            return Err(super::manager::NoteError::Parse("Too many custom metadata fields (max 50)".into()));
        }
        for (key, value) in &frontmatter.metadata {
            if key.trim().is_empty() {
                return Err(super::manager::NoteError::Parse("Empty metadata key found".into()));
            }
            if key.len() > 50 {
                return Err(super::manager::NoteError::Parse("Metadata key too long (max 50 characters)".into()));
            }
            // Validate metadata value size
            let value_str = match value {
                serde_yaml::Value::String(s) => s.clone(),
                _ => serde_yaml::to_string(value).unwrap_or_default(),
            };
            if value_str.len() > 1000 {
                return Err(super::manager::NoteError::Parse("Metadata value too long (max 1000 characters)".into()));
            }
        }

        Ok(())
    }

    /// Parse YAML frontmatter with advanced validation and normalization
    pub fn parse_and_validate_frontmatter(&self, yaml_content: &str) -> NoteResult<NoteFrontmatter> {
        // First attempt to parse the YAML
        let mut frontmatter: NoteFrontmatter = serde_yaml::from_str(yaml_content)
            .map_err(|e| super::manager::NoteError::Parse(format!("YAML parse error: {}", e)))?;

        // Normalize and clean the frontmatter
        self.normalize_frontmatter(&mut frontmatter);

        // Validate the frontmatter
        self.validate_frontmatter(&frontmatter)?;

        Ok(frontmatter)
    }

    /// Normalize frontmatter fields (trim whitespace, deduplicate, etc.)
    pub fn normalize_frontmatter(&self, frontmatter: &mut NoteFrontmatter) {
        // Normalize title
        if let Some(ref mut title) = frontmatter.title {
            *title = title.trim().to_string();
            if title.is_empty() {
                frontmatter.title = None;
            }
        }

        // Normalize author
        if let Some(ref mut author) = frontmatter.author {
            *author = author.trim().to_string();
            if author.is_empty() {
                frontmatter.author = None;
            }
        }

        // Normalize template
        if let Some(ref mut template) = frontmatter.template {
            *template = template.trim().to_string();
            if template.is_empty() {
                frontmatter.template = None;
            }
        }

        // Normalize tags (trim, deduplicate, lowercase, remove empty)
        frontmatter.tags = frontmatter.tags
            .iter()
            .map(|tag| tag.trim().to_lowercase())
            .filter(|tag| !tag.is_empty())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        frontmatter.tags.sort();

        // Normalize categories (trim, deduplicate, remove empty)
        frontmatter.categories = frontmatter.categories
            .iter()
            .map(|cat| cat.trim().to_string())
            .filter(|cat| !cat.is_empty())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        frontmatter.categories.sort();

        // Normalize aliases (trim, deduplicate, remove empty)
        frontmatter.aliases = frontmatter.aliases
            .iter()
            .map(|alias| alias.trim().to_string())
            .filter(|alias| !alias.is_empty())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        frontmatter.aliases.sort();

        // Clean up metadata (remove empty keys/values)
        frontmatter.metadata.retain(|key, value| {
            !key.trim().is_empty() && match value {
                serde_yaml::Value::String(s) => !s.trim().is_empty(),
                serde_yaml::Value::Null => false,
                _ => true,
            }
        });
    }

    /// Extract frontmatter with enhanced parsing and validation
    pub fn extract_frontmatter_advanced(&self, content: &str) -> NoteResult<(Option<NoteFrontmatter>, String)> {
        if !content.starts_with("---\n") {
            return Ok((None, content.to_string()));
        }

        // Find the end of frontmatter
        let content_lines: Vec<&str> = content.lines().collect();
        if content_lines.len() < 3 {
            return Ok((None, content.to_string()));
        }

        let mut end_line = None;
        for (i, line) in content_lines.iter().enumerate().skip(1) {
            if line.trim() == "---" {
                end_line = Some(i);
                break;
            }
        }

        let end_line = match end_line {
            Some(line) => line,
            None => return Ok((None, content.to_string())),
        };

        // Extract frontmatter YAML
        let frontmatter_lines: Vec<&str> = content_lines[1..end_line].to_vec();
        let frontmatter_yaml = frontmatter_lines.join("\n");
        
        // Parse and validate using the advanced method
        let frontmatter = match self.parse_and_validate_frontmatter(&frontmatter_yaml) {
            Ok(fm) => Some(fm),
            Err(e) => {
                // Log the error but don't fail completely - return None for malformed frontmatter
                eprintln!("Frontmatter validation failed: {}", e);
                None
            }
        };

        // Extract remaining content
        let remaining_content = if end_line + 1 < content_lines.len() {
            content_lines[end_line + 1..].join("\n")
        } else {
            String::new()
        };

        Ok((frontmatter, remaining_content))
    }

    /// Generate frontmatter as YAML string
    pub fn frontmatter_to_yaml(&self, frontmatter: &NoteFrontmatter) -> NoteResult<String> {
        serde_yaml::to_string(frontmatter)
            .map_err(|e| super::manager::NoteError::Parse(format!("YAML serialization error: {}", e)))
    }

    /// Merge two frontmatter objects, with the second taking precedence
    pub fn merge_frontmatter(&self, base: &NoteFrontmatter, override_fm: &NoteFrontmatter) -> NoteFrontmatter {
        let mut merged = base.clone();

        // Override fields if present in override_fm
        if override_fm.title.is_some() {
            merged.title = override_fm.title.clone();
        }
        if override_fm.author.is_some() {
            merged.author = override_fm.author.clone();
        }
        if override_fm.template.is_some() {
            merged.template = override_fm.template.clone();
        }
        if override_fm.draft.is_some() {
            merged.draft = override_fm.draft;
        }
        if override_fm.date.is_some() {
            merged.date = override_fm.date;
        }

        // Merge collections
        let mut all_tags = merged.tags.clone();
        all_tags.extend(override_fm.tags.clone());
        merged.tags = all_tags.into_iter().collect::<std::collections::HashSet<_>>().into_iter().collect();
        merged.tags.sort();

        let mut all_categories = merged.categories.clone();
        all_categories.extend(override_fm.categories.clone());
        merged.categories = all_categories.into_iter().collect::<std::collections::HashSet<_>>().into_iter().collect();
        merged.categories.sort();

        let mut all_aliases = merged.aliases.clone();
        all_aliases.extend(override_fm.aliases.clone());
        merged.aliases = all_aliases.into_iter().collect::<std::collections::HashSet<_>>().into_iter().collect();
        merged.aliases.sort();

        // Merge metadata (override takes precedence)
        for (key, value) in &override_fm.metadata {
            merged.metadata.insert(key.clone(), value.clone());
        }

        merged
    }

    /// Sanitize and clean content
    /// Sanitize markdown content with comprehensive validation
    pub fn sanitize_content(&self, content: &str) -> String {
        // 1. Normalize line endings
        let mut sanitized = content.replace("\r\n", "\n").replace('\r', "\n");
        
        // 2. Remove or escape potentially dangerous HTML tags if any HTML is present
        // This is basic protection - for more complex cases, would use ammonia crate
        sanitized = sanitized
            .replace("<script", "&lt;script")
            .replace("<iframe", "&lt;iframe")
            .replace("<object", "&lt;object")
            .replace("<embed", "&lt;embed")
            .replace("<form", "&lt;form");
        
        // 3. Normalize excessive whitespace (but preserve intentional formatting)
        let lines: Vec<&str> = sanitized.lines().collect();
        let mut normalized_lines = Vec::new();
        let mut consecutive_empty = 0;
        
        for line in lines {
            if line.trim().is_empty() {
                consecutive_empty += 1;
                // Limit consecutive empty lines to 2 (for paragraph breaks)
                if consecutive_empty <= 2 {
                    normalized_lines.push(line.to_string());
                }
            } else {
                consecutive_empty = 0;
                // Normalize trailing whitespace but preserve leading whitespace (for lists/code)
                normalized_lines.push(line.trim_end().to_string());
            }
        }
        
        // 4. Ensure content ends with single newline
        let result = normalized_lines.join("\n");
        if result.is_empty() {
            result
        } else if result.ends_with('\n') {
            result
        } else {
            format!("{}\n", result)
        }
    }

    /// Validate markdown content structure and format
    pub fn validate_content(&self, content: &str) -> NoteResult<()> {
        // 1. Check content length limits
        const MAX_CONTENT_LENGTH: usize = 1_000_000; // 1MB limit
        if content.len() > MAX_CONTENT_LENGTH {
            return Err(super::manager::NoteError::Parse(
                format!("Content too large: {} bytes (max {})", content.len(), MAX_CONTENT_LENGTH)
            ));
        }
        
        // 2. Validate markdown structure
        let parser = Parser::new(content);
        let mut heading_levels = Vec::new();
        let mut list_nesting_level = 0;
        const MAX_HEADING_LEVEL: u32 = 6;
        const MAX_LIST_NESTING: usize = 10;
        
        for event in parser {
            match event {
                Event::Start(Tag::Heading(level, _, _)) => {
                    let level_num = match level {
                        HeadingLevel::H1 => 1,
                        HeadingLevel::H2 => 2,
                        HeadingLevel::H3 => 3,
                        HeadingLevel::H4 => 4,
                        HeadingLevel::H5 => 5,
                        HeadingLevel::H6 => 6,
                    };
                    
                    if level_num > MAX_HEADING_LEVEL {
                        return Err(super::manager::NoteError::Parse(
                            format!("Invalid heading level: H{} (max H{})", level_num, MAX_HEADING_LEVEL)
                        ));
                    }
                    heading_levels.push(level_num);
                }
                Event::Start(Tag::CodeBlock(_)) => {
                    // Could track code block state if needed for validation
                }
                Event::End(Tag::CodeBlock(_)) => {
                    // Could track code block state if needed for validation
                }
                Event::Start(Tag::List(_)) => {
                    list_nesting_level += 1;
                    if list_nesting_level > MAX_LIST_NESTING {
                        return Err(super::manager::NoteError::Parse(
                            format!("List nesting too deep: {} levels (max {})", list_nesting_level, MAX_LIST_NESTING)
                        ));
                    }
                }
                Event::End(Tag::List(_)) => {
                    if list_nesting_level > 0 {
                        list_nesting_level -= 1;
                    }
                }
                _ => {}
            }
        }
        
        // 3. Validate heading hierarchy (should be logical progression)
        for window in heading_levels.windows(2) {
            let current = window[0];
            let next = window[1];
            
            // Allow same level or one level deeper, or any level shallower
            if next > current + 1 {
                return Err(super::manager::NoteError::Parse(
                    format!("Invalid heading progression: H{} followed by H{} (skipped levels)", current, next)
                ));
            }
        }
        
        // 4. Check for valid UTF-8 and basic character validation
        if !content.is_ascii() {
            // Ensure UTF-8 validity (should already be valid, but double-check)
            match std::str::from_utf8(content.as_bytes()) {
                Ok(_) => {}, // Valid UTF-8
                Err(_) => return Err(super::manager::NoteError::Parse("Invalid UTF-8 content".into())),
            }
        }
        
        Ok(())
    }

    /// Validate and sanitize content in one operation
    pub fn validate_and_sanitize(&self, content: &str) -> NoteResult<String> {
        // First validate the raw content
        self.validate_content(content)?;
        
        // Then sanitize it
        Ok(self.sanitize_content(content))
    }

    /// Extract metadata statistics
    pub fn extract_metadata(&self, content: &str) -> HashMap<String, String> {
        let mut metadata = HashMap::new();
        
        // Count different markdown elements
        let parser = Parser::new(content);
        let mut heading_count = 0;
        let mut link_count = 0;
        let mut code_block_count = 0;
        
        for event in parser {
            match event {
                Event::Start(Tag::Heading(_, _, _)) => heading_count += 1,
                Event::Start(Tag::Link(_, _, _)) => link_count += 1,
                Event::Start(Tag::CodeBlock(_)) => code_block_count += 1,
                _ => {}
            }
        }
        
        metadata.insert("headings".to_string(), heading_count.to_string());
        metadata.insert("links".to_string(), link_count.to_string());
        metadata.insert("code_blocks".to_string(), code_block_count.to_string());
        
        metadata
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_parser() -> MarkdownParser {
        MarkdownParser::new()
    }

    #[test]
    fn test_parser_creation() {
        let parser = create_test_parser();
        assert!(parser.wiki_link_regex.is_match("[[test link]]"));
        assert!(parser.tag_regex.is_match("#test"));
    }

    #[test]
    fn test_extract_frontmatter_with_yaml() {
        let parser = create_test_parser();
        let content = r#"---
title: "Test Note"
tags: ["test", "example"]
---

# Content Here

This is the actual content."#;

        let (frontmatter, remaining_content) = parser.extract_frontmatter(content).unwrap();
        
        assert!(frontmatter.is_some());
        let fm = frontmatter.unwrap();
        assert_eq!(fm.title, Some("Test Note".to_string()));
        assert_eq!(fm.tags.len(), 2);
        assert!(fm.tags.contains(&"test".to_string()));
        assert!(fm.tags.contains(&"example".to_string()));
        
        assert!(remaining_content.contains("# Content Here"));
        assert!(!remaining_content.contains("---"));
    }

    #[test]
    fn test_extract_frontmatter_without_yaml() {
        let parser = create_test_parser();
        let content = "# Just a regular markdown file\n\nWith some content.";

        let (frontmatter, remaining_content) = parser.extract_frontmatter(content).unwrap();
        
        assert!(frontmatter.is_none());
        assert_eq!(remaining_content, content);
    }

    #[test]
    fn test_extract_frontmatter_malformed_yaml() {
        let parser = create_test_parser();
        let content = r#"---
title: "Test Note
malformed: [unclosed
---

# Content"#;

        let (frontmatter, remaining_content) = parser.extract_frontmatter(content).unwrap();
        
        // Should gracefully handle malformed YAML
        assert!(frontmatter.is_none());
        assert_eq!(remaining_content, content);
    }

    #[test]
    fn test_extract_title_from_frontmatter() {
        let parser = create_test_parser();
        let mut frontmatter = NoteFrontmatter::new();
        frontmatter.title = Some("Frontmatter Title".to_string());
        let content = "# Content Title\n\nSome text.";

        let title = parser.extract_title(&Some(frontmatter), content, &std::path::Path::new("test.md"));
        assert_eq!(title, "Frontmatter Title");
    }

    #[test]
    fn test_extract_title_from_heading() {
        let parser = create_test_parser();
        let content = "# Main Title\n\n## Subtitle\n\nSome content.";

        let title = parser.extract_title(&None, content, &std::path::Path::new("test.md"));
        assert_eq!(title, "Main Title");
    }

    #[test]
    fn test_extract_title_from_heading_with_formatting() {
        let parser = create_test_parser();
        let content = "# **Bold Title** with *emphasis*\n\nContent.";

        let title = parser.extract_title(&None, content, &std::path::Path::new("test.md"));
        assert_eq!(title, "**Bold Title** with *emphasis*");
    }

    #[test]
    fn test_extract_wiki_links_simple() {
        let parser = create_test_parser();
        let content = "Check out [[Another Note]] and [[Third Note]].";

        let links = parser.extract_wiki_links(content);
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].link_text, "Another Note");
        assert_eq!(links[0].display_text, None);
        assert_eq!(links[1].link_text, "Third Note");
    }

    #[test]
    fn test_extract_wiki_links_with_display_text() {
        let parser = create_test_parser();
        let content = "See [[target-note|Display Text]] for details.";

        let links = parser.extract_wiki_links(content);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].link_text, "target-note");
        assert_eq!(links[0].display_text, Some("Display Text".to_string()));
    }

    #[test]
    fn test_extract_wiki_links_mixed() {
        let parser = create_test_parser();
        let content = "Links: [[Simple Link]] and [[complex-link|Complex Display]].";

        let links = parser.extract_wiki_links(content);
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].link_text, "Simple Link");
        assert_eq!(links[0].display_text, None);
        assert_eq!(links[1].link_text, "complex-link");
        assert_eq!(links[1].display_text, Some("Complex Display".to_string()));
    }

    #[test]
    fn test_extract_tags_from_content() {
        let parser = create_test_parser();
        let content = "This note has #rust and #programming tags. Also #note-taking.";

        let tags = parser.extract_tags(&None, content);
        assert_eq!(tags.len(), 3);
        assert!(tags.contains(&"note-taking".to_string()));
        assert!(tags.contains(&"programming".to_string()));
        assert!(tags.contains(&"rust".to_string()));
    }

    #[test]
    fn test_extract_tags_from_frontmatter() {
        let parser = create_test_parser();
        let mut frontmatter = NoteFrontmatter::new();
        frontmatter.add_tag("yaml-tag".to_string());
        frontmatter.add_tag("metadata".to_string());
        let content = "Content with #content-tag.";

        let tags = parser.extract_tags(&Some(frontmatter), content);
        assert_eq!(tags.len(), 3);
        assert!(tags.contains(&"content-tag".to_string()));
        assert!(tags.contains(&"metadata".to_string()));
        assert!(tags.contains(&"yaml-tag".to_string()));
    }

    #[test]
    fn test_extract_tags_deduplication() {
        let parser = create_test_parser();
        let mut frontmatter = NoteFrontmatter::new();
        frontmatter.add_tag("duplicate".to_string());
        let content = "Content with #duplicate tag mentioned twice #duplicate.";

        let tags = parser.extract_tags(&Some(frontmatter), content);
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0], "duplicate");
    }

    #[test]
    fn test_calculate_word_count() {
        let parser = create_test_parser();
        let content = "# Title\n\nThis is a **test** with _five_ words.";

        let word_count = parser.calculate_word_count(content);
        assert_eq!(word_count, 7); // "This", "is", "a", "test", "with", "five", "words"
    }

    #[test]
    fn test_calculate_word_count_with_code() {
        let parser = create_test_parser();
        let content = "Text before\n\n```rust\nlet x = 5;\n```\n\nText after.";

        let word_count = parser.calculate_word_count(content);
        assert_eq!(word_count, 7); // "Text", "before", "let", "x", "5", "Text", "after"
    }

    #[test]
    fn test_calculate_content_hash() {
        let parser = create_test_parser();
        let content1 = "Same content";
        let content2 = "Same content";
        let content3 = "Different content";

        let hash1 = parser.calculate_content_hash(content1);
        let hash2 = parser.calculate_content_hash(content2);
        let hash3 = parser.calculate_content_hash(content3);

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_eq!(hash1.len(), 64); // SHA256 hex string length
    }

    #[test]
    fn test_validate_frontmatter_valid() {
        let parser = create_test_parser();
        let mut frontmatter = NoteFrontmatter::new();
        frontmatter.title = Some("Valid Title".to_string());

        let result = parser.validate_frontmatter(&frontmatter);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_frontmatter_empty_title() {
        let parser = create_test_parser();
        let mut frontmatter = NoteFrontmatter::new();
        frontmatter.title = Some("".to_string());

        let result = parser.validate_frontmatter(&frontmatter);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_frontmatter_long_title() {
        let parser = create_test_parser();
        let mut frontmatter = NoteFrontmatter::new();
        frontmatter.title = Some("a".repeat(201)); // Too long

        let result = parser.validate_frontmatter(&frontmatter);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Title too long"));
    }

    #[test]
    fn test_validate_frontmatter_title_with_newlines() {
        let parser = create_test_parser();
        let mut frontmatter = NoteFrontmatter::new();
        frontmatter.title = Some("Title with\nnewline".to_string());

        let result = parser.validate_frontmatter(&frontmatter);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("line breaks"));
    }

    #[test]
    fn test_validate_frontmatter_too_many_tags() {
        let parser = create_test_parser();
        let mut frontmatter = NoteFrontmatter::new();
        frontmatter.tags = (0..51).map(|i| format!("tag{}", i)).collect(); // Too many

        let result = parser.validate_frontmatter(&frontmatter);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Too many tags"));
    }

    #[test]
    fn test_validate_frontmatter_invalid_tag_format() {
        let parser = create_test_parser();
        let mut frontmatter = NoteFrontmatter::new();
        frontmatter.tags = vec!["valid-tag".to_string(), "invalid tag!".to_string()];

        let result = parser.validate_frontmatter(&frontmatter);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid tag format"));
    }

    #[test]
    fn test_validate_frontmatter_future_date() {
        let parser = create_test_parser();
        let mut frontmatter = NoteFrontmatter::new();
        frontmatter.date = Some(Utc::now() + chrono::Duration::days(400)); // Too far in future

        let result = parser.validate_frontmatter(&frontmatter);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("future"));
    }

    #[test]
    fn test_normalize_frontmatter() {
        let parser = create_test_parser();
        let mut frontmatter = NoteFrontmatter::new();
        frontmatter.title = Some("  Title with spaces  ".to_string());
        frontmatter.tags = vec!["  TAG1  ".to_string(), "tag2".to_string(), "TAG1".to_string()]; // Duplicates and whitespace
        frontmatter.author = Some("  Author Name  ".to_string());

        parser.normalize_frontmatter(&mut frontmatter);

        assert_eq!(frontmatter.title, Some("Title with spaces".to_string()));
        assert_eq!(frontmatter.tags.len(), 2); // Deduplicated
        assert!(frontmatter.tags.contains(&"tag1".to_string())); // Lowercased
        assert!(frontmatter.tags.contains(&"tag2".to_string()));
        assert_eq!(frontmatter.author, Some("Author Name".to_string()));
    }

    #[test]
    fn test_parse_and_validate_frontmatter() {
        let parser = create_test_parser();
        let yaml = r#"
title: "Valid Note"
tags: ["rust", "programming"]
author: "Test Author"
"#;

        let result = parser.parse_and_validate_frontmatter(yaml);
        assert!(result.is_ok());
        
        let frontmatter = result.unwrap();
        assert_eq!(frontmatter.title, Some("Valid Note".to_string()));
        assert_eq!(frontmatter.tags.len(), 2);
        assert_eq!(frontmatter.author, Some("Test Author".to_string()));
    }

    #[test]
    fn test_parse_and_validate_frontmatter_invalid() {
        let parser = create_test_parser();
        let yaml = r#"
title: ""
tags: ["valid", "invalid tag with spaces!"]
"#;

        let result = parser.parse_and_validate_frontmatter(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_frontmatter_to_yaml() {
        let parser = create_test_parser();
        let mut frontmatter = NoteFrontmatter::new();
        frontmatter.title = Some("Test Note".to_string());
        frontmatter.tags = vec!["test".to_string(), "yaml".to_string()];

        let result = parser.frontmatter_to_yaml(&frontmatter);
        assert!(result.is_ok());
        
        let yaml = result.unwrap();
        assert!(yaml.contains("title: Test Note"));
        assert!(yaml.contains("tags:"));
    }

    #[test]
    fn test_merge_frontmatter() {
        let parser = create_test_parser();
        
        let mut base = NoteFrontmatter::new();
        base.title = Some("Base Title".to_string());
        base.tags = vec!["base".to_string()];
        base.author = Some("Base Author".to_string());

        let mut override_fm = NoteFrontmatter::new();
        override_fm.title = Some("Override Title".to_string());
        override_fm.tags = vec!["override".to_string()];
        // No author in override

        let merged = parser.merge_frontmatter(&base, &override_fm);

        assert_eq!(merged.title, Some("Override Title".to_string())); // Overridden
        assert_eq!(merged.author, Some("Base Author".to_string())); // Kept from base
        assert_eq!(merged.tags.len(), 2); // Merged
        assert!(merged.tags.contains(&"base".to_string()));
        assert!(merged.tags.contains(&"override".to_string()));
    }

    #[test]
    fn test_extract_frontmatter_advanced() {
        let parser = create_test_parser();
        let content = r#"---
title: "  Advanced Test  "
tags: ["TAG1", "tag2", "TAG1"]
author: "  Test Author  "
---

# Content Here"#;

        let result = parser.extract_frontmatter_advanced(content);
        assert!(result.is_ok());
        
        let (frontmatter, remaining_content) = result.unwrap();
        assert!(frontmatter.is_some());
        
        let fm = frontmatter.unwrap();
        assert_eq!(fm.title, Some("Advanced Test".to_string())); // Trimmed
        assert_eq!(fm.tags.len(), 2); // Deduplicated and lowercased
        assert!(fm.tags.contains(&"tag1".to_string()));
        assert!(fm.tags.contains(&"tag2".to_string()));
        assert_eq!(fm.author, Some("Test Author".to_string())); // Trimmed
        
        assert!(remaining_content.contains("# Content Here"));
    }

    // Enhanced Wiki-Link Tests

    #[test]
    fn test_extract_wiki_links_with_positions() {
        let parser = create_test_parser();
        let content = "First line with [[Link One]]\nSecond line with [[Link Two|Display Text]]\nThird line";

        let links_with_positions = parser.extract_wiki_links_with_positions(content);
        assert_eq!(links_with_positions.len(), 2);
        
        let (link1, line1) = &links_with_positions[0];
        assert_eq!(link1.link_text, "Link One");
        assert_eq!(*line1, 1);
        
        let (link2, line2) = &links_with_positions[1];
        assert_eq!(link2.link_text, "Link Two");
        assert_eq!(link2.display_text, Some("Display Text".to_string()));
        assert_eq!(*line2, 2);
    }

    #[test]
    fn test_parse_wiki_link_content_embed() {
        let parser = create_test_parser();
        let (target, display, link_type) = parser.parse_wiki_link_content("!image.png");
        
        assert_eq!(target, "image.png");
        assert_eq!(display, None);
        assert_eq!(link_type, LinkType::Embed);
    }

    #[test]
    fn test_parse_wiki_link_content_block_reference() {
        let parser = create_test_parser();
        let (target, display, link_type) = parser.parse_wiki_link_content("note#^block123");
        
        assert_eq!(target, "note#^block123");
        assert_eq!(display, None);
        assert_eq!(link_type, LinkType::Block);
    }

    #[test]
    fn test_parse_wiki_link_content_tag() {
        let parser = create_test_parser();
        let (target, display, link_type) = parser.parse_wiki_link_content("#important");
        
        assert_eq!(target, "important");
        assert_eq!(display, None);
        assert_eq!(link_type, LinkType::Tag);
    }

    #[test]
    fn test_parse_wiki_link_content_with_display() {
        let parser = create_test_parser();
        let (target, display, link_type) = parser.parse_wiki_link_content("target-note|Custom Display");
        
        assert_eq!(target, "target-note");
        assert_eq!(display, Some("Custom Display".to_string()));
        assert_eq!(link_type, LinkType::Wiki);
    }

    #[test]
    fn test_extract_all_links() {
        let parser = create_test_parser();
        let content = "Wiki [[Note]] and markdown [Google](https://google.com) links.";

        let links = parser.extract_all_links(content);
        assert_eq!(links.len(), 2);
        
        // Wiki link
        assert_eq!(links[0].link_text, "Note");
        assert_eq!(links[0].display_text, None);
        
        // Markdown link
        assert_eq!(links[1].link_text, "https://google.com");
        assert_eq!(links[1].display_text, Some("Google".to_string()));
    }

    #[test]
    fn test_validate_wiki_link_target_valid() {
        let parser = create_test_parser();
        
        let result = parser.validate_wiki_link_target("Valid Note Name");
        assert!(result.is_ok());
        
        let result = parser.validate_wiki_link_target("note-with-hyphens");
        assert!(result.is_ok());
        
        let result = parser.validate_wiki_link_target("note_with_underscores");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_wiki_link_target_invalid() {
        let parser = create_test_parser();
        
        // Empty target
        let result = parser.validate_wiki_link_target("");
        assert!(result.is_err());
        
        // Too long
        let result = parser.validate_wiki_link_target(&"a".repeat(201));
        assert!(result.is_err());
        
        // Invalid characters
        let result = parser.validate_wiki_link_target("note<with>invalid:chars");
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_wiki_link_paths() {
        let parser = create_test_parser();
        let base_dir = std::path::Path::new("/notes");
        
        let paths = parser.resolve_wiki_link_paths("My Note", base_dir);
        
        assert!(paths.contains(&base_dir.join("My Note.md")));
        assert!(paths.contains(&base_dir.join("my note.md")));
        assert!(paths.contains(&base_dir.join("My-Note.md")));
        assert!(paths.contains(&base_dir.join("My_Note.md")));
        
        // Check slugified version exists
        let slugified_exists = paths.iter().any(|p| p.to_string_lossy().contains("my-note.md"));
        assert!(slugified_exists);
    }

    #[test]
    fn test_update_wiki_links_in_content() {
        let parser = create_test_parser();
        let content = "Link to [[Old Name]] and [[Old Name|Custom Display]].";
        
        let updated = parser.update_wiki_links_in_content(content, "Old Name", "New Name");
        
        assert!(updated.contains("[[New Name]]"));
        assert!(updated.contains("[[New Name|Custom Display]]"));
        assert!(!updated.contains("[[Old Name]]"));
    }

    #[test]
    fn test_parse_link_with_display_edge_cases() {
        let parser = create_test_parser();
        
        // Empty display text
        let result = parser.parse_link_with_display("target|");
        assert_eq!(result, Some(("target".to_string(), None)));
        
        // Multiple pipes (only first is used as separator)
        let result = parser.parse_link_with_display("target|display|extra");
        assert_eq!(result, Some(("target".to_string(), Some("display|extra".to_string()))));
        
        // No pipe
        let result = parser.parse_link_with_display("just-target");
        assert_eq!(result, Some(("just-target".to_string(), None)));
    }

    #[test]
    fn test_complex_wiki_link_parsing() {
        let parser = create_test_parser();
        let content = r#"
# Wiki Link Examples

Standard: [[Note One]]
With display: [[note-two|Note Two]]
Embed: ![[image.png]]
Tag link: [[#important]]
Block ref: [[note#^block123]]
Multiple on line: [[First]] and [[Second|Display]]
"#;

        let links = parser.extract_wiki_links(content);
        assert_eq!(links.len(), 7);
        
        // Check specific link types
        let embed_links: Vec<_> = links.iter().filter(|l| l.link_type == LinkType::Embed).collect();
        assert_eq!(embed_links.len(), 1);
        assert_eq!(embed_links[0].link_text, "image.png");
        
        let tag_links: Vec<_> = links.iter().filter(|l| l.link_type == LinkType::Tag).collect();
        assert_eq!(tag_links.len(), 1);
        assert_eq!(tag_links[0].link_text, "important");
        
        let block_links: Vec<_> = links.iter().filter(|l| l.link_type == LinkType::Block).collect();
        assert_eq!(block_links.len(), 1);
        assert_eq!(block_links[0].link_text, "note#^block123");
    }

    #[test]
    fn test_sanitize_content_basic() {
        let parser = create_test_parser();
        let content = "Line 1\r\nLine 2\rLine 3\nLine 4";

        let sanitized = parser.sanitize_content(content);
        assert_eq!(sanitized, "Line 1\nLine 2\nLine 3\nLine 4\n");
    }

    #[test]
    fn test_sanitize_content_html_escaping() {
        let parser = create_test_parser();
        let content = r#"# Safe Content

<script>alert('xss')</script>
<iframe src="evil.com"></iframe>
<object data="malware.swf"></object>
<embed src="virus.exe"></embed>
<form action="steal-data.php"></form>

Normal **markdown** content.
"#;

        let sanitized = parser.sanitize_content(content);
        
        // Check that dangerous HTML tags are escaped
        assert!(sanitized.contains("&lt;script"));
        assert!(sanitized.contains("&lt;iframe"));
        assert!(sanitized.contains("&lt;object"));
        assert!(sanitized.contains("&lt;embed"));
        assert!(sanitized.contains("&lt;form"));
        
        // Check that markdown is preserved
        assert!(sanitized.contains("**markdown**"));
        assert!(sanitized.contains("# Safe Content"));
    }

    #[test]
    fn test_sanitize_content_whitespace_normalization() {
        let parser = create_test_parser();
        let content = r#"# Title


Line 1




Line 2   
Line 3	


Line 4"#;

        let sanitized = parser.sanitize_content(content);
        
        // Should limit consecutive empty lines to 2
        let lines: Vec<&str> = sanitized.lines().collect();
        let mut consecutive_empty = 0;
        let mut max_consecutive = 0;
        
        for line in &lines {
            if line.trim().is_empty() {
                consecutive_empty += 1;
                max_consecutive = max_consecutive.max(consecutive_empty);
            } else {
                consecutive_empty = 0;
            }
        }
        
        assert!(max_consecutive <= 2, "Too many consecutive empty lines: {}", max_consecutive);
        
        // Should trim trailing whitespace
        for line in &lines {
            if !line.is_empty() {
                assert_eq!(*line, line.trim_end(), "Line has trailing whitespace: '{}'", line);
            }
        }
    }

    #[test]
    fn test_validate_content_valid() {
        let parser = create_test_parser();
        let content = r#"# Main Title

## Section 1

Some content with **bold** and *italic* text.

### Subsection

- List item 1
- List item 2
  - Nested item
  - Another nested item

## Section 2

```rust
let code = "example";
```

Some more content.
"#;

        let result = parser.validate_content(content);
        assert!(result.is_ok(), "Valid content should pass validation: {:?}", result);
    }

    #[test]
    fn test_validate_content_too_large() {
        let parser = create_test_parser();
        let content = "a".repeat(1_000_001); // Exceeds 1MB limit

        let result = parser.validate_content(&content);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Content too large"));
    }

    #[test]
    fn test_validate_content_invalid_heading_progression() {
        let parser = create_test_parser();
        let content = r#"# Main Title

### Skipped H2

This should be invalid because we jumped from H1 to H3.
"#;

        let result = parser.validate_content(content);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid heading progression"));
    }

    #[test]
    fn test_validate_content_deep_list_nesting() {
        let parser = create_test_parser();
        let mut content = String::from("# Title\n\n");
        
        // Create deeply nested list (11 levels, should exceed limit of 10)
        for i in 0..11 {
            content.push_str(&"  ".repeat(i));
            content.push_str("- Item at level ");
            content.push_str(&i.to_string());
            content.push('\n');
        }

        let result = parser.validate_content(&content);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("List nesting too deep"));
    }

    #[test]
    fn test_validate_and_sanitize() {
        let parser = create_test_parser();
        let content = r#"# Good Content

Some text with <script>alert('test')</script> potential danger.



Excessive whitespace above should be normalized.
"#;

        let result = parser.validate_and_sanitize(content);
        assert!(result.is_ok());
        
        let sanitized = result.unwrap();
        assert!(sanitized.contains("&lt;script"));
        
        // Check whitespace normalization
        let empty_line_count = sanitized.matches("\n\n").count();
        assert!(empty_line_count <= 2, "Too many consecutive empty lines after sanitization");
    }

    #[test]
    fn test_validate_content_valid_heading_progression() {
        let parser = create_test_parser();
        let content = r#"# Main Title

## Section 1

### Subsection A

#### Details

### Subsection B

## Section 2

# Another Main Title
"#;

        let result = parser.validate_content(content);
        assert!(result.is_ok(), "Valid heading progression should pass: {:?}", result);
    }

    #[test]
    fn test_extract_metadata() {
        let parser = create_test_parser();
        let content = r#"
# Main Heading

## Sub Heading

Some text with [external link](http://example.com).

```rust
let code = "example";
```

### Another Heading
"#;

        let metadata = parser.extract_metadata(content);
        assert_eq!(metadata.get("headings"), Some(&"3".to_string()));
        assert_eq!(metadata.get("links"), Some(&"1".to_string()));
        assert_eq!(metadata.get("code_blocks"), Some(&"1".to_string()));
    }

    #[test]
    fn test_parse_note_complete() {
        let parser = create_test_parser();
        let content = r#"---
title: "Complete Test Note"
tags: ["test", "complete"]
---

# Test Note

This is a test note with [[linked note]] and #hashtag.

## Section

More content here.
"#;

        let note = parser.parse_note(
            "test-note".to_string(),
            PathBuf::from("/test/note.md"),
            content
        ).unwrap();

        assert_eq!(note.id, "test-note");
        assert_eq!(note.title, "Complete Test Note");
        assert_eq!(note.links.len(), 1);
        assert_eq!(note.links[0].link_text, "linked note");
        assert_eq!(note.tags.len(), 3); // "complete", "hashtag", "test" (sorted)
        assert!(note.tags.contains(&"test".to_string()));
        assert!(note.tags.contains(&"complete".to_string()));
        assert!(note.tags.contains(&"hashtag".to_string()));
        assert!(note.word_count > 0);
        assert!(!note.content_hash.is_empty());
    }

    #[test]
    fn test_parse_note_minimal() {
        let parser = create_test_parser();
        let content = "Just some plain content.";

        let note = parser.parse_note(
            "minimal-note".to_string(),
            PathBuf::from("/test/minimal.md"),
            content
        ).unwrap();

        assert_eq!(note.id, "minimal-note");
        assert!(!note.title.is_empty());
        assert_eq!(note.links.len(), 0);
        assert_eq!(note.tags.len(), 0);
        assert_eq!(note.frontmatter, None);
        assert_eq!(note.word_count, 4);
    }
}