# Calendar System Specification

This is the calendar system specification for the spec detailed in @.agent-os/specs/2025-07-25-modern-ui-enhancement/spec.md

> Created: 2025-07-25
> Version: 1.0.0

## Calendar Architecture Overview

The calendar system is designed as a fully integrated component that seamlessly blends with the email interface while providing comprehensive CalDAV functionality. The architecture supports multiple calendar providers, offline capability, and rich integration with email workflows.

### Core Components

1. **CalDAV Client Engine**: RFC 4791 compliant client for calendar synchronization
2. **Calendar UI Framework**: TUI components for calendar visualization and interaction
3. **Event Management System**: CRUD operations for calendar events with conflict resolution
4. **Meeting Integration Layer**: Email-calendar bridge for invitation handling
5. **Synchronization Engine**: Bidirectional sync with conflict detection and resolution

## CalDAV Protocol Implementation

### Supported CalDAV Features

- **Calendar Discovery**: PROPFIND operations for calendar enumeration
- **Event Synchronization**: REPORT queries with sync-token support for incremental updates
- **Event Management**: PUT/DELETE operations for event lifecycle management
- **Timezone Support**: Full VTIMEZONE handling for global calendar compatibility
- **Recurring Events**: RRULE processing with exception handling
- **Free/Busy Queries**: Schedule availability checking for meeting planning

### Provider Compatibility

**Google Calendar**:
- OAuth2 authentication flow
- Google-specific event extensions
- Attachment handling for Google Drive integration
- Video conferencing link detection and display

**Microsoft Outlook/Exchange**:
- Exchange Web Services (EWS) fallback for older systems
- Microsoft Teams meeting integration
- Corporate directory integration for attendee lookup
- Exchange-specific recurrence patterns

**Generic CalDAV**:
- Standards-compliant CalDAV servers (NextCloud, Radicale, etc.)
- Basic authentication and digest authentication
- Custom server configurations and discovery methods

## Calendar UI Components

### Calendar Views

**Month View**:
```
┌─────────────────────────────────────────────────────┐
│ December 2025                              ←    →   │
├─────────────────────────────────────────────────────┤
│ Mon  Tue  Wed  Thu  Fri  Sat  Sun                  │
│  1    2    3    4    5    6    7                   │
│  •         •              •                        │
│  8    9   10   11   12   13   14                   │
│      •    ••         •    •                        │
│ 15   16   17   18   19   20   21                   │
│           •              •                         │
│ 22   23   24   25   26   27   28                   │
│  •              •                                  │
│ 29   30   31                                       │
│                                                    │
├─────────────────────────────────────────────────────┤
│ Selected: Dec 15 - Team Meeting (10:00-11:00)     │
└─────────────────────────────────────────────────────┘
```

**Week View**:
```
┌─────────────────────────────────────────────────────┐
│ Week of Dec 15-21, 2025                   ←    →   │
├────────┬────────┬────────┬────────┬────────┬────────┤
│   Mon  │   Tue  │   Wed  │   Thu  │   Fri  │   Sat  │
│   15   │   16   │   17   │   18   │   19   │   20   │
├────────┼────────┼────────┼────────┼────────┼────────┤
│ 09:00  │        │        │        │        │        │
│ ██████ │        │ ████   │        │        │        │
│ 10:00  │        │ ████   │        │ ██████ │        │
│        │ ██████ │        │        │ ██████ │        │
│ 11:00  │ ██████ │        │        │        │        │
│        │        │        │ ████   │        │        │
│ 12:00  │        │        │ ████   │        │        │
├────────┼────────┼────────┼────────┼────────┼────────┤
│ Team   │ Client │ Standup│        │ Review │        │
│ Meet   │ Call   │        │ Lunch  │        │        │
└────────┴────────┴────────┴────────┴────────┴────────┘
```

**Day View**:
```
┌─────────────────────────────────────────────────────┐
│ Monday, December 15, 2025              ←  Today  → │
├─────────────────────────────────────────────────────┤
│ 08:00 │                                           │
│ 09:00 │ ████████████████████████████████████████  │
│       │ Team Standup                              │
│ 10:00 │ Location: Conference Room A               │
│       │ Attendees: Alice, Bob, Carol              │
│ 11:00 │                                           │
│ 12:00 │ ████████████████████████████████████████  │
│       │ Lunch with Client                         │
│ 13:00 │ ████████████████████████████████████████  │
│       │                                           │
│ 14:00 │                                           │
│ 15:00 │ ████████████████████████████████████████  │
│       │ Code Review Session                       │
│ 16:00 │ ████████████████████████████████████████  │
│       │                                           │
│ 17:00 │                                           │
├─────────────────────────────────────────────────────┤
│ [n]ew event  [e]dit  [d]elete  [r]eply  [v]iew     │
└─────────────────────────────────────────────────────┘
```

### Event Creation and Editing

**Event Form Interface**:
```
┌─────────────────────────────────────────────────────┐
│ Create New Event                                    │
├─────────────────────────────────────────────────────┤
│ Title:      [Team Planning Meeting____________]     │
│ Location:   [Conference Room B________________]     │
│ Calendar:   [Work ▼]                               │
│                                                    │
│ Start Date: [2025-12-15] Time: [14:00]            │
│ End Date:   [2025-12-15] Time: [15:30]            │
│ Timezone:   [America/New_York ▼]                  │
│                                                    │
│ Recurrence: [None ▼]                              │
│ Reminder:   [15 minutes before ▼]                 │
│                                                    │
│ Attendees:                                         │
│ alice@company.com, bob@company.com                 │
│                                                    │
│ Description:                                       │
│ ┌─────────────────────────────────────────────────┐ │
│ │ Quarterly planning session to discuss Q1       │ │
│ │ objectives and resource allocation.             │ │
│ │                                                 │ │
│ └─────────────────────────────────────────────────┘ │
│                                                    │
│ [Save]  [Cancel]  [Save & Send Invites]          │
└─────────────────────────────────────────────────────┘
```

### Meeting Invitation Handling

**Invitation Email Integration**:
```
┌─────────────────────────────────────────────────────┐
│ 📧 Meeting Invitation Detected                     │
├─────────────────────────────────────────────────────┤
│ From: alice@company.com                            │
│ Subject: Team Planning Meeting - Q1 2025          │
│                                                    │
│ 📅 Event Details:                                 │
│ When: Dec 15, 2025 2:00 PM - 3:30 PM             │
│ Where: Conference Room B                           │
│ Attendees: You, Alice, Bob                        │
│                                                    │
│ Description: Quarterly planning session...         │
│                                                    │
│ Your Response: [Not Responded]                     │
│                                                    │
│ Actions:                                           │
│ [Accept]  [Tentative]  [Decline]  [View in Cal]   │
│                                                    │
│ ☐ Add to my calendar                              │
│ ☐ Send response email                             │
└─────────────────────────────────────────────────────┘
```

## Event Management Features

### Event CRUD Operations

**Create Event**:
- Form-based event creation with validation
- Template support for common meeting types
- Conflict detection and alternative time suggestions
- Automatic timezone conversion for multi-timezone teams

**Read/Query Events**:
- Date range queries with pagination
- Full-text search across event content
- Filter by calendar, attendee, location, or status
- Quick views for today, this week, next week

**Update Event**:
- In-place editing with change tracking
- Automatic invitation updates to attendees
- Conflict resolution for scheduling changes
- Version history for event modifications

**Delete Event**:
- Soft delete with recovery options
- Cascade deletion for recurring series
- Automatic cancellation notifications
- Cleanup of related meeting materials

### Recurring Event Support

**Recurrence Rule Processing**:
- Daily, weekly, monthly, yearly patterns
- Complex rules (every 2nd Tuesday, last Friday of month)
- Exclusion dates and modified instances
- Timezone-aware recurrence calculations

**Series Management**:
- Edit single instance vs. entire series
- Exception handling for modified occurrences
- Split series at specific dates
- Import/export of recurring event definitions

### Conflict Detection and Resolution

**Scheduling Conflicts**:
```rust
pub struct ConflictDetector {
    pub fn detect_conflicts(&self, event: &CalendarEvent) -> Vec<ConflictInfo>;
    pub fn suggest_alternatives(&self, event: &CalendarEvent, preferences: &SchedulingPrefs) -> Vec<TimeSlot>;
    pub fn find_free_time(&self, attendees: &[String], duration: Duration, constraints: &TimeConstraints) -> Option<TimeSlot>;
}

pub struct ConflictInfo {
    pub conflicting_event: CalendarEvent,
    pub overlap_duration: Duration,
    pub conflict_type: ConflictType, // Hard, Soft, Tentative
    pub resolution_suggestions: Vec<ConflictResolution>,
}
```

## Email-Calendar Integration

### Meeting Invitation Processing

**Invitation Detection**:
- Parse iCalendar attachments from emails
- Detect meeting requests in email body text
- Extract scheduling information from common formats
- Support for various email client invitation styles

**RSVP Workflow**:
1. **Detection**: Automatic recognition of meeting invitations
2. **Parsing**: Extract event details and attendee information
3. **Presentation**: Display invitation in both email and calendar contexts
4. **Response**: Integrated RSVP with calendar updates
5. **Confirmation**: Email response and calendar synchronization

**Event Creation from Email**:
```rust
pub struct EmailToCalendarConverter {
    pub fn extract_scheduling_info(&self, email: &Email) -> Option<SchedulingInfo>;
    pub fn create_event_from_email(&self, email: &Email, details: EventCreationDetails) -> Result<CalendarEvent>;
    pub fn suggest_event_details(&self, email: &Email) -> EventSuggestions;
}

pub struct SchedulingInfo {
    pub suggested_title: Option<String>,
    pub participants: Vec<String>,
    pub proposed_times: Vec<TimeSlot>,
    pub location_hints: Vec<String>,
    pub context: String,
}
```

### Calendar-Aware Email Features

**Smart Email Scheduling**:
- Show recipient availability when composing emails
- Suggest meeting times based on mutual free slots
- Timezone-aware scheduling for global teams
- Integration with "schedule send" functionality

**Context Integration**:
- Display upcoming meetings in email sidebar
- Link emails to related calendar events
- Show meeting preparation materials in calendar
- Archive email threads by meeting/project

## Synchronization and Offline Support

### Sync Strategy

**Bidirectional Synchronization**:
1. **Initial Sync**: Full calendar download with sync tokens
2. **Incremental Sync**: Delta updates using CalDAV sync-collection
3. **Conflict Resolution**: Three-way merge with user preferences
4. **Offline Changes**: Queue modifications for later sync

**Sync Conflict Resolution**:
```rust
pub enum SyncConflictResolution {
    PreferLocal,
    PreferRemote,
    PreferNewest,
    ManualResolve(ConflictDetails),
}

pub struct ConflictDetails {
    pub local_event: CalendarEvent,
    pub remote_event: CalendarEvent,
    pub last_sync: DateTime<Utc>,
    pub resolution_options: Vec<MergeOption>,
}
```

### Offline Capabilities

**Local Storage**:
- Complete calendar data cached locally
- Offline event creation and modification
- Search and view functionality without network
- Conflict queue for sync resolution

**Network Recovery**:
- Automatic retry with exponential backoff
- Batch operations for efficient sync
- Connection state awareness and user feedback
- Graceful degradation for limited connectivity

## Multi-Calendar Management

### Calendar Organization

**Calendar Types**:
- Personal calendars (local and synced)
- Shared calendars with different permission levels
- Read-only calendars (holidays, company events)
- Subscription calendars (RSS, ICS feeds)

**Calendar Display**:
```
┌─────────────────────────────────────────────────────┐
│ Calendars                                          │
├─────────────────────────────────────────────────────┤
│ ☑ Personal        (12 events) 🔵                  │
│ ☑ Work            (8 events)  🔴                   │
│ ☐ Holidays        (3 events)  🟡                   │
│ ☑ Team Shared     (5 events)  🟢                   │
│ ☐ Birthdays       (2 events)  🟣                   │
│                                                    │
│ [+] Add Calendar                                   │
│ [⚙] Manage Calendars                              │
└─────────────────────────────────────────────────────┘
```

### Permission Management

**Access Levels**:
- **Owner**: Full read/write access, can share and manage permissions
- **Editor**: Can create, modify, and delete events
- **Contributor**: Can create events, modify own events
- **Reader**: View-only access to events
- **Free/Busy**: Can see availability but not event details

## Search and Filtering

### Calendar Search Features

**Search Capabilities**:
- Full-text search across event titles, descriptions, and locations
- Date range filtering with natural language support
- Attendee-based filtering and grouping
- Calendar-specific search with boolean operators
- Saved searches and quick filters

**Search Interface**:
```
┌─────────────────────────────────────────────────────┐
│ Search: [team meeting location:conference room___] │
├─────────────────────────────────────────────────────┤
│ Filters: [This Month ▼] [All Calendars ▼]         │
│                                                    │
│ Results (8 found):                                 │
│                                                    │
│ 📅 Dec 15 - Team Planning Meeting                 │
│    2:00 PM, Conference Room B                     │
│    Work Calendar                                   │
│                                                    │
│ 📅 Dec 10 - Team Standup                         │
│    9:00 AM, Conference Room A                     │
│    Work Calendar                                   │
│                                                    │
│ 📅 Dec 8 - All Hands Meeting                     │
│    10:00 AM, Main Conference Room                 │
│    Company Calendar                                │
│                                                    │
│ [Enter] to view  [/] to refine search            │
└─────────────────────────────────────────────────────┘
```

## Performance and Scalability

### Optimization Strategies

**Data Management**:
- Lazy loading of event details
- Intelligent caching with TTL expiration
- Database indexing for date range queries
- Pagination for large calendar views

**UI Performance**:
- Virtual scrolling for large event lists
- Efficient rendering for calendar grids
- Incremental updates to minimize redraws
- Responsive layout calculations

**Sync Optimization**:
- Batch API requests to reduce round trips
- Compressed data transfer where supported
- Smart sync scheduling based on user activity
- Background sync with rate limiting

This calendar specification provides a comprehensive framework for integrating full calendar functionality into Comunicado while maintaining the terminal-focused user experience and professional interface standards established in the core product vision.