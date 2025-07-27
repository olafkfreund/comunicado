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
        let mut expecting_literal_content = false;
        
        let lines: Vec<&str> = response.lines().collect();
        let mut i = 0;
        
        while i < lines.len() {
            let line = lines[i];
            
            if line.starts_with("* ") && line.contains(" FETCH ") {
                // Start of new message
                if let Some(msg) = current_message.take() {
                    messages.push(msg);
                }
                
                // Parse sequence number
                let seq_str = line[2..].split_whitespace().next().unwrap_or("0");
                let seq_num: u32 = seq_str.parse().unwrap_or(0);
                
                current_message = Some(ImapMessage::new(seq_num));
                expecting_literal_content = false;
                
                // Parse FETCH data in the same line
                if let Some(msg) = &mut current_message {
                    Self::parse_fetch_data(line, msg)?;
                    
                    // Check if this line indicates a literal follows
                    if line.contains("BODY[] {") && Self::extract_literal_size_from_line(line).is_some() {
                        expecting_literal_content = true;
                    }
                }
            } else if expecting_literal_content && current_message.is_some() {
                // This line should be the literal content
                if let Some(msg) = &mut current_message {
                    msg.body = Some(line.to_string());
                    tracing::debug!("Set message body from literal, length: {} chars", line.len());
                }
                expecting_literal_content = false;
            } else if let Some(msg) = &mut current_message {
                // Continue parsing multi-line FETCH response
                Self::parse_fetch_data(line, msg)?;
                
                // Check if this line indicates a literal follows
                if line.contains("BODY[] {") && Self::extract_literal_size_from_line(line).is_some() {
                    expecting_literal_content = true;
                }
            }
            
            i += 1;
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
                    // Parse IMAP date format: "17-Jul-1996 02:44:25 -0700"
                    // For now, store as string - can be parsed to DateTime later
                    if message.envelope.is_none() {
                        message.envelope = Some(crate::imap::MessageEnvelope::new());
                    }
                    if let Some(ref mut envelope) = message.envelope {
                        envelope.date = Some(date_str.to_string());
                    }
                }
            }
        }
        
        // Parse ENVELOPE
        if line.contains("ENVELOPE ") {
            tracing::debug!("Found ENVELOPE in line: {}", line);
            if let Some(start) = line.find("ENVELOPE ") {
                let envelope_start = start + 9; // Skip "ENVELOPE "
                if let Some(envelope) = Self::parse_envelope(&line[envelope_start..])? {
                    tracing::debug!("Successfully parsed envelope: subject={:?}, from_count={}", 
                                   envelope.subject, envelope.from.len());
                    message.envelope = Some(envelope);
                } else {
                    tracing::warn!("Failed to parse envelope from line: {}", line);
                }
            }
        }
        
        // Parse BODY[] (message body content)
        if line.contains("BODY[] ") {
            tracing::debug!("Found BODY[] in line: {}", line);
            if let Some(start) = line.find("BODY[] ") {
                let body_start = start + 7; // Skip "BODY[] "
                // Look for literal size indicator {size}
                if let Some(brace_start) = line[body_start..].find('{') {
                    if let Some(brace_end) = line[body_start + brace_start..].find('}') {
                        let size_str = &line[body_start + brace_start + 1..body_start + brace_start + brace_end];
                        if let Ok(_size) = size_str.parse::<u32>() {
                            tracing::debug!("Found body size: {}", _size);
                            // The body content is expected to follow in subsequent lines
                            // For now, we'll mark that we found a body indicator
                            // The actual body content parsing will need to be handled in the calling function
                            // since it spans multiple lines
                        }
                    }
                } else {
                    // Handle case where body content is on the same line (for small messages)
                    let body_content = &line[body_start..];
                    if !body_content.trim().is_empty() {
                        message.body = Some(body_content.to_string());
                        tracing::debug!("Parsed inline body content, length: {}", body_content.len());
                    }
                }
            }
        }
        
        // TODO: Parse BODYSTRUCTURE and other FETCH items
        
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
    
    /// Format AUTHENTICATE XOAUTH2 command (first step)
    pub fn format_authenticate_xoauth2_start() -> String {
        "AUTHENTICATE XOAUTH2".to_string()
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
    
    /// Extract literal size from a line containing {size}
    fn extract_literal_size_from_line(line: &str) -> Option<usize> {
        // Look for {size} pattern
        if let Some(start) = line.rfind('{') {
            if let Some(end) = line[start..].find('}') {
                let size_str = &line[start + 1..start + end];
                if let Ok(size) = size_str.parse::<usize>() {
                    return Some(size);
                }
            }
        }
        None
    }
    
    /// Parse ENVELOPE from FETCH response
    /// ENVELOPE format: (date subject from sender reply-to to cc bcc in-reply-to message-id)
    /// Each address field is a list: ((name route mailbox host)...)
    fn parse_envelope(data: &str) -> ImapResult<Option<crate::imap::MessageEnvelope>> {
        tracing::debug!("Parsing envelope from data: {}", data);
        
        // Find the opening parenthesis for the envelope
        let data = data.trim();
        if !data.starts_with('(') {
            tracing::warn!("Envelope data doesn't start with '(': {}", data);
            return Ok(None);
        }
        
        // Find the matching closing parenthesis
        let envelope_end = Self::find_matching_paren(data, 0)?;
        let envelope_content = &data[1..envelope_end]; // Remove outer parentheses
        
        tracing::debug!("Envelope content: {}", envelope_content);
        
        // Parse the 10 fields of the envelope
        let fields = Self::parse_envelope_fields(envelope_content)?;
        if fields.len() != 10 {
            tracing::warn!("Expected 10 envelope fields, got {}", fields.len());
            return Ok(None);
        }
        
        let mut envelope = crate::imap::MessageEnvelope::new();
        
        // Field 0: date
        envelope.date = Self::parse_string_or_nil(&fields[0]);
        
        // Field 1: subject  
        envelope.subject = Self::parse_string_or_nil(&fields[1]);
        
        // Field 2: from
        envelope.from = Self::parse_address_list(&fields[2])?;
        
        // Field 3: sender
        envelope.sender = Self::parse_address_list(&fields[3])?;
        
        // Field 4: reply-to
        envelope.reply_to = Self::parse_address_list(&fields[4])?;
        
        // Field 5: to
        envelope.to = Self::parse_address_list(&fields[5])?;
        
        // Field 6: cc
        envelope.cc = Self::parse_address_list(&fields[6])?;
        
        // Field 7: bcc
        envelope.bcc = Self::parse_address_list(&fields[7])?;
        
        // Field 8: in-reply-to
        envelope.in_reply_to = Self::parse_string_or_nil(&fields[8]);
        
        // Field 9: message-id
        envelope.message_id = Self::parse_string_or_nil(&fields[9]);
        
        tracing::debug!("Parsed envelope: subject={:?}, from_count={}", envelope.subject, envelope.from.len());
        Ok(Some(envelope))
    }
    
    /// Find matching closing parenthesis
    fn find_matching_paren(data: &str, start: usize) -> ImapResult<usize> {
        let chars: Vec<char> = data.chars().collect();
        let mut depth = 0;
        let mut in_quote = false;
        let mut escape_next = false;
        
        for i in start..chars.len() {
            let ch = chars[i];
            
            if escape_next {
                escape_next = false;
                continue;
            }
            
            if ch == '\\' {
                escape_next = true;
                continue;
            }
            
            if ch == '"' {
                in_quote = !in_quote;
                continue;
            }
            
            if !in_quote {
                if ch == '(' {
                    depth += 1;
                } else if ch == ')' {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(i);
                    }
                }
            }
        }
        
        Err(ImapError::parse("Unmatched parentheses in envelope"))
    }
    
    /// Parse envelope fields (split by spaces, respecting parentheses and quotes)
    fn parse_envelope_fields(content: &str) -> ImapResult<Vec<String>> {
        let mut fields = Vec::new();
        let mut current_field = String::new();
        let chars: Vec<char> = content.chars().collect();
        let mut i = 0;
        
        while i < chars.len() {
            let ch = chars[i];
            
            if ch.is_whitespace() && current_field.is_empty() {
                // Skip leading whitespace
                i += 1;
                continue;
            }
            
            if ch == '(' {
                // Parse parenthesized content
                let start = i;
                let end = Self::find_matching_paren(content, start)?;
                current_field.push_str(&content[start..=end]);
                i = end + 1;
            } else if ch == '"' {
                // Parse quoted string
                let start = i;
                i += 1; // Skip opening quote
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' {
                        i += 1; // Skip escape character
                    }
                    i += 1;
                }
                if i < chars.len() {
                    i += 1; // Skip closing quote
                }
                current_field.push_str(&content[start..i]);
            } else if ch.is_whitespace() {
                // End of field
                if !current_field.is_empty() {
                    fields.push(current_field.trim().to_string());
                    current_field.clear();
                }
                i += 1;
            } else if ch.is_alphabetic() && current_field.is_empty() {
                // Handle NIL
                let start = i;
                while i < chars.len() && chars[i].is_alphabetic() {
                    i += 1;
                }
                current_field.push_str(&content[start..i]);
            } else {
                current_field.push(ch);
                i += 1;
            }
        }
        
        // Don't forget the last field
        if !current_field.is_empty() {
            fields.push(current_field.trim().to_string());
        }
        
        Ok(fields)
    }
    
    /// Parse string or NIL value
    fn parse_string_or_nil(field: &str) -> Option<String> {
        let field = field.trim();
        if field.eq_ignore_ascii_case("NIL") {
            None
        } else if field.starts_with('"') && field.ends_with('"') {
            Some(field[1..field.len()-1].to_string())
        } else {
            Some(field.to_string())
        }
    }
    
    /// Parse address list from IMAP format
    /// Format: ((name route mailbox host)(name route mailbox host)...) or NIL
    fn parse_address_list(field: &str) -> ImapResult<Vec<crate::imap::Address>> {
        let field = field.trim();
        if field.eq_ignore_ascii_case("NIL") {
            return Ok(Vec::new());
        }
        
        if !field.starts_with('(') || !field.ends_with(')') {
            return Ok(Vec::new());
        }
        
        let content = &field[1..field.len()-1]; // Remove outer parentheses
        let mut addresses = Vec::new();
        let mut i = 0;
        let chars: Vec<char> = content.chars().collect();
        
        while i < chars.len() {
            // Skip whitespace
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            
            if i >= chars.len() {
                break;
            }
            
            if chars[i] == '(' {
                // Parse single address
                let start = i;
                let end = Self::find_matching_paren(content, start)?;
                let address_content = &content[start+1..end]; // Remove parentheses
                
                if let Some(address) = Self::parse_single_address(address_content)? {
                    addresses.push(address);
                }
                
                i = end + 1;
            } else {
                i += 1;
            }
        }
        
        Ok(addresses)
    }
    
    /// Parse a single address from IMAP format
    /// Format: (name route mailbox host)
    fn parse_single_address(content: &str) -> ImapResult<Option<crate::imap::Address>> {
        let fields = Self::parse_envelope_fields(content)?;
        if fields.len() != 4 {
            tracing::warn!("Expected 4 address fields, got {}: {:?}", fields.len(), fields);
            return Ok(None);
        }
        
        let name = Self::parse_string_or_nil(&fields[0]);
        let route = Self::parse_string_or_nil(&fields[1]);
        let mailbox = Self::parse_string_or_nil(&fields[2]);
        let host = Self::parse_string_or_nil(&fields[3]);
        
        // Mailbox and host are required for a valid address
        if let (Some(mailbox), Some(host)) = (mailbox, host) {
            let mut address = crate::imap::Address::new(mailbox, host);
            address.name = name;
            address.route = route;
            Ok(Some(address))
        } else {
            Ok(None)
        }
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