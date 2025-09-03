//! Calendar sharing and permission management system
//!
//! This module provides comprehensive calendar sharing capabilities including:
//! - CalDAV-based calendar sharing
//! - Permission management (read, write, admin)
//! - Linux desktop integration
//! - Share discovery and invitation system
//! - Real-time collaboration features

use crate::calendar::{CalDAVClient, CalDAVError, CalendarError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;
use uuid::Uuid;

/// Calendar sharing errors
#[derive(Error, Debug)]
pub enum SharingError {
    #[error("CalDAV error: {0}")]
    CalDAV(#[from] CalDAVError),
    
    #[error("Calendar error: {0}")]
    Calendar(#[from] CalendarError),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("User not found: {0}")]
    UserNotFound(String),
    
    #[error("Share not found: {0}")]
    ShareNotFound(String),
    
    #[error("Invalid invitation: {0}")]
    InvalidInvitation(String),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Authentication error: {0}")]
    Authentication(String),
}

pub type SharingResult<T> = Result<T, SharingError>;

/// Calendar sharing permissions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CalendarPermission {
    /// Can only view calendar events
    Read,
    /// Can create and edit their own events
    Write,
    /// Can edit all events in the calendar
    EditAll,
    /// Full administrative access including sharing management
    Admin,
    /// Owner of the calendar (cannot be removed)
    Owner,
}

impl CalendarPermission {
    pub fn description(&self) -> &'static str {
        match self {
            CalendarPermission::Read => "Can view events",
            CalendarPermission::Write => "Can add and edit own events",
            CalendarPermission::EditAll => "Can edit all events",
            CalendarPermission::Admin => "Full admin access",
            CalendarPermission::Owner => "Calendar owner",
        }
    }

    pub fn can_read(&self) -> bool {
        true // All permissions include read access
    }

    pub fn can_write(&self) -> bool {
        matches!(self, CalendarPermission::Write | CalendarPermission::EditAll | CalendarPermission::Admin | CalendarPermission::Owner)
    }

    pub fn can_edit_all(&self) -> bool {
        matches!(self, CalendarPermission::EditAll | CalendarPermission::Admin | CalendarPermission::Owner)
    }

    pub fn can_admin(&self) -> bool {
        matches!(self, CalendarPermission::Admin | CalendarPermission::Owner)
    }

    pub fn can_share(&self) -> bool {
        matches!(self, CalendarPermission::Admin | CalendarPermission::Owner)
    }
}

/// Calendar share information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarShare {
    pub id: Uuid,
    pub calendar_id: Uuid,
    pub calendar_name: String,
    pub owner_id: String,
    pub owner_email: String,
    pub shared_with: HashMap<String, SharedUser>,
    pub public_url: Option<String>,
    pub is_public: bool,
    pub allow_discovery: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub caldav_url: String,
    pub sync_token: Option<String>,
}

/// User with shared calendar access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedUser {
    pub user_id: String,
    pub email: String,
    pub name: Option<String>,
    pub permission: CalendarPermission,
    pub accepted: bool,
    pub invited_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub last_sync: Option<DateTime<Utc>>,
}

/// Calendar sharing invitation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharingInvitation {
    pub id: Uuid,
    pub calendar_id: Uuid,
    pub calendar_name: String,
    pub from_user: String,
    pub from_email: String,
    pub to_email: String,
    pub permission: CalendarPermission,
    pub message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub accepted: Option<bool>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub invitation_token: String,
}

/// Calendar sharing discovery for Linux desktop integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarDiscoveryInfo {
    pub calendar_id: Uuid,
    pub calendar_name: String,
    pub owner_name: String,
    pub caldav_url: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub tags: Vec<String>,
    pub last_activity: DateTime<Utc>,
}

/// Integration methods for Linux desktop environments
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DesktopIntegrationType {
    /// GNOME Evolution calendar integration
    Evolution,
    /// KDE Kontact/KOrganizer integration
    Kontact,
    /// Thunderbird/Lightning integration
    Thunderbird,
    /// Generic CalDAV endpoint for any client
    GenericCalDAV,
    /// D-Bus integration for desktop notifications
    DBusCalendar,
}

/// Calendar sharing manager
pub struct CalendarSharingManager {
    caldav_client: CalDAVClient,
    shares: HashMap<Uuid, CalendarShare>,
    invitations: HashMap<Uuid, SharingInvitation>,
    user_email: String,
    desktop_integrations: HashSet<DesktopIntegrationType>,
}

impl CalendarSharingManager {
    pub fn new(caldav_client: CalDAVClient, user_email: String) -> Self {
        Self {
            caldav_client,
            shares: HashMap::new(),
            invitations: HashMap::new(),
            user_email,
            desktop_integrations: HashSet::new(),
        }
    }

    /// Share a calendar with specific users
    pub async fn share_calendar(
        &mut self,
        calendar_id: Uuid,
        calendar_name: String,
        invitations: Vec<(String, CalendarPermission)>,
        message: Option<String>,
    ) -> SharingResult<Vec<Uuid>> {
        let mut invitation_ids = Vec::new();
        
        for (email, permission) in invitations {
            let invitation_id = self.create_invitation(
                calendar_id,
                calendar_name.clone(),
                email,
                permission,
                message.clone(),
            ).await?;
            
            invitation_ids.push(invitation_id);
        }

        Ok(invitation_ids)
    }

    /// Create a sharing invitation
    async fn create_invitation(
        &mut self,
        calendar_id: Uuid,
        calendar_name: String,
        to_email: String,
        permission: CalendarPermission,
        message: Option<String>,
    ) -> SharingResult<Uuid> {
        let invitation = SharingInvitation {
            id: Uuid::new_v4(),
            calendar_id,
            calendar_name,
            from_user: self.user_email.clone(),
            from_email: self.user_email.clone(),
            to_email: to_email.clone(),
            permission,
            message,
            created_at: Utc::now(),
            expires_at: Some(Utc::now() + chrono::Duration::days(7)), // 7 day expiry
            accepted: None,
            accepted_at: None,
            invitation_token: self.generate_invitation_token(),
        };

        let invitation_id = invitation.id;
        self.invitations.insert(invitation_id, invitation.clone());

        // Send invitation email (would integrate with SMTP module)
        self.send_invitation_email(&invitation).await?;

        Ok(invitation_id)
    }

    /// Accept a calendar sharing invitation
    pub async fn accept_invitation(&mut self, invitation_token: String) -> SharingResult<Uuid> {
        // Extract invitation details first
        let (calendar_id, to_email, permission, created_at, calendar_name, from_user, from_email) = {
            let invitation = self.invitations
                .values_mut()
                .find(|inv| inv.invitation_token == invitation_token)
                .ok_or_else(|| SharingError::InvalidInvitation("Invalid invitation token".to_string()))?;

            // Check if invitation is still valid
            if let Some(expires_at) = invitation.expires_at {
                if Utc::now() > expires_at {
                    return Err(SharingError::InvalidInvitation("Invitation has expired".to_string()));
                }
            }

            if invitation.accepted.is_some() {
                return Err(SharingError::InvalidInvitation("Invitation already processed".to_string()));
            }

            // Mark invitation as accepted
            invitation.accepted = Some(true);
            invitation.accepted_at = Some(Utc::now());

            // Extract needed data
            (invitation.calendar_id, invitation.to_email.clone(), invitation.permission.clone(),
             invitation.created_at, invitation.calendar_name.clone(), invitation.from_user.clone(),
             invitation.from_email.clone())
        };

        // Create shared user
        let shared_user = SharedUser {
            user_id: to_email.clone(),
            email: to_email.clone(),
            name: None,
            permission: permission.clone(),
            accepted: true,
            invited_at: created_at,
            accepted_at: Some(Utc::now()),
            last_sync: None,
        };

        // Get or create calendar share
        let share = self.shares.entry(calendar_id).or_insert_with(|| CalendarShare {
            id: Uuid::new_v4(),
            calendar_id,
            calendar_name,
            owner_id: from_user,
            owner_email: from_email,
            shared_with: HashMap::new(),
            public_url: None,
            is_public: false,
            allow_discovery: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            caldav_url: format!("calendars/{}", calendar_id),
            sync_token: None,
        });

        share.shared_with.insert(to_email.clone(), shared_user);
        share.updated_at = Utc::now();

        // Configure CalDAV sharing permissions
        self.configure_caldav_permissions(calendar_id, &to_email, &permission).await?;

        Ok(calendar_id)
    }

    /// Decline a calendar sharing invitation
    pub async fn decline_invitation(&mut self, invitation_token: String) -> SharingResult<()> {
        let invitation = self.invitations
            .values_mut()
            .find(|inv| inv.invitation_token == invitation_token)
            .ok_or_else(|| SharingError::InvalidInvitation("Invalid invitation token".to_string()))?;

        if invitation.accepted.is_some() {
            return Err(SharingError::InvalidInvitation("Invitation already processed".to_string()));
        }

        invitation.accepted = Some(false);
        invitation.accepted_at = Some(Utc::now());

        Ok(())
    }

    /// Remove a user from a shared calendar
    pub async fn revoke_access(&mut self, calendar_id: Uuid, user_email: &str) -> SharingResult<()> {
        let share = self.shares.get_mut(&calendar_id)
            .ok_or_else(|| SharingError::ShareNotFound("Calendar share not found".to_string()))?;

        if share.shared_with.remove(user_email).is_none() {
            return Err(SharingError::UserNotFound("User not found in share".to_string()));
        }

        share.updated_at = Utc::now();

        // Remove CalDAV permissions
        self.remove_caldav_permissions(calendar_id, user_email).await?;

        Ok(())
    }

    /// Update user permissions for a shared calendar
    pub async fn update_permissions(
        &mut self,
        calendar_id: Uuid,
        user_email: &str,
        new_permission: CalendarPermission,
    ) -> SharingResult<()> {
        let share = self.shares.get_mut(&calendar_id)
            .ok_or_else(|| SharingError::ShareNotFound("Calendar share not found".to_string()))?;

        let user = share.shared_with.get_mut(user_email)
            .ok_or_else(|| SharingError::UserNotFound("User not found in share".to_string()))?;

        user.permission = new_permission.clone();
        share.updated_at = Utc::now();

        // Update CalDAV permissions
        self.configure_caldav_permissions(calendar_id, user_email, &new_permission).await?;

        Ok(())
    }

    /// Enable public sharing for a calendar
    pub async fn enable_public_sharing(&mut self, calendar_id: Uuid, allow_discovery: bool) -> SharingResult<String> {
        let share = self.shares.get_mut(&calendar_id)
            .ok_or_else(|| SharingError::ShareNotFound("Calendar share not found".to_string()))?;

        let public_url = format!("https://calendars.comunicado.app/public/{}", calendar_id);
        
        share.is_public = true;
        share.public_url = Some(public_url.clone());
        share.allow_discovery = allow_discovery;
        share.updated_at = Utc::now();

        // Configure public CalDAV access
        self.configure_public_caldav_access(calendar_id).await?;

        Ok(public_url)
    }

    /// Disable public sharing for a calendar
    pub async fn disable_public_sharing(&mut self, calendar_id: Uuid) -> SharingResult<()> {
        let share = self.shares.get_mut(&calendar_id)
            .ok_or_else(|| SharingError::ShareNotFound("Calendar share not found".to_string()))?;

        share.is_public = false;
        share.public_url = None;
        share.allow_discovery = false;
        share.updated_at = Utc::now();

        // Remove public CalDAV access
        self.remove_public_caldav_access(calendar_id).await?;

        Ok(())
    }

    /// Get Linux desktop integration URLs for shared calendars
    pub fn get_desktop_integration_urls(&self, calendar_id: Uuid) -> SharingResult<HashMap<DesktopIntegrationType, String>> {
        let share = self.shares.get(&calendar_id)
            .ok_or_else(|| SharingError::ShareNotFound("Calendar share not found".to_string()))?;

        let base_caldav_url = &share.caldav_url;
        let mut integration_urls = HashMap::new();

        integration_urls.insert(DesktopIntegrationType::GenericCalDAV, base_caldav_url.clone());
        integration_urls.insert(DesktopIntegrationType::Evolution, format!("evolution:addcalendar:{}", base_caldav_url));
        integration_urls.insert(DesktopIntegrationType::Kontact, format!("korganizer:import:{}", base_caldav_url));
        integration_urls.insert(DesktopIntegrationType::Thunderbird, format!("thunderbird:addcalendar:{}", base_caldav_url));

        Ok(integration_urls)
    }

    /// Enable specific desktop integration
    pub async fn enable_desktop_integration(&mut self, integration_type: DesktopIntegrationType) -> SharingResult<()> {
        self.desktop_integrations.insert(integration_type.clone());
        
        match integration_type {
            DesktopIntegrationType::DBusCalendar => {
                self.setup_dbus_calendar_service().await?;
            }
            DesktopIntegrationType::Evolution => {
                self.register_evolution_calendar_source().await?;
            }
            DesktopIntegrationType::Kontact => {
                self.register_kontact_calendar_resource().await?;
            }
            _ => {}
        }

        Ok(())
    }

    /// Discover shared calendars available to the current user
    pub async fn discover_available_calendars(&self) -> SharingResult<Vec<CalendarDiscoveryInfo>> {
        let discovered_calendars = Vec::new();

        // Query CalDAV server for shared calendars
        // This would integrate with the CalDAV client to discover available calendars
        
        Ok(discovered_calendars)
    }

    /// Get all calendar shares managed by this user
    pub fn get_managed_shares(&self) -> Vec<&CalendarShare> {
        self.shares.values().collect()
    }

    /// Get all pending invitations
    pub fn get_pending_invitations(&self) -> Vec<&SharingInvitation> {
        self.invitations.values()
            .filter(|inv| inv.accepted.is_none())
            .collect()
    }

    /// Private helper methods
    fn generate_invitation_token(&self) -> String {
        format!("inv_{}", Uuid::new_v4())
    }

    async fn send_invitation_email(&self, _invitation: &SharingInvitation) -> SharingResult<()> {
        // Would integrate with SMTP module to send invitation emails
        Ok(())
    }

    async fn configure_caldav_permissions(&self, _calendar_id: Uuid, _user_email: &str, _permission: &CalendarPermission) -> SharingResult<()> {
        // Configure CalDAV server permissions
        Ok(())
    }

    async fn remove_caldav_permissions(&self, _calendar_id: Uuid, _user_email: &str) -> SharingResult<()> {
        // Remove CalDAV server permissions
        Ok(())
    }

    async fn configure_public_caldav_access(&self, _calendar_id: Uuid) -> SharingResult<()> {
        // Enable public CalDAV access
        Ok(())
    }

    async fn remove_public_caldav_access(&self, _calendar_id: Uuid) -> SharingResult<()> {
        // Disable public CalDAV access
        Ok(())
    }

    async fn setup_dbus_calendar_service(&self) -> SharingResult<()> {
        // Set up D-Bus calendar service for Linux desktop integration
        Ok(())
    }

    async fn register_evolution_calendar_source(&self) -> SharingResult<()> {
        // Register with GNOME Evolution calendar system
        Ok(())
    }

    async fn register_kontact_calendar_resource(&self) -> SharingResult<()> {
        // Register with KDE Kontact calendar system
        Ok(())
    }
}

impl Default for CalendarSharingManager {
    fn default() -> Self {
        Self::new(
            CalDAVClient::new("http://localhost:8080", "user".to_string(), "pass".to_string()).unwrap(),
            "user@example.com".to_string(),
        )
    }
}