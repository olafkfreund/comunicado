use crate::imap::{
    ImapError, ImapResult, ImapCapability, ImapFolder, ImapMessage, 
    MessageFlag, FolderAttribute, SearchCriteria
};

/// IMAP protocol parser and formatter
pub struct ImapProtocol;

impl ImapProtocol {
    /// Parse CAPABILITY response
    pub fn parse_capabilities(response: &str) -> ImapResult<Vec<ImapCapability>> {
        let mut capabilities = Vec::new();
        
        for line in response.lines() {
            if line.starts_with("* CAPABILITY ") {
                let caps_str = &line[13..]; // Skip "* CAPABILITY "
                for cap in caps_str.split_whitespace() {
                    capabilities.push(ImapCapability::from_str(cap));
                }
            }
        }
        
        Ok(capabilities)
    }
    
    /// Parse LIST/LSUB response into folders
    pub fn parse_folders(response: &str) -> ImapResult<Vec<ImapFolder>> {
        let mut folders = Vec::new();
        
        for line in response.lines() {
            if line.starts_with("* LIST ") || line.starts_with("* LSUB ") {
                if let Some(folder) = Self::parse_folder_line(line)? {
                    folders.push(folder);
                }
            }
        }
        
        Ok(folders)
    }
    
    /// Parse a single LIST/LSUB response line
    fn parse_folder_line(line: &str) -> ImapResult<Option<ImapFolder>> {
        // Example: * LIST (\HasNoChildren) "/" "INBOX"
        // Example: * LIST (\HasChildren \Noselect) "/" "[Gmail]"
        
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            return Ok(None);
        }
        
        // Parse attributes
        let attrs_str = line.split('(').nth(1)
            .and_then(|s| s.split(')').next())
            .unwrap_or("");
        
        let mut attributes = Vec::new();
        for attr in attrs_str.split_whitespace() {
            attributes.push(FolderAttribute::from_str(attr));
        }
        
        // Parse delimiter
        let delimiter_start = line.find(") ").ok_or_else(|| {
            ImapError::parse("Invalid LIST response format")
        })? + 2;
        
        let remaining = &line[delimiter_start..];
        let parts: Vec<&str> = remaining.splitn(2, ' ').collect();
        
        if parts.len() != 2 {
            return Ok(None);
        }
        
        let delimiter = if parts[0] == "NIL" {
            None
        } else {
            Some(parts[0].trim_matches('"').to_string())
        };
        
        // Parse folder name (remove quotes)
        let full_name = parts[1].trim_matches('"').to_string();
        let name = if let Some(delim) = &delimiter {
            full_name.split(delim).last().unwrap_or(&full_name).to_string()
        } else {
            full_name.clone()
        };
        
        let mut folder = ImapFolder::new(name, full_name);
        folder.delimiter = delimiter;
        folder.attributes = attributes;
        
        Ok(Some(folder))
    }
    
    /// Parse SELECT/EXAMINE response to update folder information
    pub fn parse_select_response(response: &str) -> ImapResult<ImapFolder> {
        let mut folder = ImapFolder::new("Unknown".to_string(), "Unknown".to_string());
        
        for line in response.lines() {
            if line.starts_with("* ") {
                if let Some(exists_str) = line.strip_prefix("* ").and_then(|s| s.strip_suffix(" EXISTS")) {
                    folder.exists = exists_str.parse().ok();
                } else if let Some(recent_str) = line.strip_prefix("* ").and_then(|s| s.strip_suffix(" RECENT")) {
                    folder.recent = recent_str.parse().ok();
                } else if line.contains("[UNSEEN ") {
                    // Parse [UNSEEN n] from OK response
                    if let Some(start) = line.find("[UNSEEN ") {
                        if let Some(end) = line[start..].find(']') {
                            let unseen_str = &line[start + 8..start + end];
                            folder.unseen = unseen_str.parse().ok();
                        }
                    }
                } else if line.contains("[UIDVALIDITY ") {
                    if let Some(start) = line.find("[UIDVALIDITY ") {
                        if let Some(end) = line[start..].find(']') {
                            let uid_str = &line[start + 13..start + end];
                            folder.uid_validity = uid_str.parse().ok();
                        }
                    }
                } else if line.contains("[UIDNEXT ") {
                    if let Some(start) = line.find("[UIDNEXT ") {
                        if let Some(end) = line[start..].find(']') {
                            let uid_str = &line[start + 9..start + end];
                            folder.uid_next = uid_str.parse().ok();
                        }
                    }
                }
            }
        }
        
        Ok(folder)
    }
    
    /// Parse FETCH response into messages
    pub fn parse_fetch_response(response: &str) -> ImapResult<Vec<ImapMessage>> {
        let mut messages = Vec::new();
        let mut current_message: Option<ImapMessage> = None;
        
        for line in response.lines() {
            if line.starts_with("* ") && line.contains(" FETCH ") {
                // Start of new message
                if let Some(msg) = current_message.take() {
                    messages.push(msg);
                }
                
                // Parse sequence number
                let seq_str = line[2..].split_whitespace().next().unwrap_or("0");
                let seq_num: u32 = seq_str.parse().unwrap_or(0);
                
                current_message = Some(ImapMessage::new(seq_num));
                
                // Parse FETCH data in the same line
                if let Some(msg) = &mut current_message {
                    Self::parse_fetch_data(line, msg)?;
                }
            } else if let Some(msg) = &mut current_message {
                // Continue parsing multi-line FETCH response
                Self::parse_fetch_data(line, msg)?;
            }
        }
        
        // Don't forget the last message
        if let Some(msg) = current_message {
            messages.push(msg);
        }
        
        Ok(messages)
    }
    
    /// Parse FETCH data into message
    fn parse_fetch_data(line: &str, message: &mut ImapMessage) -> ImapResult<()> {
        // Parse FLAGS
        if line.contains("FLAGS (") {
            if let Some(start) = line.find("FLAGS (") {
                if let Some(end) = line[start..].find(')') {
                    let flags_str = &line[start + 7..start + end];
                    for flag_str in flags_str.split_whitespace() {
                        message.flags.push(MessageFlag::from_str(flag_str));
                    }
                }
            }
        }
        
        // Parse UID
        if line.contains("UID ") {
            if let Some(uid_pos) = line.find("UID ") {
                let uid_start = uid_pos + 4;
                let uid_end = line[uid_start..].find(' ').map(|i| uid_start + i).unwrap_or(line.len());
                if let Ok(uid) = line[uid_start..uid_end].parse::<u32>() {
                    message.uid = Some(uid);
                }
            }
        }
        
        // Parse RFC822.SIZE
        if line.contains("RFC822.SIZE ") {
            if let Some(size_pos) = line.find("RFC822.SIZE ") {
                let size_start = size_pos + 12;
                let size_end = line[size_start..].find(' ').map(|i| size_start + i).unwrap_or(line.len());
                if let Ok(size) = line[size_start..size_end].parse::<u32>() {
                    message.size = Some(size);
                }
            }
        }
        
        // Parse INTERNALDATE
        if line.contains("INTERNALDATE \"") {
            if let Some(start) = line.find("INTERNALDATE \"") {
                if let Some(end) = line[start + 14..].find('"') {
                    let date_str = &line[start + 14..start + 14 + end];
                    // TODO: Parse IMAP date format properly
                    // For now, just store as string in envelope if exists
                }
            }
        }
        
        // TODO: Parse ENVELOPE, BODYSTRUCTURE, and other FETCH items
        // This is a simplified implementation
        
        Ok(())
    }
    
    /// Parse SEARCH response
    pub fn parse_search_response(response: &str) -> ImapResult<Vec<u32>> {
        let mut message_ids = Vec::new();
        
        for line in response.lines() {
            if line.starts_with("* SEARCH ") {
                let ids_str = &line[9..]; // Skip "* SEARCH "
                for id_str in ids_str.split_whitespace() {
                    if let Ok(id) = id_str.parse::<u32>() {
                        message_ids.push(id);
                    }
                }
            }
        }
        
        Ok(message_ids)
    }
    
    /// Format LOGIN command
    pub fn format_login(username: &str, password: &str) -> String {
        format!("LOGIN \"{}\" \"{}\"", username, password)
    }
    
    /// Format AUTHENTICATE PLAIN command
    pub fn format_authenticate_plain(username: &str, password: &str) -> ImapResult<String> {
        // PLAIN SASL mechanism: \0username\0password
        let auth_string = format!("\0{}\0{}", username, password);
        use base64::{Engine as _, engine::general_purpose};
        let encoded = general_purpose::STANDARD.encode(auth_string);
        Ok(format!("AUTHENTICATE PLAIN {}", encoded))
    }
    
    /// Format AUTHENTICATE XOAUTH2 command
    pub fn format_authenticate_xoauth2(xoauth2_string: &str) -> String {
        format!("AUTHENTICATE XOAUTH2 {}", xoauth2_string)
    }
    
    /// Format SELECT command
    pub fn format_select(folder: &str) -> String {
        format!("SELECT \"{}\"", folder)
    }
    
    /// Format EXAMINE command
    pub fn format_examine(folder: &str) -> String {
        format!("EXAMINE \"{}\"", folder)
    }
    
    /// Format LIST command
    pub fn format_list(reference: &str, pattern: &str) -> String {
        format!("LIST \"{}\" \"{}\"", reference, pattern)
    }
    
    /// Format LSUB command
    pub fn format_lsub(reference: &str, pattern: &str) -> String {
        format!("LSUB \"{}\" \"{}\"", reference, pattern)
    }
    
    /// Format FETCH command
    pub fn format_fetch(sequence_set: &str, items: &[&str]) -> String {
        format!("FETCH {} ({})", sequence_set, items.join(" "))
    }
    
    /// Format UID FETCH command
    pub fn format_uid_fetch(uid_set: &str, items: &[&str]) -> String {
        format!("UID FETCH {} ({})", uid_set, items.join(" "))
    }
    
    /// Format SEARCH command
    pub fn format_search(criteria: &SearchCriteria) -> String {
        format!("SEARCH {}", criteria.to_imap_string())
    }
    
    /// Format UID SEARCH command
    pub fn format_uid_search(criteria: &SearchCriteria) -> String {
        format!("UID SEARCH {}", criteria.to_imap_string())
    }
    
    /// Format STORE command
    pub fn format_store(sequence_set: &str, flags: &[MessageFlag], action: &str) -> String {
        let flags_str: Vec<String> = flags.iter().map(|f| f.to_string()).collect();
        format!("STORE {} {} ({})", sequence_set, action, flags_str.join(" "))
    }
    
    /// Format UID STORE command
    pub fn format_uid_store(uid_set: &str, flags: &[MessageFlag], action: &str) -> String {
        let flags_str: Vec<String> = flags.iter().map(|f| f.to_string()).collect();
        format!("UID STORE {} {} ({})", uid_set, action, flags_str.join(" "))
    }
    
    /// Format COPY command
    pub fn format_copy(sequence_set: &str, destination: &str) -> String {
        format!("COPY {} \"{}\"", sequence_set, destination)
    }
    
    /// Format UID COPY command
    pub fn format_uid_copy(uid_set: &str, destination: &str) -> String {
        format!("UID COPY {} \"{}\"", uid_set, destination)
    }
    
    /// Format EXPUNGE command
    pub fn format_expunge() -> String {
        "EXPUNGE".to_string()
    }
    
    /// Format CREATE command
    pub fn format_create(folder: &str) -> String {
        format!("CREATE \"{}\"", folder)
    }
    
    /// Format DELETE command
    pub fn format_delete(folder: &str) -> String {
        format!("DELETE \"{}\"", folder)
    }
    
    /// Format RENAME command
    pub fn format_rename(old_name: &str, new_name: &str) -> String {
        format!("RENAME \"{}\" \"{}\"", old_name, new_name)
    }
    
    /// Format SUBSCRIBE command
    pub fn format_subscribe(folder: &str) -> String {
        format!("SUBSCRIBE \"{}\"", folder)
    }
    
    /// Format UNSUBSCRIBE command
    pub fn format_unsubscribe(folder: &str) -> String {
        format!("UNSUBSCRIBE \"{}\"", folder)
    }
    
    /// Format STATUS command
    pub fn format_status(folder: &str, items: &[&str]) -> String {
        format!("STATUS \"{}\" ({})", folder, items.join(" "))
    }
    
    /// Format IDLE command
    pub fn format_idle() -> String {
        "IDLE".to_string()
    }
    
    /// Format DONE command (to exit IDLE)
    pub fn format_done() -> String {
        "DONE".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_capabilities() {
        let response = "* CAPABILITY IMAP4rev1 STARTTLS AUTH=PLAIN AUTH=LOGIN\nA001 OK CAPABILITY completed\n";
        let capabilities = ImapProtocol::parse_capabilities(response).unwrap();
        
        assert!(capabilities.contains(&ImapCapability::Imap4Rev1));
        assert!(capabilities.contains(&ImapCapability::StartTls));
        assert!(capabilities.contains(&ImapCapability::AuthPlain));
        assert!(capabilities.contains(&ImapCapability::AuthLogin));
    }
    
    #[test]
    fn test_parse_folder_line() {
        let line = "* LIST (\\HasNoChildren) \"/\" \"INBOX\"";
        let folder = ImapProtocol::parse_folder_line(line).unwrap().unwrap();
        
        assert_eq!(folder.name, "INBOX");
        assert_eq!(folder.full_name, "INBOX");
        assert_eq!(folder.delimiter, Some("/".to_string()));
        assert!(folder.attributes.contains(&FolderAttribute::HasNoChildren));
    }
    
    #[test]
    fn test_format_commands() {
        assert_eq!(
            ImapProtocol::format_login("user", "pass"),
            "LOGIN \"user\" \"pass\""
        );
        
        assert_eq!(
            ImapProtocol::format_select("INBOX"),
            "SELECT \"INBOX\""
        );
        
        assert_eq!(
            ImapProtocol::format_list("", "*"),
            "LIST \"\" \"*\""
        );
        
        assert_eq!(
            ImapProtocol::format_fetch("1:10", &["FLAGS", "UID"]),
            "FETCH 1:10 (FLAGS UID)"
        );
    }
    
    #[test]
    fn test_search_criteria() {
        let criteria = SearchCriteria::From("test@example.com".to_string());
        assert_eq!(criteria.to_imap_string(), "FROM \"test@example.com\"");
        
        let criteria = SearchCriteria::Or(
            Box::new(SearchCriteria::Unseen),
            Box::new(SearchCriteria::Flagged)
        );
        assert_eq!(criteria.to_imap_string(), "OR UNSEEN FLAGGED");
    }
}