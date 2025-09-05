//! Collaboration features for cloud synchronization

use super::{CloudSyncError, CloudSyncResult, SyncDataType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Collaboration manager for shared resources
pub struct CollaborationManager {
    shared_resources: HashMap<String, SharedResource>,
    user_sessions: HashMap<String, UserSession>,
    permission_manager: PermissionManager,
    presence_tracker: PresenceTracker,
    activity_log: ActivityLog,
}

/// Shared resource with collaboration metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedResource {
    pub id: String,
    pub data_type: SyncDataType,
    pub name: String,
    pub owner_id: String,
    pub created_at: DateTime<Utc>,
    pub last_modified: DateTime<Utc>,
    pub permissions: Vec<ResourcePermission>,
    pub sharing_settings: SharingSettings,
    pub collaboration_state: CollaborationState,
}

/// Permission levels for shared resources
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Permission {
    /// Read-only access
    Read,
    /// Read and comment
    Comment,
    /// Read, comment, and edit
    Edit,
    /// Full control including sharing
    Admin,
    /// Owner has all permissions
    Owner,
}

/// Resource permission entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePermission {
    pub user_id: String,
    pub permission: Permission,
    pub granted_at: DateTime<Utc>,
    pub granted_by: String,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Sharing settings for resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharingSettings {
    pub public_sharing_enabled: bool,
    pub link_sharing_enabled: bool,
    pub require_authentication: bool,
    pub allow_downloads: bool,
    pub allow_comments: bool,
    pub auto_approve_requests: bool,
    pub sharing_link: Option<String>,
}

/// Collaboration state tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationState {
    pub active_editors: Vec<String>,
    pub locked_sections: Vec<ResourceLock>,
    pub pending_changes: Vec<PendingChange>,
    pub conflict_regions: Vec<ConflictRegion>,
    pub version: u64,
}

/// Resource lock for collaborative editing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLock {
    pub id: Uuid,
    pub user_id: String,
    pub section_id: String,
    pub lock_type: LockType,
    pub acquired_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

/// Types of locks for collaboration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LockType {
    /// Exclusive write lock
    Exclusive,
    /// Shared read lock
    Shared,
    /// Advisory lock (warning only)
    Advisory,
    /// Section-specific lock
    Section(String),
}

/// Pending change for collaborative editing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingChange {
    pub id: Uuid,
    pub user_id: String,
    pub resource_id: String,
    pub change_type: ChangeType,
    pub data: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub applied: bool,
}

/// Types of collaborative changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeType {
    TextInsert { position: usize, text: String },
    TextDelete { position: usize, length: usize },
    TextReplace { position: usize, old_text: String, new_text: String },
    PropertyUpdate { path: String, value: serde_json::Value },
    StructureChange { operation: String, data: serde_json::Value },
}

/// Conflict region in collaborative editing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictRegion {
    pub id: Uuid,
    pub resource_id: String,
    pub start_position: usize,
    pub end_position: usize,
    pub conflicting_users: Vec<String>,
    pub detected_at: DateTime<Utc>,
    pub resolution_strategy: Option<ConflictResolutionStrategy>,
}

/// Conflict resolution strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolutionStrategy {
    /// Accept all changes in order
    AcceptAll,
    /// Accept changes from specific user
    AcceptUser(String),
    /// Manual resolution required
    Manual,
    /// Merge automatically
    AutoMerge,
}

/// User session for collaboration
#[derive(Debug, Clone)]
pub struct UserSession {
    pub user_id: String,
    pub session_id: Uuid,
    pub connected_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub active_resources: Vec<String>,
    pub permissions: HashMap<String, Permission>,
    pub cursor_positions: HashMap<String, CursorPosition>,
}

/// Cursor position for collaborative editing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPosition {
    pub resource_id: String,
    pub position: usize,
    pub selection_start: Option<usize>,
    pub selection_end: Option<usize>,
    pub updated_at: DateTime<Utc>,
}

/// Permission manager for access control
pub struct PermissionManager {
    permissions: HashMap<String, Vec<ResourcePermission>>,
    permission_cache: HashMap<String, Permission>,
}

/// Presence tracker for collaborative sessions
pub struct PresenceTracker {
    active_users: HashMap<String, UserPresence>,
    resource_watchers: HashMap<String, Vec<String>>,
}

/// User presence information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPresence {
    pub user_id: String,
    pub status: PresenceStatus,
    pub last_seen: DateTime<Utc>,
    pub current_resource: Option<String>,
    pub cursor_position: Option<CursorPosition>,
}

/// Presence status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PresenceStatus {
    Online,
    Away,
    Busy,
    Offline,
}

/// Activity log for collaboration events
pub struct ActivityLog {
    events: Vec<ActivityEvent>,
    max_events: usize,
}

/// Activity event for logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEvent {
    pub id: Uuid,
    pub event_type: ActivityEventType,
    pub user_id: String,
    pub resource_id: String,
    pub timestamp: DateTime<Utc>,
    pub details: HashMap<String, String>,
}

/// Types of activity events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivityEventType {
    ResourceShared,
    ResourceUnshared,
    PermissionChanged,
    UserJoined,
    UserLeft,
    EditStarted,
    EditCompleted,
    ConflictDetected,
    ConflictResolved,
    LockAcquired,
    LockReleased,
}

impl CollaborationManager {
    pub fn new() -> CloudSyncResult<Self> {
        Ok(Self {
            shared_resources: HashMap::new(),
            user_sessions: HashMap::new(),
            permission_manager: PermissionManager::new(),
            presence_tracker: PresenceTracker::new(),
            activity_log: ActivityLog::new(),
        })
    }

    /// Share a resource with other users
    pub async fn share_resource(
        &mut self,
        resource_id: String,
        data_type: SyncDataType,
        name: String,
        owner_id: String,
        permissions: Vec<ResourcePermission>,
    ) -> CloudSyncResult<SharedResource> {
        let shared_resource = SharedResource {
            id: resource_id.clone(),
            data_type,
            name,
            owner_id: owner_id.clone(),
            created_at: Utc::now(),
            last_modified: Utc::now(),
            permissions,
            sharing_settings: SharingSettings::default(),
            collaboration_state: CollaborationState::new(),
        };

        self.shared_resources.insert(resource_id.clone(), shared_resource.clone());

        // Log activity
        self.activity_log.log_event(ActivityEvent {
            id: Uuid::new_v4(),
            event_type: ActivityEventType::ResourceShared,
            user_id: owner_id,
            resource_id,
            timestamp: Utc::now(),
            details: HashMap::new(),
        });

        Ok(shared_resource)
    }

    /// Add user to shared resource
    pub async fn add_user_to_resource(
        &mut self,
        resource_id: &str,
        user_id: String,
        permission: Permission,
        granted_by: String,
    ) -> CloudSyncResult<()> {
        if let Some(resource) = self.shared_resources.get_mut(resource_id) {
            let resource_permission = ResourcePermission {
                user_id: user_id.clone(),
                permission,
                granted_at: Utc::now(),
                granted_by: granted_by.clone(),
                expires_at: None,
            };

            resource.permissions.push(resource_permission);
            resource.last_modified = Utc::now();

            // Update permission manager
            self.permission_manager.add_permission(resource_id, user_id.clone(), permission.clone());

            // Log activity
            self.activity_log.log_event(ActivityEvent {
                id: Uuid::new_v4(),
                event_type: ActivityEventType::PermissionChanged,
                user_id: granted_by,
                resource_id: resource_id.to_string(),
                timestamp: Utc::now(),
                details: HashMap::from([
                    ("target_user".to_string(), user_id),
                    ("permission".to_string(), format!("{:?}", permission)),
                ]),
            });

            Ok(())
        } else {
            Err(CloudSyncError::ConflictResolution(
                format!("Resource not found: {}", resource_id)
            ))
        }
    }

    /// Start user collaboration session
    pub async fn start_user_session(
        &mut self,
        user_id: String,
        resource_id: String,
    ) -> CloudSyncResult<Uuid> {
        let session_id = Uuid::new_v4();
        
        // Check permissions
        if !self.permission_manager.has_permission(&resource_id, &user_id, Permission::Read) {
            return Err(CloudSyncError::PermissionDenied(
                "Insufficient permissions to access resource".to_string()
            ));
        }

        let session = UserSession {
            user_id: user_id.clone(),
            session_id,
            connected_at: Utc::now(),
            last_activity: Utc::now(),
            active_resources: vec![resource_id.clone()],
            permissions: HashMap::from([(
                resource_id.clone(),
                self.permission_manager.get_permission(&resource_id, &user_id)
            )]),
            cursor_positions: HashMap::new(),
        };

        self.user_sessions.insert(session_id.to_string(), session);

        // Update presence
        self.presence_tracker.update_user_presence(user_id.clone(), PresenceStatus::Online);

        // Update collaboration state
        if let Some(resource) = self.shared_resources.get_mut(&resource_id) {
            resource.collaboration_state.active_editors.push(user_id.clone());
        }

        // Log activity
        self.activity_log.log_event(ActivityEvent {
            id: Uuid::new_v4(),
            event_type: ActivityEventType::UserJoined,
            user_id,
            resource_id,
            timestamp: Utc::now(),
            details: HashMap::from([("session_id".to_string(), session_id.to_string())]),
        });

        Ok(session_id)
    }

    /// Acquire lock on resource section
    pub async fn acquire_lock(
        &mut self,
        resource_id: &str,
        user_id: &str,
        section_id: &str,
        lock_type: LockType,
        duration_seconds: u64,
    ) -> CloudSyncResult<Uuid> {
        // Check if user has edit permissions
        if !self.permission_manager.has_permission(resource_id, user_id, Permission::Edit) {
            return Err(CloudSyncError::PermissionDenied(
                "Edit permissions required to acquire lock".to_string()
            ));
        }

        // Check for existing locks
        if let Some(resource) = self.shared_resources.get(resource_id) {
            for existing_lock in &resource.collaboration_state.locked_sections {
                if existing_lock.section_id == section_id {
                    match (&existing_lock.lock_type, &lock_type) {
                        (LockType::Exclusive, _) | (_, LockType::Exclusive) => {
                            return Err(CloudSyncError::ConflictResolution(
                                "Section is already exclusively locked".to_string()
                            ));
                        }
                        _ => {} // Allow shared locks
                    }
                }
            }
        }

        let lock_id = Uuid::new_v4();
        let expires_at = Utc::now() + chrono::Duration::seconds(duration_seconds as i64);

        let resource_lock = ResourceLock {
            id: lock_id,
            user_id: user_id.to_string(),
            section_id: section_id.to_string(),
            lock_type,
            acquired_at: Utc::now(),
            expires_at,
            metadata: HashMap::new(),
        };

        // Add lock to resource
        if let Some(resource) = self.shared_resources.get_mut(resource_id) {
            resource.collaboration_state.locked_sections.push(resource_lock);
        }

        // Log activity
        self.activity_log.log_event(ActivityEvent {
            id: Uuid::new_v4(),
            event_type: ActivityEventType::LockAcquired,
            user_id: user_id.to_string(),
            resource_id: resource_id.to_string(),
            timestamp: Utc::now(),
            details: HashMap::from([
                ("lock_id".to_string(), lock_id.to_string()),
                ("section_id".to_string(), section_id.to_string()),
            ]),
        });

        Ok(lock_id)
    }

    /// Release resource lock
    pub async fn release_lock(
        &mut self,
        resource_id: &str,
        lock_id: Uuid,
        user_id: &str,
    ) -> CloudSyncResult<()> {
        if let Some(resource) = self.shared_resources.get_mut(resource_id) {
            // Find and remove the lock
            if let Some(pos) = resource.collaboration_state.locked_sections
                .iter()
                .position(|lock| lock.id == lock_id && lock.user_id == user_id)
            {
                resource.collaboration_state.locked_sections.remove(pos);

                // Log activity
                self.activity_log.log_event(ActivityEvent {
                    id: Uuid::new_v4(),
                    event_type: ActivityEventType::LockReleased,
                    user_id: user_id.to_string(),
                    resource_id: resource_id.to_string(),
                    timestamp: Utc::now(),
                    details: HashMap::from([("lock_id".to_string(), lock_id.to_string())]),
                });

                Ok(())
            } else {
                Err(CloudSyncError::ConflictResolution(
                    "Lock not found or not owned by user".to_string()
                ))
            }
        } else {
            Err(CloudSyncError::ConflictResolution(
                format!("Resource not found: {}", resource_id)
            ))
        }
    }

    /// Update cursor position
    pub async fn update_cursor_position(
        &mut self,
        session_id: &str,
        resource_id: &str,
        position: usize,
        selection: Option<(usize, usize)>,
    ) -> CloudSyncResult<()> {
        if let Some(session) = self.user_sessions.get_mut(session_id) {
            let cursor_position = CursorPosition {
                resource_id: resource_id.to_string(),
                position,
                selection_start: selection.map(|(start, _)| start),
                selection_end: selection.map(|(_, end)| end),
                updated_at: Utc::now(),
            };

            session.cursor_positions.insert(resource_id.to_string(), cursor_position.clone());
            session.last_activity = Utc::now();

            // Update presence tracker
            self.presence_tracker.update_cursor_position(&session.user_id, cursor_position);

            Ok(())
        } else {
            Err(CloudSyncError::ConflictResolution(
                "Session not found".to_string()
            ))
        }
    }

    /// Get active collaborators for resource
    pub fn get_active_collaborators(&self, resource_id: &str) -> Vec<UserPresence> {
        if let Some(resource) = self.shared_resources.get(resource_id) {
            resource.collaboration_state.active_editors
                .iter()
                .filter_map(|user_id| self.presence_tracker.get_user_presence(user_id))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get activity log for resource
    pub fn get_activity_log(&self, resource_id: &str, limit: Option<usize>) -> Vec<ActivityEvent> {
        self.activity_log.get_events_for_resource(resource_id, limit.unwrap_or(100))
    }

    /// Clean up expired locks and sessions
    pub async fn cleanup_expired_items(&mut self) -> CloudSyncResult<()> {
        let now = Utc::now();

        // Clean up expired locks
        for resource in self.shared_resources.values_mut() {
            resource.collaboration_state.locked_sections.retain(|lock| {
                if lock.expires_at > now {
                    true
                } else {
                    // Log lock expiration
                    self.activity_log.log_event(ActivityEvent {
                        id: Uuid::new_v4(),
                        event_type: ActivityEventType::LockReleased,
                        user_id: lock.user_id.clone(),
                        resource_id: resource.id.clone(),
                        timestamp: now,
                        details: HashMap::from([
                            ("lock_id".to_string(), lock.id.to_string()),
                            ("reason".to_string(), "expired".to_string()),
                        ]),
                    });
                    false
                }
            });
        }

        // Clean up inactive sessions
        let inactive_threshold = now - chrono::Duration::minutes(30);
        self.user_sessions.retain(|_, session| {
            session.last_activity > inactive_threshold
        });

        Ok(())
    }
}

impl PermissionManager {
    fn new() -> Self {
        Self {
            permissions: HashMap::new(),
            permission_cache: HashMap::new(),
        }
    }

    fn add_permission(&mut self, resource_id: &str, user_id: String, permission: Permission) {
        let cache_key = format!("{}:{}", resource_id, user_id);
        self.permission_cache.insert(cache_key, permission);
    }

    fn has_permission(&self, resource_id: &str, user_id: &str, required_permission: Permission) -> bool {
        let cache_key = format!("{}:{}", resource_id, user_id);
        if let Some(user_permission) = self.permission_cache.get(&cache_key) {
            Self::permission_allows(user_permission, &required_permission)
        } else {
            false
        }
    }

    fn get_permission(&self, resource_id: &str, user_id: &str) -> Permission {
        let cache_key = format!("{}:{}", resource_id, user_id);
        self.permission_cache.get(&cache_key)
            .cloned()
            .unwrap_or(Permission::Read)
    }

    fn permission_allows(user_permission: &Permission, required_permission: &Permission) -> bool {
        match (user_permission, required_permission) {
            (Permission::Owner, _) => true,
            (Permission::Admin, Permission::Owner) => false,
            (Permission::Admin, _) => true,
            (Permission::Edit, Permission::Admin | Permission::Owner) => false,
            (Permission::Edit, _) => true,
            (Permission::Comment, Permission::Edit | Permission::Admin | Permission::Owner) => false,
            (Permission::Comment, _) => true,
            (Permission::Read, Permission::Read) => true,
            (Permission::Read, _) => false,
        }
    }
}

impl PresenceTracker {
    fn new() -> Self {
        Self {
            active_users: HashMap::new(),
            resource_watchers: HashMap::new(),
        }
    }

    fn update_user_presence(&mut self, user_id: String, status: PresenceStatus) {
        let presence = self.active_users.entry(user_id.clone()).or_insert_with(|| {
            UserPresence {
                user_id: user_id.clone(),
                status: PresenceStatus::Offline,
                last_seen: Utc::now(),
                current_resource: None,
                cursor_position: None,
            }
        });

        presence.status = status;
        presence.last_seen = Utc::now();
    }

    fn update_cursor_position(&mut self, user_id: &str, cursor_position: CursorPosition) {
        if let Some(presence) = self.active_users.get_mut(user_id) {
            presence.cursor_position = Some(cursor_position.clone());
            presence.current_resource = Some(cursor_position.resource_id);
            presence.last_seen = Utc::now();
        }
    }

    fn get_user_presence(&self, user_id: &str) -> Option<UserPresence> {
        self.active_users.get(user_id).cloned()
    }
}

impl ActivityLog {
    fn new() -> Self {
        Self {
            events: Vec::new(),
            max_events: 10000,
        }
    }

    fn log_event(&mut self, event: ActivityEvent) {
        self.events.push(event);
        
        // Keep only recent events
        if self.events.len() > self.max_events {
            self.events.drain(0..1000); // Remove oldest 1000 events
        }
    }

    fn get_events_for_resource(&self, resource_id: &str, limit: usize) -> Vec<ActivityEvent> {
        self.events
            .iter()
            .filter(|event| event.resource_id == resource_id)
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }
}

impl CollaborationState {
    fn new() -> Self {
        Self {
            active_editors: Vec::new(),
            locked_sections: Vec::new(),
            pending_changes: Vec::new(),
            conflict_regions: Vec::new(),
            version: 1,
        }
    }
}

impl Default for SharingSettings {
    fn default() -> Self {
        Self {
            public_sharing_enabled: false,
            link_sharing_enabled: false,
            require_authentication: true,
            allow_downloads: false,
            allow_comments: true,
            auto_approve_requests: false,
            sharing_link: None,
        }
    }
}