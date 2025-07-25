use crate::email::{EmailMessage, EmailThread};
use std::cmp::Ordering;

/// Sort order for email and thread sorting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Ascending,
    Descending,
}

/// Criteria for sorting emails and threads
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SortCriteria {
    Date(SortOrder),
    Sender(SortOrder),
    Subject(SortOrder),
    Size(SortOrder),
    Priority(SortOrder),
    ThreadDepth(SortOrder),
    MessageCount(SortOrder), // For threads
    HasUnread(SortOrder),
}

impl SortCriteria {
    /// Compare two email messages according to this sort criteria
    pub fn compare(&self, a: &EmailMessage, b: &EmailMessage) -> Ordering {
        let ordering = match self {
            SortCriteria::Date(_) => a.timestamp().cmp(b.timestamp()),
            SortCriteria::Sender(_) => a.sender().cmp(b.sender()),
            SortCriteria::Subject(_) => {
                let a_subject = EmailThread::normalize_subject(a.subject());
                let b_subject = EmailThread::normalize_subject(b.subject());
                a_subject.cmp(&b_subject)
            },
            SortCriteria::Size(_) => {
                // For now, use content length as a proxy for size
                a.content().len().cmp(&b.content().len())
            },
            SortCriteria::Priority(_) => {
                // Priority: important > unread > read
                let a_priority = if a.is_important() { 2 } else if !a.is_read() { 1 } else { 0 };
                let b_priority = if b.is_important() { 2 } else if !b.is_read() { 1 } else { 0 };
                a_priority.cmp(&b_priority)
            },
            _ => Ordering::Equal, // Thread-specific criteria don't apply to individual messages
        };
        
        // Apply sort order
        match self.get_sort_order() {
            SortOrder::Ascending => ordering,
            SortOrder::Descending => ordering.reverse(),
        }
    }
    
    /// Compare two email threads according to this sort criteria
    pub fn compare_threads(&self, a: &EmailThread, b: &EmailThread) -> Ordering {
        let ordering = match self {
            SortCriteria::Date(_) => a.latest_timestamp().cmp(&b.latest_timestamp()),
            SortCriteria::Sender(_) => a.root_message().sender().cmp(b.root_message().sender()),
            SortCriteria::Subject(_) => a.normalized_subject().cmp(&b.normalized_subject()),
            SortCriteria::Size(_) => {
                // Compare total content size of all messages in thread
                let a_size: usize = a.get_all_messages().iter()
                    .map(|msg| msg.content().len())
                    .sum();
                let b_size: usize = b.get_all_messages().iter()
                    .map(|msg| msg.content().len())
                    .sum();
                a_size.cmp(&b_size)
            },
            SortCriteria::Priority(_) => {
                // Thread priority: has important > has unread > all read
                let a_priority = if a.get_all_messages().iter().any(|msg| msg.is_important()) { 2 }
                    else if a.has_unread() { 1 } else { 0 };
                let b_priority = if b.get_all_messages().iter().any(|msg| msg.is_important()) { 2 }
                    else if b.has_unread() { 1 } else { 0 };
                a_priority.cmp(&b_priority)
            },
            SortCriteria::ThreadDepth(_) => a.depth().cmp(&b.depth()),
            SortCriteria::MessageCount(_) => a.message_count().cmp(&b.message_count()),
            SortCriteria::HasUnread(_) => {
                let a_unread = if a.has_unread() { 1 } else { 0 };
                let b_unread = if b.has_unread() { 1 } else { 0 };
                a_unread.cmp(&b_unread)
            },
        };
        
        // Apply sort order
        match self.get_sort_order() {
            SortOrder::Ascending => ordering,
            SortOrder::Descending => ordering.reverse(),
        }
    }
    
    /// Get the sort order for this criteria
    pub fn get_sort_order(&self) -> SortOrder {
        match self {
            SortCriteria::Date(order) => *order,
            SortCriteria::Sender(order) => *order,
            SortCriteria::Subject(order) => *order,
            SortCriteria::Size(order) => *order,
            SortCriteria::Priority(order) => *order,
            SortCriteria::ThreadDepth(order) => *order,
            SortCriteria::MessageCount(order) => *order,
            SortCriteria::HasUnread(order) => *order,
        }
    }
    
    /// Get a human-readable description of this sort criteria
    pub fn description(&self) -> String {
        let (name, order) = match self {
            SortCriteria::Date(order) => ("Date", order),
            SortCriteria::Sender(order) => ("Sender", order),
            SortCriteria::Subject(order) => ("Subject", order),
            SortCriteria::Size(order) => ("Size", order),
            SortCriteria::Priority(order) => ("Priority", order),
            SortCriteria::ThreadDepth(order) => ("Thread Depth", order),
            SortCriteria::MessageCount(order) => ("Message Count", order),
            SortCriteria::HasUnread(order) => ("Unread Status", order),
        };
        
        let order_str = match order {
            SortOrder::Ascending => "ascending",
            SortOrder::Descending => "descending",
        };
        
        format!("{} ({})", name, order_str)
    }
    
    /// Get available sort criteria for emails
    pub fn email_criteria() -> Vec<SortCriteria> {
        vec![
            SortCriteria::Date(SortOrder::Descending),
            SortCriteria::Date(SortOrder::Ascending),
            SortCriteria::Sender(SortOrder::Ascending),
            SortCriteria::Sender(SortOrder::Descending),
            SortCriteria::Subject(SortOrder::Ascending),
            SortCriteria::Subject(SortOrder::Descending),
            SortCriteria::Size(SortOrder::Descending),
            SortCriteria::Size(SortOrder::Ascending),
            SortCriteria::Priority(SortOrder::Descending),
        ]
    }
    
    /// Get available sort criteria for threads
    pub fn thread_criteria() -> Vec<SortCriteria> {
        vec![
            SortCriteria::Date(SortOrder::Descending),
            SortCriteria::Date(SortOrder::Ascending),
            SortCriteria::Sender(SortOrder::Ascending),
            SortCriteria::Sender(SortOrder::Descending),
            SortCriteria::Subject(SortOrder::Ascending),
            SortCriteria::Subject(SortOrder::Descending),
            SortCriteria::MessageCount(SortOrder::Descending),
            SortCriteria::MessageCount(SortOrder::Ascending),
            SortCriteria::ThreadDepth(SortOrder::Descending),
            SortCriteria::ThreadDepth(SortOrder::Ascending),
            SortCriteria::Priority(SortOrder::Descending),
            SortCriteria::HasUnread(SortOrder::Descending),
        ]
    }
}

/// Multi-criteria sorting for complex email organization
#[derive(Debug, Clone)]
pub struct MultiCriteriaSorter {
    criteria: Vec<SortCriteria>,
}

impl MultiCriteriaSorter {
    /// Create a new multi-criteria sorter
    pub fn new(criteria: Vec<SortCriteria>) -> Self {
        Self { criteria }
    }
    
    /// Add a sort criteria to the sorter
    pub fn add_criteria(&mut self, criteria: SortCriteria) {
        self.criteria.push(criteria);
    }
    
    /// Remove all criteria
    pub fn clear(&mut self) {
        self.criteria.clear();
    }
    
    /// Compare two email messages using all criteria
    pub fn compare_messages(&self, a: &EmailMessage, b: &EmailMessage) -> Ordering {
        for criteria in &self.criteria {
            let result = criteria.compare(a, b);
            if result != Ordering::Equal {
                return result;
            }
        }
        Ordering::Equal
    }
    
    /// Compare two email threads using all criteria
    pub fn compare_threads(&self, a: &EmailThread, b: &EmailThread) -> Ordering {
        for criteria in &self.criteria {
            let result = criteria.compare_threads(a, b);
            if result != Ordering::Equal {
                return result;
            }
        }
        Ordering::Equal
    }
    
    /// Sort a vector of messages using all criteria
    pub fn sort_messages(&self, messages: &mut Vec<EmailMessage>) {
        messages.sort_by(|a, b| self.compare_messages(a, b));
    }
    
    /// Sort a vector of threads using all criteria
    pub fn sort_threads(&self, threads: &mut Vec<EmailThread>) {
        threads.sort_by(|a, b| self.compare_threads(a, b));
    }
    
    /// Get a description of all criteria
    pub fn description(&self) -> String {
        if self.criteria.is_empty() {
            "No sorting".to_string()
        } else {
            self.criteria.iter()
                .map(|c| c.description())
                .collect::<Vec<_>>()
                .join(", then by ")
        }
    }
}

impl Default for MultiCriteriaSorter {
    fn default() -> Self {
        // Default: sort by date (newest first), then by sender
        Self::new(vec![
            SortCriteria::Date(SortOrder::Descending),
            SortCriteria::Sender(SortOrder::Ascending),
        ])
    }
}

/// Presets for common sorting patterns
impl SortCriteria {
    /// Most recent emails first
    pub fn newest_first() -> MultiCriteriaSorter {
        MultiCriteriaSorter::new(vec![
            SortCriteria::Date(SortOrder::Descending),
        ])
    }
    
    /// Oldest emails first
    pub fn oldest_first() -> MultiCriteriaSorter {
        MultiCriteriaSorter::new(vec![
            SortCriteria::Date(SortOrder::Ascending),
        ])
    }
    
    /// Priority-based sorting (important, then unread, then date)
    pub fn priority_first() -> MultiCriteriaSorter {
        MultiCriteriaSorter::new(vec![
            SortCriteria::Priority(SortOrder::Descending),
            SortCriteria::Date(SortOrder::Descending),
        ])
    }
    
    /// Alphabetical by sender
    pub fn sender_alphabetical() -> MultiCriteriaSorter {
        MultiCriteriaSorter::new(vec![
            SortCriteria::Sender(SortOrder::Ascending),
            SortCriteria::Date(SortOrder::Descending),
        ])
    }
    
    /// Alphabetical by subject
    pub fn subject_alphabetical() -> MultiCriteriaSorter {
        MultiCriteriaSorter::new(vec![
            SortCriteria::Subject(SortOrder::Ascending),
            SortCriteria::Date(SortOrder::Descending),
        ])
    }
    
    /// Thread-optimized sorting (by activity, then size)
    pub fn thread_activity() -> MultiCriteriaSorter {
        MultiCriteriaSorter::new(vec![
            SortCriteria::HasUnread(SortOrder::Descending),
            SortCriteria::Date(SortOrder::Descending),
            SortCriteria::MessageCount(SortOrder::Descending),
        ])
    }
}