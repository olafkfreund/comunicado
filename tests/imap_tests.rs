use comunicado::imap::protocol::ImapProtocol;
use comunicado::imap::{
    FolderAttribute, ImapCapability, ImapClient, ImapConfig, ImapError, ImapFolder, ImapMessage,
    MessageFlag, SearchCriteria,
};

#[test]
fn test_imap_config_creation() {
    let config = ImapConfig::new(
        "imap.example.com".to_string(),
        993,
        "user@example.com".to_string(),
        "password".to_string(),
    );

    assert_eq!(config.hostname, "imap.example.com");
    assert_eq!(config.port, 993);
    assert_eq!(config.username, "user@example.com");
    if let ImapAuthMethod::Password(password) = &config.auth_method {
        assert_eq!(password, "password");
    } else {
        panic!("Expected password auth method");
    }
    assert!(config.use_tls); // Should default to true for port 993
    assert_eq!(config.timeout_seconds, 30);
}

#[test]
fn test_predefined_configs() {
    let gmail = ImapConfig::gmail("user@gmail.com".to_string(), "password".to_string());
    assert_eq!(gmail.hostname, "imap.gmail.com");
    assert_eq!(gmail.port, 993);
    assert!(gmail.use_tls);

    let outlook = ImapConfig::outlook("user@outlook.com".to_string(), "password".to_string());
    assert_eq!(outlook.hostname, "outlook.office365.com");
    assert_eq!(outlook.port, 993);
    assert!(outlook.use_tls);

    let yahoo = ImapConfig::yahoo("user@yahoo.com".to_string(), "password".to_string());
    assert_eq!(yahoo.hostname, "imap.mail.yahoo.com");
    assert_eq!(yahoo.port, 993);
    assert!(yahoo.use_tls);
}

#[test]
fn test_config_builders() {
    let config = ImapConfig::new(
        "imap.example.com".to_string(),
        143,
        "user".to_string(),
        "pass".to_string(),
    )
    .with_tls(false)
    .with_starttls(true)
    .with_timeout(60)
    .with_certificate_validation(false);

    assert!(!config.use_tls);
    assert!(config.use_starttls);
    assert_eq!(config.timeout_seconds, 60);
    assert!(!config.validate_certificates);
}

#[test]
fn test_imap_capability_parsing() {
    assert_eq!(
        ImapCapability::from_str("IMAP4REV1"),
        ImapCapability::Imap4Rev1
    );
    assert_eq!(
        ImapCapability::from_str("STARTTLS"),
        ImapCapability::StartTls
    );
    assert_eq!(
        ImapCapability::from_str("AUTH=PLAIN"),
        ImapCapability::AuthPlain
    );
    assert_eq!(ImapCapability::from_str("IDLE"), ImapCapability::Idle);

    match ImapCapability::from_str("CUSTOM-CAP") {
        ImapCapability::Custom(s) => assert_eq!(s, "CUSTOM-CAP"),
        _ => panic!("Expected custom capability"),
    }
}

#[test]
fn test_folder_attribute_parsing() {
    assert_eq!(
        FolderAttribute::from_str("\\HasChildren"),
        FolderAttribute::HasChildren
    );
    assert_eq!(
        FolderAttribute::from_str("\\Noselect"),
        FolderAttribute::Noselect
    );
    assert_eq!(FolderAttribute::from_str("\\Sent"), FolderAttribute::Sent);
    assert_eq!(
        FolderAttribute::from_str("\\Drafts"),
        FolderAttribute::Drafts
    );

    match FolderAttribute::from_str("\\CustomAttr") {
        FolderAttribute::Custom(s) => assert_eq!(s, "\\CustomAttr"),
        _ => panic!("Expected custom attribute"),
    }
}

#[test]
fn test_message_flag_parsing() {
    assert_eq!(MessageFlag::from_str("\\Seen"), MessageFlag::Seen);
    assert_eq!(MessageFlag::from_str("\\Flagged"), MessageFlag::Flagged);
    assert_eq!(MessageFlag::from_str("\\Deleted"), MessageFlag::Deleted);
    assert_eq!(MessageFlag::from_str("\\Draft"), MessageFlag::Draft);
    assert_eq!(MessageFlag::from_str("\\Recent"), MessageFlag::Recent);

    match MessageFlag::from_str("CustomFlag") {
        MessageFlag::Custom(s) => assert_eq!(s, "CustomFlag"),
        _ => panic!("Expected custom flag"),
    }
}

#[test]
fn test_message_flag_to_string() {
    assert_eq!(MessageFlag::Seen.to_string(), "\\Seen");
    assert_eq!(MessageFlag::Flagged.to_string(), "\\Flagged");
    assert_eq!(MessageFlag::Deleted.to_string(), "\\Deleted");
    assert_eq!(
        MessageFlag::Custom("MyFlag".to_string()).to_string(),
        "MyFlag"
    );
}

#[test]
fn test_imap_folder_creation() {
    let folder = ImapFolder::new("INBOX".to_string(), "INBOX".to_string());
    assert_eq!(folder.name, "INBOX");
    assert_eq!(folder.full_name, "INBOX");
    assert!(folder.is_selectable());
    assert!(!folder.has_children());
    assert!(folder.is_inbox());
}

#[test]
fn test_imap_folder_with_attributes() {
    let mut folder = ImapFolder::new("Archive".to_string(), "INBOX/Archive".to_string());
    folder.attributes = vec![FolderAttribute::HasChildren, FolderAttribute::Archive];
    folder.unseen = Some(5);
    folder.exists = Some(100);

    assert!(folder.has_children());
    assert!(folder.is_selectable());
    assert!(!folder.is_inbox());
    assert_eq!(folder.unseen, Some(5));
    assert_eq!(folder.exists, Some(100));
}

#[test]
fn test_imap_folder_noselect() {
    let mut folder = ImapFolder::new("Parent".to_string(), "Parent".to_string());
    folder.attributes = vec![FolderAttribute::Noselect, FolderAttribute::HasChildren];

    assert!(!folder.is_selectable());
    assert!(folder.has_children());
}

#[test]
fn test_imap_message_creation() {
    let message = ImapMessage::new(1);
    assert_eq!(message.sequence_number, 1);
    assert!(message.uid.is_none());
    assert!(message.flags.is_empty());
    assert!(!message.is_seen());
    assert!(!message.is_flagged());
    assert!(!message.is_deleted());
}

#[test]
fn test_imap_message_flags() {
    let mut message = ImapMessage::new(1);
    message.flags = vec![
        MessageFlag::Seen,
        MessageFlag::Flagged,
        MessageFlag::Custom("Important".to_string()),
    ];

    assert!(message.is_seen());
    assert!(message.is_flagged());
    assert!(!message.is_deleted());
    assert!(!message.is_draft());
    assert!(!message.is_recent());
}

#[test]
fn test_search_criteria_formatting() {
    let criteria = SearchCriteria::All;
    assert_eq!(criteria.to_imap_string(), "ALL");

    let criteria = SearchCriteria::From("test@example.com".to_string());
    assert_eq!(criteria.to_imap_string(), "FROM \"test@example.com\"");

    let criteria = SearchCriteria::Subject("Test Subject".to_string());
    assert_eq!(criteria.to_imap_string(), "SUBJECT \"Test Subject\"");

    let criteria = SearchCriteria::Unseen;
    assert_eq!(criteria.to_imap_string(), "UNSEEN");

    let criteria = SearchCriteria::Not(Box::new(SearchCriteria::Deleted));
    assert_eq!(criteria.to_imap_string(), "NOT DELETED");

    let criteria = SearchCriteria::Or(
        Box::new(SearchCriteria::Flagged),
        Box::new(SearchCriteria::Recent),
    );
    assert_eq!(criteria.to_imap_string(), "OR FLAGGED RECENT");
}

#[test]
fn test_protocol_capability_parsing() {
    let response = "* CAPABILITY IMAP4rev1 STARTTLS AUTH=PLAIN AUTH=LOGIN IDLE NAMESPACE\nA001 OK CAPABILITY completed\n";
    let capabilities = ImapProtocol::parse_capabilities(response).unwrap();

    assert!(capabilities.contains(&ImapCapability::Imap4Rev1));
    assert!(capabilities.contains(&ImapCapability::StartTls));
    assert!(capabilities.contains(&ImapCapability::AuthPlain));
    assert!(capabilities.contains(&ImapCapability::AuthLogin));
    assert!(capabilities.contains(&ImapCapability::Idle));
    assert!(capabilities.contains(&ImapCapability::Namespace));
}

#[test]
fn test_protocol_folder_parsing() {
    let response = r#"* LIST (\HasNoChildren) "/" "INBOX"
* LIST (\HasChildren \Noselect) "/" "[Gmail]"
* LIST (\HasNoChildren \All) "/" "[Gmail]/All Mail"
* LIST (\HasNoChildren \Drafts) "/" "[Gmail]/Drafts"
* LIST (\HasNoChildren \Sent) "/" "[Gmail]/Sent Mail"
A002 OK LIST completed
"#;

    let folders = ImapProtocol::parse_folders(response).unwrap();
    assert_eq!(folders.len(), 5);

    let inbox = &folders[0];
    assert_eq!(inbox.name, "INBOX");
    assert_eq!(inbox.full_name, "INBOX");
    assert_eq!(inbox.delimiter, Some("/".to_string()));
    assert!(inbox.attributes.contains(&FolderAttribute::HasNoChildren));

    let gmail_parent = &folders[1];
    assert_eq!(gmail_parent.name, "[Gmail]");
    assert_eq!(gmail_parent.full_name, "[Gmail]");
    assert!(gmail_parent
        .attributes
        .contains(&FolderAttribute::HasChildren));
    assert!(gmail_parent.attributes.contains(&FolderAttribute::Noselect));

    let drafts = &folders[3];
    assert_eq!(drafts.name, "Drafts");
    assert_eq!(drafts.full_name, "[Gmail]/Drafts");
    assert!(drafts.attributes.contains(&FolderAttribute::Drafts));
}

#[test]
fn test_protocol_select_parsing() {
    let response = r#"* 18 EXISTS
* 0 RECENT
* OK [UNSEEN 4] Message 4 is first unseen
* OK [UIDVALIDITY 1234567890] UIDs valid
* OK [UIDNEXT 19] Predicted next UID
* FLAGS (\Answered \Flagged \Deleted \Seen \Draft)
* OK [PERMANENTFLAGS (\Answered \Flagged \Deleted \Seen \Draft \*)] Flags permitted
A003 OK [READ-WRITE] SELECT completed
"#;

    let folder = ImapProtocol::parse_select_response(response).unwrap();
    assert_eq!(folder.exists, Some(18));
    assert_eq!(folder.recent, Some(0));
    assert_eq!(folder.unseen, Some(4));
    assert_eq!(folder.uid_validity, Some(1234567890));
    assert_eq!(folder.uid_next, Some(19));
}

#[test]
fn test_protocol_search_parsing() {
    let response = "* SEARCH 2 4 7 9 12\nA004 OK SEARCH completed\n";
    let message_ids = ImapProtocol::parse_search_response(response).unwrap();
    assert_eq!(message_ids, vec![2, 4, 7, 9, 12]);

    let empty_response = "* SEARCH\nA005 OK SEARCH completed\n";
    let empty_ids = ImapProtocol::parse_search_response(empty_response).unwrap();
    assert!(empty_ids.is_empty());
}

#[test]
fn test_protocol_command_formatting() {
    assert_eq!(
        ImapProtocol::format_login("user", "pass"),
        "LOGIN \"user\" \"pass\""
    );

    assert_eq!(ImapProtocol::format_select("INBOX"), "SELECT \"INBOX\"");

    assert_eq!(ImapProtocol::format_examine("Drafts"), "EXAMINE \"Drafts\"");

    assert_eq!(ImapProtocol::format_list("", "*"), "LIST \"\" \"*\"");

    assert_eq!(
        ImapProtocol::format_fetch("1:10", &["FLAGS", "UID", "RFC822.SIZE"]),
        "FETCH 1:10 (FLAGS UID RFC822.SIZE)"
    );

    assert_eq!(
        ImapProtocol::format_uid_fetch("100:110", &["ENVELOPE", "BODYSTRUCTURE"]),
        "UID FETCH 100:110 (ENVELOPE BODYSTRUCTURE)"
    );

    assert_eq!(
        ImapProtocol::format_create("Test Folder"),
        "CREATE \"Test Folder\""
    );

    assert_eq!(
        ImapProtocol::format_delete("Old Folder"),
        "DELETE \"Old Folder\""
    );

    assert_eq!(
        ImapProtocol::format_rename("Old Name", "New Name"),
        "RENAME \"Old Name\" \"New Name\""
    );
}

#[test]
fn test_protocol_store_formatting() {
    let flags = vec![MessageFlag::Seen, MessageFlag::Flagged];

    assert_eq!(
        ImapProtocol::format_store("1:5", &flags, "FLAGS"),
        "STORE 1:5 FLAGS (\\Seen \\Flagged)"
    );

    assert_eq!(
        ImapProtocol::format_store("1:5", &flags, "+FLAGS"),
        "STORE 1:5 +FLAGS (\\Seen \\Flagged)"
    );

    assert_eq!(
        ImapProtocol::format_uid_store("100:105", &flags, "-FLAGS"),
        "UID STORE 100:105 -FLAGS (\\Seen \\Flagged)"
    );
}

#[test]
fn test_protocol_copy_formatting() {
    assert_eq!(
        ImapProtocol::format_copy("1:10", "Archive"),
        "COPY 1:10 \"Archive\""
    );

    assert_eq!(
        ImapProtocol::format_uid_copy("100:110", "Sent"),
        "UID COPY 100:110 \"Sent\""
    );
}

#[test]
fn test_protocol_authenticate_plain() {
    let result = ImapProtocol::format_authenticate_plain("user", "pass").unwrap();

    // The base64 encoding of "\0user\0pass" should be consistent
    use base64::{engine::general_purpose, Engine as _};
    let expected_encoding = general_purpose::STANDARD.encode("\0user\0pass");
    assert_eq!(result, format!("AUTHENTICATE PLAIN {}", expected_encoding));
}

#[test]
fn test_imap_client_creation() {
    let config = ImapConfig::gmail("user@gmail.com".to_string(), "password".to_string());
    let client = ImapClient::new(config);

    assert!(!client.is_connected());
    assert!(!client.is_authenticated());
    assert!(client.selected_folder().is_none());
    assert_eq!(client.capabilities().len(), 0);
}

#[test]
fn test_imap_error_types() {
    let conn_error = ImapError::connection("Failed to connect");
    assert!(conn_error.is_connection_error());
    assert!(conn_error.is_recoverable());

    let auth_error = ImapError::authentication("Invalid credentials");
    assert!(auth_error.is_auth_error());
    assert!(!auth_error.is_recoverable());

    let protocol_error = ImapError::protocol("Invalid response");
    assert!(!protocol_error.is_connection_error());
    assert!(!protocol_error.is_recoverable());
}

#[test]
fn test_address_creation() {
    let addr = comunicado::imap::Address::new("john".to_string(), "example.com".to_string())
        .with_name("John Doe".to_string());

    assert_eq!(addr.email_address(), Some("john@example.com".to_string()));
    assert_eq!(addr.display_name(), "John Doe <john@example.com>");

    let simple_addr = comunicado::imap::Address::new("jane".to_string(), "test.com".to_string());
    assert_eq!(simple_addr.display_name(), "jane@test.com");
}

#[test]
fn test_body_structure_checks() {
    let text_plain = comunicado::imap::BodyStructure::new("text".to_string(), "plain".to_string());
    assert!(text_plain.is_text());
    assert!(text_plain.is_plain_text());
    assert!(!text_plain.is_html());
    assert!(!text_plain.is_multipart());

    let text_html = comunicado::imap::BodyStructure::new("text".to_string(), "html".to_string());
    assert!(text_html.is_text());
    assert!(text_html.is_html());
    assert!(!text_html.is_plain_text());

    let multipart =
        comunicado::imap::BodyStructure::new("multipart".to_string(), "mixed".to_string());
    assert!(!multipart.is_text());
    assert!(multipart.is_multipart());
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    // Note: These are mock tests since we don't have a real IMAP server
    // In a real implementation, you'd use a test IMAP server or mocking framework

    #[test]
    fn test_full_workflow_simulation() {
        // This test simulates the workflow without actual network calls
        let config = ImapConfig::gmail("test@gmail.com".to_string(), "password".to_string());
        let mut client = ImapClient::new(config);

        // Simulate connection state changes
        assert!(!client.is_connected());

        // In a real test, you would:
        // 1. Start a mock IMAP server
        // 2. Connect the client
        // 3. Authenticate
        // 4. Perform operations
        // 5. Verify results

        // For now, just verify the client can be created properly
        assert_eq!(client.capabilities().len(), 0);
        assert!(client.selected_folder().is_none());
    }
}
