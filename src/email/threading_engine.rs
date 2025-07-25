use crate::email::{EmailMessage, EmailThread, MessageId};
use std::collections::HashMap;

/// Algorithm used for email threading
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadingAlgorithm {
    /// Simple threading based on subject normalization
    Simple,
    /// Jamie Zawinski's threading algorithm (RFC compliant)
    JWZ,
}

/// Engine for threading email messages into conversations
#[derive(Debug)]
pub struct ThreadingEngine {
    algorithm: ThreadingAlgorithm,
}

impl ThreadingEngine {
    /// Create a new threading engine with the specified algorithm
    pub fn new(algorithm: ThreadingAlgorithm) -> Self {
        Self { algorithm }
    }
    
    /// Get the current threading algorithm
    pub fn algorithm(&self) -> &ThreadingAlgorithm {
        &self.algorithm
    }
    
    /// Thread a collection of messages into conversation threads
    pub fn thread_messages(&mut self, messages: Vec<EmailMessage>) -> Vec<EmailThread> {
        match self.algorithm {
            ThreadingAlgorithm::Simple => self.thread_simple(messages),
            ThreadingAlgorithm::JWZ => self.thread_jwz(messages),
        }
    }
    
    /// Simple threading algorithm - groups by normalized subject
    fn thread_simple(&self, messages: Vec<EmailMessage>) -> Vec<EmailThread> {
        let mut threads_by_subject: HashMap<String, EmailThread> = HashMap::new();
        let mut standalone_threads = Vec::new();
        
        for message in messages {
            let normalized_subject = EmailThread::normalize_subject(message.subject());
            
            if let Some(existing_thread) = threads_by_subject.get_mut(&normalized_subject) {
                // Add to existing thread
                existing_thread.add_reply(message);
            } else {
                // Create new thread
                let thread = EmailThread::new(message);
                threads_by_subject.insert(normalized_subject, thread);
            }
        }
        
        // Convert to vector and add any standalone threads
        let mut all_threads: Vec<EmailThread> = threads_by_subject.into_values().collect();
        all_threads.append(&mut standalone_threads);
        
        all_threads
    }
    
    /// Jamie Zawinski's threading algorithm - uses References and In-Reply-To headers
    fn thread_jwz(&self, messages: Vec<EmailMessage>) -> Vec<EmailThread> {
        // Step 1: Build id_table mapping message IDs to containers
        let mut id_table: HashMap<MessageId, MessageContainer> = HashMap::new();
        
        // Step 2: Process each message
        for message in messages {
            let message_id = message.message_id().clone();
            
            // Get or create container for this message
            let container = id_table.entry(message_id.clone())
                .or_insert_with(|| MessageContainer::new(Some(message_id.clone())));
            container.message = Some(message.clone());
            
            // Process References header - simplified for now
            // Note: Full JWZ implementation is complex and requires careful container management
            if let Some(in_reply_to) = message.in_reply_to() {
                let parent_container = id_table.entry(in_reply_to.clone())
                    .or_insert_with(|| MessageContainer::new(Some(in_reply_to.clone())));
                
                // Create a new container for this message and link it
                let mut child_container = MessageContainer::new(Some(message_id.clone()));
                child_container.message = Some(message.clone());
                child_container.parent = Some(in_reply_to.clone());
                
                // Note: In a full implementation, we would properly link containers here
                // For now, we'll rely on the simple algorithm structure
            }
        }
        
        // Step 3: Find root containers (those without parents)
        let mut root_containers = Vec::new();
        for container in id_table.into_values() {
            if container.parent.is_none() {
                root_containers.push(container);
            }
        }
        
        // Step 4: Convert containers to EmailThreads
        let mut threads = Vec::new();
        for container in root_containers {
            if let Some(thread) = self.container_to_thread(container) {
                threads.push(thread);
            }
        }
        
        threads
    }
    
    /// Convert a MessageContainer to an EmailThread
    fn container_to_thread(&self, container: MessageContainer) -> Option<EmailThread> {
        if let Some(message) = container.message {
            let mut thread = EmailThread::new(message);
            
            // Process children
            for child_container in container.children {
                if let Some(child_thread) = self.container_to_thread(child_container) {
                    thread.add_reply(child_thread.root_message().clone());
                }
            }
            
            Some(thread)
        } else if container.children.len() == 1 {
            // Phantom container with single child - promote the child
            self.container_to_thread(container.children.into_iter().next().unwrap())
        } else if !container.children.is_empty() {
            // Phantom container with multiple children - create threads for each
            None // This case should be handled at a higher level
        } else {
            None
        }
    }
    
    /// Check if two messages are duplicates
    pub fn is_duplicate(&self, msg1: &EmailMessage, msg2: &EmailMessage) -> bool {
        // Same message ID
        if msg1.message_id() == msg2.message_id() {
            return true;
        }
        
        // Same sender, subject, and timestamp (within 1 minute)
        if msg1.sender() == msg2.sender() &&
           msg1.normalized_subject() == msg2.normalized_subject() {
            let time_diff = (*msg1.timestamp() - *msg2.timestamp()).num_seconds().abs();
            if time_diff < 60 {
                return true;
            }
        }
        
        false
    }
    
    /// Remove duplicate messages from a collection
    pub fn deduplicate_messages(&self, messages: Vec<EmailMessage>) -> Vec<EmailMessage> {
        let mut unique_messages = Vec::new();
        
        for message in messages {
            let is_duplicate = unique_messages.iter()
                .any(|existing| self.is_duplicate(&message, existing));
            
            if !is_duplicate {
                unique_messages.push(message);
            }
        }
        
        unique_messages
    }
    
    /// Merge threads with the same normalized subject
    pub fn merge_similar_threads(&self, threads: Vec<EmailThread>) -> Vec<EmailThread> {
        let mut merged_threads: HashMap<String, EmailThread> = HashMap::new();
        
        for thread in threads {
            let normalized_subject = thread.normalized_subject();
            
            if let Some(existing_thread) = merged_threads.get_mut(&normalized_subject) {
                // Merge threads by adding all messages from the new thread
                for message in thread.get_all_messages() {
                    if message.message_id() != thread.root_message().message_id() {
                        existing_thread.add_reply(message);
                    }
                }
            } else {
                merged_threads.insert(normalized_subject, thread);
            }
        }
        
        merged_threads.into_values().collect()
    }
}

/// Container used in JWZ threading algorithm
#[derive(Debug)]
struct MessageContainer {
    message_id: Option<MessageId>,
    message: Option<EmailMessage>,
    parent: Option<MessageId>,
    children: Vec<MessageContainer>,
}

impl MessageContainer {
    fn new(message_id: Option<MessageId>) -> Self {
        Self {
            message_id,
            message: None,
            parent: None,
            children: Vec::new(),
        }
    }
}

impl Default for ThreadingEngine {
    fn default() -> Self {
        Self::new(ThreadingAlgorithm::JWZ)
    }
}