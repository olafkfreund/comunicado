use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// IMAP folder/mailbox information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImapFolder {
    pub name: String,
    pub full_name: String,
    pub delimiter: Option<String>,
    pub attributes: Vec<FolderAttribute>,
    pub exists: Option<u32>,
    pub recent: Option<u32>,
    pub unseen: Option<u32>,
    pub uid_validity: Option<u32>,
    pub uid_next: Option<u32>,
}

impl ImapFolder {
    pub fn new(name: String, full_name: String) -> Self {
        Self {
            name,
            full_name,
            delimiter: None,
            attributes: Vec::new(),
            exists: None,
            recent: None,
            unseen: None,
            uid_validity: None,
            uid_next: None,
        }
    }

    pub fn is_selectable(&self) -> bool {
        !self.attributes.contains(&FolderAttribute::Noselect)
    }

    pub fn has_children(&self) -> bool {
        self.attributes.contains(&FolderAttribute::HasChildren)
    }

    pub fn is_inbox(&self) -> bool {
        self.full_name.to_uppercase() == "INBOX"
    }
}

/// IMAP folder attributes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FolderAttribute {
    Noinferiors,
    Noselect,
    Marked,
    Unmarked,
    HasChildren,
    HasNoChildren,
    All,
    Archive,
    Drafts,
    Flagged,
    Junk,
    Sent,
    Trash,
    Custom(String),
}

impl FolderAttribute {
    pub fn from_str(attr: &str) -> Self {
        match attr.to_uppercase().as_str() {
            "\\NOINFERIORS" => FolderAttribute::Noinferiors,
            "\\NOSELECT" => FolderAttribute::Noselect,
            "\\MARKED" => FolderAttribute::Marked,
            "\\UNMARKED" => FolderAttribute::Unmarked,
            "\\HASCHILDREN" => FolderAttribute::HasChildren,
            "\\HASNOCHILDREN" => FolderAttribute::HasNoChildren,
            "\\ALL" => FolderAttribute::All,
            "\\ARCHIVE" => FolderAttribute::Archive,
            "\\DRAFTS" => FolderAttribute::Drafts,
            "\\FLAGGED" => FolderAttribute::Flagged,
            "\\JUNK" => FolderAttribute::Junk,
            "\\SENT" => FolderAttribute::Sent,
            "\\TRASH" => FolderAttribute::Trash,
            _ => FolderAttribute::Custom(attr.to_string()),
        }
    }
}

/// IMAP message information
#[derive(Debug, Clone)]
pub struct ImapMessage {
    pub sequence_number: u32,
    pub uid: Option<u32>,
    pub flags: Vec<MessageFlag>,
    pub internal_date: Option<DateTime<Utc>>,
    pub size: Option<u32>,
    pub envelope: Option<MessageEnvelope>,
    pub body_structure: Option<BodyStructure>,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

impl ImapMessage {
    pub fn new(sequence_number: u32) -> Self {
        Self {
            sequence_number,
            uid: None,
            flags: Vec::new(),
            internal_date: None,
            size: None,
            envelope: None,
            body_structure: None,
            headers: HashMap::new(),
            body: None,
        }
    }

    pub fn is_seen(&self) -> bool {
        self.flags.contains(&MessageFlag::Seen)
    }

    pub fn is_flagged(&self) -> bool {
        self.flags.contains(&MessageFlag::Flagged)
    }

    pub fn is_deleted(&self) -> bool {
        self.flags.contains(&MessageFlag::Deleted)
    }

    pub fn is_draft(&self) -> bool {
        self.flags.contains(&MessageFlag::Draft)
    }

    pub fn is_recent(&self) -> bool {
        self.flags.contains(&MessageFlag::Recent)
    }
}

/// IMAP message flags
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageFlag {
    Seen,
    Answered,
    Flagged,
    Deleted,
    Draft,
    Recent,
    Custom(String),
}

impl MessageFlag {
    pub fn from_str(flag: &str) -> Self {
        match flag.to_uppercase().as_str() {
            "\\SEEN" => MessageFlag::Seen,
            "\\ANSWERED" => MessageFlag::Answered,
            "\\FLAGGED" => MessageFlag::Flagged,
            "\\DELETED" => MessageFlag::Deleted,
            "\\DRAFT" => MessageFlag::Draft,
            "\\RECENT" => MessageFlag::Recent,
            _ => MessageFlag::Custom(flag.to_string()),
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            MessageFlag::Seen => "\\Seen".to_string(),
            MessageFlag::Answered => "\\Answered".to_string(),
            MessageFlag::Flagged => "\\Flagged".to_string(),
            MessageFlag::Deleted => "\\Deleted".to_string(),
            MessageFlag::Draft => "\\Draft".to_string(),
            MessageFlag::Recent => "\\Recent".to_string(),
            MessageFlag::Custom(flag) => flag.clone(),
        }
    }
}

/// Message envelope information
#[derive(Debug, Clone)]
pub struct MessageEnvelope {
    pub date: Option<String>,
    pub subject: Option<String>,
    pub from: Vec<Address>,
    pub sender: Vec<Address>,
    pub reply_to: Vec<Address>,
    pub to: Vec<Address>,
    pub cc: Vec<Address>,
    pub bcc: Vec<Address>,
    pub in_reply_to: Option<String>,
    pub message_id: Option<String>,
}

impl MessageEnvelope {
    pub fn new() -> Self {
        Self {
            date: None,
            subject: None,
            from: Vec::new(),
            sender: Vec::new(),
            reply_to: Vec::new(),
            to: Vec::new(),
            cc: Vec::new(),
            bcc: Vec::new(),
            in_reply_to: None,
            message_id: None,
        }
    }
}

impl Default for MessageEnvelope {
    fn default() -> Self {
        Self::new()
    }
}

/// Email address
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Address {
    pub name: Option<String>,
    pub route: Option<String>,
    pub mailbox: Option<String>,
    pub host: Option<String>,
}

impl Address {
    pub fn new(mailbox: String, host: String) -> Self {
        Self {
            name: None,
            route: None,
            mailbox: Some(mailbox),
            host: Some(host),
        }
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    pub fn email_address(&self) -> Option<String> {
        if let (Some(mailbox), Some(host)) = (&self.mailbox, &self.host) {
            Some(format!("{}@{}", mailbox, host))
        } else {
            None
        }
    }

    pub fn display_name(&self) -> String {
        if let Some(name) = &self.name {
            if let Some(email) = self.email_address() {
                format!("{} <{}>", name, email)
            } else {
                name.clone()
            }
        } else {
            self.email_address()
                .unwrap_or_else(|| "Unknown".to_string())
        }
    }
}

/// Message body structure
#[derive(Debug, Clone)]
pub struct BodyStructure {
    pub media_type: String,
    pub media_subtype: String,
    pub parameters: HashMap<String, String>,
    pub content_id: Option<String>,
    pub content_description: Option<String>,
    pub content_transfer_encoding: Option<String>,
    pub size: Option<u32>,
    pub parts: Vec<BodyStructure>,
}

impl BodyStructure {
    pub fn new(media_type: String, media_subtype: String) -> Self {
        Self {
            media_type,
            media_subtype,
            parameters: HashMap::new(),
            content_id: None,
            content_description: None,
            content_transfer_encoding: None,
            size: None,
            parts: Vec::new(),
        }
    }

    pub fn is_text(&self) -> bool {
        self.media_type.to_lowercase() == "text"
    }

    pub fn is_html(&self) -> bool {
        self.is_text() && self.media_subtype.to_lowercase() == "html"
    }

    pub fn is_plain_text(&self) -> bool {
        self.is_text() && self.media_subtype.to_lowercase() == "plain"
    }

    pub fn is_multipart(&self) -> bool {
        self.media_type.to_lowercase() == "multipart"
    }

    pub fn is_attachment(&self) -> bool {
        self.parameters.get("name").is_some() || self.content_id.is_some()
    }
}

/// IMAP search criteria
#[derive(Debug, Clone)]
pub enum SearchCriteria {
    All,
    Answered,
    Bcc(String),
    Before(DateTime<Utc>),
    Body(String),
    Cc(String),
    Deleted,
    Draft,
    Flagged,
    From(String),
    Header(String, String),
    Keyword(String),
    Larger(u32),
    New,
    Not(Box<SearchCriteria>),
    Old,
    On(DateTime<Utc>),
    Or(Box<SearchCriteria>, Box<SearchCriteria>),
    Recent,
    Seen,
    Since(DateTime<Utc>),
    Subject(String),
    Text(String),
    To(String),
    Uid(String),
    Unanswered,
    Undeleted,
    Unflagged,
    Unkeyword(String),
    Unseen,
}

impl SearchCriteria {
    /// Convert search criteria to IMAP search command string
    pub fn to_imap_string(&self) -> String {
        match self {
            SearchCriteria::All => "ALL".to_string(),
            SearchCriteria::Answered => "ANSWERED".to_string(),
            SearchCriteria::Bcc(addr) => format!("BCC \"{}\"", addr),
            SearchCriteria::Before(date) => format!("BEFORE \"{}\"", date.format("%d-%b-%Y")),
            SearchCriteria::Body(text) => format!("BODY \"{}\"", text),
            SearchCriteria::Cc(addr) => format!("CC \"{}\"", addr),
            SearchCriteria::Deleted => "DELETED".to_string(),
            SearchCriteria::Draft => "DRAFT".to_string(),
            SearchCriteria::Flagged => "FLAGGED".to_string(),
            SearchCriteria::From(addr) => format!("FROM \"{}\"", addr),
            SearchCriteria::Header(name, value) => format!("HEADER \"{}\" \"{}\"", name, value),
            SearchCriteria::Keyword(kw) => format!("KEYWORD \"{}\"", kw),
            SearchCriteria::Larger(size) => format!("LARGER {}", size),
            SearchCriteria::New => "NEW".to_string(),
            SearchCriteria::Not(criteria) => format!("NOT {}", criteria.to_imap_string()),
            SearchCriteria::Old => "OLD".to_string(),
            SearchCriteria::On(date) => format!("ON \"{}\"", date.format("%d-%b-%Y")),
            SearchCriteria::Or(c1, c2) => {
                format!("OR {} {}", c1.to_imap_string(), c2.to_imap_string())
            }
            SearchCriteria::Recent => "RECENT".to_string(),
            SearchCriteria::Seen => "SEEN".to_string(),
            SearchCriteria::Since(date) => format!("SINCE \"{}\"", date.format("%d-%b-%Y")),
            SearchCriteria::Subject(text) => format!("SUBJECT \"{}\"", text),
            SearchCriteria::Text(text) => format!("TEXT \"{}\"", text),
            SearchCriteria::To(addr) => format!("TO \"{}\"", addr),
            SearchCriteria::Uid(uid) => format!("UID {}", uid),
            SearchCriteria::Unanswered => "UNANSWERED".to_string(),
            SearchCriteria::Undeleted => "UNDELETED".to_string(),
            SearchCriteria::Unflagged => "UNFLAGGED".to_string(),
            SearchCriteria::Unkeyword(kw) => format!("UNKEYWORD \"{}\"", kw),
            SearchCriteria::Unseen => "UNSEEN".to_string(),
        }
    }
}
