# Product Roadmap

> Last Updated: 2025-07-25
> Version: 1.0.0
> Status: Planning

## Phase 1: Core Email Client (4-6 weeks)

**Goal:** Establish basic email functionality with modern TUI interface
**Success Criteria:** Users can connect to IMAP accounts, read/send emails, and navigate with keyboard shortcuts

### Must-Have Features

- [x] Basic TUI Interface with Ratatui - Create main application window with folder tree and message list `L`
- [x] IMAP Connection and Authentication - Implement basic IMAP client with login support `L`
- [x] Email List and Reading - Display email list and allow reading individual messages `M`
- [x] Basic Email Composition - Create and send simple text emails via SMTP `M`
- [x] Account Configuration - Setup wizard for adding IMAP/SMTP accounts `M`

### Should-Have Features

- [x] Folder Navigation - Browse IMAP folders and manage email organization `S`
- [x] Search Functionality - Basic email search across subject and sender `S`

### Dependencies

- Ratatui TUI framework setup
- Tokio async runtime configuration
- Basic IMAP protocol implementation

## Phase 2: Modern Email Features (3-4 weeks)

**Goal:** Add HTML rendering, OAuth2 support, and multi-account management
**Success Criteria:** Users can view HTML emails with images and manage multiple accounts securely

### Must-Have Features

- [x] HTML Email Rendering - Parse and display HTML emails in terminal-friendly format `L`
- [x] OAuth2 Authentication - Support Gmail, Outlook OAuth2 login flows `L`
- [x] Multi-Account Support - Manage multiple email accounts simultaneously `M`
- [x] Image Display Support - Show images in emails using terminal graphics protocols `L`
- [x] Account Management - Add and remove accounts with Ctrl+A/Ctrl+X shortcuts `M`

### Should-Have Features

- [x] Email Filters - Basic rule-based email filtering and organization `M`
- [x] Attachment Handling - View and save email attachments `S`
- [x] Draft Management - Save and resume email drafts `S`

### Dependencies

- OAuth2 library integration
- HTML parser implementation
- Terminal graphics protocol support

## Phase 3: Calendar Integration Foundation (3-4 weeks)

**Goal:** Implement CalDAV calendar functionality and email-calendar integration
**Success Criteria:** Users can view, create, and manage calendar events with CalDAV sync

### Must-Have Features

- [ ] CalDAV Client Implementation - Connect to CalDAV servers and sync calendars `XL`
- [ ] Calendar Event Display - Show calendar events in TUI interface `M`
- [ ] Event Creation and Editing - Create, modify, and delete calendar events `L`
- [ ] Meeting Invitation Handling - Process calendar invites from emails with RSVP `L`

### Should-Have Features

- [ ] Multiple Calendar Support - Manage calendars from different providers `M`
- [ ] Calendar Search - Search through calendar events and appointments `S`

### Dependencies

- CalDAV protocol implementation
- iCalendar format parsing
- Calendar UI components

## Phase 4: Advanced Features and Polish (4-5 weeks)

**Goal:** Add advanced functionality and improve user experience
**Success Criteria:** Feature-complete application with excellent UX and performance

### Must-Have Features

- [ ] Advanced Search - Full-text search across emails and calendar events `M`
- [ ] Maildir Support - Import/export emails in standard Maildir format `M`
- [ ] Performance Optimization - Optimize loading times and memory usage `L`
- [ ] Animation Support - Display GIFs and basic animations in compatible terminals `M`

### Should-Have Features

- [ ] Email Threading - Group related emails into conversation threads `L`
- [ ] Calendar Views - Multiple calendar view modes (day, week, month) `M`
- [ ] Keyboard Customization - User-configurable keyboard shortcuts `S`
- [ ] Notification System - Desktop notifications for new emails and events `S`

### Dependencies

- Full-text search indexing
- Animation rendering libraries
- Desktop notification integration

## Phase 5: Enterprise and Integration Features (3-4 weeks)

**Goal:** Add enterprise features and desktop environment integration
**Success Criteria:** Production-ready application with enterprise features and seamless Linux integration

### Must-Have Features

- [ ] Calendar Sharing - Share calendar data with other Linux applications via CalDAV `L`
- [ ] Advanced Email Filters - Complex filtering rules with multiple conditions `M`
- [ ] Data Import/Export - Import from Thunderbird, mutt, and other email clients `M`
- [ ] Backup and Sync - Backup user data and sync across multiple devices `L`

### Should-Have Features

- [ ] Plugin Architecture - Support for community-developed plugins and extensions `XL`
- [ ] Email Encryption - GPG integration for email encryption and signing `L`
- [ ] Calendar Sharing UI - Interface for managing shared calendars and permissions `M`
- [ ] Advanced Configuration - Power-user configuration options and scripting `S`

### Dependencies

- GPG library integration  
- Plugin system architecture
- Advanced CalDAV features

## Development Milestones

### Alpha Release (End of Phase 2)
- Basic email functionality complete
- HTML rendering working
- OAuth2 authentication implemented
- Multi-account support available

### Beta Release (End of Phase 3)
- Calendar functionality integrated
- CalDAV synchronization working
- Meeting invitation handling complete
- Community testing and feedback

### 1.0 Release (End of Phase 4)
- All core features complete
- Performance optimized
- Comprehensive documentation
- Package distribution ready

### 1.1 Release (End of Phase 5)
- Enterprise features complete
- Desktop integration finalized
- Plugin architecture available
- Long-term maintenance mode